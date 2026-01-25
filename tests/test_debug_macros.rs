//! Integration test for debug_macros / debug_target.
//!
//! Part 1: Compile-time checks — verifies generated symbols are accessible.
//! Part 2: Expansion output checks — runs `cargo-macra --show-expansion --no-hook`
//!         on debug-target and verifies the output blocks.

use std::path::PathBuf;
use std::process::Command;

// =========================================================================
// Helpers (same pattern as show_expansion.rs)
// =========================================================================

fn cargo_macra_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cargo-macra"))
}

fn debug_target_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/debug_target/Cargo.toml")
}

#[derive(Debug)]
struct ExpansionBlock {
    caller: String,
    input: String,
    output: String,
}

fn parse_expansion_blocks(stdout: &str) -> Vec<ExpansionBlock> {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(caller) = line.strip_prefix("== ").and_then(|s| s.strip_suffix(" ==")) {
            let caller = caller.to_string();
            i += 1;

            let mut input_lines = Vec::new();
            while i < lines.len() && lines[i] != "---" {
                input_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() && lines[i] == "---" {
                i += 1;
            }

            let mut output_lines = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                if l.starts_with("== ") && l.ends_with(" ==") {
                    break;
                }
                output_lines.push(l);
                i += 1;
            }
            while output_lines.last().is_some_and(|l| l.is_empty()) {
                output_lines.pop();
            }

            blocks.push(ExpansionBlock {
                caller,
                input: input_lines.join("\n"),
                output: output_lines.join("\n"),
            });
        } else {
            i += 1;
        }
    }

    blocks
}

fn find_blocks<'a>(blocks: &'a [ExpansionBlock], caller: &str) -> Vec<&'a ExpansionBlock> {
    blocks.iter().filter(|b| b.caller == caller).collect()
}

// =========================================================================
// Part 1: Compile-time symbol checks
// =========================================================================

#[test]
fn fnlike_alpha_and_beta_are_distinct() {
    assert_eq!(
        debug_target::case_fnlike::__TRACE_FNLIKE_alpha,
        "fnlike:alpha"
    );
    assert_eq!(
        debug_target::case_fnlike::__TRACE_FNLIKE_beta,
        "fnlike:beta"
    );
}

#[test]
fn fnlike_generated_macro_rules_invoked() {
    assert_eq!(
        debug_target::case_fnlike::__TRACE_GENERATED_alpha,
        "generated:alpha:from_alpha"
    );
    assert_eq!(
        debug_target::case_fnlike::__TRACE_GENERATED_beta,
        "generated:beta:from_beta"
    );
}

#[test]
fn attr_different_targets_are_distinct() {
    assert_eq!(
        debug_target::case_attr_different_targets::__TRACE_ATTR_alpha_first_FOR_A1,
        "attr:alpha:first:A1"
    );
    assert_eq!(
        debug_target::case_attr_different_targets::__TRACE_ATTR_alpha_first_FOR_A2,
        "attr:alpha:first:A2"
    );
}

#[test]
fn attr_same_target_different_instances() {
    assert_eq!(
        debug_target::case_attr_same_target::__TRACE_ATTR_alpha_first_FOR_A,
        "attr:alpha:first:A"
    );
    assert_eq!(
        debug_target::case_attr_same_target::__TRACE_ATTR_alpha_second_FOR_A,
        "attr:alpha:second:A"
    );
}

#[test]
fn derive_different_targets_are_distinct() {
    assert_eq!(
        debug_target::case_derive::__TRACE_DERIVE_alpha_first_FOR_D1,
        "derive:alpha:first:D1"
    );
    assert_eq!(
        debug_target::case_derive::__TRACE_DERIVE_alpha_second_FOR_D2,
        "derive:alpha:second:D2"
    );
}

#[test]
fn mixed_all_kinds_coexist() {
    assert_eq!(
        debug_target::case_mixed::__TRACE_FNLIKE_gamma,
        "fnlike:gamma"
    );
    assert_eq!(
        debug_target::case_mixed::__TRACE_GENERATED_gamma,
        "generated:gamma:from_gamma"
    );
    assert_eq!(
        debug_target::case_mixed::__TRACE_FNLIKE_delta,
        "fnlike:delta"
    );
    assert_eq!(
        debug_target::case_mixed::__TRACE_GENERATED_delta,
        "generated:delta:from_delta"
    );
    assert_eq!(
        debug_target::case_mixed::__TRACE_ATTR_beta_one_FOR_M1,
        "attr:beta:one:M1"
    );
    assert_eq!(
        debug_target::case_mixed::__TRACE_DERIVE_beta_two_FOR_M2,
        "derive:beta:two:M2"
    );
}

#[test]
fn decl_macro_baseline() {
    assert_eq!(debug_target::case_decl_macro::__TRACE_DECL, "decl:alpha");
    assert_eq!(
        debug_target::case_decl_macro::sub::__TRACE_DECL,
        "decl:beta"
    );
}

// =========================================================================
// Part 2: cargo-macra --show-expansion --no-hook output checks
//
// Note: --no-hook uses -Z trace-macros, which only captures function-like
// (bang-style) macro invocations. Attribute and derive proc macros are NOT
// visible in this mode, but macro_rules! generated BY proc macros ARE.
// =========================================================================

/// Run cargo-macra and return parsed expansion blocks.
fn run_show_expansion() -> Vec<ExpansionBlock> {
    let output = Command::new(cargo_macra_bin())
        .arg("--show-expansion")
        .arg("--manifest-path")
        .arg(debug_target_manifest())
        .output()
        .expect("failed to run cargo-macra");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "cargo-macra failed.\nstderr:\n{}",
        stderr
    );

    let blocks = parse_expansion_blocks(&stdout);
    assert!(
        !blocks.is_empty(),
        "Expected at least one expansion block.\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    blocks
}

#[test]
fn show_expansion_generated_macro_rules_alpha() {
    let blocks = run_show_expansion();

    // __trace_generated_alpha!(from_alpha) — macro_rules! generated by emit_trace!(alpha)
    let alpha = find_blocks(&blocks, "__trace_generated_alpha!");
    assert!(
        !alpha.is_empty(),
        "Expected __trace_generated_alpha! block.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        alpha[0].input.contains("from_alpha"),
        "__trace_generated_alpha! input should contain 'from_alpha'.\nblock: {:?}",
        alpha[0]
    );
    assert!(
        alpha[0].output.contains("__TRACE_GENERATED_alpha")
            && alpha[0].output.contains("generated:")
            && alpha[0].output.contains("alpha"),
        "__trace_generated_alpha! output should contain the generated const.\nblock: {:?}",
        alpha[0]
    );
}

#[test]
fn show_expansion_generated_macro_rules_beta() {
    let blocks = run_show_expansion();

    // __trace_generated_beta!(from_beta) — macro_rules! generated by emit_trace!(beta)
    let beta = find_blocks(&blocks, "__trace_generated_beta!");
    assert!(
        !beta.is_empty(),
        "Expected __trace_generated_beta! block.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        beta[0].input.contains("from_beta"),
        "__trace_generated_beta! input should contain 'from_beta'.\nblock: {:?}",
        beta[0]
    );
    assert!(
        beta[0].output.contains("__TRACE_GENERATED_beta")
            && beta[0].output.contains("generated:")
            && beta[0].output.contains("beta"),
        "__trace_generated_beta! output should contain the generated const.\nblock: {:?}",
        beta[0]
    );
}

#[test]
fn show_expansion_generated_macro_rules_are_distinct() {
    let blocks = run_show_expansion();

    let alpha = find_blocks(&blocks, "__trace_generated_alpha!");
    let beta = find_blocks(&blocks, "__trace_generated_beta!");

    assert!(!alpha.is_empty(), "Missing __trace_generated_alpha! block");
    assert!(!beta.is_empty(), "Missing __trace_generated_beta! block");

    // Inputs must differ — alpha receives from_alpha, beta receives from_beta
    assert!(
        alpha[0].input.contains("from_alpha"),
        "alpha input should contain from_alpha"
    );
    assert!(
        beta[0].input.contains("from_beta"),
        "beta input should contain from_beta"
    );
    assert_ne!(
        alpha[0].input, beta[0].input,
        "alpha and beta inputs should differ"
    );

    // The outputs must differ too
    assert_ne!(
        alpha[0].output, beta[0].output,
        "alpha and beta generated macro outputs should differ"
    );
}

#[test]
fn show_expansion_generated_macro_rules_gamma_delta() {
    let blocks = run_show_expansion();

    // gamma and delta come from case_mixed
    let gamma = find_blocks(&blocks, "__trace_generated_gamma!");
    let delta = find_blocks(&blocks, "__trace_generated_delta!");

    assert!(!gamma.is_empty(), "Missing __trace_generated_gamma! block");
    assert!(!delta.is_empty(), "Missing __trace_generated_delta! block");

    assert!(
        gamma[0].output.contains("__TRACE_GENERATED_gamma"),
        "gamma output should contain its const name.\nblock: {:?}",
        gamma[0]
    );
    assert!(
        delta[0].output.contains("__TRACE_GENERATED_delta"),
        "delta output should contain its const name.\nblock: {:?}",
        delta[0]
    );
    assert_ne!(
        gamma[0].output, delta[0].output,
        "gamma and delta outputs should differ"
    );
}

#[test]
fn show_expansion_decl_trace_alpha_and_beta() {
    let blocks = run_show_expansion();

    // decl_trace!(alpha) and decl_trace!(beta) from case_decl_macro
    let decl = find_blocks(&blocks, "decl_trace!");
    assert!(
        decl.len() >= 2,
        "Expected at least 2 decl_trace! blocks, found {}.\nall callers: {:?}",
        decl.len(),
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );

    let decl_alpha = decl.iter().find(|b| b.input.trim() == "alpha");
    let decl_beta = decl.iter().find(|b| b.input.trim() == "beta");

    assert!(
        decl_alpha.is_some(),
        "Expected a decl_trace! block with input 'alpha'.\ndecl blocks: {:?}",
        decl
    );
    assert!(
        decl_beta.is_some(),
        "Expected a decl_trace! block with input 'beta'.\ndecl blocks: {:?}",
        decl
    );

    // Both should produce __TRACE_DECL const
    let alpha_out = &decl_alpha.unwrap().output;
    let beta_out = &decl_beta.unwrap().output;
    assert!(
        alpha_out.contains("__TRACE_DECL"),
        "decl_trace!(alpha) output should contain __TRACE_DECL.\noutput: {}",
        alpha_out
    );
    assert!(
        beta_out.contains("__TRACE_DECL"),
        "decl_trace!(beta) output should contain __TRACE_DECL.\noutput: {}",
        beta_out
    );

    // Inputs are distinct
    assert_ne!(
        decl_alpha.unwrap().input.trim(),
        decl_beta.unwrap().input.trim(),
        "decl_trace alpha and beta should have different inputs"
    );
}

#[test]
fn show_expansion_all_generated_macros_present() {
    let blocks = run_show_expansion();

    // Verify all 4 generated macro_rules! from emit_trace! are captured
    let expected = [
        "__trace_generated_alpha!",
        "__trace_generated_beta!",
        "__trace_generated_gamma!",
        "__trace_generated_delta!",
    ];
    for name in &expected {
        let found = find_blocks(&blocks, name);
        assert!(
            !found.is_empty(),
            "Missing expansion block for {}.\nall callers: {:?}",
            name,
            blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
        );
    }
}
