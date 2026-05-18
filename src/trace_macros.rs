use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::sync::atomic::{AtomicU64, Ordering};

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
static HOOK_OUTPUT_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "macos")]
static LINKER_WRAPPER_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(serde::Deserialize)]
struct HookRecord {
    name: String,
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

        // Append -Z trace-macros to existing RUSTFLAGS
        let mut rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
        if !rustflags.is_empty() {
            rustflags.push(' ');
        }
        rustflags.push_str("-Z trace-macros");
        cmd.env("RUSTFLAGS", rustflags);

        // Set up macra-hook via LD_PRELOAD if available
        let hook_output_dir = if !self.args.hook_lib.as_os_str().is_empty() {
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
                if let Some(wrapper_exe) = crate::find_wrapper_exe(
                    std::env::current_exe().ok().as_deref(),
                ) {
                    cmd.env("RUSTC_WRAPPER", &wrapper_exe);
                    cmd.env("MACRA_HOOK_DLL_PATH", &lib);
                }
            } else {
                cmd.env("LD_PRELOAD", &lib);
            }

            // Direct hook output to per-process files in a temp directory
            // instead of stderr, avoiding pipe-buffer atomicity issues when
            // multiple concurrent rustc processes write large JSON lines.
            // Use an atomic counter to guarantee unique dir names when
            // multiple tests run in parallel within the same process.
            let seq = HOOK_OUTPUT_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "macra-hook-output-{}-{}-{}",
                std::process::id(),
                seq,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::create_dir_all(&dir);
            cmd.env("MACRA_HOOK_OUTPUT_DIR", &dir);
            Some(dir)
        } else {
            None
        };

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "failed to capture stderr"))?;

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
            let wait_result: io::Result<ExitStatus> = child.wait();

            // Read hook output from the per-process files written by the hook
            // library.  Each rustc process writes to {pid}.jsonl.
            if let Some(ref dir) = hook_output_dir {
                let mut hook_file_count = 0u32;
                let mut hook_line_count = 0u32;
                let mut hook_expansion_count = 0u32;
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().is_some_and(|e| e == "jsonl") {
                            hook_file_count += 1;
                            if let Ok(contents) = std::fs::read_to_string(&path) {
                                for line in contents.lines() {
                                    hook_line_count += 1;
                                    if let Some(expansion) = parse_hook_json(line) {
                                        hook_expansion_count += 1;
                                        let _ = tx.send(Ok(expansion));
                                    }
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("[macra-debug] read_dir({}) failed", dir.display());
                }
                eprintln!(
                    "[macra-debug] hook_output_dir={} files={} lines={} expansions={}",
                    dir.display(), hook_file_count, hook_line_count, hook_expansion_count,
                );
                let _ = std::fs::remove_dir_all(dir);
            }

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
        })
    }
}

#[cfg(target_os = "macos")]
fn create_macos_linker_wrapper() -> io::Result<PathBuf> {
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
}
