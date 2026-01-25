use test_proc_macros::{Describe, Greet, add_hello_method, make_answer};

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
