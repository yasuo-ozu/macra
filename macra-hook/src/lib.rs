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
