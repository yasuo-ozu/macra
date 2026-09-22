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

    install_bytes(&dest, HOOK_LIB_BYTES)?;
    // The arch alias is the path handed out, so it has to carry this build too, not
    // merely exist. The legacy plain name is kept for older cargo-macra binaries.
    install_bytes(&dest_plain, HOOK_LIB_BYTES)?;
    let _ = install_bytes(&cache_dir.join(lib_name), HOOK_LIB_BYTES);

    Some(dest_plain)
}

/// Make `dest` hold exactly `bytes`, replacing it atomically when it does not.
///
/// The comparison is on content, not size. This cache is what the test suite runs
/// the hook from, and a hook edit that keeps the byte count — a changed constant, a
/// same-length string such as the `GLIBC_2.xx` version names in `hook_linux.rs` —
/// produces a same-sized file, which a size check kept serving from the previous
/// build: the edit appeared to do nothing, in the tests and in the tool. (A stale
/// embedded hook once caused the same symptom; `build.rs` explains that half.)
/// Reading the ~12 MB back costs nothing next to the `cargo check` that follows.
///
/// The write goes to a private temp file that is then `rename`d over `dest`. The
/// tests call this from several threads at once, and a plain `fs::write` truncates
/// in place, so a thread that had already returned could hand rustc a path another
/// thread was in the middle of rewriting. `rename` replaces the target atomically on
/// the same filesystem, so every observer sees either the old complete file or the
/// new one.
fn install_bytes(dest: &Path, bytes: &[u8]) -> Option<()> {
    let up_to_date = || std::fs::read(dest).is_ok_and(|have| have == bytes);
    if up_to_date() {
        return Some(());
    }
    let dir = dest.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    // Unique per writer: threads of one process share a pid.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = dir.join(format!(
        ".{}.{}.{}.tmp",
        dest.file_name()?.to_string_lossy(),
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if std::fs::write(&tmp, bytes).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return None;
    }
    if std::fs::rename(&tmp, dest).is_err() {
        let _ = std::fs::remove_file(&tmp);
        // Windows refuses to replace a DLL some rustc still has mapped. The file in
        // place is then usable only if a concurrent writer already put this build there.
        return up_to_date().then_some(());
    }
    Some(())
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

    install_bytes(&dest, WRAPPER_EXE_BYTES)?;
    install_bytes(&dest_plain, WRAPPER_EXE_BYTES)?;

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

/// A rustc version as (major, minor), whether it is a pre-release, and the date of
/// the commit it was built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustcVersion {
    pub major: u32,
    pub minor: u32,
    pub prerelease: bool,
    /// The `commit-date` from the parenthesised build info, as `(year, month, day)`,
    /// or `None` when the line carries none (a from-source build without git prints
    /// `(unknown)`). A pre-release's number does not say which side of a mid-cycle
    /// ABI change it is on; the date of the commit it was built from does — see the
    /// 1.100 arm of [`bridge_abi_for`].
    pub commit_date: Option<(u32, u32, u32)>,
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
    // `(<hash> <YYYY-MM-DD>)` follows the version; the date is its last word.
    let commit_date = rest
        .split_once('(')
        .and_then(|(_, tail)| tail.split_once(')'))
        .and_then(|(info, _)| info.split_whitespace().last())
        .and_then(parse_ymd);
    Some(RustcVersion {
        major,
        minor,
        prerelease,
        commit_date,
    })
}

/// `YYYY-MM-DD` as a tuple, so that `<`/`>=` order chronologically.
fn parse_ymd(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((year, month, day))
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
    // The 1.95 flattening put `ts_from_str` and `ts_to_string` here, and they held
    // through 1.99. Counting `with_api!` in the `library/proc_macro/src/bridge/mod.rs`
    // shipped by 1.98.0 and by 1.99.0-beta.7 (rust-src component) gives, from 0:
    // injected_env_var, track_env_var, track_path, literal_from_str, emit_diagnostic,
    // ts_drop, ts_clone, ts_is_empty, ts_expand_expr, ts_from_str = 9,
    // ts_to_string = 10. `declare_tags` turns that list into `enum ApiTags` in
    // declaration order and `rpc_encode_decode!(enum ..)` sends `Tag::$variant as u8`.
    const FLAT_95: RpcTags = RpcTags::Flat {
        from_str: 9,
        to_string: 10,
    };
    // 1.100 removed `injected_env_var`, the first method in the list (rust-lang/rust
    // fadc7c6a3d, "Remove experimental `--env-set` option"), so everything after it
    // moved down one: the same count in the 1.100.0-nightly (cea272fa3 2026-09-07)
    // source starts at track_env_var = 0 and gives ts_from_str = 8, ts_to_string = 9.
    // Confirmed by hand-driving the hook on that nightly: 8/9 intercepts all fourteen
    // expansions of the probe crate with exit 0 and no panic, while 9/10 makes each
    // one print `thread 'rustc' panicked at .../bridge/rpc.rs:118` ("range end index
    // 4 out of range for slice of length 1"), then `.../bridge/mod.rs:412` ("entered
    // unreachable code") and "the compiler unexpectedly panicked" — the hook's own
    // `ts_to_string` request lands on `ts_from_token_tree`, which cannot decode a
    // handle. The bridge catches those, so cargo still exits 0; the user sees 33
    // ICE reports in their build.
    const FLAT_100: RpcTags = RpcTags::Flat {
        from_str: 8,
        to_string: 9,
    };
    // The day the shift reached master, as `rustc -vV`'s `commit-date` reports it.
    // fadc7c6a3d merged in rollup #162035 at 2026-08-31T04:41Z (merge 5321a4f40c).
    // The official channel manifests place the shipped nightlies either side of it:
    // nightly-2026-08-31 was built from 908501772 (commit-date 2026-08-30), 37 commits
    // behind the merge; nightly-2026-09-01 from 0dfb098f3 (commit-date 2026-08-31),
    // 40 commits ahead of it. So a `1.100.0-nightly` whose commit-date is on or after
    // this day has the shift, and one dated earlier does not. (A from-source build of
    // a master commit made in the four hours before the merge on this day would be
    // misjudged; rustup never shipped one.)
    const FLAT_100_LANDED: (u32, u32, u32) = (2026, 8, 31);
    let abi = |table, rpc| Some(BridgeAbi { table, rpc });
    match (v.major, v.minor) {
        // A pre-release's number does not say which side of an ABI change it is on.
        // rustc's master bumps its minor when the previous beta branches, so every
        // nightly of a six-week cycle reports the same `1.N.0-nightly`, and a change
        // that lands mid-cycle splits that one number across two ABIs. Reproduced:
        // `nightly-2026-01-22` and `nightly-2026-02-05` both say `1.95.0-nightly`, yet
        // the former still has the nested tags (no `enum ApiTags` in its
        // `library/proc_macro/src/bridge/mod.rs`) and the latter the flat ones.
        // Driving the former with the flat numbering exits 101 with zero
        // expansions, `thread 'rustc' panicked at library/proc_macro/src/bridge/
        // mod.rs:190` and "the compiler unexpectedly panicked" — to the user an ICE
        // in their own crate. So a pre-release gets nothing at a minor where the ABI
        // moved: 95 (tags) and 98 (table). Minors between boundaries are unambiguous
        // — an early `1.94.0-nightly` carries 1.93's bridge, which is the same one —
        // and stay mapped, including 86: 1.85's and 1.86's `bridge/` differ only in
        // `pub` becoming `pub(crate)`. A beta is in fact safe, since it branches
        // after the cycle's changes have landed, but `prerelease` does not tell the
        // two apart and the loss is only proc-macro capture on a boundary beta, so
        // both are treated the same. (1.100 is a boundary too, but one whose landing
        // day is known, so it gets its own arm below instead of a blanket `None`.)
        (1, 95) | (1, 98) if v.prerelease => None,
        (1, 86..=94) => abi(TableLayout::ProcMacroEnum, RpcTags::Nested),
        (1, 95..=97) => abi(TableLayout::ProcMacroEnum, FLAT_95),
        // 1.98 replaced the `&[ProcMacro]` table with a slice of bare `Client`s
        // (rust-lang/rust 7edb1c086b "Remove ProcMacro enum from proc macro ABI",
        // merged 2026-06-10 in #157683, inside the 1.98 cycle master began on
        // 2026-05-22) and kept the 1.95 tags. Macro names and kinds now come from
        // crate metadata, which is `rustc_meta`'s concern, not the bridge's. 1.99
        // changed nothing here: GitHub's history of `library/proc_macro/src/bridge`
        // jumps from 2026-06-15 straight to 2026-08-21, past both the 1.99 bump
        // (merged 2026-07-05) and the 1.100 bump (merged 2026-08-15), so every
        // `1.99.0-nightly` carries 1.98's bridge and stays mapped. Verified by
        // hand-driving the hook with `slice,flat,9,10`: 1.98.0 and 1.99.0-beta.7 both
        // exit 0 with all fourteen bang, attribute and derive expansions intercepted
        // and no panic.
        (1, 98..=99) => abi(TableLayout::ClientSlice, FLAT_95),
        // A 1.100 pre-release is a boundary case like 95: the tag shift landed sixteen
        // days into the cycle, so `1.100.0-nightly` names both numberings. Unlike 95
        // the landing is pinned to a day (`FLAT_100_LANDED`), so the commit-date
        // decides. Nightlies from before it are left unmapped rather than given the
        // 1.99 numbering they carry: nothing installed can confirm that side, and a
        // nightly from those sixteen days is not one anybody is still building with.
        // An undated line (`(unknown)`) is unmapped for the same reason.
        (1, 100) if v.prerelease => match v.commit_date {
            Some(date) if date >= FLAT_100_LANDED => abi(TableLayout::ClientSlice, FLAT_100),
            _ => None,
        },
        // A stable 1.100 necessarily contains the merge — its beta branches from master
        // weeks after it — and nothing else has touched `bridge/` on master through
        // 2026-09-23 (e4d8f07417, 16-bit support, changes only `arena.rs` and
        // `fxhash.rs`, neither on the wire). Not yet released, so this is the one arm
        // established from source alone; the nightly above is the same code.
        (1, 100) => abi(TableLayout::ClientSlice, FLAT_100),
        // Anything newer is unverified. Guessing does not fail cleanly, see above.
        _ => None,
    }
}

/// A short id for the embedded hook's contents.
///
/// Cargo replays the cached stderr of a "Fresh" crate, and the hook's expansion
/// records travel on stderr — so a crate compiled by an older hook keeps reporting
/// that hook's output until something makes cargo consider it dirty. Folding this
/// into the `--cfg` the driver passes does exactly that: a different hook is a
/// different fingerprint. Without it, editing the hook and re-running showed the
/// previous hook's macro names, which is indistinguishable from the hook being wrong.
///
/// Not cryptographic and not stable across releases; it only has to differ when the
/// bytes differ.
pub fn hook_build_id() -> u64 {
    use std::hash::{Hash, Hasher};
    static ID: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *ID.get_or_init(|| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        HOOK_LIB_BYTES.hash(&mut h);
        h.finish()
    })
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
mod cache_tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("cargo-macra-{name}-{}", std::process::id()))
    }

    /// A size-only check served the old build whenever a hook edit kept the byte
    /// count, so the edit looked like a no-op in the tests and the tool.
    #[test]
    fn same_length_content_change_replaces_the_cached_file() {
        let dir = scratch("cache");
        let dest = dir.join("sub").join("hook.bin");
        install_bytes(&dest, b"GLIBC_2.34").expect("first install");
        install_bytes(&dest, b"GLIBC_2.35").expect("second install");
        assert_eq!(std::fs::read(&dest).unwrap(), b"GLIBC_2.35");
        // Nothing but the installed file may be left behind.
        let names: Vec<_> = std::fs::read_dir(dest.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["hook.bin"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Concurrent writers each replace the file whole; no reader ever sees a
    /// truncated or half-written one.
    #[test]
    fn concurrent_installs_never_expose_a_partial_file() {
        let dir = scratch("cache-concurrent");
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("hook.bin");
        let a = vec![0xAAu8; 1 << 20];
        let b = vec![0xBBu8; 1 << 20];
        let writers: Vec<_> = (0..8)
            .map(|i| {
                let (dest, a, b) = (dest.clone(), a.clone(), b.clone());
                std::thread::spawn(move || {
                    for _ in 0..10 {
                        install_bytes(&dest, if i % 2 == 0 { &a } else { &b }).expect("install");
                        let seen = std::fs::read(&dest).unwrap();
                        assert!(
                            seen == a || seen == b,
                            "partial file of {} bytes",
                            seen.len()
                        );
                    }
                })
            })
            .collect();
        for w in writers {
            w.join().unwrap();
        }
        let _ = std::fs::remove_dir_all(dir);
    }
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

        assert_eq!(stable.commit_date, Some((2025, 9, 14)));

        let nightly =
            parse_rustc_version("rustc 1.100.0-nightly (cea272fa3 2026-09-07)\n").unwrap();
        assert_eq!(
            (nightly.major, nightly.minor, nightly.prerelease),
            (1, 100, true)
        );
        // The commit-date is what places a 1.100 nightly relative to the tag shift.
        assert_eq!(nightly.commit_date, Some((2026, 9, 7)));

        let beta = parse_rustc_version("rustc 1.99.0-beta.7 (aa0593682 2026-09-19)\n").unwrap();
        assert_eq!((beta.minor, beta.prerelease), (99, true));
        assert_eq!(beta.commit_date, Some((2026, 9, 19)));

        // `rustc -vV` puts the same first line above a details block.
        let verbose = parse_rustc_version("rustc 1.88.0 (abc 2025-06-01)\nbinary: rustc\n");
        assert_eq!(verbose.unwrap().minor, 88);

        // A build without git info, or one printing no build info at all, still
        // parses; it just has no date to place it with.
        let unknown = parse_rustc_version("rustc 1.100.0-dev (unknown)\n").unwrap();
        assert_eq!((unknown.minor, unknown.prerelease), (100, true));
        assert_eq!(unknown.commit_date, None);
        let bare = parse_rustc_version("rustc 1.86.0\n").unwrap();
        assert_eq!(
            (bare.minor, bare.prerelease, bare.commit_date),
            (86, false, None)
        );

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
            commit_date: None,
        };
        let flat_95 = RpcTags::Flat {
            from_str: 9,
            to_string: 10,
        };
        let flat_100 = RpcTags::Flat {
            from_str: 8,
            to_string: 9,
        };
        let slice = |rpc| BridgeAbi {
            table: TableLayout::ClientSlice,
            rpc,
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
        // 1.98 swapped the table for a slice of `Client`s and kept the 1.95 tags; 1.99
        // touched neither. 1.100 dropped `injected_env_var` and both tags moved down.
        for minor in 98..=99 {
            assert_eq!(
                bridge_abi_for(v(minor, false)),
                Some(slice(flat_95)),
                "1.{minor}"
            );
        }
        assert_eq!(bridge_abi_for(v(100, false)), Some(slice(flat_100)));

        // Every nightly of a cycle carries the same number, so `1.95.0-nightly` names
        // both a nested-tag and a flat-tag compiler (nightly-2026-01-22 vs
        // nightly-2026-02-05); the flat numbering panics the former inside the bridge.
        assert_eq!(
            bridge_abi_for(v(95, true)),
            None,
            "a 1.95 pre-release may predate the tag flattening"
        );
        // Likewise `1.98.0-nightly` names both table layouts (the slice landed
        // 2026-06-10, nineteen days into the cycle).
        assert_eq!(
            bridge_abi_for(v(98, true)),
            None,
            "a 1.98 pre-release may predate the slice table"
        );
        // Between boundaries the number is unambiguous: an early 1.N nightly has
        // 1.(N-1)'s bridge, and that is the same one. Disabling those too would turn
        // off capture on every nightly, which is not the goal.
        for (minor, rpc) in [
            (86, RpcTags::Nested),
            (94, RpcTags::Nested),
            (96, flat_95),
            (97, flat_95),
            (99, flat_95),
        ] {
            let abi = bridge_abi_for(v(minor, true))
                .unwrap_or_else(|| panic!("1.{minor} pre-release is not at an ABI boundary"));
            assert_eq!(abi.rpc, rpc, "1.{minor}-nightly");
        }
        // 1.100 is a boundary whose landing day is known (the shift merged on
        // 2026-08-31; nightly-2026-08-31 is dated 08-30 and predates it,
        // nightly-2026-09-01 is dated 08-31 and has it), so the commit-date decides:
        // nothing without one, nothing before the day, the new tags from it on.
        let dated = |commit_date| RustcVersion {
            major: 1,
            minor: 100,
            prerelease: true,
            commit_date: Some(commit_date),
        };
        assert_eq!(
            bridge_abi_for(v(100, true)),
            None,
            "an undated 1.100 pre-release could be on either side of the shift"
        );
        for date in [(2026, 8, 14), (2026, 8, 30)] {
            assert_eq!(
                bridge_abi_for(dated(date)),
                None,
                "a 1.100 nightly dated {date:?} still has the 1.99 tags"
            );
        }
        for date in [(2026, 8, 31), (2026, 9, 7), (2026, 10, 1)] {
            assert_eq!(
                bridge_abi_for(dated(date)),
                Some(slice(flat_100)),
                "a 1.100 pre-release dated {date:?} has the shifted tags"
            );
        }
        // The date rule is specific to the 1.100 boundary; it does not vouch for a
        // later minor just because it is recent.
        assert_eq!(
            bridge_abi_for(RustcVersion {
                major: 1,
                minor: 101,
                prerelease: true,
                commit_date: Some((2026, 9, 30)),
            }),
            None
        );
        // Newer minors are unverified and select nothing.
        assert_eq!(bridge_abi_for(v(101, false)), None);
        assert_eq!(bridge_abi_for(v(101, true)), None);
        assert_eq!(bridge_abi_for(v(85, false)), None);
        assert_eq!(
            bridge_abi_for(RustcVersion {
                major: 2,
                minor: 0,
                prerelease: false,
                commit_date: None,
            }),
            None
        );
    }

    #[test]
    fn abi_round_trips_through_the_handshake() {
        for minor in [86, 95, 97, 98, 100] {
            let abi = bridge_abi_for(RustcVersion {
                major: 1,
                minor,
                prerelease: false,
                commit_date: None,
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
