use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::process::{Child, Command, Stdio};
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::parse_trace::{MacroExpansion, MacroExpansionKind, parse_trace};

/// Cargo arguments for macro tracing (no clap dependency).
#[derive(Debug, Clone, Default)]
pub struct Args {
    pub package: Option<String>,
    pub bin: Option<String>,
    pub lib: bool,
    pub test: Option<String>,
    pub example: Option<String>,
    pub manifest_path: Option<String>,
    pub cargo_args: Vec<String>,
    /// Path to the macra-hook shared library (e.g. `libmacra_hook.so`).
    /// When non-empty, the library is injected via `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES`.
    pub hook_lib: PathBuf,
}

/// Spawns `cargo check` with `-Z trace-macros` (and optionally the macra-hook)
/// and yields [`MacroExpansion`]s as they become available.
pub struct TraceMacros {
    cargo_path: PathBuf,
    args: Args,
}

/// Blocking iterator over [`MacroExpansion`] items produced by a running cargo
/// process.
pub struct MacroExpansionIter {
    rx: mpsc::Receiver<io::Result<MacroExpansion>>,
}

/// Result of spawning trace-macros collection.
pub struct TraceRun {
    pub iter: MacroExpansionIter,
    /// Receives cargo check result once the child exits.
    pub check_result: mpsc::Receiver<io::Result<CheckResult>>,
    /// The spawned `cargo check`, shared with the reader thread that `wait()`s on
    /// it once the pipes hit EOF. Lock and `kill()` to abort a run early: the
    /// readers then see EOF and wind down on their own.
    pub child: Arc<Mutex<Child>>,
}

/// Result details for the traced `cargo check` execution.
pub struct CheckResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl MacroExpansionIter {
    /// Non-blocking attempt to receive the next expansion.
    ///
    /// Returns `Ok(Some(exp))` if an item was ready, `Ok(None)` if the channel
    /// is still open but nothing is available yet, or `Err(())` if the channel
    /// has been closed (the background thread finished).
    #[allow(clippy::result_unit_err)]
    pub fn try_next(&mut self) -> Result<Option<io::Result<MacroExpansion>>, ()> {
        match self.rx.try_recv() {
            Ok(item) => Ok(Some(item)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(()),
        }
    }
}

impl Iterator for MacroExpansionIter {
    type Item = io::Result<MacroExpansion>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rx.recv().ok()
    }
}

const HOOK_LINE_PREFIX: &str = "__MACRA_HOOK__:";
#[cfg(target_os = "macos")]
static LINKER_WRAPPER_COUNTER: AtomicU64 = AtomicU64::new(0);
/// Path of the linker wrapper already built by this process, if any.
///
/// The wrapper's source is a compile-time constant, so within one process it cannot go
/// stale, and its path embeds this process id, so it is never shared with (or picked up
/// from) another process or an older cargo-macra binary. Rebuilt only if the temp file
/// has vanished in the meantime.
#[cfg(target_os = "macos")]
static LINKER_WRAPPER_CACHE: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(serde::Deserialize)]
struct HookRecord {
    name: String,
    #[serde(default)]
    krate: String,
    kind: String,
    #[serde(default)]
    arguments: String,
    input: String,
    output: String,
}

fn parse_hook_json(json: &str) -> Option<MacroExpansion> {
    let record: HookRecord = serde_json::from_str(json).ok()?;
    let kind = match record.kind.as_str() {
        "CustomDerive" => MacroExpansionKind::Derive,
        "Attr" => MacroExpansionKind::Attribute,
        _ => MacroExpansionKind::Bang,
    };

    let expanding = match kind {
        MacroExpansionKind::Derive => record.name.clone(),
        MacroExpansionKind::Attribute => {
            if record.input.contains('(') || record.input.contains('{') {
                record.input.clone()
            } else {
                format!("{} {{ {} }}", record.name, record.input)
            }
        }
        MacroExpansionKind::Bang => record.input.clone(),
    };

    Some(MacroExpansion {
        krate: record.krate,
        expanding,
        arguments: record.arguments,
        to: record.output,
        name: record.name,
        kind,
        input: record.input,
    })
}

impl TraceMacros {
    pub fn new(cargo_path: &Path, args: &Args) -> Self {
        Self {
            cargo_path: cargo_path.to_path_buf(),
            args: args.clone(),
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    /// Spawn `cargo check` and return a blocking iterator of macro expansions.
    ///
    /// Hook-based expansions (proc-macros captured via `LD_PRELOAD`) are emitted
    /// immediately as the child process writes them.  Trace-macros expansions
    /// (from rustc's `-Z trace-macros`) are emitted after the child exits.
    /// The rustc that `cargo` will drive, as reported by `rustc -vV`.
    ///
    /// Resolved through the same `PATH` and rustup settings cargo itself uses, so a
    /// directory override or `RUSTUP_TOOLCHAIN` is honoured.
    pub fn detect_rustc_version(&self) -> Option<crate::RustcVersion> {
        self.rustc_version_line()
            .as_deref()
            .and_then(crate::parse_rustc_version)
    }

    /// The first line of `rustc -vV`, e.g. `rustc 1.98.1 (48a229cea 2026-09-01)`.
    ///
    /// Comes from `$RUSTC`, else the `rustc` on `PATH`. With rustup proxies that is
    /// the compiler cargo will spawn; the comment on `MACRA_ABI` in [`Self::run`]
    /// lists the configurations where it is not, and nothing downstream checks.
    pub fn rustc_version_line(&self) -> Option<String> {
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let out = Command::new(rustc).arg("-vV").output().ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .map(|l| l.trim().to_string())
    }

    pub fn run(&self) -> io::Result<TraceRun> {
        let mut cmd = Command::new(&self.cargo_path);
        cmd.arg("check");
        cmd.env("RUSTC_BOOTSTRAP", "1");

        if let Some(ref pkg) = self.args.package {
            cmd.arg("-p").arg(pkg);
        }
        if let Some(ref bin) = self.args.bin {
            cmd.arg("--bin").arg(bin);
        }
        if self.args.lib {
            cmd.arg("--lib");
        }
        if let Some(ref test) = self.args.test {
            cmd.arg("--test").arg(test);
        }
        if let Some(ref example) = self.args.example {
            cmd.arg("--example").arg(example);
        }
        if let Some(ref manifest_path) = self.args.manifest_path {
            cmd.arg("--manifest-path").arg(manifest_path);
        }

        for arg in &self.args.cargo_args {
            cmd.arg(arg);
        }

        // Start from the flags cargo would pass on its own, then add ours. This used
        // to set `RUSTFLAGS` to "inherited `RUSTFLAGS` + `-Z trace-macros`", which is
        // not additive: cargo takes the first non-empty source in the order
        // `CARGO_ENCODED_RUSTFLAGS`, `RUSTFLAGS`, `target.<triple>.rustflags` plus
        // `target.'cfg(..)'.rustflags`, `build.rustflags`, so an environment value
        // discards whatever the project put in `.cargo/config.toml`. Reproduced with
        // `[build] rustflags = ["--cfg", "need_me"]` and a `#[cfg(not(need_me))]
        // compile_error!`: plain `cargo check` passes, `RUSTC_BOOTSTRAP=1
        // RUSTFLAGS="-Z trace-macros" cargo check` fails, and the TUI showed the
        // errors with nothing pointing at macra. Real shapes are `--cfg
        // tokio_unstable`, `-C target-cpu` and `-C link-arg`.
        let mut rustflags = self.baseline_rustflags();
        rustflags.extend(["-Z", "trace-macros"].map(String::from));

        // The hook reinterprets rustc's internal macro table, so it may only be
        // loaded into a compiler whose layout macra actually knows. On anything else
        // it is left out entirely: `-Z trace-macros` still yields bang macros, which
        // degrades the feature instead of aborting the build.
        let detected = self.detect_rustc_version();
        let abi = detected.and_then(crate::bridge_abi_for);
        if std::env::var_os("MACRA_HOOK_DEBUG").is_some() {
            eprintln!(
                "[macra] detected rustc {detected:?} -> abi {abi:?}; hook_lib {:?}",
                self.args.hook_lib
            );
        }

        // Set up macra-hook via LD_PRELOAD if available
        if abi.is_some() && !self.args.hook_lib.as_os_str().is_empty() {
            let lib = self
                .args
                .hook_lib
                .canonicalize()
                .unwrap_or_else(|_| self.args.hook_lib.clone());
            if cfg!(target_os = "macos") {
                cmd.env("DYLD_INSERT_LIBRARIES", &lib);
                // DYLD_INSERT_LIBRARIES propagates into the linker process (cc),
                // which can fail due to arch constraints on newer macOS runners.
                // Route linker invocations through a tiny wrapper that unsets DYLD.
                #[cfg(target_os = "macos")]
                if let Ok(wrapper) = create_macos_linker_wrapper() {
                    cmd.env("CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER", &wrapper);
                    cmd.env("CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER", &wrapper);
                    // Also cover toolchains/build scripts that consult CC directly.
                    cmd.env("CC", &wrapper);
                }
            } else if cfg!(target_os = "windows") {
                // On Windows, use RUSTC_WRAPPER to inject the hook DLL into
                // rustc via CreateRemoteThread + LoadLibraryW.
                #[cfg(target_os = "windows")]
                if let Some(wrapper_exe) =
                    crate::find_wrapper_exe(std::env::current_exe().ok().as_deref())
                {
                    cmd.env("RUSTC_WRAPPER", &wrapper_exe);
                    cmd.env("MACRA_HOOK_DLL_PATH", &lib);
                }
            } else {
                cmd.env("LD_PRELOAD", &lib);
            }

            // Include a hook-specific cfg flag in RUSTFLAGS so that cargo's
            // build fingerprint changes when switching between hook-enabled
            // and non-hook builds.  Without this, a previous build without the
            // hook (but with the same -Z trace-macros flag) would leave cached
            // artifacts that cargo considers "Fresh", causing it to replay
            // the cached stderr — which lacks hook output.
            //
            // The id makes that true between two *different* hooks as well, not just
            // hook versus no hook. Both report through stderr, so a crate built by an
            // older hook stays Fresh and cargo replays its records verbatim: after the
            // metadata scan was fixed, a stale `target/` still served the old scan's
            // wrong macro names, which reads exactly like the fix not working.
            rustflags.extend([
                "--cfg".to_string(),
                format!("macra_hook_active_{:016x}", crate::hook_build_id()),
            ]);

            // All platforms use stderr for hook output.  The hook library
            // writes JSON lines to stderr with a `__MACRA_HOOK__:` prefix.
            // Cargo caches and replays stderr diagnostics for "Fresh" crates,
            // so hook output from previously-compiled dependencies is
            // automatically available even when cargo skips recompilation.
            // This is critical because parallel tests share a target directory
            // and only the first test to compile a dependency crate triggers
            // actual rustc invocations.
            //
            // On Windows (RUSTC_WRAPPER), rustc inherits the wrapper's stderr
            // handle via bInheritHandles=TRUE in CreateProcessW, so hook
            // output reaches cargo's stderr pipe just like on Linux/macOS.
        }

        // `MACRA_ABI` is the only guard: the hook refuses to touch the table without
        // it. It is derived from the `rustc -vV` resolved above — `$RUSTC`, else the
        // `rustc` on `PATH` — and nothing checks that the compiler cargo actually
        // spawns is that one. Confirmed ways for the two to diverge, each ending in
        // the wrong-stride / wrong-tag rustc abort described on `bridge_abi_for`:
        //   - `CARGO_BUILD_RUSTC` or `build.rustc` in a cargo config (verified:
        //     `CARGO_BUILD_RUSTC=<1.97 rustc> cargo check -v` drove 1.97 while
        //     `rustc -V` said 1.100-nightly);
        //   - `--config build.rustc=...` passed through `cargo_args`;
        //   - a distro `/usr/bin/rustc` ahead of `~/.cargo/bin` on `PATH` when macra
        //     is invoked as a bare binary rather than through the `cargo` proxy;
        //   - a nested `cargo +nightly` from a build script, which inherits this
        //     environment, `MACRA_ABI` included.
        // A `MACRA_RUSTC_VERSION` variable used to be exported alongside, with a
        // comment saying the hook checked it against each dylib's `.rustc` metadata.
        // The hook never read it — only the producing side was ever written — so the
        // variable is gone rather than left implying a check that does not exist.
        match abi {
            Some(abi) => {
                cmd.env("MACRA_ABI", abi.as_env());
            }
            // Never let a value inherited from the caller stand in for a decision
            // this run did not make.
            None => {
                cmd.env_remove("MACRA_ABI");
            }
        }

        // The encoded form keeps a flag that contains a space intact, which a config
        // array can express and a space-separated `RUSTFLAGS` cannot; cargo reads it
        // first. `RUSTFLAGS` is still set for anything else that looks there.
        cmd.env("CARGO_ENCODED_RUSTFLAGS", rustflags.join("\x1f"));
        cmd.env("RUSTFLAGS", rustflags.join(" "));
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("failed to capture stderr"))?;

        let child = Arc::new(Mutex::new(child));
        let child_for_wait = Arc::clone(&child);

        let (tx, rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();

        // Drain stdout in a background thread to prevent the child from blocking.
        // Keep a copy because some cargo/rustc setups emit diagnostics on stdout.
        let stdout_thread = thread::spawn(move || {
            use std::io::Read;
            let mut stdout = stdout;
            let mut collected = String::new();
            let mut buf = [0u8; 4096];
            loop {
                match stdout.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    }
                }
            }
            collected
        });

        // Read stderr: handle hook lines (legacy fallback), collect the rest
        // for trace-macros parsing after the child exits.
        thread::spawn(move || {
            use std::io::BufRead;
            let reader = io::BufReader::new(stderr);
            let mut stderr_buf = String::new();

            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                };
                // Legacy path: hook output on stderr (when MACRA_HOOK_OUTPUT_DIR
                // is not used or the hook falls back to stderr).
                if let Some(json) = line.strip_prefix(HOOK_LINE_PREFIX) {
                    if let Some(expansion) = parse_hook_json(json) {
                        let _ = tx.send(Ok(expansion));
                    }
                } else {
                    stderr_buf.push_str(&line);
                    stderr_buf.push('\n');
                }
            }

            // Wait for stdout draining and child process to finish
            let stdout_buf = stdout_thread.join().unwrap_or_default();
            // Held only from pipe-EOF until the child is reaped, so a concurrent
            // `kill()` never blocks for long.
            let wait_result: io::Result<ExitStatus> = child_for_wait
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .wait();

            // Parse plain-text trace-macros output from stderr and stdout.
            for group in parse_trace(stderr_buf.as_bytes()) {
                for expansion in group.expansions {
                    let _ = tx.send(Ok(expansion));
                }
            }
            for group in parse_trace(stdout_buf.as_bytes()) {
                for expansion in group.expansions {
                    let _ = tx.send(Ok(expansion));
                }
            }

            match wait_result {
                Ok(status) => {
                    let _ = status_tx.send(Ok(CheckResult {
                        success: status.success(),
                        stdout: stdout_buf,
                        stderr: stderr_buf,
                    }));
                }
                Err(e) => {
                    let _ = status_tx.send(Err(e));
                }
            }

            // tx drops here, closing the channel
        });

        Ok(TraceRun {
            iter: MacroExpansionIter { rx },
            check_result: status_rx,
            child,
        })
    }
}

/// Resolving the rustflags cargo would apply on its own, with cargo's precedence.
///
/// There is no stable command that prints the effective list, so it is rebuilt from
/// the parts: `cargo config get` for the config values (the `-Z unstable-options` it
/// needs works on every cargo under `RUSTC_BOOTSTRAP=1`, which the traced check sets
/// anyway; confirmed on 1.86) and `cargo rustc --print cfg` for the target cfg that
/// selects the `target.'cfg(..)'` entries — that one already folds the project's
/// `--cfg` flags in, as cargo itself does when matching. Every failure falls back to
/// "no config flags", i.e. exactly what this did before, so a cargo that lacks one of
/// these cannot break a build that used to work.
impl TraceMacros {
    /// The flags cargo would pass if macra set none.
    fn baseline_rustflags(&self) -> Vec<String> {
        // An environment value shadows the config even when it is empty, so its
        // presence is what counts, not its content.
        if let Some(encoded) = std::env::var_os("CARGO_ENCODED_RUSTFLAGS") {
            return encoded
                .to_string_lossy()
                .split('\x1f')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
        }
        if let Some(flags) = std::env::var_os("RUSTFLAGS") {
            return split_flags(&flags.to_string_lossy());
        }
        match self.config_rustflags() {
            Ok(flags) => flags,
            Err(why) => {
                if std::env::var_os("MACRA_HOOK_DEBUG").is_some() {
                    eprintln!(
                        "[macra] could not resolve the project's rustflags ({why}); any \
                         `build.rustflags` or `target.*.rustflags` it sets is dropped for this run"
                    );
                }
                Vec::new()
            }
        }
    }

    /// `target.<triple>.rustflags` and every matching `target.'cfg(..)'.rustflags`,
    /// or `build.rustflags` when those add up to nothing — the same first-non-empty
    /// rule cargo applies.
    fn config_rustflags(&self) -> Result<Vec<String>, String> {
        let triple = self.requested_target()?;
        let mut flags = Vec::new();
        let mut cfg_entries = Vec::new();
        if let Some(serde_json::Value::Object(table)) = self.cargo_config_get("target")? {
            for (key, entry) in table {
                let list = match entry.get("rustflags") {
                    Some(v) => string_list(v)?,
                    None => continue,
                };
                if key == triple {
                    flags.extend(list);
                } else if key.starts_with("cfg(") {
                    cfg_entries.push((key, list));
                }
            }
        }
        // `cargo config get` only mentions this variable in a note; cargo proper
        // appends it after the file value.
        let env_key = format!(
            "CARGO_TARGET_{}_RUSTFLAGS",
            triple.to_uppercase().replace('-', "_")
        );
        if let Ok(v) = std::env::var(env_key) {
            flags.extend(split_flags(&v));
        }
        if !cfg_entries.is_empty() {
            // cargo walks these in hash order, so no order is more faithful than another.
            cfg_entries.sort();
            let cfg = self.target_cfg()?;
            for (key, list) in cfg_entries {
                match cfg_matches(&key, &cfg) {
                    Some(true) => flags.extend(list),
                    Some(false) => {}
                    None => return Err(format!("cannot evaluate `{key}`")),
                }
            }
        }
        if !flags.is_empty() {
            return Ok(flags);
        }
        match self.cargo_config_get("build.rustflags")? {
            Some(v) => string_list(&v),
            None => Ok(Vec::new()),
        }
    }

    /// The single triple this check builds for: `--target`, else `build.target`
    /// (which `cargo config get` merges with `CARGO_BUILD_TARGET`), else the host.
    fn requested_target(&self) -> Result<String, String> {
        let mut targets: Vec<String> = self
            .cargo_args()
            .into_iter()
            .filter(|(flag, _)| flag == "--target")
            .map(|(_, value)| value)
            .collect();
        if targets.is_empty() {
            targets = match self.cargo_config_get("build.target")? {
                None => Vec::new(),
                Some(serde_json::Value::String(s)) => vec![s],
                Some(serde_json::Value::Array(a)) => a
                    .iter()
                    .map(|v| v.as_str().map(String::from))
                    .collect::<Option<_>>()
                    .ok_or("`build.target` is not a list of strings")?,
                Some(_) => return Err("`build.target` is neither a string nor a list".into()),
            };
        }
        if targets.len() > 1 {
            // Flags differ per target and one environment cannot carry two sets.
            return Err(format!("{} targets requested", targets.len()));
        }
        let Some(target) = targets.pop() else {
            return self.host_triple();
        };
        // A custom spec is named by its file stem in `target.<name>`, as cargo does.
        Ok(match target.strip_suffix(".json") {
            Some(_) => Path::new(&target)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or(target),
            None => target,
        })
    }

    fn host_triple(&self) -> Result<String, String> {
        let out = Command::new(&self.cargo_path)
            .arg("-vV")
            .output()
            .map_err(|e| format!("cargo -vV: {e}"))?;
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|l| l.strip_prefix("host: "))
            .map(|h| h.trim().to_string())
            .ok_or_else(|| "cargo -vV printed no host".to_string())
    }

    /// `--target` and `--config` pairs from `cargo_args`, in both `--x v` and `--x=v`
    /// spellings, so the probes below see what the check will see.
    fn cargo_args(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        let mut it = self.args.cargo_args.iter();
        while let Some(arg) = it.next() {
            for flag in ["--target", "--config"] {
                if arg == flag {
                    if let Some(value) = it.next() {
                        pairs.push((flag.to_string(), value.clone()));
                    }
                } else if let Some(value) = arg.strip_prefix(flag).and_then(|r| r.strip_prefix('='))
                {
                    pairs.push((flag.to_string(), value.to_string()));
                }
            }
        }
        pairs
    }

    /// One config value as `cargo config get --format json-value` reports it, or
    /// `None` when it is not set anywhere.
    fn cargo_config_get(&self, key: &str) -> Result<Option<serde_json::Value>, String> {
        let mut cmd = Command::new(&self.cargo_path);
        cmd.args([
            "config",
            "get",
            "-Z",
            "unstable-options",
            "--format",
            "json-value",
        ]);
        for (flag, value) in self.cargo_args() {
            if flag == "--config" {
                cmd.arg(flag).arg(value);
            }
        }
        cmd.arg(key).env("RUSTC_BOOTSTRAP", "1");
        let out = cmd.output().map_err(|e| format!("cargo config get: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.contains("is not set") {
                return Ok(None);
            }
            return Err(format!("cargo config get {key}: {}", stderr.trim()));
        }
        serde_json::from_slice(&out.stdout)
            .map(Some)
            .map_err(|e| format!("cargo config get {key}: {e}"))
    }

    /// The cfg set cargo matches `target.'cfg(..)'` entries against.
    fn target_cfg(&self) -> Result<Vec<(String, Option<String>)>, String> {
        let mut cmd = Command::new(&self.cargo_path);
        cmd.args(["rustc", "-Z", "unstable-options", "--print", "cfg"]);
        if let Some(ref manifest_path) = self.args.manifest_path {
            cmd.arg("--manifest-path").arg(manifest_path);
        }
        for (flag, value) in self.cargo_args() {
            cmd.arg(flag).arg(value);
        }
        cmd.env("RUSTC_BOOTSTRAP", "1");
        let out = cmd
            .output()
            .map_err(|e| format!("cargo rustc --print cfg: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "cargo rustc --print cfg: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(parse_cfg_lines(&String::from_utf8_lossy(&out.stdout)))
    }
}

/// Space-separated flags the way cargo splits `RUSTFLAGS`.
fn split_flags(s: &str) -> Vec<String> {
    s.split(' ')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// A cargo `StringList`: either an array of strings or one string to split.
fn string_list(v: &serde_json::Value) -> Result<Vec<String>, String> {
    match v {
        serde_json::Value::String(s) => Ok(split_flags(s)),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|i| i.as_str().map(String::from))
            .collect::<Option<_>>()
            .ok_or_else(|| "rustflags list holds a non-string".to_string()),
        _ => Err("rustflags is neither a string nor a list".to_string()),
    }
}

/// `rustc --print cfg` output: one `name` or `name="value"` per line.
fn parse_cfg_lines(s: &str) -> Vec<(String, Option<String>)> {
    s.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| match l.split_once('=') {
            Some((k, v)) => (
                k.to_string(),
                Some(
                    v.trim_matches('"')
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\"),
                ),
            ),
            None => (l.trim().to_string(), None),
        })
        .collect()
}

/// Evaluate a `cfg(..)` config key against a target's cfg set, following
/// cargo-platform: `all`, `any`, `not`, bare names, `name = "value"`, and the
/// literal `true`/`false`. `None` means the key could not be parsed.
fn cfg_matches(key: &str, cfg: &[(String, Option<String>)]) -> Option<bool> {
    #[derive(PartialEq)]
    enum Tok {
        Ident(String),
        Str(String),
        Open,
        Close,
        Comma,
        Eq,
    }
    let inner = key.strip_prefix("cfg(")?.strip_suffix(')')?;
    let mut toks = Vec::new();
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let single = match c {
            '(' => Some(Tok::Open),
            ')' => Some(Tok::Close),
            ',' => Some(Tok::Comma),
            '=' => Some(Tok::Eq),
            _ => None,
        };
        if let Some(tok) = single {
            toks.push(tok);
            i += 1;
            continue;
        }
        match c {
            c if c.is_whitespace() => i += 1,
            '"' => {
                let start = i + 1;
                i = start;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                if i >= chars.len() {
                    return None;
                }
                toks.push(Tok::Str(chars[start..i].iter().collect()));
                i += 1;
            }
            c if c.is_alphanumeric() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                toks.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            _ => return None,
        }
    }

    fn eval(toks: &[Tok], pos: &mut usize, cfg: &[(String, Option<String>)]) -> Option<bool> {
        let name = match toks.get(*pos)? {
            Tok::Ident(n) => n.clone(),
            _ => return None,
        };
        *pos += 1;
        if toks.get(*pos) == Some(&Tok::Open) {
            *pos += 1;
            let mut results = Vec::new();
            loop {
                if toks.get(*pos) == Some(&Tok::Close) {
                    *pos += 1;
                    break;
                }
                results.push(eval(toks, pos, cfg)?);
                match toks.get(*pos)? {
                    Tok::Comma => *pos += 1,
                    Tok::Close => {
                        *pos += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            return match name.as_str() {
                "all" => Some(results.iter().all(|&r| r)),
                "any" => Some(results.iter().any(|&r| r)),
                "not" if results.len() == 1 => Some(!results[0]),
                _ => None,
            };
        }
        if toks.get(*pos) == Some(&Tok::Eq) {
            let value = match toks.get(*pos + 1)? {
                Tok::Str(s) => s.clone(),
                _ => return None,
            };
            *pos += 2;
            return Some(
                cfg.iter()
                    .any(|(k, v)| *k == name && v.as_deref() == Some(&value)),
            );
        }
        match name.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => Some(cfg.iter().any(|(k, v)| *k == name && v.is_none())),
        }
    }

    let mut pos = 0;
    let result = eval(&toks, &mut pos, cfg)?;
    (pos == toks.len()).then_some(result)
}

#[cfg(target_os = "macos")]
fn create_macos_linker_wrapper() -> io::Result<PathBuf> {
    // Building the wrapper shells out to `cc`, and `run()` is called again on the UI
    // thread on every `r` reload. Reuse the binary this process already built.
    let mut cache = LINKER_WRAPPER_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(path) = cache.as_deref() {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
    }
    let path = build_macos_linker_wrapper()?;
    *cache = Some(path.clone());
    Ok(path)
}

#[cfg(target_os = "macos")]
fn build_macos_linker_wrapper() -> io::Result<PathBuf> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let unique = LINKER_WRAPPER_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let bin_path = dir.join(format!(
        "cargo-macra-linker-wrapper-{}-{}",
        std::process::id(),
        unique
    ));

    // Compile a native arm64 binary wrapper instead of a shell script.
    // On newer macOS, system binaries like /bin/sh are arm64e.  dyld refuses
    // to inject an arm64 dylib into an arm64e process, so the shell script
    // approach dies with SIGABRT before the `unset` line ever runs.
    // A compiled arm64 binary can load the arm64 hook dylib harmlessly, then
    // unset DYLD_INSERT_LIBRARIES before exec-ing the real (arm64e) linker.
    let c_src = concat!(
        "#include <stdlib.h>\n",
        "#include <unistd.h>\n",
        "#include <string.h>\n",
        "int main(int argc, char *argv[]) {\n",
        "    (void)argc;\n",
        "    unsetenv(\"DYLD_INSERT_LIBRARIES\");\n",
        "    if (argv[1]) {\n",
        "        const char *b = strrchr(argv[1], '/');\n",
        "        if (!b) b = argv[1]; else b++;\n",
        "        if (strcmp(b,\"cc\")==0||strcmp(b,\"clang\")==0||strcmp(b,\"gcc\")==0) {\n",
        "            execvp(argv[1], argv+1);\n",
        "            _exit(127);\n",
        "        }\n",
        "    }\n",
        "    argv[0] = \"/usr/bin/cc\";\n",
        "    execvp(\"/usr/bin/cc\", argv);\n",
        "    _exit(127);\n",
        "}\n",
    );

    let src_path = dir.join(format!(
        "cargo-macra-linker-wrapper-{}-{}.c",
        std::process::id(),
        unique
    ));
    fs::write(&src_path, c_src)?;
    let compile = std::process::Command::new("cc")
        .arg("-o")
        .arg(&bin_path)
        .arg(&src_path)
        // Reload (`r`) runs this while the TUI owns the screen; inherited stdio would
        // let any cc diagnostic scribble over the alternate screen.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let _ = fs::remove_file(&src_path);

    if let Ok(st) = compile {
        if st.success() {
            return Ok(bin_path);
        }
    }

    // Fallback: shell script (works on systems where /bin/sh is arm64).
    let script_path = dir.join(format!(
        "cargo-macra-linker-wrapper-{}-{}.sh",
        std::process::id(),
        unique
    ));
    let script = r#"#!/bin/sh
unset DYLD_INSERT_LIBRARIES
if [ "$#" -gt 0 ]; then
  case "$1" in
    */cc|cc|*/clang|clang|*/gcc|gcc)
      linker="$1"
      shift
      exec "$linker" "$@"
      ;;
  esac
fi
exec /usr/bin/cc "$@"
"#;
    fs::write(&script_path, script)?;
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    Ok(script_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hook_json_bang() {
        let json = r#"{"name":"println","kind":"Bang","arguments":"","input":"println!(\"hello\")","output":"{ ::std::io::_print(format_args!(\"hello\\n\")); }"}"#;
        let exp = parse_hook_json(json).unwrap();
        assert_eq!(exp.name, "println");
        assert_eq!(exp.kind, MacroExpansionKind::Bang);
        assert_eq!(exp.expanding, "println!(\"hello\")");
    }

    #[test]
    fn test_parse_hook_json_derive() {
        let json = r#"{"name":"Debug","kind":"CustomDerive","arguments":"","input":"struct Foo {}","output":"impl Debug for Foo {}"}"#;
        let exp = parse_hook_json(json).unwrap();
        assert_eq!(exp.name, "Debug");
        assert_eq!(exp.kind, MacroExpansionKind::Derive);
        assert_eq!(exp.expanding, "Debug");
    }

    #[test]
    fn test_parse_hook_json_attribute() {
        // Input contains '{' so expanding == input (not wrapped)
        let json = r#"{"name":"test","kind":"Attr","arguments":"","input":"fn foo() {}","output":"fn foo() { /* test */ }"}"#;
        let exp = parse_hook_json(json).unwrap();
        assert_eq!(exp.name, "test");
        assert_eq!(exp.kind, MacroExpansionKind::Attribute);
        assert_eq!(exp.expanding, "fn foo() {}");
    }

    #[test]
    fn test_parse_hook_json_attribute_simple_input() {
        // Input without '(' or '{' gets wrapped as "name { input }"
        let json = r#"{"name":"cfg","kind":"Attr","arguments":"","input":"feature = \"foo\"","output":""}"#;
        let exp = parse_hook_json(json).unwrap();
        assert_eq!(exp.name, "cfg");
        assert_eq!(exp.kind, MacroExpansionKind::Attribute);
        assert_eq!(exp.expanding, "cfg { feature = \"foo\" }");
    }

    #[test]
    fn test_parse_hook_json_invalid() {
        assert!(parse_hook_json("not json").is_none());
    }

    fn cfg(pairs: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.map(String::from)))
            .collect()
    }

    #[test]
    fn cfg_keys_evaluate_like_cargo_platform() {
        let linux = cfg(&[
            ("unix", None),
            ("target_os", Some("linux")),
            ("target_has_atomic", None),
            ("target_has_atomic", Some("64")),
        ]);
        // `cfg(all())` is the idiom for "every target", and the common reason a
        // project's flags live under `target.*` instead of `build`.
        assert_eq!(cfg_matches("cfg(all())", &linux), Some(true));
        assert_eq!(cfg_matches("cfg(any())", &linux), Some(false));
        assert_eq!(cfg_matches("cfg(unix)", &linux), Some(true));
        assert_eq!(cfg_matches("cfg(windows)", &linux), Some(false));
        assert_eq!(cfg_matches("cfg(not(windows))", &linux), Some(true));
        assert_eq!(
            cfg_matches(r#"cfg(all(unix, target_os = "linux"))"#, &linux),
            Some(true)
        );
        assert_eq!(
            cfg_matches(r#"cfg(any(windows, target_os="macos"))"#, &linux),
            Some(false)
        );
        // A bare name and a key-value pair with that name are distinct entries.
        assert_eq!(
            cfg_matches(r#"cfg(target_has_atomic = "64")"#, &linux),
            Some(true)
        );
        assert_eq!(
            cfg_matches(r#"cfg(target_has_atomic = "128")"#, &linux),
            Some(false)
        );
        assert_eq!(cfg_matches("cfg(all(unix, ))", &linux), Some(true));
        assert_eq!(cfg_matches("cfg(true)", &[]), Some(true));
        // Anything unparsable is reported, not guessed.
        for bad in [
            "cfg(all(unix",
            "unix",
            "cfg(not(a, b))",
            "cfg(a = b)",
            "cfg()",
        ] {
            assert_eq!(cfg_matches(bad, &linux), None, "{bad:?}");
        }
    }

    #[test]
    fn config_lists_accept_both_spellings() {
        assert_eq!(
            string_list(&serde_json::json!("-C  target-cpu=native")).unwrap(),
            ["-C", "target-cpu=native"]
        );
        assert_eq!(
            string_list(&serde_json::json!(["--cfg", "a b"])).unwrap(),
            ["--cfg", "a b"]
        );
        assert!(string_list(&serde_json::json!(1)).is_err());
        assert_eq!(
            parse_cfg_lines("unix\ntarget_os=\"linux\"\n\n"),
            cfg(&[("unix", None), ("target_os", Some("linux"))])
        );
    }

    /// A throwaway crate that compiles only when `need_me` is set, checked through
    /// `run()` with the flag supplied the way a project's `.cargo/config.toml` would
    /// (via `--config`, since config discovery follows the process cwd, which the
    /// tests cannot change per thread). Setting `RUSTFLAGS` from the environment made
    /// cargo drop the config value, so before the fix this crate failed to build under
    /// macra and nowhere else.
    fn check_needs_cfg_from(config: &str) -> (Vec<MacroExpansion>, CheckResult) {
        static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cargo-macra-rustflags-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"needs_cfg\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "#[cfg(not(need_me))]\ncompile_error!(\"build.rustflags from the config were dropped\");\n\
             macro_rules! seen { () => { 1 } }\npub const X: i32 = seen!();\n",
        )
        .unwrap();
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let args = Args {
            manifest_path: Some(dir.join("Cargo.toml").to_string_lossy().into_owned()),
            cargo_args: vec![
                "--config".into(),
                config.into(),
                "--target-dir".into(),
                dir.join("target").to_string_lossy().into_owned(),
            ],
            ..Args::default()
        };
        let run = TraceMacros::new(Path::new(&cargo), &args)
            .run()
            .expect("spawn cargo");
        // Draining also lets the reader thread reap the child.
        let expansions = run.iter.filter_map(Result::ok).collect();
        let result = run.check_result.recv().unwrap().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        (expansions, result)
    }

    /// The config flags must survive, and `-Z trace-macros` must still be on top of
    /// them: a merge that kept the project's flags but lost ours would silently
    /// remove the `macro_rules!` half of the tool instead.
    fn assert_config_kept_and_traced(config: &str) {
        let (expansions, result) = check_needs_cfg_from(config);
        assert!(result.success, "stderr:\n{}", result.stderr);
        assert!(
            expansions.iter().any(|e| e.name == "seen"),
            "`seen!` was not traced; got {:?}",
            expansions.iter().map(|e| &e.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn build_rustflags_from_the_config_survive() {
        assert_config_kept_and_traced(r#"build.rustflags=["--cfg","need_me"]"#);
    }

    #[test]
    fn target_cfg_rustflags_from_the_config_survive() {
        assert_config_kept_and_traced(r#"target."cfg(all())".rustflags=["--cfg","need_me"]"#);
    }
}
