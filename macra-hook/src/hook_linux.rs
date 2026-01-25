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
unsafe fn real_dlsym(
    handle: *mut libc::c_void,
    symbol: *const libc::c_char,
) -> *mut libc::c_void {
    unsafe extern "C" {
        fn dlvsym(
            handle: *mut libc::c_void,
            symbol: *const libc::c_char,
            version: *const libc::c_char,
        ) -> *mut libc::c_void;
    }

    // First get the real dlsym via dlvsym
    let dlsym_name = b"dlsym\0".as_ptr() as *const libc::c_char;
    let glibc_version = b"GLIBC_2.2.5\0".as_ptr() as *const libc::c_char;

    let real_dlsym_ptr = unsafe {
        dlvsym(libc::RTLD_NEXT as *mut libc::c_void, dlsym_name, glibc_version)
    };

    if real_dlsym_ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Call the real dlsym
    let real_dlsym: unsafe extern "C" fn(*mut libc::c_void, *const libc::c_char) -> *mut libc::c_void =
        unsafe { std::mem::transmute(real_dlsym_ptr) };

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

    // result is a pointer to `static DECLS: &[ProcMacro]` (a thin pointer to a fat pointer).
    // Pass it to our interception logic which returns a pointer to a new fat pointer.
    unsafe { trampoline::intercept_proc_macro_table(result) }
}
