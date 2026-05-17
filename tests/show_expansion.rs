use std::path::PathBuf;

use cargo_macra::parse_trace::{MacroExpansion, MacroExpansionKind};
use cargo_macra::trace_macros::{Args as TraceArgs, TraceMacros};

fn test_usage_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test-usage/Cargo.toml")
}

fn test_usage_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test-usage/src/lib.rs")
}

fn expansion_caller(expansion: &MacroExpansion) -> String {
    match expansion.kind {
        MacroExpansionKind::Bang => format!("{}!", expansion.name),
        MacroExpansionKind::Attribute => {
            if expansion.arguments.is_empty() {
                format!("#[{}]", expansion.name)
            } else {
                format!(
                    "#[{}({})]",
                    expansion.name,
                    expansion.arguments.replace('\n', " ")
                )
            }
        }
        MacroExpansionKind::Derive => format!("#[derive({})]", expansion.name),
    }
}

fn find_expansions<'a>(expansions: &'a [MacroExpansion], caller: &str) -> Vec<&'a MacroExpansion> {
    expansions
        .iter()
        .filter(|e| expansion_caller(e) == caller)
        .collect()
}

fn assert_exact(expansions: &[MacroExpansion], caller: &str, expected_input: &str, expected_to: &str) {
    let matching: Vec<_> = expansions
        .iter()
        .filter(|e| expansion_caller(e) == caller && e.input.trim() == expected_input.trim())
        .collect();
    assert_eq!(
        matching.len(),
        1, "Expected exactly 1 expansion for caller={:?}, input={:?}, found {}.",
        caller,
        expected_input,
        matching.len(),
    );
    assert_eq!(
        matching[0].to.trim(),
        expected_to.trim(),
        "Output mismatch for caller={:?}, input={:?}",
        caller,
        expected_input,
    );
}

/// Run trace-macros directly on the test-usage crate and return raw expansions.
fn run_show_expansion_test_usage() -> Vec<MacroExpansion> {
    filetime::set_file_mtime(test_usage_lib(), filetime::FileTime::now())
        .expect("failed to touch test-usage/src/lib.rs");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let tm_args = TraceArgs {
        package: None,
        bin: None,
        lib: false,
        test: None,
        example: None,
        manifest_path: Some(test_usage_manifest().to_string_lossy().to_string()),
        cargo_args: Vec::new(),
        hook_lib: cargo_macra::find_hook_lib(std::env::current_exe().ok().as_deref())
            .unwrap_or_default(),
    };
    let tm = TraceMacros::new(std::path::Path::new(&cargo), &tm_args);
    let run = tm.run().expect("failed to run trace macros");
    let expansions: Vec<_> = run
        .iter
        .collect::<std::io::Result<Vec<_>>>()
        .expect("trace macro collection failed");
    let check_success = run
        .check_success
        .recv()
        .expect("failed to receive cargo check status")
        .expect("failed to wait cargo check status");
    assert!(
        check_success,
        "cargo check failed while collecting macro expansions"
    );
    expansions
}

#[test]
fn show_expansion() {
    let expansions = run_show_expansion_test_usage();
    assert!(
        !expansions.is_empty(),
        "Expected macro expansions.",
    );
    // ---------------------------------------------------------------
    // All macro types are trapped
    // ---------------------------------------------------------------

    // macro_rules!
    assert!(
        expansions.iter().any(|e| expansion_caller(e) == "repeat_twice!"),
        "Missing macro_rules! expansion (repeat_twice!).",
    );

    // proc_macro (function-like)
    assert!(
        expansions.iter().any(|e| expansion_caller(e) == "make_answer!"),
        "Missing proc_macro (bang) expansion (make_answer!).",
    );

    // proc_macro_attribute
    assert!(
        expansions
            .iter()
            .any(|e| expansion_caller(e) == "#[add_hello_method]"),
        "Missing proc_macro_attribute expansion (#[add_hello_method]).",
    );

    // proc_macro_derive
    assert!(
        expansions
            .iter()
            .any(|e| expansion_caller(e) == "#[derive(Greet)]"),
        "Missing proc_macro_derive expansion (#[derive(Greet)]).",
    );

    // macro_rules! emitted by a proc macro
    assert!(
        expansions
            .iter()
            .any(|e| expansion_caller(e) == "mystruct_hello!"),
        "Missing macro_rules! emitted by proc macro (mystruct_hello!).",
    );

    // ---------------------------------------------------------------
    // macro_rules!: repeat_twice!
    // ---------------------------------------------------------------
    let repeat = find_expansions(&expansions, "repeat_twice!");
    assert_eq!(
        repeat.len(),
        1,
        "Expected exactly 1 repeat_twice! expansion, found {}.",
        repeat.len(),
    );
    assert_eq!(
        repeat[0].input.trim(),
        "get_answer()",
        "repeat_twice! input mismatch.\nblock: {:?}",
        repeat[0]
    );
    assert_eq!(
        repeat[0].to.trim(),
        "(get_answer(), get_answer())",
        "repeat_twice! output mismatch.\nexpansion: {:?}",
        repeat[0]
    );

    // ---------------------------------------------------------------
    // macro_rules!: make_struct!
    // ---------------------------------------------------------------
    let mks = find_expansions(&expansions, "make_struct!");
    assert_eq!(
        mks.len(),
        1,
        "Expected exactly 1 make_struct! expansion, found {}.",
        mks.len(),
    );
    assert_eq!(
        mks[0].input.trim(),
        "AutoGreeter",
        "make_struct! input mismatch.\nblock: {:?}",
        mks[0]
    );
    assert_eq!(
        mks[0].to.trim(),
        "#[derive(Greet)] pub struct AutoGreeter;",
        "make_struct! output mismatch.\nblock: {:?}",
        mks[0]
    );

    // ---------------------------------------------------------------
    // macro_rules! emitted by proc macro: mystruct_hello!
    // ---------------------------------------------------------------
    let msh = find_expansions(&expansions, "mystruct_hello!");
    assert_eq!(
        msh.len(),
        1,
        "Expected exactly 1 mystruct_hello! expansion, found {}.",
        msh.len(),
    );
    assert!(
        msh[0].input.trim().is_empty(),
        "mystruct_hello! should have empty input.\nblock: {:?}",
        msh[0]
    );
    assert!(
        msh[0].to.contains("println!") && msh[0].to.contains("macro_rules! invoked for"),
        "mystruct_hello! output mismatch.\nblock: {:?}",
        msh[0]
    );

    // ---------------------------------------------------------------
    // proc_macro (function-like): make_answer!
    // ---------------------------------------------------------------
    let ma = find_expansions(&expansions, "make_answer!");
    assert!(
        ma.len() >= 1,
        "Expected at least 1 make_answer! expansion, found {}.",
        ma.len(),
    );
    let ma_orig = ma.iter().find(|b| b.input.trim() == "get_answer");
    assert!(
        ma_orig.is_some(),
        "Expected a make_answer! block with input 'get_answer'.\nblocks: {:?}",
        ma
    );
    assert!(
        ma_orig.unwrap().to.contains("fn get_answer()"),
        "make_answer! output should contain 'fn get_answer()'.\nblock: {:?}",
        ma_orig.unwrap()
    );

    // ---------------------------------------------------------------
    // proc_macro_attribute: #[add_hello_method]
    // ---------------------------------------------------------------
    let ahm = find_expansions(&expansions, "#[add_hello_method]");
    assert!(
        ahm.len() >= 1,
        "Expected at least 1 #[add_hello_method] expansion, found {}.",
        ahm.len(),
    );
    let ahm_orig = ahm.iter().find(|b| b.input.contains("MyStruct"));
    assert!(
        ahm_orig.is_some(),
        "#[add_hello_method] should have a block for 'MyStruct'.\nblocks: {:?}",
        ahm
    );
    assert!(
        ahm_orig.unwrap().to.contains("impl MyStruct") && ahm_orig.unwrap().to.contains("fn hello"),
        "#[add_hello_method] output should contain 'impl MyStruct' and 'fn hello'.\nblock: {:?}",
        ahm_orig.unwrap()
    );

    // ---------------------------------------------------------------
    // proc_macro_derive: #[derive(Greet)]
    // ---------------------------------------------------------------
    let greet = find_expansions(&expansions, "#[derive(Greet)]");
    assert!(
        greet.len() >= 4,
        "Expected at least 4 #[derive(Greet)] expansions, found {}.",
        greet.len(),
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
        greeter.unwrap().to.contains("fn greet()"),
        "#[derive(Greet)] output should contain 'fn greet()'.\nblock: {:?}",
        greeter.unwrap()
    );

    let auto_greeter = greet.iter().find(|b| b.input.contains("AutoGreeter"));
    assert!(
        auto_greeter.is_some(),
        "Expected a #[derive(Greet)] block for AutoGreeter.\nblocks: {:?}",
        greet
    );

    // ---------------------------------------------------------------
    // Multi-segment path: functional proc macro
    // test_proc_macros::make_answer!(get_answer_path)
    // ---------------------------------------------------------------
    let path_fn = expansions
        .iter()
        .find(|e| expansion_caller(e).contains("make_answer") && e.input.contains("get_answer_path"));
    assert!(
        path_fn.is_some(),
        "Expected expansion for path-invoked make_answer!(get_answer_path)."
    );
    assert!(
        path_fn.unwrap().to.contains("fn get_answer_path()"),
        "Path-invoked make_answer! output should contain 'fn get_answer_path()'.\nexpansion: {:?}",
        path_fn.unwrap()
    );

    // ---------------------------------------------------------------
    // Multi-segment path: macro_rules! via module re-export
    // inner_macros::repeat_thrice!(1)
    // ---------------------------------------------------------------
    let path_mr = expansions
        .iter()
        .find(|e| expansion_caller(e).contains("repeat_thrice"));
    assert!(
        path_mr.is_some(),
        "Expected expansion for path-invoked inner_macros::repeat_thrice!."
    );
    assert!(
        path_mr.unwrap().to.contains("(1"),
        "repeat_thrice! output should contain expanded tuple.\nexpansion: {:?}",
        path_mr.unwrap()
    );

    // ---------------------------------------------------------------
    // Multi-segment path: attribute proc macro
    // #[test_proc_macros::add_hello_method] on PathStruct
    // ---------------------------------------------------------------
    let path_attr = expansions
        .iter()
        .find(|e| expansion_caller(e).contains("add_hello_method") && e.input.contains("PathStruct"));
    assert!(
        path_attr.is_some(),
        "Expected expansion for path-invoked #[add_hello_method] on PathStruct."
    );
    assert!(
        path_attr.unwrap().to.contains("impl PathStruct"),
        "Path-invoked #[add_hello_method] output should contain 'impl PathStruct'.\nexpansion: {:?}",
        path_attr.unwrap()
    );

    // ---------------------------------------------------------------
    // Multi-segment path: derive proc macro
    // #[derive(test_proc_macros::Greet)] on PathGreeter
    // ---------------------------------------------------------------
    let path_derive = expansions
        .iter()
        .find(|e| expansion_caller(e).contains("Greet") && e.input.contains("PathGreeter"));
    assert!(
        path_derive.is_some(),
        "Expected expansion for path-invoked #[derive(Greet)] on PathGreeter."
    );
    assert!(
        path_derive.unwrap().to.contains("fn greet()"),
        "Path-invoked #[derive(Greet)] on PathGreeter should contain 'fn greet()'.\nexpansion: {:?}",
        path_derive.unwrap()
    );

    // ---------------------------------------------------------------
    // Attribute proc macro with complex arguments
    // #[test_proc_macros::tag_item(name = "tagged", items = [1, 2, 3],
    //     opts = (verbose, debug), config = {key: value})]
    // ---------------------------------------------------------------
    let tag = expansions
        .iter()
        .find(|e| expansion_caller(e).contains("tag_item") && e.input.contains("TaggedStruct"));
    assert!(
        tag.is_some(),
        "Expected expansion for #[tag_item(...)] on TaggedStruct."
    );
    let tag_block = tag.unwrap();
    assert!(
        tag_block.to.contains("TaggedStruct"),
        "#[tag_item] output should preserve 'TaggedStruct'.\nexpansion: {:?}",
        tag_block
    );
    assert!(
        tag_block.to.contains("__TAG_ARGS_FOR_TaggedStruct"),
        "#[tag_item] output should contain generated const.\nexpansion: {:?}",
        tag_block
    );
    // Verify the caller reflects argument syntax with assignment and groups
    let tag_caller = expansion_caller(tag_block);
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
    let multi_attr_tag = expansions
        .iter()
        .find(|e| expansion_caller(e).contains("tag_item") && e.input.contains("MultiAttrStruct"));
    assert!(
        multi_attr_tag.is_some(),
        "Expected #[tag_item] expansion for MultiAttrStruct."
    );
    let multi_attr_tag = multi_attr_tag.unwrap();
    assert!(
        multi_attr_tag
            .to
            .contains("__TAG_ARGS_FOR_MultiAttrStruct"),
        "#[tag_item] on MultiAttrStruct should generate const.\nexpansion: {:?}",
        multi_attr_tag
    );
    // tag_item receives the item WITH #[add_hello_method] still on it
    assert!(
        multi_attr_tag.input.contains("add_hello_method"),
        "#[tag_item] input should contain remaining #[add_hello_method].\nexpansion: {:?}",
        multi_attr_tag
    );

    let multi_attr_hello = expansions
        .iter()
        .find(|e| expansion_caller(e).contains("add_hello_method") && e.input.contains("MultiAttrStruct"));
    assert!(
        multi_attr_hello.is_some(),
        "Expected #[add_hello_method] expansion for MultiAttrStruct."
    );
    assert!(
        multi_attr_hello
            .unwrap()
            .to
            .contains("impl MultiAttrStruct"),
        "#[add_hello_method] on MultiAttrStruct should contain impl.\nexpansion: {:?}",
        multi_attr_hello.unwrap()
    );

    // ---------------------------------------------------------------
    // Multiple derive macros on one attribute: MultiDeriveOneAttr
    // #[derive(Greet, Describe)]
    // ---------------------------------------------------------------
    let one_attr_greet = expansions
        .iter()
        .find(|e| expansion_caller(e) == "#[derive(Greet)]" && e.input.contains("MultiDeriveOneAttr"));
    assert!(
        one_attr_greet.is_some(),
        "Expected #[derive(Greet)] for MultiDeriveOneAttr."
    );
    assert!(
        one_attr_greet.unwrap().to.contains("fn greet()"),
        "#[derive(Greet)] on MultiDeriveOneAttr should contain greet().\nexpansion: {:?}",
        one_attr_greet.unwrap()
    );

    let one_attr_describe = expansions
        .iter()
        .find(|e| expansion_caller(e) == "#[derive(Describe)]" && e.input.contains("MultiDeriveOneAttr"));
    assert!(
        one_attr_describe.is_some(),
        "Expected #[derive(Describe)] for MultiDeriveOneAttr."
    );
    assert!(
        one_attr_describe.unwrap().to.contains("fn describe()"),
        "#[derive(Describe)] on MultiDeriveOneAttr should contain describe().\nexpansion: {:?}",
        one_attr_describe.unwrap()
    );

    // ---------------------------------------------------------------
    // Multiple derive macros on two attributes: MultiDeriveTwoAttr
    // #[derive(Greet)]
    // #[derive(Describe)]
    // ---------------------------------------------------------------
    let two_attr_greet = expansions
        .iter()
        .find(|e| expansion_caller(e) == "#[derive(Greet)]" && e.input.contains("MultiDeriveTwoAttr"));
    assert!(
        two_attr_greet.is_some(),
        "Expected #[derive(Greet)] for MultiDeriveTwoAttr."
    );
    assert!(
        two_attr_greet.unwrap().to.contains("fn greet()"),
        "#[derive(Greet)] on MultiDeriveTwoAttr should contain greet().\nexpansion: {:?}",
        two_attr_greet.unwrap()
    );
    // Greet on MultiDeriveTwoAttr receives input WITH #[derive(Describe)] still present
    assert!(
        two_attr_greet.unwrap().input.contains("Describe"),
        "#[derive(Greet)] on MultiDeriveTwoAttr input should contain remaining #[derive(Describe)].\nexpansion: {:?}",
        two_attr_greet.unwrap()
    );

    let two_attr_describe = expansions
        .iter()
        .find(|e| expansion_caller(e) == "#[derive(Describe)]" && e.input.contains("MultiDeriveTwoAttr"));
    assert!(
        two_attr_describe.is_some(),
        "Expected #[derive(Describe)] for MultiDeriveTwoAttr."
    );
    assert!(
        two_attr_describe.unwrap().to.contains("fn describe()"),
        "#[derive(Describe)] on MultiDeriveTwoAttr should contain describe().\nexpansion: {:?}",
        two_attr_describe.unwrap()
    );
}

/// Exact-match test for every macro expansion originating from the test-usage
/// crate.  Each assertion verifies the caller, input, and output verbatim.
#[test]
fn test_usage_expansion() {
    let expansions = run_show_expansion_test_usage();

    // =================================================================
    // Hook-based proc-macro expansions (14 blocks)
    // =================================================================

    // 1. #[add_hello_method] on MyStruct
    assert_exact(
        &expansions,
        "#[add_hello_method]",
        "pub struct MyStruct { pub value: i32, }",
        r#"pub struct MyStruct { pub value : i32, } impl MyStruct
{
    pub fn hello(& self)
    { println! ("Hello from {}!", stringify! (MyStruct)); }
} macro_rules! mystruct_hello
{
    () =>
    {
        println!
        (concat! ("macro_rules! invoked for ", stringify! (MyStruct)));
    };
}"#,
    );

    // 2. make_answer!(get_answer)
    assert_exact(
        &expansions,
        "make_answer!",
        "get_answer",
        "pub fn get_answer() -> i32 { vec! [42i32].into_iter().sum() }",
    );

    // 3. #[derive(Greet)] on Greeter
    assert_exact(
        &expansions,
        "#[derive(Greet)]",
        "pub struct Greeter;",
        r#"impl Greeter
{
    pub fn greet() -> String
    { format! ("Hello from {}", stringify! (Greeter)) }
}"#,
    );

    // 4. #[derive(Greet)] on AutoGreeter (generated by make_struct!)
    assert_exact(
        &expansions,
        "#[derive(Greet)]",
        "pub struct AutoGreeter;",
        r#"impl AutoGreeter
{
    pub fn greet() -> String
    { format! ("Hello from {}", stringify! (AutoGreeter)) }
}"#,
    );

    // 5. make_answer!(get_answer_path) via crate path
    assert_exact(
        &expansions,
        "make_answer!",
        "get_answer_path",
        "pub fn get_answer_path() -> i32 { vec! [42i32].into_iter().sum() }",
    );

    // 6. #[add_hello_method] on PathStruct via crate path
    assert_exact(
        &expansions,
        "#[add_hello_method]",
        "pub struct PathStruct { pub value: i32, }",
        r#"pub struct PathStruct { pub value : i32, } impl PathStruct
{
    pub fn hello(& self)
    { println! ("Hello from {}!", stringify! (PathStruct)); }
} macro_rules! pathstruct_hello
{
    () =>
    {
        println!
        (concat! ("macro_rules! invoked for ", stringify! (PathStruct)));
    };
}"#,
    );

    // 7. #[derive(Greet)] on PathGreeter via crate path
    assert_exact(
        &expansions,
        "#[derive(Greet)]",
        "pub struct PathGreeter;",
        r#"impl PathGreeter
{
    pub fn greet() -> String
    { format! ("Hello from {}", stringify! (PathGreeter)) }
}"#,
    );

    // 8. #[tag_item(...)] on TaggedStruct with complex arguments
    assert_exact(
        &expansions,
        r#"#[tag_item(name = "tagged", items = [1, 2, 3], opts = (verbose, debug), config = {key: value})]"#,
        "pub struct TaggedStruct;",
        r#"pub struct TaggedStruct; pub const __TAG_ARGS_FOR_TaggedStruct : & str =
"name = \"tagged\", items = [1, 2, 3], opts = (verbose, debug), config =\n{key: value}";"#,
    );

    // 9. #[tag_item(role = "primary")] on MultiAttrStruct
    assert_exact(
        &expansions,
        r#"#[tag_item(role = "primary")]"#,
        "#[add_hello_method] pub struct MultiAttrStruct { pub id: u32, }",
        r#"#[add_hello_method] pub struct MultiAttrStruct { pub id : u32, } pub const
__TAG_ARGS_FOR_MultiAttrStruct : & str = "role = \"primary\"";"#,
    );

    // 10. #[add_hello_method] on MultiAttrStruct (after tag_item)
    assert_exact(
        &expansions,
        "#[add_hello_method]",
        "pub struct MultiAttrStruct { pub id : u32, }",
        r#"pub struct MultiAttrStruct { pub id : u32, } impl MultiAttrStruct
{
    pub fn hello(& self)
    { println! ("Hello from {}!", stringify! (MultiAttrStruct)); }
} macro_rules! multiattrstruct_hello
{
    () =>
    {
        println!
        (concat! ("macro_rules! invoked for ", stringify! (MultiAttrStruct)));
    };
}"#,
    );

    // 11. #[derive(Greet)] on MultiDeriveOneAttr
    assert_exact(
        &expansions,
        "#[derive(Greet)]",
        "pub struct MultiDeriveOneAttr;",
        r#"impl MultiDeriveOneAttr
{
    pub fn greet() -> String
    { format! ("Hello from {}", stringify! (MultiDeriveOneAttr)) }
}"#,
    );

    // 12. #[derive(Describe)] on MultiDeriveOneAttr
    assert_exact(
        &expansions,
        "#[derive(Describe)]",
        "pub struct MultiDeriveOneAttr;",
        r#"impl MultiDeriveOneAttr
{
    pub fn describe() -> String
    { format! ("{} is a struct", stringify! (MultiDeriveOneAttr)) }
}"#,
    );

    // 13. #[derive(Greet)] on MultiDeriveTwoAttr (input includes remaining
    //     #[derive(Describe)])
    assert_exact(
        &expansions,
        "#[derive(Greet)]",
        "#[derive(Describe)] pub struct MultiDeriveTwoAttr;",
        r#"impl MultiDeriveTwoAttr
{
    pub fn greet() -> String
    { format! ("Hello from {}", stringify! (MultiDeriveTwoAttr)) }
}"#,
    );

    // 14. #[derive(Describe)] on MultiDeriveTwoAttr
    assert_exact(
        &expansions,
        "#[derive(Describe)]",
        "pub struct MultiDeriveTwoAttr;",
        r#"impl MultiDeriveTwoAttr
{
    pub fn describe() -> String
    { format! ("{} is a struct", stringify! (MultiDeriveTwoAttr)) }
}"#,
    );

    // =================================================================
    // Trace-macros expansions: user-written macro_rules! (4 blocks)
    // =================================================================

    // 15. repeat_twice!(get_answer())
    assert_exact(
        &expansions,
        "repeat_twice!",
        "get_answer()",
        "(get_answer(), get_answer())",
    );

    // 16. mystruct_hello!() — generated by #[add_hello_method]
    assert_exact(
        &expansions,
        "mystruct_hello!",
        "",
        r#"println! (concat! ("macro_rules! invoked for ", stringify! (MyStruct)));"#,
    );

    // 17. make_struct!(AutoGreeter)
    assert_exact(
        &expansions,
        "make_struct!",
        "AutoGreeter",
        "#[derive(Greet)] pub struct AutoGreeter;",
    );

    // 18. repeat_thrice!(1) via inner_macros path
    assert_exact(&expansions, "repeat_thrice!", "1", "(1, 1, 1)");
}
