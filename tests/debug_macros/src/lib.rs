//! Proc macro crate for macro expansion debugging.
//!
//! Provides three kinds of proc macros, each generating identifiable items
//! so that a macro expansion debugger can distinguish every expansion.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, punctuated::Punctuated, Ident, ItemStruct, Token};

// ---------------------------------------------------------------------------
// A. Function-like proc macro: emit_trace!(label)
// ---------------------------------------------------------------------------
//
// Generates:
//   - const __TRACE_FNLIKE_<label>: &str = "...";
//   - macro_rules! __trace_generated_<label> { ... }
//   - invocation of that macro_rules!, producing:
//       const __TRACE_GENERATED_<label>: &str = "...";

#[proc_macro]
pub fn emit_trace(input: TokenStream) -> TokenStream {
    let label = parse_macro_input!(input as Ident);

    let const_name = format_ident!("__TRACE_FNLIKE_{}", label);
    let macro_name = format_ident!("__trace_generated_{}", label);
    let generated_const = format_ident!("__TRACE_GENERATED_{}", label);
    let value_fnlike = format!("fnlike:{}", label);
    let tag = format_ident!("from_{}", label);

    let output = quote! {
        pub const #const_name: &str = #value_fnlike;

        macro_rules! #macro_name {
            ($marker:ident) => {
                pub const #generated_const: &str =
                    concat!("generated:", stringify!(#label), ":", stringify!($marker));
            };
        }

        #macro_name!(#tag);
    };
    output.into()
}

// ---------------------------------------------------------------------------
// B. Attribute proc macro: #[trace_attr(label, instance)]
// ---------------------------------------------------------------------------
//
// Preserves the target item and additionally generates:
//   const __TRACE_ATTR_<label>_<instance>_FOR_<ItemName>: &str = "...";

#[proc_macro_attribute]
pub fn trace_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Ident, Token![,]>::parse_terminated);
    let args: Vec<&Ident> = args.iter().collect();
    assert!(args.len() == 2, "trace_attr expects exactly 2 idents: label, instance");
    let label = args[0];
    let instance = args[1];

    let input = parse_macro_input!(item as ItemStruct);
    let target = &input.ident;

    let const_name = format_ident!("__TRACE_ATTR_{}_{}_FOR_{}", label, instance, target);
    let value = format!("attr:{}:{}:{}", label, instance, target);

    let output = quote! {
        #input

        pub const #const_name: &str = #value;
    };
    output.into()
}

// ---------------------------------------------------------------------------
// C. Derive proc macro: #[derive(TraceDerive)]
//    with helper attribute: #[trace_derive(label, instance)]
// ---------------------------------------------------------------------------
//
// Generates:
//   const __TRACE_DERIVE_<label>_<instance>_FOR_<TypeName>: &str = "...";

#[proc_macro_derive(TraceDerive, attributes(trace_derive))]
pub fn derive_trace(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let target = &input.ident;

    // Find the #[trace_derive(label, instance)] attribute
    let trace_attr = input
        .attrs
        .iter()
        .find(|a| a.path().is_ident("trace_derive"))
        .expect("TraceDerive requires #[trace_derive(label, instance)]");

    let args: Punctuated<Ident, Token![,]> = trace_attr
        .parse_args_with(Punctuated::parse_terminated)
        .expect("trace_derive expects (label, instance)");
    let args: Vec<&Ident> = args.iter().collect();
    assert!(args.len() == 2, "trace_derive expects exactly 2 idents");
    let label = args[0];
    let instance = args[1];

    let const_name = format_ident!("__TRACE_DERIVE_{}_{}_FOR_{}", label, instance, target);
    let value = format!("derive:{}:{}:{}", label, instance, target);

    let output = quote! {
        pub const #const_name: &str = #value;
    };
    output.into()
}
