//! Linux dlsym hook via LD_PRELOAD.
//!
//! We export a `dlsym` function that intercepts lookups for
//! `__rustc_proc_macro_decls_*` symbols, wrapping the returned
//! proc macro table with our trampoline interceptors.

use crate::trampoline;
use std::ffi::CStr;

/// Call the real `dlsym` using `dlvsym(RTLD_NEXT, "dlsym", "GLIBC_2.2.5")`.
///
/// We use `dlvsym` with a specific GLIBC version to avoid recursion:
/// our `dlsym` replaces the default one, so calling `dlsym(RTLD_NEXT, "dlsym")`
/// would recurse. Using `dlvsym` with a version string bypasses our hook.
unsafe fn real_dlsym(handle: *mut libc::c_void, symbol: *const libc::c_char) -> *mut libc::c_void {
    unsafe extern "C" {
        fn dlvsym(
            handle: *mut libc::c_void,
            symbol: *const libc::c_char,
            version: *const libc::c_char,
        ) -> *mut libc::c_void;
    }

    // First get the real dlsym via dlvsym.
    //
    // The version string is glibc's *baseline* for the architecture, and it differs
    // per target: `GLIBC_2.2.5` is x86_64 only. Hard-coding it made `dlvsym` return
    // null everywhere else, and since the failure falls through to returning null for
    // *every* symbol, the hook broke all `dlsym` in the process — rustc could not load
    // any dylib. Try the known baselines and use whichever resolves.
    let dlsym_name = c"dlsym".as_ptr();
    const GLIBC_BASELINES: [&std::ffi::CStr; 7] = [
        c"GLIBC_2.34",   // every arch, glibc >= 2.34 (dlsym moved into libc)
        c"GLIBC_2.2.5",  // x86_64
        c"GLIBC_2.17",   // aarch64, ppc64le
        c"GLIBC_2.27",   // riscv64
        c"GLIBC_2.4",    // arm
        c"GLIBC_2.2",    // s390x
        c"GLIBC_2.0",    // i686
    ];

    let mut real_dlsym_ptr = std::ptr::null_mut();
    for version in GLIBC_BASELINES {
        real_dlsym_ptr = unsafe { dlvsym(libc::RTLD_NEXT, dlsym_name, version.as_ptr()) };
        if !real_dlsym_ptr.is_null() {
            break;
        }
    }

    if real_dlsym_ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Call the real dlsym
    let real_dlsym: unsafe extern "C" fn(
        *mut libc::c_void,
        *const libc::c_char,
    ) -> *mut libc::c_void = unsafe { std::mem::transmute(real_dlsym_ptr) };

    unsafe { real_dlsym(handle, symbol) }
}

/// Hooked `dlsym` — exported as the public dlsym symbol via LD_PRELOAD.
///
/// When rustc calls `dlsym(handle, "__rustc_proc_macro_decls_...")`, we intercept
/// the result and wrap the proc macro table with our trampolines.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlsym(
    handle: *mut libc::c_void,
    symbol: *const libc::c_char,
) -> *mut libc::c_void {
    // Always call through to the real dlsym first
    let result = unsafe { real_dlsym(handle, symbol) };

    if result.is_null() || symbol.is_null() {
        return result;
    }

    // Check if this is a proc macro decls symbol
    let sym_name = unsafe { CStr::from_ptr(symbol) };
    let sym_bytes = sym_name.to_bytes();

    if !sym_bytes.starts_with(b"__rustc_proc_macro_decls_") {
        return result;
    }

    // Only reinterpret the table when cargo-macra has told us which layout this
    // compiler uses. Without the handshake the safe move is to hand rustc its own
    // pointer back untouched: guessing walks the table at the wrong stride and
    // aborts the compiler, which surfaces as an unrelated crate failing to build.
    let Some(abi) = crate::types::selected_abi() else {
        if std::env::var_os("MACRA_HOOK_DEBUG").is_some() {
            eprintln!(
                "[macra-hook] saw decls symbol but MACRA_ABI is {:?}",
                std::env::var("MACRA_ABI")
            );
        }
        return result;
    };

    // result is a pointer to `static DECLS: &[ProcMacro]` (a thin pointer to a fat pointer).
    // Pass it to our interception logic which returns a pointer to a new fat pointer.
    match abi.table {
        cargo_macra::TableLayout::ProcMacroEnum => unsafe {
            trampoline::intercept_proc_macro_table(result)
        },
        cargo_macra::TableLayout::ClientSlice => unsafe {
            trampoline::intercept_client_slice_table(result)
        },
    }
}
