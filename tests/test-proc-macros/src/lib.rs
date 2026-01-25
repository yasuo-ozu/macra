use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident, Item, ItemStruct};

/// Attribute macro that emits the original struct, an `impl` block with a method
/// whose body calls `println!()`, and a `macro_rules!` definition.
#[proc_macro_attribute]
pub fn add_hello_method(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let macro_name = Ident::new(&format!("{}_hello", name).to_lowercase(), name.span());

    let output = quote! {
        #input

        impl #name {
            pub fn hello(&self) {
                println!("Hello from {}!", stringify!(#name));
            }
        }

        macro_rules! #macro_name {
            () => {
                println!(concat!("macro_rules! invoked for ", stringify!(#name)));
            };
        }
    };
    output.into()
}

/// Bang macro that takes an identifier and emits a function whose body contains
/// a `vec![]` macro call.
#[proc_macro]
pub fn make_answer(input: TokenStream) -> TokenStream {
    let ident = parse_macro_input!(input as Ident);

    let output = quote! {
        pub fn #ident() -> i32 {
            vec![42i32].into_iter().sum()
        }
    };
    output.into()
}

/// Derive macro that emits an `impl` block with a `greet()` method whose body
/// contains `format!()` and `stringify!()` macro calls.
#[proc_macro_derive(Greet)]
pub fn derive_greet(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let output = quote! {
        impl #name {
            pub fn greet() -> String {
                format!("Hello from {}", stringify!(#name))
            }
        }
    };
    output.into()
}

/// Derive macro that emits an `impl` block with a `describe()` method.
#[proc_macro_derive(Describe)]
pub fn derive_describe(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let output = quote! {
        impl #name {
            pub fn describe() -> String {
                format!("{} is a struct", stringify!(#name))
            }
        }
    };
    output.into()
}

/// Attribute macro that accepts complex arguments (assignments, various group
/// delimiters) and preserves the annotated item.  Generates a const whose value
/// is the stringified attribute arguments.
#[proc_macro_attribute]
pub fn tag_item(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as Item);
    let args_str = attr.to_string();
    let const_name = Ident::new(
        &format!(
            "__TAG_ARGS_FOR_{}",
            match &input {
                Item::Struct(s) => s.ident.to_string(),
                Item::Fn(f) => f.sig.ident.to_string(),
                Item::Enum(e) => e.ident.to_string(),
                _ => "unknown".to_string(),
            }
        ),
        proc_macro2::Span::call_site(),
    );

    let output = quote! {
        #input
        pub const #const_name: &str = #args_str;
    };
    output.into()
}
