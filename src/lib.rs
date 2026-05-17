use std::path::{Path, PathBuf};

pub mod parse_normal;
pub mod parse_trace;
pub mod trace_macros;

#[cfg(target_os = "macos")]
const HOOK_LIB_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/libmacra_hook.dylib"));

#[cfg(target_os = "windows")]
const HOOK_LIB_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/macra_hook.dll"));

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const HOOK_LIB_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/libmacra_hook.so"));

/// Extract the embedded hook library to `~/.cache/cargo-macra/` if needed,
/// returning the path to the cached file.
fn ensure_hook_lib() -> Option<PathBuf> {
    let lib_name = if cfg!(target_os = "macos") {
        "libmacra_hook.dylib"
    } else if cfg!(target_os = "windows") {
        "macra_hook.dll"
    } else {
        "libmacra_hook.so"
    };

    let version = env!("CARGO_PKG_VERSION");
    let file_name = if cfg!(target_os = "windows") {
        format!("macra_hook-{}.dll", version)
    } else if cfg!(target_os = "macos") {
        format!("libmacra_hook-{}.dylib", version)
    } else {
        format!("libmacra_hook-{}.so", version)
    };

    let cache_dir = dirs_cache()?;
    let dest = cache_dir.join(&file_name);

    // Also place a copy with the plain lib name so callers can find it
    let dest_plain = cache_dir.join(lib_name);

    // If cached file exists with the right size, reuse it
    if let Ok(meta) = std::fs::metadata(&dest) {
        if meta.len() == HOOK_LIB_BYTES.len() as u64 {
            // Ensure the plain-name symlink/copy exists too
            if !dest_plain.exists() {
                let _ = std::fs::copy(&dest, &dest_plain);
            }
            return Some(dest_plain);
        }
    }

    // Write the embedded bytes
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }
    if std::fs::write(&dest, HOOK_LIB_BYTES).is_err() {
        return None;
    }
    let _ = std::fs::copy(&dest, &dest_plain);

    Some(dest_plain)
}

/// Return `~/.cache/cargo-macra/` (or platform equivalent).
fn dirs_cache() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("cargo-macra"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .map(|d| d.join("cargo-macra"))
    }
}

/// Find the macra-hook shared library.
///
/// `current_exe` should be the current executable path when available.
pub fn find_hook_lib(current_exe: Option<&Path>) -> Option<PathBuf> {
    let lib_name = if cfg!(target_os = "macos") {
        "libmacra_hook.dylib"
    } else if cfg!(target_os = "windows") {
        "macra_hook.dll"
    } else {
        "libmacra_hook.so"
    };

    if let Some(exe) = current_exe {
        if let Some(dir) = exe.parent() {
            let hook_lib = dir.join(lib_name);
            if hook_lib.exists() {
                return Some(hook_lib);
            }
        }
    }

    let paths = [
        PathBuf::from(format!("./target/debug/{}", lib_name)),
        PathBuf::from(format!("./target/release/{}", lib_name)),
    ];

    for path in paths {
        if path.exists() {
            return Some(path);
        }
    }

    // Fallback: extract embedded library to cache
    ensure_hook_lib()
}
