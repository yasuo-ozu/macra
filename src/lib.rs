use std::path::{Path, PathBuf};

pub mod parse_normal;
pub mod parse_trace;
pub mod trace_macros;

/// Normalize token-like text for resilient comparisons.
///
/// - Removes spaces adjacent to punctuation (e.g., `a :: b` -> `a::b`)
/// - Collapses remaining whitespace to a single space
/// - Normalizes bracket types (`{}`, `[]` -> `()`)
pub fn normalize_tokens(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(chars.len());

    fn is_punct(c: char) -> bool {
        !c.is_alphanumeric() && c != '_' && c != '"' && c != '\'' && !c.is_whitespace()
    }

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            let prev = result.chars().last();
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            let next = chars.get(i).copied();
            let prev_is_punct = prev.is_none_or(is_punct);
            let next_is_punct = next.is_none_or(is_punct);
            if !prev_is_punct && !next_is_punct {
                result.push(' ');
            }
        } else {
            match c {
                '{' | '[' => result.push('('),
                '}' | ']' => result.push(')'),
                _ => result.push(c),
            }
            i += 1;
        }
    }

    result
}

#[cfg(target_os = "macos")]
const HOOK_LIB_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libmacra_hook.dylib"));

#[cfg(target_os = "windows")]
const HOOK_LIB_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/macra_hook.dll"));

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const HOOK_LIB_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libmacra_hook.so"));

#[cfg(target_os = "windows")]
const WRAPPER_EXE_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/macra-rustc-wrapper.exe"));

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
    let arch = std::env::consts::ARCH;

    let version = env!("CARGO_PKG_VERSION");
    let file_name = if cfg!(target_os = "windows") {
        format!("macra_hook-{}-{}.dll", version, arch)
    } else if cfg!(target_os = "macos") {
        format!("libmacra_hook-{}-{}.dylib", version, arch)
    } else {
        format!("libmacra_hook-{}-{}.so", version, arch)
    };

    let cache_dir = dirs_cache()?;
    let dest = cache_dir.join(&file_name);
    let dest_plain = if cfg!(target_os = "windows") {
        cache_dir.join(format!("macra_hook-{}.dll", arch))
    } else if cfg!(target_os = "macos") {
        cache_dir.join(format!("libmacra_hook-{}.dylib", arch))
    } else {
        cache_dir.join(format!("libmacra_hook-{}.so", arch))
    };

    // If cached file exists with the right size, reuse it
    if let Ok(meta) = std::fs::metadata(&dest) {
        if meta.len() == HOOK_LIB_BYTES.len() as u64 {
            // Ensure the arch-specific alias copy exists too.
            if !dest_plain.exists() {
                let _ = std::fs::copy(&dest, &dest_plain);
            }
            // Keep backward compatibility with the legacy plain file name.
            let _ = std::fs::copy(&dest, cache_dir.join(lib_name));
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
    let _ = std::fs::copy(&dest, cache_dir.join(lib_name));

    Some(dest_plain)
}

/// Extract the embedded RUSTC_WRAPPER executable to the cache directory (Windows only).
#[cfg(target_os = "windows")]
fn ensure_wrapper_exe() -> Option<PathBuf> {
    let version = env!("CARGO_PKG_VERSION");
    let arch = std::env::consts::ARCH;
    let file_name = format!("macra-rustc-wrapper-{}-{}.exe", version, arch);

    let cache_dir = dirs_cache()?;
    let dest = cache_dir.join(&file_name);
    let dest_plain = cache_dir.join(format!("macra-rustc-wrapper-{}.exe", arch));

    if let Ok(meta) = std::fs::metadata(&dest) {
        if meta.len() == WRAPPER_EXE_BYTES.len() as u64 {
            if !dest_plain.exists() {
                let _ = std::fs::copy(&dest, &dest_plain);
            }
            return Some(dest_plain);
        }
    }

    if std::fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }
    if std::fs::write(&dest, WRAPPER_EXE_BYTES).is_err() {
        return None;
    }
    let _ = std::fs::copy(&dest, &dest_plain);

    Some(dest_plain)
}

/// Find the macra-rustc-wrapper executable (Windows only).
#[cfg(target_os = "windows")]
pub fn find_wrapper_exe(current_exe: Option<&Path>) -> Option<PathBuf> {
    let exe_name = "macra-rustc-wrapper.exe";
    let arch = std::env::consts::ARCH;
    let arch_name = format!("macra-rustc-wrapper-{}.exe", arch);

    if let Some(exe) = current_exe {
        if let Some(dir) = exe.parent() {
            let wrapper = dir.join(exe_name);
            if wrapper.exists() {
                return Some(wrapper);
            }
        }
    }

    let paths = [
        PathBuf::from(format!("./target/debug/{}", arch_name)),
        PathBuf::from(format!("./target/release/{}", arch_name)),
        PathBuf::from(format!("./target/debug/{}", exe_name)),
        PathBuf::from(format!("./target/release/{}", exe_name)),
    ];

    for path in paths {
        if path.exists() {
            return Some(path);
        }
    }

    ensure_wrapper_exe()
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
    let arch = std::env::consts::ARCH;
    let arch_name = if cfg!(target_os = "windows") {
        format!("macra_hook-{}.dll", arch)
    } else if cfg!(target_os = "macos") {
        format!("libmacra_hook-{}.dylib", arch)
    } else {
        format!("libmacra_hook-{}.so", arch)
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
        PathBuf::from(format!("./target/debug/{}", arch_name)),
        PathBuf::from(format!("./target/release/{}", arch_name)),
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

/// The proc-macro bridge ABI a given rustc exposes.
///
/// The hook reinterprets rustc's `__rustc_proc_macro_decls_*` table using
/// hand-written mirrors of `proc_macro::bridge` types. Those are compiler
/// internals with no stability guarantee, so the layout has to be selected per
/// compiler rather than assumed — reading the wrong one walks the table at the
/// wrong stride and aborts rustc, which is what "could not compile <some
/// unrelated crate>" used to mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeAbi {
    /// `ProcMacro` enum carrying name/kind, with a two-pointer `Client`
    /// (`handle_counters` + `run`). Verified for 1.86 through 1.91.
    ProcMacroEnum,
    /// The table is a bare `&[Client]` of single `run` pointers; names and kinds
    /// live in crate metadata instead. Seen from 1.100.0-nightly.
    ClientSliceOnly,
}

impl BridgeAbi {
    /// Name used for the `MACRA_ABI` handshake with the hook.
    pub fn as_str(self) -> &'static str {
        match self {
            BridgeAbi::ProcMacroEnum => "proc-macro-enum",
            BridgeAbi::ClientSliceOnly => "client-slice-only",
        }
    }

    /// Named `from_name` rather than `from_str` so it is not mistaken for
    /// `std::str::FromStr`, which this deliberately is not — an unknown value means
    /// "no ABI", not a parse error to be surfaced.
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "proc-macro-enum" => Some(BridgeAbi::ProcMacroEnum),
            "client-slice-only" => Some(BridgeAbi::ClientSliceOnly),
            _ => None,
        }
    }
}

/// A rustc version as (major, minor), plus whether it is a pre-release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustcVersion {
    pub major: u32,
    pub minor: u32,
    pub prerelease: bool,
}

/// Parse the `rustc -vV`/`--version` first line, e.g.
/// `rustc 1.90.0 (abc 2025-01-01)` or `rustc 1.100.0-nightly (cea 2026-09-07)`.
pub fn parse_rustc_version(output: &str) -> Option<RustcVersion> {
    let line = output.lines().next()?;
    let rest = line.strip_prefix("rustc ")?.trim_start();
    let ver = rest.split_whitespace().next()?;
    // Split off a `-nightly` / `-beta` suffix before parsing the numbers.
    let (numbers, prerelease) = match ver.split_once('-') {
        Some((n, _)) => (n, true),
        None => (ver, false),
    };
    let mut parts = numbers.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some(RustcVersion {
        major,
        minor,
        prerelease,
    })
}

/// Which bridge ABI this rustc exposes, or `None` when it is outside every range
/// macra has been verified against.
///
/// Deliberately conservative: an unknown compiler gets `None` so the caller can
/// refuse, rather than guessing a layout and corrupting rustc's macro table.
pub fn bridge_abi_for(v: RustcVersion) -> Option<BridgeAbi> {
    match (v.major, v.minor) {
        (1, 86..=91) if !v.prerelease => Some(BridgeAbi::ProcMacroEnum),
        (1, 100) => Some(BridgeAbi::ClientSliceOnly),
        _ => None,
    }
}

#[cfg(test)]
mod abi_tests {
    use super::*;

    #[test]
    fn parses_stable_and_prerelease_versions() {
        let stable = parse_rustc_version("rustc 1.90.0 (1159e78c4 2025-09-14)\n").unwrap();
        assert_eq!((stable.major, stable.minor, stable.prerelease), (1, 90, false));

        let nightly =
            parse_rustc_version("rustc 1.100.0-nightly (cea272fa3 2026-09-07)\n").unwrap();
        assert_eq!((nightly.major, nightly.minor, nightly.prerelease), (1, 100, true));

        // `rustc -vV` puts the same first line above a details block.
        let verbose = parse_rustc_version("rustc 1.88.0 (abc 2025-06-01)\nbinary: rustc\n");
        assert_eq!(verbose.unwrap().minor, 88);

        assert!(parse_rustc_version("not rustc at all").is_none());
        assert!(parse_rustc_version("").is_none());
    }

    /// An unverified compiler must select no ABI at all. Guessing one walks rustc's
    /// macro table at the wrong stride and aborts it, which is how an unsupported
    /// toolchain used to surface as an unrelated crate failing to compile.
    #[test]
    fn only_verified_versions_select_an_abi() {
        let v = |minor, prerelease| RustcVersion {
            major: 1,
            minor,
            prerelease,
        };

        for minor in 86..=91 {
            assert_eq!(
                bridge_abi_for(v(minor, false)),
                Some(BridgeAbi::ProcMacroEnum),
                "1.{minor} is covered by CI and must be supported"
            );
        }
        assert_eq!(
            bridge_abi_for(v(100, true)),
            Some(BridgeAbi::ClientSliceOnly)
        );

        // Between the two known windows the layout changed without us verifying it.
        assert_eq!(bridge_abi_for(v(96, false)), None);
        assert_eq!(bridge_abi_for(v(92, false)), None);
        assert_eq!(bridge_abi_for(v(85, false)), None);
        // A pre-release of an otherwise-supported version is not the same compiler.
        assert_eq!(bridge_abi_for(v(90, true)), None);
    }

    #[test]
    fn abi_names_round_trip() {
        for abi in [BridgeAbi::ProcMacroEnum, BridgeAbi::ClientSliceOnly] {
            assert_eq!(BridgeAbi::from_name(abi.as_str()), Some(abi));
        }
        assert_eq!(BridgeAbi::from_name("something-else"), None);
    }
}
