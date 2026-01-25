//! Dispatch closure interception for capturing proc macro input/output strings.
//!
//! The rustc bridge protocol sends RPC requests through a dispatch closure.
//! We wrap this closure to intercept token stream operations and capture
//! the string representations of proc macro inputs and outputs.
//!
//! Protocol format (nightly 1.95, single-byte method tags):
//! - Method tag: u8 enum discriminant
//! - Handle: u32_le
//! - String encoding: usize_le length + UTF-8 bytes

use crate::types::{Buffer, Closure};
use std::cell::RefCell;

/// Captured data from a proc macro invocation
#[derive(Debug, Default, Clone)]
pub struct CapturedStrings {
    /// Strings passed to `ts_from_str` (proc macro creating output tokens from string)
    pub from_str_calls: Vec<String>,
    /// Strings from `ts_to_string` calls (if the proc macro calls it explicitly)
    pub to_string_results: Vec<String>,
}

thread_local! {
    /// The original dispatch closure for the current invocation
    static ORIGINAL_DISPATCH: RefCell<Option<OriginalDispatch>> = const { RefCell::new(None) };

    /// Captured strings from the current invocation
    static CAPTURED: RefCell<CapturedStrings> = RefCell::new(CapturedStrings::default());
}

struct OriginalDispatch {
    call: unsafe extern "C" fn(*mut u8, Buffer) -> Buffer,
    env: *mut u8,
}

// Safety: The dispatch closure is only used within a single thread during a proc macro invocation.
unsafe impl Send for OriginalDispatch {}

/// Bridge API method tags (from `with_api!` macro order in mod.rs)
const TAG_TS_FROM_STR: u8 = 0x09;
const TAG_TS_TO_STRING: u8 = 0x0A;

/// Read a usize in little-endian from a byte slice at the given offset.
/// On 64-bit systems, usize is 8 bytes.
fn read_usize_le(data: &[u8], offset: usize) -> Option<usize> {
    let size = std::mem::size_of::<usize>();
    if offset + size > data.len() {
        return None;
    }
    let mut bytes = [0u8; 8];
    bytes[..size].copy_from_slice(&data[offset..offset + size]);
    Some(usize::from_le_bytes(bytes))
}

/// Extract a length-prefixed UTF-8 string from a buffer at the given offset.
/// Format: usize_le length + utf8 bytes
fn extract_string(data: &[u8], offset: usize) -> Option<String> {
    let len = read_usize_le(data, offset)?;
    let size = std::mem::size_of::<usize>();
    let start = offset + size;
    if len > 10_000_000 || start + len > data.len() {
        return None;
    }
    String::from_utf8(data[start..start + len].to_vec()).ok()
}

/// The wrapped dispatch function that intercepts bridge RPC calls
unsafe extern "C" fn wrapped_dispatch(env: *mut u8, request: Buffer) -> Buffer {
    let request_data = request.as_slice().to_vec();
    let tag = request_data.first().copied();

    // Forward to original dispatch
    let response = ORIGINAL_DISPATCH.with(|orig| {
        let orig = orig.borrow();
        let orig = orig.as_ref().expect("original dispatch not set");
        unsafe { (orig.call)(orig.env, request) }
    });

    // POST-FORWARD: Check the method tag and capture strings
    if let Some(tag) = tag {
        if tag == TAG_TS_FROM_STR {
            // ts_from_str: request = [0x09] [usize_le len] [UTF-8 string]
            if let Some(s) = extract_string(&request_data, 1) {
                CAPTURED.with(|c| {
                    c.borrow_mut().from_str_calls.push(s);
                });
            }
        } else if tag == TAG_TS_TO_STRING {
            // ts_to_string: response = [0x00 Ok] [usize_le len] [UTF-8 string]
            let response_data = response.as_slice();
            if response_data.first() == Some(&0x00) {
                if let Some(s) = extract_string(response_data, 1) {
                    CAPTURED.with(|c| {
                        c.borrow_mut().to_string_results.push(s);
                    });
                }
            }
        }
    }

    let _ = env;
    response
}

/// Install the dispatch interceptor, replacing the closure in the BridgeConfig.
///
/// Returns a new Closure that wraps the original dispatch.
///
/// # Safety
/// The original closure must be valid for the lifetime of the returned closure.
pub unsafe fn install_dispatch_interceptor<'a>(
    original: &Closure<'a, Buffer, Buffer>,
) -> Closure<'a, Buffer, Buffer> {
    // Store the original dispatch in thread-local
    ORIGINAL_DISPATCH.with(|orig| {
        *orig.borrow_mut() = Some(OriginalDispatch {
            call: original.call,
            env: original.env,
        });
    });

    // Clear any previously captured strings
    CAPTURED.with(|c| {
        *c.borrow_mut() = CapturedStrings::default();
    });

    Closure {
        call: wrapped_dispatch,
        env: std::ptr::null_mut(),
        _marker: std::marker::PhantomData,
    }
}

/// Call to_string on a TokenStream handle through the original dispatch.
///
/// Must be called while the dispatch interceptor is installed (between
/// install_dispatch_interceptor and cleanup_dispatch).
///
/// # Safety
/// The dispatch closure's env pointer must still be valid.
pub unsafe fn call_to_string_on_handle(handle: u32) -> Option<String> {
    let mut request = vec![TAG_TS_TO_STRING];
    request.extend_from_slice(&handle.to_le_bytes());
    let buf = Buffer::from_vec(request);

    let response = ORIGINAL_DISPATCH.with(|orig| {
        let orig = orig.borrow();
        let orig = orig.as_ref()?;
        Some(unsafe { (orig.call)(orig.env, buf) })
    })?;

    let resp_data = response.as_slice();
    // Response format: [0x00 Ok marker] [usize_le len] [UTF-8 string]
    if resp_data.first() == Some(&0x00) {
        extract_string(resp_data, 1)
    } else {
        None
    }
}

/// Take the captured strings from the current invocation, resetting the state.
pub fn take_captured() -> CapturedStrings {
    CAPTURED.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// Clean up the original dispatch reference
pub fn cleanup_dispatch() {
    ORIGINAL_DISPATCH.with(|orig| {
        *orig.borrow_mut() = None;
    });
}
