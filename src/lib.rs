use std::path::{Path, PathBuf};

pub mod parse_normal;
pub mod parse_trace;
pub mod trace_macros;

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

    None
}
