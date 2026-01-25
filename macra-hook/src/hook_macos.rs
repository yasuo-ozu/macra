//! macOS hook via DYLD_INSERT_LIBRARIES + DYLD_INTERPOSE.
//!
//! On macOS, we use the `__DATA,__interpose` section to tell dyld
//! to replace `dlsym` with our hooked version.

use crate::trampoline;
use std::ffi::CStr;

unsafe extern "C" {
    /// The real dlsym from libdl. We reference it so DYLD_INTERPOSE can redirect.
    fn dlsym(
        handle: *mut libc::c_void,
        symbol: *const libc::c_char,
    ) -> *mut libc::c_void;
}

/// Our hooked dlsym implementation.
unsafe extern "C" fn hooked_dlsym(
    handle: *mut libc::c_void,
    symbol: *const libc::c_char,
) -> *mut libc::c_void {
    // Call the real dlsym
    let result = unsafe { dlsym(handle, symbol) };

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
    unsafe { trampoline::intercept_proc_macro_table(result) }
}

/// DYLD_INTERPOSE structure for macOS dyld interposition.
///
/// This tells dyld to replace calls to `dlsym` with `hooked_dlsym`.
/// The struct must be placed in the `__DATA,__interpose` section.
#[repr(C)]
struct DyldInterpose {
    replacement: unsafe extern "C" fn(*mut libc::c_void, *const libc::c_char) -> *mut libc::c_void,
    original: unsafe extern "C" fn(*mut libc::c_void, *const libc::c_char) -> *mut libc::c_void,
}

// Safety: DyldInterpose only contains function pointers.
unsafe impl Sync for DyldInterpose {}

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE: DyldInterpose = DyldInterpose {
    replacement: hooked_dlsym,
    original: dlsym,
};
