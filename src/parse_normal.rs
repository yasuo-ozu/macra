use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CargoMessage {
    reason: String,
}

/// Cargo status line prefixes that should be filtered out
const CARGO_STATUS_PREFIXES: &[&str] = &[
    "   Compiling ",
    "    Checking ",
    "    Finished ",
    "     Running ",
    "       Fresh ",
    "   Documenting ",
    "     Locking ",
    "    Updating ",
    "  Downloading ",
    "   Downloaded ",
];

/// Parses the output of `cargo +nightly rustc --message-format=json -- -Z unpretty=normal`
/// and returns the normalized source code.
///
/// The output contains JSON messages from cargo (with a "reason" field) interleaved with
/// the unpretty source code. This function filters out the JSON messages and cargo status
/// lines, returning only the source code.
pub fn parse_normal_output(output: &str) -> String {
    let mut source_lines = Vec::new();

    for line in output.lines() {
        // Try to parse as JSON cargo message
        if let Ok(msg) = serde_json::from_str::<CargoMessage>(line) {
            // This is a cargo JSON message, skip it
            let _ = msg.reason;
            continue;
        }

        // Skip cargo status lines
        if CARGO_STATUS_PREFIXES.iter().any(|prefix| line.starts_with(prefix)) {
            continue;
        }

        // Not a cargo JSON message or status line, this is source code
        source_lines.push(line);
    }

    source_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_normal_output() {
        let output = r#"fn main() { println!("Hello, world!"); }
{"reason":"compiler-artifact","package_id":"test","manifest_path":"/test/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"test","src_path":"/test/src/main.rs","edition":"2024","doc":true,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":[],"executable":"/test/target/debug/test","fresh":false}
{"reason":"build-finished","success":true}"#;

        let result = parse_normal_output(output);
        assert_eq!(result, r#"fn main() { println!("Hello, world!"); }"#);
    }

    #[test]
    fn test_parse_multiline_source() {
        let output = r#"fn main() {
    println!("Hello");
}
{"reason":"build-finished","success":true}"#;

        let result = parse_normal_output(output);
        assert_eq!(
            result,
            r#"fn main() {
    println!("Hello");
}"#
        );
    }

    #[test]
    fn test_parse_with_cargo_status_lines() {
        let output = r#"{"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#proc-macro2@1.0.103","manifest_path":"/test/Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"proc_macro2","src_path":"/test/src/lib.rs","edition":"2021","doc":true,"doctest":true,"test":true},"profile":{"opt_level":"0","debuginfo":0,"debug_assertions":true,"overflow_checks":true,"test":false},"features":[],"filenames":[],"executable":null,"fresh":true}
   Compiling myproject v0.1.0 (/home/user/myproject)
use std::fmt::Debug;

pub struct MyStruct {
    value: i32,
}

impl Debug for MyStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyStruct({})", self.value)
    }
}
{"reason":"compiler-artifact","package_id":"path+file:///home/user/myproject#0.1.0","manifest_path":"/home/user/myproject/Cargo.toml","target":{"kind":["test"],"crate_types":["bin"],"name":"test","src_path":"/home/user/myproject/tests/test.rs","edition":"2021","doc":false,"doctest":false,"test":true},"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":true},"features":[],"filenames":[],"executable":"/home/user/myproject/target/debug/deps/test-abc123","fresh":false}
{"reason":"build-finished","success":true}
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s"#;

        let result = parse_normal_output(output);
        assert_eq!(
            result,
            r#"use std::fmt::Debug;

pub struct MyStruct {
    value: i32,
}

impl Debug for MyStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyStruct({})", self.value)
    }
}"#
        );
    }
}
