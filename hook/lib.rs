#![allow(improper_ctypes_definitions)]

//! macra-hook: LD_PRELOAD/DYLD_INSERT_LIBRARIES library that intercepts
//! proc macro loading by rustc to capture macro inputs and outputs.
//!
//! When loaded via LD_PRELOAD (Linux) or DYLD_INSERT_LIBRARIES (macOS),
//! this library hooks `dlsym` to intercept `__rustc_proc_macro_decls_*`
//! symbol lookups. The returned proc macro tables are wrapped with
//! trampolines that capture the string representations of inputs/outputs
//! and log them to stderr with a `__MACRA_HOOK__:` prefix.

mod dispatch;
mod logging;
mod trampoline;
pub mod types;

#[cfg(target_os = "linux")]
mod hook_linux;

#[cfg(target_os = "macos")]
mod hook_macos;

#[cfg(target_os = "windows")]
mod hook_windows;

/// DllMain entry point for Windows DLL injection.
///
/// When this DLL is loaded into a rustc process by the RUSTC_WRAPPER,
/// DllMain calls `hook_windows::install_hook()` to patch the IAT.
#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _dll_module: *mut u8,
    call_reason: u32,
    _reserved: *mut u8,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if call_reason == DLL_PROCESS_ATTACH {
        hook_windows::install_hook();
    }
    1 // TRUE
}
