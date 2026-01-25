//! Trampoline system for intercepting proc macro `run` function pointers.
//!
//! Each proc macro entry in the `__rustc_proc_macro_decls_*` table has a `Client`
//! with a `run` function pointer. We replace these with trampolines that wrap the
//! dispatch closure to capture input/output strings.
//!
//! The `__rustc_proc_macro_decls_*` symbol is a `static &[ProcMacro]` — i.e.,
//! `dlsym` returns a pointer to a fat pointer (data ptr + length). Rustc reads
//! the fat pointer from that address. We intercept by:
//! 1. Reading the original `&[ProcMacro]` from the dlsym result
//! 2. Building a new table with replaced `run` fn ptrs
//! 3. Leaking a new fat pointer
//! 4. Returning a pointer to the leaked fat pointer

use crate::dispatch;
use crate::logging::{self, ExpansionRecord};
use crate::types::{BridgeConfig, Buffer, ProcMacro};
use std::sync::Mutex;

/// Information about an intercepted proc macro slot
struct TrampolineSlot {
    /// Original `run` function pointer
    original_run: extern "C" fn(BridgeConfig<'_>) -> Buffer,
    /// Macro name (from the ProcMacro table)
    name: String,
    /// Macro kind string
    kind: String,
}

static SLOTS: Mutex<Vec<Option<TrampolineSlot>>> = Mutex::new(Vec::new());

// Include build-script-generated trampoline functions and trampoline array.
// (We only use the trampoline fns, not the table return fns.)
include!(concat!(env!("OUT_DIR"), "/trampolines_generated.rs"));

/// Extract TokenStream handle IDs from the BridgeConfig input buffer.
///
/// Input buffer format: [ExpnGlobals: 3 x u32_le span handles] [TokenStream handle(s)]
/// - Bang/CustomDerive: 1 TokenStream handle (u32_le) at offset 12
/// - Attr: 2 TokenStream handles (u32_le each) at offset 12 and 16
fn extract_input_handles(input_buf: &[u8], kind: &str) -> Vec<u32> {
    let mut handles = Vec::new();
    // Skip ExpnGlobals (3 x u32 = 12 bytes)
    let offset = 12;
    if input_buf.len() >= offset + 4 {
        let h = u32::from_le_bytes(input_buf[offset..offset + 4].try_into().unwrap());
        handles.push(h);
    }
    // Attr macros have a second TokenStream handle
    if kind == "Attr" && input_buf.len() >= offset + 8 {
        let h = u32::from_le_bytes(input_buf[offset + 4..offset + 8].try_into().unwrap());
        handles.push(h);
    }
    handles
}

/// The core trampoline implementation called by each generated trampoline function.
///
/// 1. Wraps the dispatch closure with our interceptor
/// 2. Calls to_string on input TokenStream handles to capture input
/// 3. Calls the original `run` function with the modified config
/// 4. Captures from_str calls as output
fn trampoline_impl(idx: usize, config: BridgeConfig<'_>) -> Buffer {
    let (original_run, name, kind) = {
        let slots = SLOTS.lock().unwrap();
        match slots.get(idx).and_then(|s| s.as_ref()) {
            Some(slot) => (slot.original_run, slot.name.clone(), slot.kind.clone()),
            None => {
                eprintln!("[macra-hook] No slot for trampoline index {}", idx);
                return Buffer::from_vec(Vec::new());
            }
        }
    };

    // Extract input handle IDs from the input buffer before it's consumed
    let input_handles = extract_input_handles(config.input.as_slice(), &kind);

    // Install our dispatch interceptor
    let wrapped_dispatch = unsafe { dispatch::install_dispatch_interceptor(&config.dispatch) };

    // Call to_string on input handles to capture input text
    let mut input_strings = Vec::new();
    for &handle in &input_handles {
        if let Some(s) = unsafe { dispatch::call_to_string_on_handle(handle) } {
            input_strings.push(s);
        }
    }

    // Create a new BridgeConfig with the wrapped dispatch
    let wrapped_config = BridgeConfig {
        input: config.input,
        dispatch: wrapped_dispatch,
        force_show_panics: config.force_show_panics,
        _marker: std::marker::PhantomData,
    };

    // Call the original run function
    let result = (original_run)(wrapped_config);

    // Capture the intercepted strings (from_str calls = fallback output)
    let captured = dispatch::take_captured();

    // Try to extract the output handle from the result buffer and call to_string.
    // Result encoding: Result<Option<TokenStream>, PanicMessage>
    //   [0x00 Ok | 0x01 Err] [0x00 Some | 0x01 None] [u32_le handle if Some]
    // Option uses enum order: Some=0x00, None=0x01
    // The dispatch is still valid here because our trampoline hasn't returned
    // to the server yet (the server's stack frame with the dispatch closure is alive).
    let output = {
        let result_data = result.as_slice();
        let output_from_handle = if result_data.len() >= 6
            && result_data[0] == 0x00  // Result::Ok
            && result_data[1] == 0x00
        // Option::Some
        {
            let handle = u32::from_le_bytes(result_data[2..6].try_into().unwrap());
            unsafe { dispatch::call_to_string_on_handle(handle) }
        } else {
            None
        };

        // Use handle-based output if available, otherwise fall back to from_str captures
        output_from_handle.unwrap_or_else(|| captured.from_str_calls.join(""))
    };

    // For Attr macros, the first input handle is the attribute arguments
    // and the second is the annotated item. Split them accordingly.
    let (arguments, input) = if kind == "Attr" && input_strings.len() >= 2 {
        (input_strings[0].clone(), input_strings[1..].join("\n"))
    } else {
        (String::new(), input_strings.join("\n"))
    };

    dispatch::cleanup_dispatch();

    if !input.is_empty() || !output.is_empty() {
        logging::log_expansion(&ExpansionRecord {
            name,
            kind,
            arguments,
            input,
            output,
        });
    }

    result
}

/// Intercept a proc macro table pointed to by a dlsym result.
///
/// `dlsym_result` is a pointer to `static DECLS: &[ProcMacro]`, i.e. a thin
/// pointer to a fat pointer. Rustc will read `*(dlsym_result as *const &[ProcMacro])`.
///
/// We read the original table, build a new one with intercepted `run` fn ptrs,
/// and return a pointer to a leaked fat pointer containing our new table.
///
/// # Safety
/// `dlsym_result` must point to a valid `&'static [ProcMacro]` fat pointer.
pub unsafe fn intercept_proc_macro_table(dlsym_result: *mut libc::c_void) -> *mut libc::c_void {
    // Read the original fat pointer: &'static [ProcMacro]
    let fat_ptr_location = dlsym_result as *const &'static [ProcMacro];
    let original_table: &'static [ProcMacro] = unsafe { fat_ptr_location.read() };

    if original_table.is_empty() {
        return dlsym_result;
    }

    let mut slots = SLOTS.lock().unwrap();

    let base_idx = slots.len();
    let needed = original_table.len();

    if base_idx + needed > NUM_TRAMPOLINES {
        eprintln!(
            "[macra-hook] Too many proc macros ({} + {} > {}), skipping interception",
            base_idx, needed, NUM_TRAMPOLINES
        );
        return dlsym_result;
    }

    // Build new table with replaced run function pointers
    let mut new_table: Vec<ProcMacro> = Vec::with_capacity(original_table.len());

    for (i, pm) in original_table.iter().enumerate() {
        let slot_idx = base_idx + i;
        let trampoline_fn = TRAMPOLINE_FNS[slot_idx];

        // Ensure slots vector is large enough
        while slots.len() <= slot_idx {
            slots.push(None);
        }

        match pm {
            ProcMacro::CustomDerive {
                trait_name,
                attributes,
                client,
            } => {
                slots[slot_idx] = Some(TrampolineSlot {
                    original_run: client.run,
                    name: trait_name.to_string(),
                    kind: "CustomDerive".to_string(),
                });

                new_table.push(ProcMacro::CustomDerive {
                    trait_name,
                    attributes,
                    client: crate::types::Client {
                        handle_counters: client.handle_counters,
                        run: trampoline_fn,
                        _marker: std::marker::PhantomData,
                    },
                });
            }
            ProcMacro::Attr { name, client } => {
                // The Attr client has type Client<(TokenStream, TokenStream), TokenStream>
                // but at the ABI level, `run` has the same signature: fn(BridgeConfig) -> Buffer
                slots[slot_idx] = Some(TrampolineSlot {
                    original_run: unsafe { std::mem::transmute(client.run) },
                    name: name.to_string(),
                    kind: "Attr".to_string(),
                });

                new_table.push(ProcMacro::Attr {
                    name,
                    client: crate::types::Client {
                        handle_counters: client.handle_counters,
                        run: trampoline_fn,
                        _marker: std::marker::PhantomData,
                    },
                });
            }
            ProcMacro::Bang { name, client } => {
                slots[slot_idx] = Some(TrampolineSlot {
                    original_run: client.run,
                    name: name.to_string(),
                    kind: "Bang".to_string(),
                });

                new_table.push(ProcMacro::Bang {
                    name,
                    client: crate::types::Client {
                        handle_counters: client.handle_counters,
                        run: trampoline_fn,
                        _marker: std::marker::PhantomData,
                    },
                });
            }
        }
    }

    // Leak the new table so it lives forever
    let leaked_table: &'static [ProcMacro] = Box::leak(new_table.into_boxed_slice());

    // Leak a new fat pointer (the &[ProcMacro] itself) so rustc can read it
    // Rustc does: *(result as *const &[ProcMacro])
    // So we need to return a pointer to a location that holds our fat pointer.
    let fat_ptr_box: Box<&'static [ProcMacro]> = Box::new(leaked_table);
    let fat_ptr_ptr: *const &'static [ProcMacro] = Box::leak(fat_ptr_box);

    fat_ptr_ptr as *mut libc::c_void
}
