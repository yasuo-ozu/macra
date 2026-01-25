//! JSON Lines output for captured proc macro expansions.

use serde::Serialize;
use std::io::Write;

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

/// Write an expansion record to stderr.
pub fn log_expansion(record: &ExpansionRecord) {
    let json = match serde_json::to_string(record) {
        Ok(s) => s,
        Err(_) => return,
    };

    let _ = writeln!(std::io::stderr(), "{}{}", HOOK_LINE_PREFIX, json);
}
