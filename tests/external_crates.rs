// Windows support uses RUSTC_WRAPPER + DLL injection + IAT patching.
// All platforms are now supported.

use std::path::PathBuf;

use cargo_macra::parse_trace::{MacroExpansion, MacroExpansionKind};
use cargo_macra::trace_macros::{Args as TraceArgs, TraceMacros};

fn repo_manifest(repo: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/external_crates").join(repo).join("Cargo.toml")
}

fn run_trace_for_repo(repo: &str, test: Option<&str>) -> Vec<MacroExpansion> {
    let manifest_path = repo_manifest(repo);
    assert!(manifest_path.exists());
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let tm_args = TraceArgs {
        package: None, bin: None, lib: false, test: test.map(|s| s.to_string()), example: None,
        manifest_path: Some(manifest_path.to_string_lossy().to_string()), cargo_args: Vec::new(),
        hook_lib: cargo_macra::find_hook_lib(std::env::current_exe().ok().as_deref()).unwrap_or_default(),
    };
    let tm = TraceMacros::new(std::path::Path::new(&cargo), &tm_args);
    let run = tm.run().unwrap_or_else(|e| panic!("TraceMacros::run() failed for {}: {}", repo, e));
    let expansions = run.iter.collect::<std::io::Result<Vec<_>>>().unwrap_or_else(|e| panic!("failed to collect expansions for {}: {}", repo, e));
    let check_result = run.check_result.recv().expect("failed to receive cargo check status").unwrap_or_else(|e| panic!("failed waiting cargo check for {}: {}", repo, e));
    assert!(
        check_result.success,
        "cargo check failed for repo `{repo}` test `{test:?}`\nmanifest: {}\nstdout:\n{}\nstderr:\n{}",
        manifest_path.display(),
        check_result.stdout,
        check_result.stderr
    );
    // Debug: show expansion summary
    let attr_names: Vec<_> = expansions.iter().filter(|e| e.kind == MacroExpansionKind::Attribute).map(|e| e.name.as_str()).collect();
    let derive_names: Vec<_> = expansions.iter().filter(|e| e.kind == MacroExpansionKind::Derive).map(|e| e.name.as_str()).collect();
    let bang_count = expansions.iter().filter(|e| e.kind == MacroExpansionKind::Bang).count();
    eprintln!(
        "[test-debug] repo={repo} test={test:?} total={} attr={attr_names:?} derive={derive_names:?} bang={bang_count} hook_lib={} stderr_len={}",
        expansions.len(), tm_args.hook_lib.display(), check_result.stderr.len()
    );
    assert!(!expansions.is_empty());
    expansions
}

fn assert_has(expansions: &[MacroExpansion], kind: MacroExpansionKind, name: &str) {
    assert!(
        expansions.iter().any(|e| e.kind == kind && e.name == name),
        "missing expansion kind={kind:?} name={name}"
    );
}

fn assert_has_prefix(expansions: &[MacroExpansion], kind: MacroExpansionKind, prefix: &str) {
    assert!(
        expansions.iter().any(|e| e.kind == kind && e.name.starts_with(prefix)),
        "missing expansion kind={kind:?} prefix={prefix}"
    );
}

fn starts_with_normalized(actual: &str, expected_prefix: &str) -> bool {
    cargo_macra::normalize_tokens(actual).starts_with(&cargo_macra::normalize_tokens(expected_prefix))
}

fn equals_normalized(actual: &str, expected: &str) -> bool {
    cargo_macra::normalize_tokens(actual) == cargo_macra::normalize_tokens(expected)
}

#[test]
fn external_crate_coinduction_test_coinduction_integration_test() {
    let expansions = run_trace_for_repo("coinduction", Some("coinduction_integration_test"));
    assert_has(&expansions, MacroExpansionKind::Attribute, "traitdef");
    assert_has(&expansions, MacroExpansionKind::Attribute, "typedef");
    assert_has(&expansions, MacroExpansionKind::Bang, "__next_step");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__CircularTrait_temporal_");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__ConstrainedStruct_temporal_");
}

#[test]
fn external_crate_coinduction_test_complex() {
    let expansions = run_trace_for_repo("coinduction", Some("complex"));
    assert_has(&expansions, MacroExpansionKind::Attribute, "coinduction");
    assert_has(&expansions, MacroExpansionKind::Attribute, "traitdef");
    assert_has(&expansions, MacroExpansionKind::Attribute, "typedef");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__TraitA_temporal_");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__TraitB_temporal_");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__Wrapper2_temporal_");
    assert_has(&expansions, MacroExpansionKind::Bang, "__next_step");
}

#[test]
fn external_crate_coinduction_test_complex_coinduction() {
    let expansions = run_trace_for_repo("coinduction", Some("complex_coinduction"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "traitdef"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "typedef"
            && starts_with_normalized(&e.input, r#"pub mod generic_types
{
    use super::*; use std::fmt::Debug; use std::hash::Hash; pub struct
    Container<T, U> { pub first: T, pub second: U, } pub struct Wrapper<T>
    where T: Clone + Debug, { pub value: T, pub count: usize, } pub struct
    MultiGeneric<T, U, V> where T: Clone, U: Send + Sync, V: Debug + Hash,
    { pub primary: T, pub secondary: U, pub metadata: V, } pub struct
    ConstrainedStruct<T> where T: Iterator + Clone, { pub iterator: T, }
    impl<T, U> TestTrait for Container<T, U> where T: Clone +
    ::std::fmt::Debug + Send, U: ::std::fmt::Debug + Default + Sync,
    { fn test_method(&self) -> String { format!("{:?}", self.first) } }
    impl<T> TestTrait for Wrapper<T> where T: Clone + Debug + ToString,
    {
        fn test_method(&self) -> String
        { format!("{}: {}", self.value.to_string(), self.count) }"#)
            && starts_with_normalized(&e.to, r#"pub mod generic_types
{
    use super :: * ; use std :: fmt :: Debug; use std :: hash :: Hash; pub
    struct Container < T, U > { pub first : T, pub second : U, } pub struct
    Wrapper < T > where T : Clone + Debug,
    { pub value : T, pub count : usize, } pub struct MultiGeneric < T, U, V >
    where T : Clone, U : Send + Sync, V : Debug + Hash,
    { pub primary : T, pub secondary : U, pub metadata : V, } pub struct
    ConstrainedStruct < T > where T : Iterator + Clone, { pub iterator : T, }
    impl < T, U > TestTrait for Container < T, U > where T : Clone + :: std ::
    fmt :: Debug + Send, U : :: std :: fmt :: Debug + Default + Sync,
    { fn test_method(& self) -> String { format! ("{:?}", self.first) } } impl
    < T > TestTrait for Wrapper < T > where T : Clone + Debug + ToString,
    {
        fn test_method(& self) -> String"#)
    }));
}

#[test]
fn external_crate_coinduction_test_min_calculator() {
    let expansions = run_trace_for_repo("coinduction", Some("min_calculator"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "coinduction"
            && starts_with_normalized(&e.input, r#"mod calculator
{
    use super::Evaluate; pub struct Expr; pub struct Term; impl Evaluate for
    Expr where Term: Evaluate,
    {
        fn evaluate(&self, input: &[&'static str], index: &mut usize) -> i32
        {
            let left_val = Term.evaluate(input, index); let op =
            input[*index]; *index += 1; let right_val =
            Term.evaluate(input, index); match op
            {
                "+" => left_val + right_val, "-" => left_val - right_val, _ =>
                left_val,
            }
        }"#)
            && starts_with_normalized(&e.to, r#"mod calculator
{
    use super :: Evaluate; pub struct Expr; pub struct Term; impl Evaluate for
    Expr
    {
        fn evaluate(& self, input : & [& 'static str], index : & mut usize) ->
        i32
        {
            let left_val = Term.evaluate(input, index); let op = input
            [* index]; * index += 1; let right_val =
            Term.evaluate(input, index); match op
            {
                "+" => left_val + right_val, "-" => left_val - right_val, _ =>
                left_val,
            }"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "traitdef"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "typedef"
            && starts_with_normalized(&e.input, r#"pub mod generic_types
{
    use super::*; use std::fmt::Debug; use std::hash::Hash; pub struct
    Container<T, U> { pub first: T, pub second: U, } pub struct Wrapper<T>
    where T: Clone + Debug, { pub value: T, pub count: usize, } pub struct
    MultiGeneric<T, U, V> where T: Clone, U: Send + Sync, V: Debug + Hash,
    { pub primary: T, pub secondary: U, pub metadata: V, } pub struct
    ConstrainedStruct<T> where T: Iterator + Clone, { pub iterator: T, }
    impl<T, U> TestTrait for Container<T, U> where T: Clone +
    ::std::fmt::Debug + Send, U: ::std::fmt::Debug + Default + Sync,
    { fn test_method(&self) -> String { format!("{:?}", self.first) } }
    impl<T> TestTrait for Wrapper<T> where T: Clone + Debug + ToString,
    {
        fn test_method(&self) -> String
        { format!("{}: {}", self.value.to_string(), self.count) }"#)
            && starts_with_normalized(&e.to, r#"pub mod generic_types
{
    use super :: * ; use std :: fmt :: Debug; use std :: hash :: Hash; pub
    struct Container < T, U > { pub first : T, pub second : U, } pub struct
    Wrapper < T > where T : Clone + Debug,
    { pub value : T, pub count : usize, } pub struct MultiGeneric < T, U, V >
    where T : Clone, U : Send + Sync, V : Debug + Hash,
    { pub primary : T, pub secondary : U, pub metadata : V, } pub struct
    ConstrainedStruct < T > where T : Iterator + Clone, { pub iterator : T, }
    impl < T, U > TestTrait for Container < T, U > where T : Clone + :: std ::
    fmt :: Debug + Send, U : :: std :: fmt :: Debug + Default + Sync,
    { fn test_method(& self) -> String { format! ("{:?}", self.first) } } impl
    < T > TestTrait for Wrapper < T > where T : Clone + Debug + ToString,
    {
        fn test_method(& self) -> String"#)
    }));
}

#[test]
fn external_crate_decycle_test_advanced_cycles() {
    let expansions = run_trace_for_repo("decycle", Some("advanced_cycles"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
}

#[test]
fn external_crate_decycle_test_bug2() {
    let expansions = run_trace_for_repo("decycle", Some("bug2"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__MyTrait_temporal_")
            && equals_normalized(&e.input, r#""decycle" "0.3.0" [MyTrait, :: decycle :: __finalize] {}
           {
               impl < 'a, 'b, const N : usize, T > MyTrait < 'a > for MyStruct < 'a, 'b,
               N, T >
               {
                   type MyTrait = T; type T = T; fn f < 'c > (& 'a self, i : & 'c [u8])
                   -> usize { 0 }
               }
           } 10usize true"#)
            && starts_with_normalized(&e.to, r#":: decycle :: __finalize !
           {
               "decycle" "0.3.0" [:: decycle :: __finalize]
               {
                   #[allow(dead_code)] pub trait MyTrait < 'a"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && starts_with_normalized(&e.input, r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    #[allow(dead_code)] pub trait MyTrait < 'a"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] mod shadowing_module"#)
    }));
}

#[test]
fn external_crate_decycle_test_bug3() {
    let expansions = run_trace_for_repo("decycle", Some("bug3"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__Unparse_temporal_")
            && equals_normalized(&e.input, r#""decycle" "0.3.0" [Unparse, :: decycle :: __finalize] {}
          {
              impl Unparse for S
              {
                  fn unparse(& self, i : usize)
                  { if i == 0 { return; } < _ as Unparse > :: unparse(self, i - 1); }
              }
          } 10usize true"#)
            && equals_normalized(&e.to, r#":: decycle :: __finalize !
          {
              "decycle" "0.3.0" [:: decycle :: __finalize]
              { #[allow(unused)] trait Unparse { fn unparse(& self, _ : usize); }, }
              {
                  impl Unparse for S
                  {
                      fn unparse(& self, i : usize)
                      {
                          if i == 0 { return; } < _ as Unparse > ::
                          unparse(self, i - 1);
                      }
                  }
              } 10usize true
          }"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && equals_normalized(&e.input, r#""decycle" "0.3.0" [:: decycle :: __finalize]
{ #[allow(unused)] trait Unparse { fn unparse(& self, _ : usize); }, }
{
    impl Unparse for S
    {
        fn unparse(& self, i : usize)
        { if i == 0 { return; } < _ as Unparse > :: unparse(self, i - 1); }
    }
} 10usize true"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] mod shadowing_module"#)
    }));
}

#[test]
fn external_crate_decycle_test_bug4() {
    let expansions = run_trace_for_repo("decycle", Some("bug4"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__Unparse_temporal_")
            && equals_normalized(&e.input, r#""decycle" "0.3.0" [Unparse, :: decycle :: __finalize] {}
           {
               impl < __A > Unparse < __A > for ItemMod
               {
                   fn unparse < B : crate :: TraitA < __A, S = B > > (_ : & mut B) {} fn
                   f(_sink : impl TraitA < __A, S = __A >) {}
               }
           } 10usize true"#)
            && starts_with_normalized(&e.to, r#":: decycle :: __finalize !
           {
               "decycle" "0.3.0" [:: decycle :: __finalize]
               {
                   pub trait Unparse < A"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && starts_with_normalized(&e.input, r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    pub trait Unparse < A"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] mod shadowing_module"#)
    }));
}

#[test]
fn external_crate_decycle_test_bug5() {
    let expansions = run_trace_for_repo("decycle", Some("bug5"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__Parse_temporal_")
            && equals_normalized(&e.input, r#""decycle" "0.3.0" [Parse, :: decycle :: __finalize] {}
          {
              impl < Item > Parse < Item > for S
              {
                  fn parse < I : :: core :: iter :: Iterator < Item = Item > > (_ : I)
                  { todo! () }
              }
          } 10usize true"#)
            && starts_with_normalized(&e.to, r#":: decycle :: __finalize !
          {
              "decycle" "0.3.0" [:: decycle :: __finalize]
              {
                  pub trait Parse < Item"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && starts_with_normalized(&e.input, r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    pub trait Parse < Item"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] mod shadowing_module"#)
    }));
}

#[test]
fn external_crate_decycle_test_bug6() {
    let expansions = run_trace_for_repo("decycle", Some("bug6"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
}

#[test]
fn external_crate_decycle_test_coinduction_integration_test() {
    let expansions = run_trace_for_repo("decycle", Some("coinduction_integration_test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__LocalTrait_temporal_")
            && starts_with_normalized(&e.input, r#""decycle" "0.3.0" [LocalTrait, :: decycle :: __finalize]
          { pub trait TestTrait { fn test_method(& self) -> String; } }
          {
              impl TestTrait for NodeA where NodeB : LocalTrait,
              {
                  fn test_method(& self) -> String
                  {
                      let child_count =
                      self.child_b.as_ref().map_or(0, | b | b.local_method()); format!
                      ("NodeA:{}:{}", self.name, child_count)
                  }
              }, impl LocalTrait for NodeB where NodeA : TestTrait,
              {
                  fn local_method(& self) -> usize
                  {"#)
            && starts_with_normalized(&e.to, r#":: decycle :: __finalize !
          {
              "decycle" "0.3.0" [:: decycle :: __finalize]
              {
                  pub trait LocalTrait { fn local_method(& self) -> usize; }, pub trait
                  TestTrait { fn test_method(& self) -> String; }
              }
              {
                  impl TestTrait for NodeA where NodeB : LocalTrait,
                  {
                      fn test_method(& self) -> String
                      {
                          let child_count =
                          self.child_b.as_ref().map_or(0, | b | b.local_method());
                          format! ("NodeA:{}:{}", self.name, child_count)"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__finalize"
            && starts_with_normalized(&e.input, r#""decycle" "0.3.0" [:: decycle :: __finalize]
{
    pub trait LocalTrait { fn local_method(& self) -> usize; }, pub trait
    TestTrait { fn test_method(& self) -> String; }
}
{
    impl TestTrait for NodeA where NodeB : LocalTrait,
    {
        fn test_method(& self) -> String
        {
            let child_count =
            self.child_b.as_ref().map_or(0, | b | b.local_method()); format!
            ("NodeA:{}:{}", self.name, child_count)
        }
    }, impl LocalTrait for NodeB where NodeA : TestTrait,"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] mod shadowing_module"#)
    }));
}

#[test]
fn external_crate_decycle_test_complex() {
    let expansions = run_trace_for_repo("decycle", Some("complex"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
}

#[test]
fn external_crate_decycle_test_complex_coinduction() {
    let expansions = run_trace_for_repo("decycle", Some("complex_coinduction"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
}

#[test]
fn external_crate_decycle_test_min_calculator() {
    let expansions = run_trace_for_repo("decycle", Some("min_calculator"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
}

#[test]
fn external_crate_decycle_test_more_cycles() {
    let expansions = run_trace_for_repo("decycle", Some("more_cycles"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
}

#[test]
fn external_crate_decycle_test_trybuild() {
    let expansions = run_trace_for_repo("decycle", Some("trybuild"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "decycle"
            && equals_normalized(&e.input, r#"pub trait LocalTrait { fn local_method(&self) -> usize; }"#)
            && starts_with_normalized(&e.to, r#"pub trait LocalTrait { fn local_method(& self) -> usize; }
#[allow(unused_macros, unused_imports, dead_code, non_local_definitions)]
#[doc(hidden)] #[macro_export] macro_rules!
__LocalTrait_temporal_"#)
    }));
}

#[test]
fn external_crate_addr_of_enum_show_expansion() {
    let expansions = run_trace_for_repo("addr_of_enum", None);
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "chars"
            && equals_normalized(&e.input, r#"_A _B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z
           _a _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z
           _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __"#)
            && equals_normalized(&e.to, r#"#[allow(non_camel_case_types)] pub struct _A
           (:: core :: convert :: Infallible); chars!
           (_B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z _a
           _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z _0
           _1 _2 _3 _4 _5 _6 _7 _8 _9 __);"#)
    }));
}

#[test]
fn external_crate_addr_of_enum_test_test() {
    let expansions = run_trace_for_repo("addr_of_enum", Some("test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "addr_of_enum"
            && equals_normalized(&e.input, r#"&e1, E1, 0"#)
            && equals_normalized(&e.to, r#"< _ as $crate :: EnumHasTagAndField < $crate :: macro_def :: get_tstr!
           ($crate, E1), $crate :: macro_def :: get_tstr! ($crate, 0), >> ::
           addr_of(&e1 as * const _)"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "chars"
            && equals_normalized(&e.input, r#"_A _B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z
           _a _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z
           _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __"#)
            && equals_normalized(&e.to, r#"#[allow(non_camel_case_types)] pub struct _A
           (:: core :: convert :: Infallible); chars!
           (_B _C _D _E _F _G _H _I _J _K _L _M _N _O _P _Q _R _S _T _U _V _W _X _Y _Z _a
           _b _c _d _e _f _g _h _i _j _k _l _m _n _o _p _q _r _s _t _u _v _w _x _y _z _0
           _1 _2 _3 _4 _5 _6 _7 _8 _9 __);"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "get_discriminant"
            && equals_normalized(&e.input, r#"E<u8>, E1"#)
            && equals_normalized(&e.to, r#"< E<u8> as $crate :: EnumHasTag < $crate :: macro_def :: get_tstr!
           ($crate, E1)>> :: discriminant()"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "get_tstr"
            && equals_normalized(&e.input, r#"$crate, E1"#)
            && equals_normalized(&e.to, r#"($crate :: _tstr :: _E, $crate :: _tstr :: _1)"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Derive
            && e.name == "AddrOfEnum"
            && equals_normalized(&e.input, r#"#[repr(C)] #[derive(PartialEq, Eq)] enum E<T>
{ E1(usize, u8, u16), E2 { item1: u32, item2: T, }, #[allow(unused)] E3, }"#)
            && starts_with_normalized(&e.to, r#"const _ : () =
{
    #[automatically_derived] unsafe impl < T > :: addr_of_enum :: AddrOfEnum
    for E < T > {} unsafe impl < T > :: addr_of_enum :: EnumHasTag <
    (:: addr_of_enum :: _tstr :: _E, :: addr_of_enum :: _tstr :: _1), > for E
    < T >
    {
        fn discriminant() -> core :: mem :: Discriminant < Self >
        {
            let val : GhostEnum < T > = GhostEnum
            ::E1(:: core :: mem :: MaybeUninit :: uninit(), :: core :: mem ::
            MaybeUninit :: uninit(), :: core :: mem :: MaybeUninit ::
            uninit(),); #[doc = " SAFETY: both has same memory layout"] unsafe
            {
                :: core :: mem ::"#)
    }));
}

#[test]
fn external_crate_discriminant_test_test() {
    let expansions = run_trace_for_repo("discriminant", Some("test"));
    assert_has(&expansions, MacroExpansionKind::Derive, "Enum");
}

#[test]
fn external_crate_newer_type_test_enum() {
    let expansions = run_trace_for_repo("newer-type", Some("enum"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && equals_normalized(&e.input, r#"enum BasicEnum
{
    VariantA(#[implement(BasicEnumTrait)] i32),
    VariantB(#[implement(BasicEnumTrait)] i32),
}"#)
            && equals_normalized(&e.to, r#"enum BasicEnum { VariantA(i32), VariantB(i32), } BasicEnumTrait!
{
    (BasicEnumTrait) enum BasicEnum
    {
        VariantA(#[implement(BasicEnumTrait)] i32),
        VariantB(#[implement(BasicEnumTrait)] i32),
    }
}"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && equals_normalized(&e.input, r#"trait BasicEnumTrait { fn value(&self) -> i32; }"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && starts_with_normalized(&e.input, r#"((MultiImplementTrait) enum MultiImplementEnum
{
    VariantOne(#[implement(MultiImplementTrait)] i32),
    VariantTwo(#[implement(MultiImplementTrait)] i32, i32),
}) trait MultiImplementTrait { fn double(& self) -> i32; }, , :: newer_type,
(i32), Repeater, "#)
            && starts_with_normalized(&e.to, r#"#[automatically_derived] impl < > MultiImplementTrait for MultiImplementEnum
where i32 : MultiImplementTrait <> , i32 : MultiImplementTrait <>
{
    fn double(& self) -> < Self as Repeater < "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(MultiImplementTrait) enum MultiImplementEnum
            {
                VariantOne(#[implement(MultiImplementTrait)] i32),
                VariantTwo(#[implement(MultiImplementTrait)] i32, i32),
            }"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
            {
                ((MultiImplementTrait) enum MultiImplementEnum
                {
                    VariantOne(#[implement(MultiImplementTrait)] i32),
                    VariantTwo(#[implement(MultiImplementTrait)] i32, i32),
                }) trait MultiImplementTrait { fn double(& self) -> i32; }, , ::
                newer_type, (i32), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(NestedEnumTrait) enum NestedEnum
            { Variant(#[implement(NestedEnumTrait)] Box < i32 >), }"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
            {
                ((NestedEnumTrait) enum NestedEnum
                { Variant(#[implement(NestedEnumTrait)] Box < i32 >), }) trait
                NestedEnumTrait { fn nested_value(& self) -> i32; }, , :: newer_type,
                (i32), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(ComplexEnumTrait) enum ComplexEnum
            {
                Named { id : u32, #[implement(ComplexEnumTrait)] data : (i32, i32), },
                Tuple(u32, #[implement(ComplexEnumTrait)] (i32, i32)),
            }"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
            {
                ((ComplexEnumTrait) enum ComplexEnum
                {
                    Named { id : u32, #[implement(ComplexEnumTrait)] data : (i32, i32), },
                    Tuple(u32, #[implement(ComplexEnumTrait)] (i32, i32)),
                }) trait ComplexEnumTrait { fn compute(& self) -> i32; }, , :: newer_type,
                (i32), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(GenericEnumTrait < U >) enum GenericEnum < U : Clone + Debug >
           {
               First(#[implement(GenericEnumTrait<U>)] U),
               Second(#[implement(GenericEnumTrait<U>)] U),
           }"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((GenericEnumTrait < U >) enum GenericEnum < U : Clone + Debug >
               {
                   First(#[implement(GenericEnumTrait<U>)] U),
                   Second(#[implement(GenericEnumTrait<U>)] U),
               }) trait GenericEnumTrait < T > { fn describe(& self) -> String; }, , ::
               newer_type, (String), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(NamedEnumTrait) enum NamedEnum
           {
               Named { a : i32, #[implement(NamedEnumTrait)] b : i32, },
               Tuple(#[implement(NamedEnumTrait)] i32, i32),
           }"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((NamedEnumTrait) enum NamedEnum
               {
                   Named { a : i32, #[implement(NamedEnumTrait)] b : i32, },
                   Tuple(#[implement(NamedEnumTrait)] i32, i32),
               }) trait NamedEnumTrait { fn sum(& self) -> i32; }, , :: newer_type,
               (i32), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(BasicEnumTrait) enum BasicEnum
           {
               VariantA(#[implement(BasicEnumTrait)] i32),
               VariantB(#[implement(BasicEnumTrait)] i32),
           }"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((BasicEnumTrait) enum BasicEnum
               {
                   VariantA(#[implement(BasicEnumTrait)] i32),
                   VariantB(#[implement(BasicEnumTrait)] i32),
               }) trait BasicEnumTrait { fn value(& self) -> i32; }, , :: newer_type,
               (i32), Repeater, "#)
    }));
}

#[test]
fn external_crate_newer_type_test_multi_self() {
    let expansions = run_trace_for_repo("newer-type", Some("multi_self"));
    assert_has(&expansions, MacroExpansionKind::Attribute, "implement");
    assert_has(&expansions, MacroExpansionKind::Attribute, "target");
    assert_has(&expansions, MacroExpansionKind::Bang, "__implement_internal");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__newer_type_macro__");
}

#[test]
fn external_crate_newer_type_test_string() {
    let expansions = run_trace_for_repo("newer-type", Some("string"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && equals_normalized(&e.input, r#"#[allow(unused)] struct MyStruct { slot: u8, }"#)
            && equals_normalized(&e.to, r#"#[allow(unused)] struct MyStruct { slot : u8, } ToString!
{ (ToString) #[allow(unused)] struct MyStruct { slot : u8, } }"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && equals_normalized(&e.input, r#"pub trait ToString { fn to_string(&self) -> String; }"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && starts_with_normalized(&e.input, r#"((ToString) #[allow(unused)] struct MyStruct { slot : u8, }) pub trait
ToString { fn to_string(& self) -> String; }, :: std :: string :: ToString, ::
newer_type, (String), Repeater, "#)
            && starts_with_normalized(&e.to, r#"#[automatically_derived] impl < > :: std :: string :: ToString for MyStruct
where u8 : :: std :: string :: ToString <>
{
    fn to_string(& self) -> < Self as Repeater < "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(ToString) #[allow(unused)] struct MyStruct { slot : u8, }"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((ToString) #[allow(unused)] struct MyStruct { slot : u8, }) pub trait
               ToString { fn to_string(& self) -> String; }, :: std :: string ::
               ToString, :: newer_type, (String), Repeater, "#)
    }));
}

#[test]
fn external_crate_newer_type_test_test2() {
    let expansions = run_trace_for_repo("newer-type", Some("test2"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && equals_normalized(&e.input, r#"struct MyNewType(MyExistingType);"#)
            && equals_normalized(&e.to, r#"struct MyNewType(MyExistingType); MyTrait!
{ (MyTrait) struct MyNewType(MyExistingType); }"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && equals_normalized(&e.input, r#"trait MyTrait { fn value(&self) -> i32; }"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && starts_with_normalized(&e.input, r#"((DefaultTrait) struct DefaultNewType(MyExistingType);) trait DefaultTrait
{ fn default_value(& self) -> i32 { 999 } }, , :: newer_type, (i32), Repeater,
"#)
            && starts_with_normalized(&e.to, r#"#[automatically_derived] impl < > DefaultTrait for DefaultNewType where
MyExistingType : DefaultTrait <>
{
    fn default_value(& self) -> < Self as Repeater < "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(DefaultTrait) struct DefaultNewType(MyExistingType);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
            {
                ((DefaultTrait) struct DefaultNewType(MyExistingType);) trait DefaultTrait
                { fn default_value(& self) -> i32 { 999 } }, , :: newer_type, (i32),
                Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(MyTrait) struct CopyNewType(MyExistingType);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
            {
                ((MyTrait) struct CopyNewType(MyExistingType);) trait MyTrait
                { fn value(& self) -> i32; }, , :: newer_type, (i32), Repeater,
                "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(GenericTrait < T >) struct GenericNewType < T > (Option < T >);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((GenericTrait < T >) struct GenericNewType < T > (Option < T >);) trait
               GenericTrait < T > { fn get_value(& self) -> & T; }, , :: newer_type, (),
               Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(AnotherTrait) struct DualTraitNewType(MyExistingType);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((AnotherTrait) struct DualTraitNewType(MyExistingType);) trait
               AnotherTrait { fn double_value(& self) -> i32; }, , :: newer_type, (i32),
               Repeater, "#)
    }));
}

#[test]
fn external_crate_newer_type_test_test3() {
    let expansions = run_trace_for_repo("newer-type", Some("test3"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && equals_normalized(&e.input, r#"struct BasicNewType(BasicType);"#)
            && equals_normalized(
                &e.to,
                r#"struct BasicNewType(BasicType); BasicTrait!
{ (BasicTrait) struct BasicNewType(BasicType); }"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && equals_normalized(
                &e.input,
                r#"trait BasicTrait
{
    fn get_number(&self) -> i32; fn double_number(&self) -> i32
    { self.get_number() * 2 }
}"#,
            )
            && starts_with_normalized(
                &e.to,
                r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && starts_with_normalized(
                &e.input,
                r#"((AssociatedConstTrait) struct AssociatedConstNewType(BasicType);) trait
AssociatedConstTrait
{ const VALUE : i32; fn get_const_value(& self) -> i32 { Self :: VALUE } }, ,
:: newer_type, (i32), Repeater, "#,
            )
            && starts_with_normalized(
                &e.to,
                r#"#[automatically_derived] impl < > AssociatedConstTrait for
AssociatedConstNewType where BasicType : AssociatedConstTrait <>
{
    const VALUE : i32 = <BasicType as AssociatedConstTrait >::VALUE; fn
    get_const_value(& self) -> < Self as Repeater < "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(AssociatedConstTrait) struct AssociatedConstNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((AssociatedConstTrait) struct AssociatedConstNewType(BasicType);) trait
                AssociatedConstTrait
                {
                    const VALUE : i32; fn get_const_value(& self) -> i32 { Self :: VALUE }
                }, , :: newer_type, (i32), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(ComplexConstraintTrait < String >) struct
            ComplexConstraintNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((ComplexConstraintTrait < String >) struct
                ComplexConstraintNewType(BasicType);) trait ComplexConstraintTrait < T >
                where T : :: core :: fmt :: Debug + :: core :: clone :: Clone + :: core ::
                cmp :: PartialEq + :: core :: default :: Default,
                { fn process_item(& self, item : T) -> T; }, , :: newer_type, (),
                Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(MutatingTrait) struct MutatingNewType(BasicType);"#)
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((MutatingTrait) struct MutatingNewType(BasicType);) trait MutatingTrait
                { fn increment(& mut self); }, , :: newer_type, (), Repeater,
                "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(AssociatedTypeTrait) struct AssociatedTypeNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((AssociatedTypeTrait) struct AssociatedTypeNewType(BasicType);) trait
                AssociatedTypeTrait
                { type Output; fn compute(& self) -> Self :: Output; }, , :: newer_type,
                (), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(FunctionPointerTrait) struct FunctionPointerNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((FunctionPointerTrait) struct FunctionPointerNewType(BasicType);) trait
                FunctionPointerTrait { fn apply_fn(& self, f : fn(i32) -> i32) -> i32; },
                , :: newer_type, (i32, fn(i32) -> i32), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(for < 'a, A, B > AdvancedFreeParam < 'a, A, B, String > where A : Clone +
            Debug, B : PartialEq < i32 >) struct AdvancedFreeParamNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((for < 'a, A, B > AdvancedFreeParam < 'a, A, B, String > where A : Clone
                + Debug, B : PartialEq < i32 >) struct
                AdvancedFreeParamNewType(BasicType);) trait AdvancedFreeParam < 'a, A, B,
                C > where A : :: core :: clone :: Clone + :: core :: fmt :: Debug, B : ::
                core :: cmp :: PartialEq < i32 > , C : :: core :: default :: Default,
                { fn advanced_method(& self, input : & 'a A, flag : B) -> C; }, , ::
                newer_type, (i32), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(for < 'a, A > FreeParamTrait < 'a, A, u32 > where A : Clone) struct
            FreeParamNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((for < 'a, A > FreeParamTrait < 'a, A, u32 > where A : Clone) struct
                FreeParamNewType(BasicType);) trait FreeParamTrait < 'a, A, B > where A :
                :: core :: clone :: Clone,
                { fn complex_method(& self, input : & 'a A) -> B; }, , :: newer_type, (),
                Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(UltimateTrait < String, i32 >) struct UltimateNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
            {
                ((UltimateTrait < String, i32 >) struct UltimateNewType(BasicType);) trait
                UltimateTrait < T, U > where T : :: core :: fmt :: Debug + :: core ::
                clone :: Clone, U : :: core :: cmp :: PartialEq,
                { fn combine(& self, a : T, b : U) -> (T, bool); }, , :: newer_type,
                (bool), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(ComplexTrait < String >) struct ComplexNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
           {
               ((ComplexTrait < String >) struct ComplexNewType(BasicType);) trait
               ComplexTrait < T > where T : :: core :: clone :: Clone + :: core :: fmt ::
               Debug, { fn describe(& self, item : T) -> :: std :: string :: String; }, ,
               :: newer_type, (), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(AdvancedTrait < i32 >) struct AdvancedNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
           {
               ((AdvancedTrait < i32 >) struct AdvancedNewType(BasicType);) trait
               AdvancedTrait < T >
               { fn compute < U > (& self, value : T, extra : U) -> (T, U); }, , ::
               newer_type, (), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(
                &e.input,
                r#"(GenericTrait < i32 >) struct GenericNewType(BasicType);"#,
            )
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
           {
               ((GenericTrait < i32 >) struct GenericNewType(BasicType);) trait
               GenericTrait < T > { fn process(& self, input : T) -> T; }, , ::
               newer_type, (), Repeater, "#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(BasicTrait) struct BasicNewType(BasicType);"#)
            && starts_with_normalized(
                &e.to,
                r#":: newer_type :: __implement_internal!
           {
               ((BasicTrait) struct BasicNewType(BasicType);) trait BasicTrait
               {
                   fn get_number(& self) -> i32; fn double_number(& self) -> i32
                   { self.get_number() * 2 }
               }, , :: newer_type, (i32), Repeater, "#,
            )
    }));
}

#[test]
fn external_crate_newer_type_test_test4() {
    let expansions = run_trace_for_repo("newer-type", Some("test4"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && equals_normalized(&e.input, r#"struct ComplexNewType(AdvancedType);"#)
            && equals_normalized(&e.to, r#"struct ComplexNewType(AdvancedType); ComplexTrait!
{ (ComplexTrait) struct ComplexNewType(AdvancedType); }"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && equals_normalized(&e.input, r#"trait ComplexTrait
{
    const SCALE: i32; type Output; fn compute(&self, input: i32) ->
    Self::Output;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && starts_with_normalized(&e.input, r#"((for < 'a, A > FreeParamComplex < 'a, A, String > where A : Debug + Clone)
struct FreeParamComplexNewType(AdvancedType);) trait FreeParamComplex < 'a, A,
B > where A : :: core :: fmt :: Debug + :: core :: clone :: Clone, B : :: core
:: default :: Default,
{
    const MULTIPLIER : i32; type Output; fn perform(& self, input : & 'a A) ->
    (Self :: Output, B);
}, , :: newer_type, (i32), Repeater, "#)
            && starts_with_normalized(&e.to, r#"#[automatically_derived] impl < 'a_newer_type_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(for < 'a, A > FreeParamComplex < 'a, A, String > where A : Debug + Clone)
            struct FreeParamComplexNewType(AdvancedType);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
            {
                ((for < 'a, A > FreeParamComplex < 'a, A, String > where A : Debug +
                Clone) struct FreeParamComplexNewType(AdvancedType);) trait
                FreeParamComplex < 'a, A, B > where A : :: core :: fmt :: Debug + :: core
                :: clone :: Clone, B : :: core :: default :: Default,
                {
                    const MULTIPLIER : i32; type Output; fn
                    perform(& self, input : & 'a A) -> (Self :: Output, B);
                }, , :: newer_type, (i32), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(ConstrainedTrait < String >) struct ConstrainedNewType(AdvancedType);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((ConstrainedTrait < String >) struct ConstrainedNewType(AdvancedType);)
               trait ConstrainedTrait < T > where T : :: core :: fmt :: Debug + :: core
               :: clone :: Clone + :: core :: default :: Default,
               {
                   const LIMIT : usize; type Item; fn process(& self, input : T) -> Self
                   :: Item;
               }, , :: newer_type, (usize), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(MultiAssocTrait < i32 >) struct MultiAssocNewType(AdvancedType);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((MultiAssocTrait < i32 >) struct MultiAssocNewType(AdvancedType);) trait
               MultiAssocTrait < T >
               {
                   const FACTOR : T; type Result; fn transform(& self, input : T) -> Self
                   :: Result;
               }, , :: newer_type, (), Repeater, "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(ComplexTrait) struct ComplexNewType(AdvancedType);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((ComplexTrait) struct ComplexNewType(AdvancedType);) trait ComplexTrait
               {
                   const SCALE : i32; type Output; fn compute(& self, input : i32) ->
                   Self :: Output;
               }, , :: newer_type, (i32), Repeater, "#)
    }));
}

#[test]
fn external_crate_newer_type_test_test5() {
    let expansions = run_trace_for_repo("newer-type", Some("test5"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "implement"
            && equals_normalized(&e.input, r#"#[allow(unused)] struct MyWrapper(String);"#)
            && equals_normalized(&e.to, r#"#[allow(unused)] struct MyWrapper(String); m :: MyNewTrait!
{ (m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String); }"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "target"
            && equals_normalized(&e.input, r#"pub trait MyNewTrait
{
    type MyType<'a> where Self: 'a; fn get<'a>(&'a self, a: T) ->
    Self::MyType<'a>;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[macro_export] macro_rules!
__newer_type_macro__"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "__implement_internal"
            && starts_with_normalized(&e.input, r#"((m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String);) pub trait
MyNewTrait
{
    type MyType < 'a > where Self : 'a; fn get < 'a > (& 'a self, a : T) ->
    Self :: MyType < 'a > ;
}, , :: newer_type, (T), crate :: Repeater, "#)
            && starts_with_normalized(&e.to, r#"#[automatically_derived] impl < > m :: MyNewTrait for MyWrapper where String :
m :: MyNewTrait <>
{
    type MyType < 'a > = <String as m :: MyNewTrait >::MyType < 'a > where
    Self : 'a; fn get < 'a >
    (& 'a self, a : < Self as crate :: Repeater < "#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__newer_type_macro__")
            && equals_normalized(&e.input, r#"(m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String);"#)
            && starts_with_normalized(&e.to, r#":: newer_type :: __implement_internal!
           {
               ((m :: MyNewTrait) #[allow(unused)] struct MyWrapper(String);) pub trait
               MyNewTrait
               {
                   type MyType < 'a > where Self : 'a; fn get < 'a > (& 'a self, a : T)
                   -> Self :: MyType < 'a > ;
               }, , :: newer_type, (T), crate :: Repeater, "#)
    }));
}

#[test]
fn external_crate_parametrized_show_expansion() {
    let expansions = run_trace_for_repo("parametrized", None);
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && starts_with_normalized(&e.input, r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }
            {"#)
            && starts_with_normalized(&e.to, r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>
            {
                type Mapped = Vec<M>; fn
                param_map(self, f : impl FnMut(Self :: Item) -> M) -> Self :: Mapped
                { < Self as IntoIterator > :: into_iter(self).map(f).collect() }
            } emit_impl_trait!
            ([into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>, self =
            self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && equals_normalized(&e.input, r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>; [T] into_iter
            for std::collections::BTreeSet<T>; [T] into_iter for
            std::collections::HashSet<T>; [T] into_iter for
            std::collections::BinaryHeap<T>; [T] map, into_iter, iter_mut for
            std::collections::LinkedList<T>, T = M, Mapped =
            std::collections::LinkedList<M>; [T] map, into_iter, iter_mut for
            std::collections::VecDeque<T>, T = M, Mapped = std::collections::VecDeque<M>;"#)
            && starts_with_normalized(&e.to, r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,
            self = self,
            {
                Item = < Self as IntoIterator > :: Item, MIN_LEN = 0, MAX_LEN = None,
                param_len = { self.len() },
            }
            {
                lt = 'a, Iter = < & 'a Self as IntoIterator > :: IntoIter, param_iter =
                {< & 'a Self as IntoIterator > :: into_iter(self)},
            }
            {
                IterMut = < & 'a mut Self as IntoIterator > :: IntoIter, param_iter_mut =
                {< & 'a mut Self as IntoIterator > :: into_iter(self)},
            }"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && equals_normalized(&e.input, r#"[] T []"#)
            && starts_with_normalized(&e.to, r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)
            {
                type Item = T; const MIN_LEN : usize = 1; const MAX_LEN : Option < usize >
                = Some(1); fn param_len(& self) -> usize { 1 } type Iter < 'a > = :: core
                :: iter :: Once < & 'a Self :: Item > where (Self, Self :: Item): 'a; fn
                param_iter < 'a > (& 'a self) -> Self :: Iter < 'a > where Self :: Item :
                'a
                {
                    :: core :: iter ::
                    once(impl_for_tuple!
                    (@ nth []
                    [& self.0, & self.1, & self.2, & self.3, & self.4, & self.5, & self.6,
                    & self.7, & self.8, & self.9, & self.10, & self.11]))
                }
            } impl < T > ParametrizedIterMut < {impl_for_tuple! (@ count)}> for (T,)"#)
    }));
}

#[test]
fn external_crate_parametrized_test_flatten_bug() {
    let expansions = run_trace_for_repo("parametrized", Some("flatten_bug"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "parametrized"
            && equals_normalized(&e.input, r#"#[allow(unused)] enum MyEnum<A> { E1(A), E2((A,)), }"#)
            && starts_with_normalized(
                &e.to,
                r#"#[allow(unused)] enum MyEnum < A > { E1(A), E2((A,)), }
#[:: parametrized :: _imp :: sumtype :: sumtype(:: parametrized :: _imp :: sumtype :: traits :: Iterator)] impl < A > :: parametrized :: Parametrized <0usize > for MyEnum < A >"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(
                &e.input,
                r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#,
            )
            && starts_with_normalized(
                &e.to,
                r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && starts_with_normalized(
                &e.input,
                r#"impl < A > :: parametrized :: Parametrized <0usize > for MyEnum < A >
{
    type Item = A; const MIN_LEN : usize =
    {
        const fn __parametric_type_min(a : usize, b : usize) -> usize"#,
            )
            && starts_with_normalized(
                &e.to,
                r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(
                &e.to,
                r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(
                &e.input,
                r#"{
    __Sumtype_ConstraintExprTrait_0_"#,
            )
            && starts_with_normalized(
                &e.to,
                r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_impl_trait"
            && starts_with_normalized(
                &e.input,
                r#"[map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,"#,
            )
            && starts_with_normalized(
                &e.to,
                r#"impl < T, M > ParametrizedMap < 0, M > for Vec<T>"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_all"
            && starts_with_normalized(
                &e.input,
                r#"[T] map, into_iter, iter_mut for Vec<T>, T = M, Mapped = Vec<M>;"#,
            )
            && starts_with_normalized(
                &e.to,
                r#"emit_impl_trait!
            ([map, into_iter, iter_mut,] impl_generics = [T], PARAM = 0, Self = Vec<T>,"#,
            )
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "impl_for_tuple"
            && equals_normalized(&e.input, r#"[] T []"#)
            && starts_with_normalized(
                &e.to,
                r#"impl < T > Parametrized < {impl_for_tuple! (@ count)}> for (T,)"#,
            )
    }));
}

#[test]
fn external_crate_parametrized_test_recursive() {
    let expansions = run_trace_for_repo("parametrized", Some("recursive"));
    assert_has(&expansions, MacroExpansionKind::Attribute, "parametrized");
    assert_has(&expansions, MacroExpansionKind::Attribute, "sumtrait");
    assert_has(&expansions, MacroExpansionKind::Attribute, "sumtype");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__sumtype_macro_");
    assert_has(&expansions, MacroExpansionKind::Bang, "_sumtrait_internal");
    assert_has(&expansions, MacroExpansionKind::Bang, "emit_impl_trait");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_all");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_for_tuple");
}

#[test]
fn external_crate_parametrized_test_test() {
    let expansions = run_trace_for_repo("parametrized", Some("test"));
    assert_has(&expansions, MacroExpansionKind::Attribute, "parametrized");
    assert_has(&expansions, MacroExpansionKind::Attribute, "sumtrait");
    assert_has(&expansions, MacroExpansionKind::Bang, "emit_impl_trait");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_all");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_for_tuple");
}

#[test]
fn external_crate_parametrized_test_test_enum() {
    let expansions = run_trace_for_repo("parametrized", Some("test_enum"));
    assert_has(&expansions, MacroExpansionKind::Attribute, "parametrized");
    assert_has(&expansions, MacroExpansionKind::Attribute, "sumtrait");
    assert_has(&expansions, MacroExpansionKind::Attribute, "sumtype");
    assert_has_prefix(&expansions, MacroExpansionKind::Bang, "__sumtype_macro_");
    assert_has(&expansions, MacroExpansionKind::Bang, "_sumtrait_internal");
    assert_has(&expansions, MacroExpansionKind::Bang, "emit_impl_trait");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_all");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_for_tuple");
}

#[test]
fn external_crate_parametrized_test_tuple() {
    let expansions = run_trace_for_repo("parametrized", Some("tuple"));
    assert_has(&expansions, MacroExpansionKind::Attribute, "parametrized");
    assert_has(&expansions, MacroExpansionKind::Attribute, "sumtrait");
    assert_has(&expansions, MacroExpansionKind::Bang, "emit_impl_trait");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_all");
    assert_has(&expansions, MacroExpansionKind::Bang, "impl_for_tuple");
}

#[test]
fn external_crate_sumtype_show_expansion() {
    let expansions = run_trace_for_repo("sumtype", None);
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_bug() {
    let expansions = run_trace_for_repo("sumtype", Some("bug"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && starts_with_normalized(&e.input, r#"impl<T> Parametrized<0usize> for E<T>
{
    type Item = T; type Iter <'__parametrized_lt > = sumtype!
    ['__parametrized_lt] where (Self, Self :: Item) : '__parametrized_lt; fn
    param_iter<'__parametrized_lt>(&'__parametrized_lt self) ->
    Self::Iter<'__parametrized_lt> where Self::Item: '__parametrized_lt,
    {
        #[allow(unused)] match self
        {
            E::E0(__parametric_type_id_0) =>
            {
                sumtype!
                ({
                    let __parametrized_fn : fn(& '__parametrized_lt T) -> _ = |
                    __parametrized_arg |"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_debug_display_test() {
    let expansions = run_trace_for_repo("sumtype", Some("debug_display_test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"fn get_debug(use_a: bool) -> impl std::fmt::Debug
{
    if use_a { sumtype!(TestStructA(42)) } else
    { sumtype!(TestStructB("hello".to_string())) }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_error_test() {
    let expansions = run_trace_for_repo("sumtype", Some("error_test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"fn get_error(error_type: &str) -> impl std::error::Error
{
    match error_type
    {
        "io" => sumtype!(IoError("Failed to read file".to_string())), "parse"
        => sumtype!(ParseError("Invalid JSON format".to_string())), "network"
        =>
        sumtype!(NetworkError
        { code: 404, message: "Not Found".to_string() }), _ =>
        sumtype!(IoError("Unknown error".to_string())),
    }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__SumTrait_ConstraintTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__SumTrait_ConstraintTrait_1_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_1_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: traits :: Debug!
(__SumTrait_ConstraintTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_large() {
    let expansions = run_trace_for_repo("sumtype", Some("large"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"impl MyTrait for ()
{
    type Ty<'a, T> = sumtype!['a, T] where T: 'a; fn f<'a,
    T>(i: usize, t: &'a T) -> Self::Ty<'a, T>
    {
        if i == 0
        {
            sumtype!(std::iter::empty(), for<'a, T: 'a> std::iter::Empty<&'a
            T>)
        } else
        {
            sumtype!(std::iter::repeat(t).take(i), for<'a, T: 'a>
            std::iter::Take<std::iter::Repeat<&'a T>>)
        }
    }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_module() {
    let expansions = run_trace_for_repo("sumtype", Some("module"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"mod my_module
{
    #[allow(unused)] pub struct MyStruct { iter: sumtype!(), } impl MyStruct
    {
        #[allow(unused)] pub fn new(flag: bool) -> Self
        {
            let iter = if flag { sumtype!(0..5, std::ops::Range<u32>) } else
            {
                sumtype!(vec![10, 20, 30].into_iter(),
                std::vec::IntoIter<u32>)
            }; MyStruct { iter }
        } #[allow(unused)] pub fn iterate(self)
        { for value in self.iter { println!("{}", value); } }
    }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_multi() {
    let expansions = run_trace_for_repo("sumtype", Some("multi"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"#[allow(unused)] fn f(a: usize) -> impl Iterator<Item = usize> + Clone
{
    match a
    {
        0 => sumtype!(std::iter::empty::<usize>()), 1 =>
        sumtype!(std::iter::once(a)), _ =>
        sumtype!(std::iter::repeat(a).take(a)),
    }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_1_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_1_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_read() {
    let expansions = run_trace_for_repo("sumtype", Some("read"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"#[allow(unused)] fn f1(a: bool) -> impl Read
{
    if a { sumtype!(std::io::empty()) } else
    { sumtype!(std::io::Cursor::new([1, 2, 3])) }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_sumtrait() {
    let expansions = run_trace_for_repo("sumtype", Some("sumtrait"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"#[allow(unused)] fn f1(a: bool) -> impl MySumTrait + Clone
{
    #[derive(Clone)] struct S1; #[derive(Clone)] struct S2; impl MySumTrait
    for S1 {} impl MySumTrait for S2 {} if a { sumtype!(S1) } else
    { sumtype!(S2) }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#":: sumtype :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__SumTrait_ConstraintTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__SumTrait_ConstraintTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
           ({
               __SumTrait_ConstraintTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#":: sumtype :: _sumtrait_internal!
           ({
               __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"sumtype :: traits :: Copy!
(__SumTrait_ConstraintTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_test() {
    let expansions = run_trace_for_repo("sumtype", Some("test"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"fn generate_iter<'a, T>(t: &'a T, count: usize) -> impl Iterator<Item = &'a T>
{
    match count
    {
        0 => sumtype!(std::iter::empty()), 1 => sumtype!(std::iter::once(t)),
        n => sumtype!(std::iter::repeat(t).take(n)),
    }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] trait
__Sumtype_TypeRef_Trait_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_test_gparams() {
    let expansions = run_trace_for_repo("sumtype", Some("test_gparams"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"#[allow(unused)] fn with_generics<'a, T>(t: &'a T, count: usize) -> sumtype!()
{
    match count
    {
        0 => sumtype!(std::iter::empty(), std::iter::Empty<&'a T>), 1 =>
        sumtype!(std::iter::once(t), std::iter::Once<&'a T>), n =>
        sumtype!(std::iter::repeat(t).take(n),
        std::iter::Take<std::iter::Repeat<&'a T>>),
    }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_test_mod() {
    let expansions = run_trace_for_repo("sumtype", Some("test_mod"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtype"
            && equals_normalized(&e.input, r#"mod my_module
{
    #[allow(unused)] pub struct MyStruct { iter: sumtype!(), } impl MyStruct
    {
        #[allow(unused)] pub fn new(flag: bool) -> Self
        {
            let iter = if flag { sumtype!(0..5, std::ops::Range<u32>) } else
            {
                sumtype!(vec![10, 20, 30].into_iter(),
                std::vec::IntoIter<u32>)
            }; MyStruct { iter }
        } #[allow(unused)] pub fn iterate(self)
        { for value in self.iter { println!("{}", value); } }
    }
}"#)
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] #[allow(non_camel_case_types)] #[allow(non_camel_case_types)]
struct __SumType_RefType_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name.starts_with("__sumtype_macro_")
            && starts_with_normalized(&e.input, r#"__Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"$crate :: _sumtrait_internal!
          ({
              __Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "_sumtrait_internal"
            && starts_with_normalized(&e.input, r#"{
    __Sumtype_ConstraintExprTrait_0_"#)
            && starts_with_normalized(&e.to, r#"#[allow(non_camel_case_types)] trait
__Sumtype_ConstraintExprTrait_0_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

#[test]
fn external_crate_sumtype_test_ui() {
    let expansions = run_trace_for_repo("sumtype", Some("ui"));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Attribute
            && e.name == "sumtrait"
            && equals_normalized(&e.input, r#"/// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
}"#)
            && starts_with_normalized(&e.to, r#"#[doc =
" Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`]."]
#[allow(private_bounds)] pub trait Read
{
    fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
    io :: Result < :: core :: primitive :: usize > ;
} #[doc(hidden)] #[macro_export] macro_rules!
__sumtype_macro_"#)
    }));
    assert!(expansions.iter().any(|e| {
        e.kind == MacroExpansionKind::Bang
            && e.name == "emit_traits"
            && equals_normalized(&e.input, "")
            && starts_with_normalized(&e.to, r#"#[doc(hidden)] pub struct Marker(:: core :: convert :: Infallible);
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::io::Read`].
            #[sumtrait(implement = :: std :: io :: Read, krate = $crate, marker = $crate
            :: traits :: Marker)] #[allow(private_bounds)] pub trait Read
            {
                fn read(& mut self, buf : & mut [:: core :: primitive :: u8]) -> :: std ::
                io :: Result < :: core :: primitive :: usize > ;
            } impl < T : :: std :: io :: Read > Read for T
            {
                fn read(& mut self, buf : & mut [u8]) -> std :: io :: Result < usize >
                { T :: read(self, buf) }
            }
            /// Target of [`sumtype::sumtype`] macro, which implements [`std::iter::Iterator`].
            #[sumtrait(implement = :: core :: iter :: Iterator, krate = $crate, marker =
            $crate :: traits :: Marker)] #[allow(private_bounds)] pub trait Iterator"#)
    }));
}

fn assert_has_macro_name(expansions: &[MacroExpansion], expected_name: &str) {
    assert!(
        expansions.iter().any(|e| e.name == expected_name),
        "expected expansion to contain macro `{expected_name}`, got names: {:?}",
        expansions.iter().map(|e| e.name.as_str()).collect::<Vec<_>>()
    );
}

fn assert_has_macro_name_prefix(expansions: &[MacroExpansion], expected_prefix: &str) {
    assert!(
        expansions.iter().any(|e| e.name.starts_with(expected_prefix)),
        "expected expansion to contain macro prefix `{expected_prefix}`, got names: {:?}",
        expansions.iter().map(|e| e.name.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn external_crates_expansion_contains_macros_defined_in_each_crate() {
    let addr_of_enum = run_trace_for_repo("addr_of_enum", Some("test"));
    assert_has_macro_name(&addr_of_enum, "addr_of_enum");
    assert_has_macro_name(&addr_of_enum, "get_discriminant");

    let coinduction = run_trace_for_repo("coinduction", Some("complex"));
    assert_has_macro_name(&coinduction, "coinduction");
    assert_has_macro_name(&coinduction, "traitdef");
    assert_has_macro_name(&coinduction, "typedef");

    let decycle = run_trace_for_repo("decycle", Some("bug2"));
    assert_has_macro_name(&decycle, "decycle");
    assert_has_macro_name(&decycle, "__finalize");

    let discriminant = run_trace_for_repo("discriminant", Some("test"));
    assert_has_macro_name(&discriminant, "Enum");

    let flat_enum = run_trace_for_repo("flat_enum/testing", Some("test"));
    assert_has_macro_name(&flat_enum, "flat");
    assert_has_macro_name(&flat_enum, "into_flat");
    assert_has_macro_name(&flat_enum, "FlatTarget");

    let newer_type = run_trace_for_repo("newer-type", Some("enum"));
    assert_has_macro_name(&newer_type, "implement");
    assert_has_macro_name(&newer_type, "target");
    assert_has_macro_name(&newer_type, "__implement_internal");

    let parametrized = run_trace_for_repo("parametrized", Some("test"));
    assert_has_macro_name(&parametrized, "parametrized");

    let sumtype = run_trace_for_repo("sumtype", Some("test"));
    assert_has_macro_name(&sumtype, "sumtype");
    assert_has_macro_name(&sumtype, "sumtrait");
    assert_has_macro_name(&sumtype, "_sumtrait_internal");
    assert_has_macro_name(&sumtype, "emit_traits");

    assert_has_macro_name_prefix(&coinduction, "__");
    assert_has_macro_name_prefix(&decycle, "__");
    assert_has_macro_name_prefix(&newer_type, "__newer_type_macro__");
    assert_has_macro_name_prefix(&sumtype, "__sumtype_macro_");
}
