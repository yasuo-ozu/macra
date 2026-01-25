//! Windows hook stub.
//!
//! Windows support requires hooking `GetProcAddress` via inline hooking
//! (e.g., the `retour` crate) and injecting the DLL through a RUSTC_WRAPPER
//! helper. This is a stub for v1 — not yet implemented.

/// Placeholder initialization for Windows.
///
/// TODO: Implement using `retour` crate for inline hooking of `GetProcAddress`.
/// The hook would:
/// 1. Intercept `GetProcAddress` calls
/// 2. Check for `__rustc_proc_macro_decls_` prefix
/// 3. Wrap the returned table via `trampoline::intercept_proc_macro_table`
///
/// Loading would be done via a RUSTC_WRAPPER that injects the DLL using
/// `LoadLibrary` before rustc starts loading proc macros.
pub fn init() {
    eprintln!("[macra-hook] Windows support is not yet implemented");
}
