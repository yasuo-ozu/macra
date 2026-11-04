use test_proc_macros::{Describe, Greet, WithHelpers, add_hello_method, make_answer};

// --- MBE macro definitions ---

macro_rules! repeat_twice {
    ($e:expr) => {
        ($e, $e)
    };
}

macro_rules! make_struct {
    ($name:ident) => {
        #[derive(Greet)]
        pub struct $name;
    };
}

// --- Proc macro usage ---

// Attribute macro (emits println! + macro_rules!)
#[add_hello_method]
pub struct MyStruct {
    pub value: i32,
}

// Bang macro (emits vec![])
make_answer!(get_answer);

// Derive macro (emits format! + stringify!)
#[derive(Greet)]
pub struct Greeter;

// --- MBE macro usage ---

pub fn use_mbe() -> (i32, i32) {
    repeat_twice!(get_answer())
}

// Use the macro_rules! emitted by #[add_hello_method]
pub fn call_generated_macro() {
    mystruct_hello!();
}

// MBE that expands to a derive proc macro call
make_struct!(AutoGreeter);

// --- Multi-segment path invocations ---

// macro_rules! via module re-export path
mod inner_macros {
    macro_rules! repeat_thrice {
        ($e:expr) => {
            ($e, $e, $e)
        };
    }
    pub(crate) use repeat_thrice;
}

pub fn use_path_macro() -> (i32, i32, i32) {
    inner_macros::repeat_thrice!(1)
}

// Functional proc macro via crate path (two-segment path)
test_proc_macros::make_answer!(get_answer_path);

// Attribute proc macro via crate path (two-segment path)
#[test_proc_macros::add_hello_method]
pub struct PathStruct {
    pub value: i32,
}

// Derive proc macro via crate path (two-segment path)
#[derive(test_proc_macros::Greet)]
pub struct PathGreeter;

// --- Attribute proc macro with complex arguments ---

// Uses assignment (=), brackets ([]), parentheses (()), and braces ({})
#[test_proc_macros::tag_item(name = "tagged", items = [1, 2, 3], opts = (verbose, debug), config = {key: value})]
pub struct TaggedStruct;

// --- Multiple attribute macros on one item ---

#[test_proc_macros::tag_item(role = "primary")]
#[add_hello_method]
pub struct MultiAttrStruct {
    pub id: u32,
}

// --- Multiple derive macros on one derive attribute ---

#[derive(Greet, Describe)]
pub struct MultiDeriveOneAttr;

// --- Multiple derive macros on two derive attributes ---

#[derive(Greet)]
#[derive(Describe)]
pub struct MultiDeriveTwoAttr;

// --- Derive with a helper attribute on the same item ---

// `helper_one` is declared by `#[proc_macro_derive(WithHelpers, attributes(helper_one,
// helper_two))]`: it is inert, never expands, and the hook must report it on the
// derive's record so the TUI does not take it for an attribute macro standing in
// front of the derive.
#[derive(WithHelpers)]
#[helper_one]
pub struct HelperUser;

// --- A compiler built-in derive beside a proc-macro derive ---

// `Debug` is expanded inside rustc and never crosses the proc-macro bridge, so the
// hook records `Greet` for this item and nothing at all for `Debug`. The TUI leans
// on that absence: a built-in name with no derive record is reported as built-in
// instead of failing over to a sibling, which used to expand `Greet` when the user
// asked for `Debug`.
#[derive(Debug, Greet)]
pub struct BuiltinBeside;

#[allow(dead_code)]
pub fn call_generated_path_macros() {
    pathstruct_hello!();
    multiattrstruct_hello!();
}

// --- Bang macros in type position ---

// `type Id = cxx::type_id!("std::string_view");` in an `unsafe impl` is the report
// this mirrors. A bang macro may stand wherever its expansion may, and a type is one
// of those places; `syn` gives it a node of its own, which the finder never visited.
macro_rules! unit_ty {
    () => {
        ()
    };
}

pub trait HasId {
    type Id;
}

pub struct IdHolder;

impl HasId for IdHolder {
    type Id = test_proc_macros::answer_ty!();
}

pub fn type_position_mbe() -> unit_ty!() {}

// --- Attribute macros on associated and foreign items ---

// rustc takes an attribute macro on any item, and an associated or foreign item is
// one. `syn` gives each its own node, so each was invisible to the finder until it
// had a hook of its own, and `impl S { #[my_attr] fn f() {} }` listed nothing.
pub struct Assoc;

impl Assoc {
    #[test_proc_macros::passthrough]
    pub fn assoc_method(&self) -> i32 {
        1
    }

    #[test_proc_macros::passthrough]
    pub const ASSOC_CONST: i32 = 2;
}

pub trait AssocTrait {
    #[test_proc_macros::passthrough]
    fn trait_method(&self) -> i32;
}

unsafe extern "C" {
    #[test_proc_macros::passthrough]
    pub fn foreign_fn() -> i32;
}
