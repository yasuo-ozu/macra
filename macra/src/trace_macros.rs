use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
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
    /// When set, the library is injected via `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES`.
    pub hook_lib: Option<PathBuf>,
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
    pub fn run(&self) -> io::Result<MacroExpansionIter> {
        let mut cmd = Command::new(&self.cargo_path);
        cmd.arg("check").arg("--message-format=json");
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
        if let Some(ref lib) = self.args.hook_lib {
            let lib = lib.canonicalize().unwrap_or_else(|_| lib.clone());
            if cfg!(target_os = "macos") {
                cmd.env("DYLD_INSERT_LIBRARIES", &lib);
            } else {
                cmd.env("LD_PRELOAD", &lib);
            }
        }

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

        // Read stderr for hook lines
        let tx_stderr = tx.clone();
        let stderr_thread = thread::spawn(move || {
            use std::io::BufRead;
            let reader = io::BufReader::new(stderr);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = tx_stderr.send(Err(e));
                        break;
                    }
                };
                if let Some(json) = line.strip_prefix(HOOK_LINE_PREFIX) {
                    if let Some(expansion) = parse_hook_json(json) {
                        let _ = tx_stderr.send(Ok(expansion));
                    }
                }
            }
        });

        // Read stdout for --message-format=json compiler messages
        thread::spawn(move || {
            use std::io::BufRead;
            let reader = io::BufReader::new(stdout);
            let mut rendered_buf = String::new();

            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                // Extract "rendered" field from cargo JSON messages
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(rendered) = msg
                        .get("message")
                        .and_then(|m| m.get("rendered"))
                        .and_then(|r| r.as_str())
                    {
                        rendered_buf.push_str(rendered);
                    }
                }
            }

            // Wait for stderr thread to finish
            let _ = stderr_thread.join();

            // Wait for child to finish
            let _ = child.wait();

            // Parse all rendered compiler output for trace-macros
            for group in parse_trace(rendered_buf.as_bytes()) {
                for expansion in group.expansions {
                    let _ = tx.send(Ok(expansion));
                }
            }

            // tx drops here, closing the channel
        });

        Ok(MacroExpansionIter { rx })
    }
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
