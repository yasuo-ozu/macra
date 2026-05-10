//! JSON Lines output for captured proc macro expansions.

use serde::Serialize;
use std::io::Write;
use std::sync::OnceLock;

/// Prefix used to identify hook output lines in stderr.
pub const HOOK_LINE_PREFIX: &str = "__MACRA_HOOK__:";

/// A single proc macro expansion record written as JSON Lines
#[derive(Serialize)]
pub struct ExpansionRecord {
    pub name: String,
    pub kind: String,
    pub arguments: String,
    pub input: String,
    pub output: String,
}

/// Cached output file path for file-based logging.
/// When `MACRA_HOOK_OUTPUT_DIR` is set, each process writes to its own
/// `{pid}.jsonl` file inside that directory, avoiding pipe-buffer atomicity
/// issues that cause interleaved output when multiple rustc processes write
/// to the same stderr pipe.
static OUTPUT_FILE: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();

fn output_file_path() -> &'static Option<std::path::PathBuf> {
    OUTPUT_FILE.get_or_init(|| {
        let dir = std::env::var_os("MACRA_HOOK_OUTPUT_DIR")?;
        let path = std::path::PathBuf::from(dir)
            .join(format!("{}.jsonl", std::process::id()));
        Some(path)
    })
}

/// Write an expansion record.
///
/// If `MACRA_HOOK_OUTPUT_DIR` is set, writes to a per-process file in that
/// directory.  Otherwise falls back to stderr with the `__MACRA_HOOK__:`
/// prefix (original behaviour).
pub fn log_expansion(record: &ExpansionRecord) {
    let json = match serde_json::to_string(record) {
        Ok(s) => s,
        Err(_) => return,
    };

    if let Some(path) = output_file_path() {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{}", json);
            return;
        }
    }

    // Fallback: write to stderr with prefix
    let _ = writeln!(std::io::stderr(), "{}{}", HOOK_LINE_PREFIX, json);
}
