use std::path::{Path, PathBuf};

pub mod parse_normal;
pub mod parse_trace;
pub mod rustc_meta;
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

/// How a rustc lays out its `__rustc_proc_macro_decls_*` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableLayout {
    /// `&[ProcMacro]`, the enum carrying each macro's name and kind inline, with a
    /// two-pointer `Client`. Through 1.97.
    ProcMacroEnum,
    /// `&[Client]`, one `run` pointer per macro; names and kinds moved into crate
    /// metadata. From 1.98.
    ClientSlice,
}

/// How a rustc encodes a bridge RPC request tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcTags {
    /// `with_api!` nested methods under a type, so a request began with a group byte
    /// and a method byte. Through 1.94.
    Nested,
    /// One flat `#[repr(u8)] enum ApiTags` encoded as a single byte. From 1.95 — but
    /// the indices shift as methods are added or removed, so they are carried rather
    /// than assumed.
    Flat { from_str: u8, to_string: u8 },
}

/// What a given rustc's proc-macro bridge looks like.
///
/// The table layout and the RPC encoding change independently — 1.95 flattened the
/// tags while keeping the old table, and 1.98 changed the table while keeping the
/// 1.95 tags — so they are tracked separately. Conflating them is not harmless:
/// sending 1.100's tag numbering to a 1.98 server invokes the wrong bridge method
/// and panics the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeAbi {
    pub table: TableLayout,
    pub rpc: RpcTags,
}

impl BridgeAbi {
    /// Serialised for the `MACRA_ABI` handshake with the hook.
    pub fn as_env(self) -> String {
        let table = match self.table {
            TableLayout::ProcMacroEnum => "enum",
            TableLayout::ClientSlice => "slice",
        };
        match self.rpc {
            RpcTags::Nested => format!("{table},nested"),
            RpcTags::Flat {
                from_str,
                to_string,
            } => format!("{table},flat,{from_str},{to_string}"),
        }
    }

    pub fn from_env(s: &str) -> Option<Self> {
        let mut parts = s.split(',');
        let table = match parts.next()? {
            "enum" => TableLayout::ProcMacroEnum,
            "slice" => TableLayout::ClientSlice,
            _ => return None,
        };
        let rpc = match parts.next()? {
            "nested" => RpcTags::Nested,
            "flat" => RpcTags::Flat {
                from_str: parts.next()?.parse().ok()?,
                to_string: parts.next()?.parse().ok()?,
            },
            _ => return None,
        };
        parts.next().is_none().then_some(BridgeAbi { table, rpc })
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

/// Which bridge shape this rustc exposes, or `None` when it is one macra has not
/// been verified against.
///
/// Every bound here was established by building a probe crate with that compiler and
/// reading the emitted table, and by reading the `with_api!` definition it ships.
/// Deliberately conservative: an unknown compiler gets `None` so the caller can skip
/// the hook, because guessing does not fail cleanly — the wrong table stride hands
/// rustc garbage pointers and the wrong tag numbering invokes the wrong bridge
/// method, and both abort the compiler.
pub fn bridge_abi_for(v: RustcVersion) -> Option<BridgeAbi> {
    // The 1.95 flattening kept these indices. 1.100 removed a bridge method and
    // shifted them to 8/9; that is recorded in the table in `RpcTags` but unused
    // while 1.98+ is unmapped.
    const FLAT_95: RpcTags = RpcTags::Flat {
        from_str: 9,
        to_string: 10,
    };
    let abi = |table, rpc| Some(BridgeAbi { table, rpc });
    match (v.major, v.minor) {
        (1, 86..=94) => abi(TableLayout::ProcMacroEnum, RpcTags::Nested),
        (1, 95..=97) => abi(TableLayout::ProcMacroEnum, FLAT_95),
        // 1.98 onwards moved macro names and kinds out of the table into crate
        // metadata, and recovering them from there is not trustworthy: a derive record
        // and an interned string share the tag byte `0x00`, so the scan in
        // `rustc_meta` reports a doc comment, a `#[doc(alias)]` or a
        // `#[deprecated(note = "..")]` as a derive and drops a real macro off the end
        // — reproduced on stable 1.98.0 with no `cfg_attr` involved. It also misreads
        // a derive's helper-attribute count as the next kind byte, so
        // `#[proc_macro_derive(Serialize, attributes(serde))]` makes the scan fail
        // outright. Only bang and attribute names are validated against the symbol
        // table; derives are unchecked, which is exactly the common case. Until that
        // is fixed these compilers get no hook: losing proc-macro capture is
        // recoverable, showing the wrong macro name is not.
        _ => None,
    }
}

/// Whether the rustc that will run the build is one macra has a bridge ABI for, or
/// `None` when the compiler could not be probed at all.
///
/// Proc-macro capture goes through the injected hook, and the hook only arms itself
/// when [`bridge_abi_for`] recognises the compiler. On every other toolchain macra
/// still reports `macro_rules!` expansions, but no proc-macro ones, so callers that
/// assert on proc-macro output (the test suite) use this to skip rather than fail.
///
/// The `None` case is kept distinct from `Some(false)` on purpose: a missing or broken
/// `rustc` means "unknown", not "unsupported", and collapsing the two lets an
/// environment fault turn the test suite green by skipping everything.
pub fn proc_macro_capture_supported() -> Option<bool> {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let out = std::process::Command::new(rustc).arg("-vV").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let version = parse_rustc_version(&String::from_utf8_lossy(&out.stdout))?;
    Some(bridge_abi_for(version).is_some())
}

#[cfg(test)]
mod abi_tests {
    use super::*;

    #[test]
    fn parses_stable_and_prerelease_versions() {
        let stable = parse_rustc_version("rustc 1.90.0 (1159e78c4 2025-09-14)\n").unwrap();
        assert_eq!(
            (stable.major, stable.minor, stable.prerelease),
            (1, 90, false)
        );

        let nightly =
            parse_rustc_version("rustc 1.100.0-nightly (cea272fa3 2026-09-07)\n").unwrap();
        assert_eq!(
            (nightly.major, nightly.minor, nightly.prerelease),
            (1, 100, true)
        );

        // `rustc -vV` puts the same first line above a details block.
        let verbose = parse_rustc_version("rustc 1.88.0 (abc 2025-06-01)\nbinary: rustc\n");
        assert_eq!(verbose.unwrap().minor, 88);

        assert!(parse_rustc_version("not rustc at all").is_none());
        assert!(parse_rustc_version("").is_none());
    }

    /// Every bound below was established by probing the real compiler; an
    /// unverified one must select nothing, because guessing aborts rustc rather
    /// than failing cleanly.
    #[test]
    fn only_verified_versions_select_an_abi() {
        let v = |minor, prerelease| RustcVersion {
            major: 1,
            minor,
            prerelease,
        };
        let flat_95 = RpcTags::Flat {
            from_str: 9,
            to_string: 10,
        };

        // The table stayed an enum through 1.97, but 1.95 flattened the RPC tags —
        // the two move independently, which is why they are tracked separately.
        for minor in 86..=94 {
            let abi = bridge_abi_for(v(minor, false)).expect("1.{minor} is supported");
            assert_eq!(abi.table, TableLayout::ProcMacroEnum);
            assert_eq!(
                abi.rpc,
                RpcTags::Nested,
                "1.{minor} predates the flattening"
            );
        }
        for minor in 95..=97 {
            let abi = bridge_abi_for(v(minor, false)).expect("supported");
            assert_eq!(abi.table, TableLayout::ProcMacroEnum);
            assert_eq!(abi.rpc, flat_95);
        }
        // 1.98 onwards is deliberately unsupported: its table carries no names, and
        // recovering them from crate metadata mislabels derives (see `bridge_abi_for`).
        for minor in [98, 99, 100, 101] {
            assert_eq!(
                bridge_abi_for(v(minor, false)),
                None,
                "1.{minor} must not select an ABI while the metadata scan is unsound"
            );
            assert_eq!(bridge_abi_for(v(minor, true)), None);
        }
        assert_eq!(bridge_abi_for(v(85, false)), None);
        assert_eq!(
            bridge_abi_for(RustcVersion {
                major: 2,
                minor: 0,
                prerelease: false
            }),
            None
        );
    }

    #[test]
    fn abi_round_trips_through_the_handshake() {
        for minor in [86, 95, 97] {
            let abi = bridge_abi_for(RustcVersion {
                major: 1,
                minor,
                prerelease: false,
            })
            .expect("supported");
            assert_eq!(BridgeAbi::from_env(&abi.as_env()), Some(abi), "1.{minor}");
        }
        // The tag numbering has to survive the round trip, not just the shape.
        assert_eq!(
            BridgeAbi::from_env("slice,flat,8,9").map(|a| a.rpc),
            Some(RpcTags::Flat {
                from_str: 8,
                to_string: 9
            })
        );
        for junk in [
            "",
            "enum",
            "enum,flat",
            "enum,flat,9",
            "what,nested",
            "enum,nested,extra",
        ] {
            assert_eq!(BridgeAbi::from_env(junk), None, "{junk:?} must not parse");
        }
    }
}
