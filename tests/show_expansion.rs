use std::path::PathBuf;
use std::process::Command;

fn cargo_macra_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cargo-macra"))
}

fn test_usage_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test-usage/Cargo.toml")
}

fn test_usage_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test-usage/src/lib.rs")
}

/// A parsed expansion block from `--show-expansion` output.
#[derive(Debug)]
struct ExpansionBlock {
    caller: String,
    input: String,
    output: String,
}

/// Parse the `--show-expansion` stdout into structured blocks.
///
/// Each block has the format:
///   == caller ==
///   input (may be empty / multi-line)
///   ---
///   output (may be multi-line)
fn parse_expansion_blocks(stdout: &str) -> Vec<ExpansionBlock> {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(caller) = line.strip_prefix("== ").and_then(|s| s.strip_suffix(" ==")) {
            let caller = caller.to_string();
            i += 1;

            // Collect input lines until "---"
            let mut input_lines = Vec::new();
            while i < lines.len() && lines[i] != "---" {
                input_lines.push(lines[i]);
                i += 1;
            }

            // Skip the "---" separator
            if i < lines.len() && lines[i] == "---" {
                i += 1;
            }

            // Collect output lines until next "== ... ==" or EOF
            let mut output_lines = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                if l.starts_with("== ") && l.ends_with(" ==") {
                    break;
                }
                output_lines.push(l);
                i += 1;
            }

            // Trim trailing empty lines from output
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

/// Find all blocks with a given caller name.
fn find_blocks<'a>(blocks: &'a [ExpansionBlock], caller: &str) -> Vec<&'a ExpansionBlock> {
    blocks.iter().filter(|b| b.caller == caller).collect()
}

#[test]
fn show_expansion() {
    // Touch the source to force recompilation (the hook only fires during
    // actual compilation, not when the crate is cached).
    filetime::set_file_mtime(test_usage_lib(), filetime::FileTime::now())
        .expect("failed to touch test-usage/src/lib.rs");

    let output = Command::new(cargo_macra_bin())
        .arg("--show-expansion")
        .arg("--manifest-path")
        .arg(test_usage_manifest())
        .output()
        .expect("failed to run cargo-macra");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(
        output.status.success(),
        "cargo-macra failed.\nstderr:\n{}",
        stderr
    );

    let blocks = parse_expansion_blocks(&stdout);
    assert!(
        !blocks.is_empty(),
        "Expected at least one expansion block.\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );

    // ---------------------------------------------------------------
    // All macro types are trapped
    // ---------------------------------------------------------------

    // macro_rules!
    assert!(
        blocks.iter().any(|b| b.caller == "repeat_twice!"),
        "Missing macro_rules! expansion (repeat_twice!).\nstdout:\n{}",
        stdout
    );

    // proc_macro (function-like)
    assert!(
        blocks.iter().any(|b| b.caller == "make_answer!"),
        "Missing proc_macro (bang) expansion (make_answer!).\nstdout:\n{}",
        stdout
    );

    // proc_macro_attribute
    assert!(
        blocks.iter().any(|b| b.caller == "#[add_hello_method]"),
        "Missing proc_macro_attribute expansion (#[add_hello_method]).\nstdout:\n{}",
        stdout
    );

    // proc_macro_derive
    assert!(
        blocks.iter().any(|b| b.caller == "#[derive(Greet)]"),
        "Missing proc_macro_derive expansion (#[derive(Greet)]).\nstdout:\n{}",
        stdout
    );

    // macro_rules! emitted by a proc macro
    assert!(
        blocks.iter().any(|b| b.caller == "mystruct_hello!"),
        "Missing macro_rules! emitted by proc macro (mystruct_hello!).\nstdout:\n{}",
        stdout
    );

    // ---------------------------------------------------------------
    // macro_rules!: repeat_twice!
    // ---------------------------------------------------------------
    let repeat = find_blocks(&blocks, "repeat_twice!");
    assert_eq!(
        repeat.len(),
        1,
        "Expected exactly 1 repeat_twice! block, found {}.\nstdout:\n{}",
        repeat.len(),
        stdout
    );
    assert_eq!(
        repeat[0].input.trim(),
        "get_answer()",
        "repeat_twice! input mismatch.\nblock: {:?}",
        repeat[0]
    );
    assert_eq!(
        repeat[0].output.trim(),
        "(get_answer(), get_answer())",
        "repeat_twice! output mismatch.\nblock: {:?}",
        repeat[0]
    );

    // ---------------------------------------------------------------
    // macro_rules!: make_struct!
    // ---------------------------------------------------------------
    let mks = find_blocks(&blocks, "make_struct!");
    assert_eq!(
        mks.len(),
        1,
        "Expected exactly 1 make_struct! block, found {}.\nstdout:\n{}",
        mks.len(),
        stdout
    );
    assert_eq!(
        mks[0].input.trim(),
        "AutoGreeter",
        "make_struct! input mismatch.\nblock: {:?}",
        mks[0]
    );
    assert_eq!(
        mks[0].output.trim(),
        "#[derive(Greet)] pub struct AutoGreeter;",
        "make_struct! output mismatch.\nblock: {:?}",
        mks[0]
    );

    // ---------------------------------------------------------------
    // macro_rules! emitted by proc macro: mystruct_hello!
    // ---------------------------------------------------------------
    let msh = find_blocks(&blocks, "mystruct_hello!");
    assert_eq!(
        msh.len(),
        1,
        "Expected exactly 1 mystruct_hello! block, found {}.\nstdout:\n{}",
        msh.len(),
        stdout
    );
    assert!(
        msh[0].input.trim().is_empty(),
        "mystruct_hello! should have empty input.\nblock: {:?}",
        msh[0]
    );
    assert!(
        msh[0].output.contains("println!") && msh[0].output.contains("macro_rules! invoked for"),
        "mystruct_hello! output mismatch.\nblock: {:?}",
        msh[0]
    );

    // ---------------------------------------------------------------
    // proc_macro (function-like): make_answer!
    // ---------------------------------------------------------------
    let ma = find_blocks(&blocks, "make_answer!");
    assert!(
        ma.len() >= 1,
        "Expected at least 1 make_answer! block, found {}.\nstdout:\n{}",
        ma.len(),
        stdout
    );
    let ma_orig = ma.iter().find(|b| b.input.trim() == "get_answer");
    assert!(
        ma_orig.is_some(),
        "Expected a make_answer! block with input 'get_answer'.\nblocks: {:?}",
        ma
    );
    assert!(
        ma_orig.unwrap().output.contains("fn get_answer()"),
        "make_answer! output should contain 'fn get_answer()'.\nblock: {:?}",
        ma_orig.unwrap()
    );

    // ---------------------------------------------------------------
    // proc_macro_attribute: #[add_hello_method]
    // ---------------------------------------------------------------
    let ahm = find_blocks(&blocks, "#[add_hello_method]");
    assert!(
        ahm.len() >= 1,
        "Expected at least 1 #[add_hello_method] block, found {}.\nstdout:\n{}",
        ahm.len(),
        stdout
    );
    let ahm_orig = ahm.iter().find(|b| b.input.contains("MyStruct"));
    assert!(
        ahm_orig.is_some(),
        "#[add_hello_method] should have a block for 'MyStruct'.\nblocks: {:?}",
        ahm
    );
    assert!(
        ahm_orig.unwrap().output.contains("impl MyStruct")
            && ahm_orig.unwrap().output.contains("fn hello"),
        "#[add_hello_method] output should contain 'impl MyStruct' and 'fn hello'.\nblock: {:?}",
        ahm_orig.unwrap()
    );

    // ---------------------------------------------------------------
    // proc_macro_derive: #[derive(Greet)]
    // ---------------------------------------------------------------
    let greet = find_blocks(&blocks, "#[derive(Greet)]");
    assert!(
        greet.len() >= 4,
        "Expected at least 4 #[derive(Greet)] blocks, found {}.\nstdout:\n{}",
        greet.len(),
        stdout
    );

    let greeter = greet
        .iter()
        .find(|b| b.input.contains("Greeter") && !b.input.contains("Auto"));
    assert!(
        greeter.is_some(),
        "Expected a #[derive(Greet)] block for Greeter.\nblocks: {:?}",
        greet
    );
    assert!(
        greeter.unwrap().output.contains("fn greet()"),
        "#[derive(Greet)] output should contain 'fn greet()'.\nblock: {:?}",
        greeter.unwrap()
    );

    let auto_greeter = greet
        .iter()
        .find(|b| b.input.contains("AutoGreeter"));
    assert!(
        auto_greeter.is_some(),
        "Expected a #[derive(Greet)] block for AutoGreeter.\nblocks: {:?}",
        greet
    );

    // ---------------------------------------------------------------
    // Multi-segment path: functional proc macro
    // test_proc_macros::make_answer!(get_answer_path)
    // ---------------------------------------------------------------
    let path_fn = blocks
        .iter()
        .find(|b| b.caller.contains("make_answer") && b.input.contains("get_answer_path"));
    assert!(
        path_fn.is_some(),
        "Expected expansion for path-invoked make_answer!(get_answer_path).\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        path_fn.unwrap().output.contains("fn get_answer_path()"),
        "Path-invoked make_answer! output should contain 'fn get_answer_path()'.\nblock: {:?}",
        path_fn.unwrap()
    );

    // ---------------------------------------------------------------
    // Multi-segment path: macro_rules! via module re-export
    // inner_macros::repeat_thrice!(1)
    // ---------------------------------------------------------------
    let path_mr = blocks
        .iter()
        .find(|b| b.caller.contains("repeat_thrice"));
    assert!(
        path_mr.is_some(),
        "Expected expansion for path-invoked inner_macros::repeat_thrice!.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        path_mr.unwrap().output.contains("(1"),
        "repeat_thrice! output should contain expanded tuple.\nblock: {:?}",
        path_mr.unwrap()
    );

    // ---------------------------------------------------------------
    // Multi-segment path: attribute proc macro
    // #[test_proc_macros::add_hello_method] on PathStruct
    // ---------------------------------------------------------------
    let path_attr = blocks
        .iter()
        .find(|b| b.caller.contains("add_hello_method") && b.input.contains("PathStruct"));
    assert!(
        path_attr.is_some(),
        "Expected expansion for path-invoked #[add_hello_method] on PathStruct.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        path_attr.unwrap().output.contains("impl PathStruct"),
        "Path-invoked #[add_hello_method] output should contain 'impl PathStruct'.\nblock: {:?}",
        path_attr.unwrap()
    );

    // ---------------------------------------------------------------
    // Multi-segment path: derive proc macro
    // #[derive(test_proc_macros::Greet)] on PathGreeter
    // ---------------------------------------------------------------
    let path_derive = blocks
        .iter()
        .find(|b| b.caller.contains("Greet") && b.input.contains("PathGreeter"));
    assert!(
        path_derive.is_some(),
        "Expected expansion for path-invoked #[derive(Greet)] on PathGreeter.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        path_derive.unwrap().output.contains("fn greet()"),
        "Path-invoked #[derive(Greet)] on PathGreeter should contain 'fn greet()'.\nblock: {:?}",
        path_derive.unwrap()
    );

    // ---------------------------------------------------------------
    // Attribute proc macro with complex arguments
    // #[test_proc_macros::tag_item(name = "tagged", items = [1, 2, 3],
    //     opts = (verbose, debug), config = {key: value})]
    // ---------------------------------------------------------------
    let tag = blocks
        .iter()
        .find(|b| b.caller.contains("tag_item") && b.input.contains("TaggedStruct"));
    assert!(
        tag.is_some(),
        "Expected expansion for #[tag_item(...)] on TaggedStruct.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    let tag_block = tag.unwrap();
    assert!(
        tag_block.output.contains("TaggedStruct"),
        "#[tag_item] output should preserve 'TaggedStruct'.\nblock: {:?}",
        tag_block
    );
    assert!(
        tag_block.output.contains("__TAG_ARGS_FOR_TaggedStruct"),
        "#[tag_item] output should contain generated const.\nblock: {:?}",
        tag_block
    );
    // Verify the caller reflects argument syntax with assignment and groups
    let tag_caller = &tag_block.caller;
    assert!(
        tag_caller.contains("name") && tag_caller.contains("tagged"),
        "tag_item caller should contain assignment arg.\ncaller: {}",
        tag_caller
    );
    assert!(
        tag_caller.contains("[") && tag_caller.contains("]"),
        "tag_item caller should contain bracket group.\ncaller: {}",
        tag_caller
    );
    assert!(
        tag_caller.contains("{") && tag_caller.contains("}"),
        "tag_item caller should contain brace group.\ncaller: {}",
        tag_caller
    );

    // ---------------------------------------------------------------
    // Multiple attribute macros on one item: MultiAttrStruct
    // #[tag_item(role = "primary")]
    // #[add_hello_method]
    // pub struct MultiAttrStruct { pub id: u32 }
    // ---------------------------------------------------------------
    let multi_attr_tag = blocks
        .iter()
        .find(|b| b.caller.contains("tag_item") && b.input.contains("MultiAttrStruct"));
    assert!(
        multi_attr_tag.is_some(),
        "Expected #[tag_item] expansion for MultiAttrStruct.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    let multi_attr_tag = multi_attr_tag.unwrap();
    assert!(
        multi_attr_tag.output.contains("__TAG_ARGS_FOR_MultiAttrStruct"),
        "#[tag_item] on MultiAttrStruct should generate const.\nblock: {:?}",
        multi_attr_tag
    );
    // tag_item receives the item WITH #[add_hello_method] still on it
    assert!(
        multi_attr_tag.input.contains("add_hello_method"),
        "#[tag_item] input should contain remaining #[add_hello_method].\nblock: {:?}",
        multi_attr_tag
    );

    let multi_attr_hello = blocks
        .iter()
        .find(|b| b.caller.contains("add_hello_method") && b.input.contains("MultiAttrStruct"));
    assert!(
        multi_attr_hello.is_some(),
        "Expected #[add_hello_method] expansion for MultiAttrStruct.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        multi_attr_hello.unwrap().output.contains("impl MultiAttrStruct"),
        "#[add_hello_method] on MultiAttrStruct should contain impl.\nblock: {:?}",
        multi_attr_hello.unwrap()
    );

    // ---------------------------------------------------------------
    // Multiple derive macros on one attribute: MultiDeriveOneAttr
    // #[derive(Greet, Describe)]
    // ---------------------------------------------------------------
    let one_attr_greet = blocks
        .iter()
        .find(|b| b.caller == "#[derive(Greet)]" && b.input.contains("MultiDeriveOneAttr"));
    assert!(
        one_attr_greet.is_some(),
        "Expected #[derive(Greet)] for MultiDeriveOneAttr.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        one_attr_greet.unwrap().output.contains("fn greet()"),
        "#[derive(Greet)] on MultiDeriveOneAttr should contain greet().\nblock: {:?}",
        one_attr_greet.unwrap()
    );

    let one_attr_describe = blocks
        .iter()
        .find(|b| b.caller == "#[derive(Describe)]" && b.input.contains("MultiDeriveOneAttr"));
    assert!(
        one_attr_describe.is_some(),
        "Expected #[derive(Describe)] for MultiDeriveOneAttr.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        one_attr_describe.unwrap().output.contains("fn describe()"),
        "#[derive(Describe)] on MultiDeriveOneAttr should contain describe().\nblock: {:?}",
        one_attr_describe.unwrap()
    );

    // ---------------------------------------------------------------
    // Multiple derive macros on two attributes: MultiDeriveTwoAttr
    // #[derive(Greet)]
    // #[derive(Describe)]
    // ---------------------------------------------------------------
    let two_attr_greet = blocks
        .iter()
        .find(|b| b.caller == "#[derive(Greet)]" && b.input.contains("MultiDeriveTwoAttr"));
    assert!(
        two_attr_greet.is_some(),
        "Expected #[derive(Greet)] for MultiDeriveTwoAttr.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        two_attr_greet.unwrap().output.contains("fn greet()"),
        "#[derive(Greet)] on MultiDeriveTwoAttr should contain greet().\nblock: {:?}",
        two_attr_greet.unwrap()
    );
    // Greet on MultiDeriveTwoAttr receives input WITH #[derive(Describe)] still present
    assert!(
        two_attr_greet.unwrap().input.contains("Describe"),
        "#[derive(Greet)] on MultiDeriveTwoAttr input should contain remaining #[derive(Describe)].\nblock: {:?}",
        two_attr_greet.unwrap()
    );

    let two_attr_describe = blocks
        .iter()
        .find(|b| b.caller == "#[derive(Describe)]" && b.input.contains("MultiDeriveTwoAttr"));
    assert!(
        two_attr_describe.is_some(),
        "Expected #[derive(Describe)] for MultiDeriveTwoAttr.\nall callers: {:?}",
        blocks.iter().map(|b| &b.caller).collect::<Vec<_>>()
    );
    assert!(
        two_attr_describe.unwrap().output.contains("fn describe()"),
        "#[derive(Describe)] on MultiDeriveTwoAttr should contain describe().\nblock: {:?}",
        two_attr_describe.unwrap()
    );
}
