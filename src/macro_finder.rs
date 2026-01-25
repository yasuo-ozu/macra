use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, Expr, ExprMacro, Item, ItemMacro, Macro, Stmt, StmtMacro};

/// Represents a macro call found in the source code.
#[derive(Debug, Clone)]
pub struct MacroCall {
    /// The name of the macro (e.g., "println", "derive", "test")
    pub name: String,
    /// The type of macro
    pub kind: MacroKind,
    /// The line number (1-indexed)
    pub line: usize,
    /// The start column (0-indexed)
    pub col_start: usize,
    /// The end column (0-indexed, exclusive)
    pub col_end: usize,
    /// The end line (1-indexed) of the macro call / attribute itself
    pub line_end: usize,
    /// The end line (1-indexed) of the target item (for attr/derive macros)
    pub item_line_end: usize,
    /// The macro input tokens (for strict matching)
    pub input: String,
    /// The attribute arguments (e.g., for `#[foo(a, b)]` this is `a, b`)
    pub arguments: String,
    /// For derive macros: names of all derives in the same `#[derive(...)]` attribute.
    /// Empty for non-derive macros.
    pub sibling_derives: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacroKind {
    /// Function-like macro: `println!(...)`
    Functional,
    /// Attribute macro: `#[test]`, `#[derive(...)]`
    Attribute,
    /// Derive macro: inside `#[derive(...)]`
    Derive,
}

impl MacroKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MacroKind::Functional => "fn",
            MacroKind::Attribute => "attr",
            MacroKind::Derive => "derive",
        }
    }
}

/// Get a mutable reference to the attributes of an `Item`.
fn get_item_attrs_mut(item: &mut Item) -> Option<&mut Vec<Attribute>> {
    match item {
        Item::Const(i) => Some(&mut i.attrs),
        Item::Enum(i) => Some(&mut i.attrs),
        Item::ExternCrate(i) => Some(&mut i.attrs),
        Item::Fn(i) => Some(&mut i.attrs),
        Item::ForeignMod(i) => Some(&mut i.attrs),
        Item::Impl(i) => Some(&mut i.attrs),
        Item::Macro(i) => Some(&mut i.attrs),
        Item::Mod(i) => Some(&mut i.attrs),
        Item::Static(i) => Some(&mut i.attrs),
        Item::Struct(i) => Some(&mut i.attrs),
        Item::Trait(i) => Some(&mut i.attrs),
        Item::TraitAlias(i) => Some(&mut i.attrs),
        Item::Type(i) => Some(&mut i.attrs),
        Item::Union(i) => Some(&mut i.attrs),
        Item::Use(i) => Some(&mut i.attrs),
        _ => None,
    }
}

/// Serialize an item without the attribute at the given index.
fn item_without_attr(item: &Item, attr_idx: usize) -> String {
    let mut item = item.clone();
    if let Some(attrs) = get_item_attrs_mut(&mut item) {
        if attr_idx < attrs.len() {
            attrs.remove(attr_idx);
        }
    }
    item.to_token_stream().to_string()
}

/// Visitor that collects macro calls from a syn AST.
struct MacroVisitor {
    macros: Vec<MacroCall>,
}

impl MacroVisitor {
    fn new() -> Self {
        Self { macros: Vec::new() }
    }

    fn add_macro_from_path(&mut self, mac: &Macro, kind: MacroKind) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();

        // Get span of the entire macro call
        let start_span = mac.path.segments.first().map(|s| s.ident.span());
        let end_span = mac.delimiter.span().close();

        // Skip macro_rules definitions — they are not macro invocations
        if name == "macro_rules" {
            return;
        }

        if let Some(start) = start_span {
            let line = start.start().line;
            let col_start = start.start().column;
            let line_end = end_span.end().line;
            let col_end = end_span.end().column;

            // Capture just the tokens inside the delimiters (the macro arguments)
            let input = mac.tokens.to_string();
            self.macros.push(MacroCall {
                name,
                kind,
                line,
                col_start,
                col_end,
                line_end,
                item_line_end: line_end,
                input,
                arguments: String::new(),
                sibling_derives: Vec::new(),
            });
        }
    }

    fn process_attributes(&mut self, attrs: &[Attribute], item: &Item) {
        // Get the end line of the full item (attribute + body)
        let item_span = item.to_token_stream().into_iter().last();
        let item_line_end = item_span
            .map(|t| t.span().end().line)
            .unwrap_or(0);

        for (attr_idx, attr) in attrs.iter().enumerate() {
            let name = attr
                .path()
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();

            // Get span of the entire attribute
            let attr_span = attr.span();
            let line = attr_span.start().line;
            let col_start = attr_span.start().column;
            let line_end = attr_span.end().line;
            let col_end = attr_span.end().column;

            if name == "derive" {
                // Skip the derive attribute itself; only emit individual derive macros
                // Derive macros receive the item without the #[derive(...)] attribute
                let input = item_without_attr(item, attr_idx);
                if let syn::Meta::List(list) = &attr.meta {
                    let tokens_str = list.tokens.to_string();
                    let all_derive_names: Vec<String> = tokens_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    for derive_name in &all_derive_names {
                        self.macros.push(MacroCall {
                            name: derive_name.clone(),
                            kind: MacroKind::Derive,
                            line,
                            col_start,
                            col_end,
                            line_end,
                            item_line_end,
                            input: input.clone(),
                            arguments: String::new(),
                            sibling_derives: all_derive_names.clone(),
                        });
                    }
                }
            } else {
                // Attribute macros receive the item without the triggering attribute
                let input = item_without_attr(item, attr_idx);
                // Extract attribute arguments (e.g., for `#[foo(a, b)]` → "a, b")
                let arguments = match &attr.meta {
                    syn::Meta::List(list) => list.tokens.to_string(),
                    syn::Meta::NameValue(nv) => nv.value.to_token_stream().to_string(),
                    syn::Meta::Path(_) => String::new(),
                };
                self.macros.push(MacroCall {
                    name,
                    kind: MacroKind::Attribute,
                    line,
                    col_start,
                    col_end,
                    line_end,
                    item_line_end,
                    input,
                    arguments,
                    sibling_derives: Vec::new(),
                });
            }
        }
    }
}

impl<'a> Visit<'a> for MacroVisitor {
    fn visit_item(&mut self, item: &'a Item) {
        // Process attributes on items
        match item {
            Item::Const(i) => self.process_attributes(&i.attrs, item),
            Item::Enum(i) => self.process_attributes(&i.attrs, item),
            Item::ExternCrate(i) => self.process_attributes(&i.attrs, item),
            Item::Fn(i) => self.process_attributes(&i.attrs, item),
            Item::ForeignMod(i) => self.process_attributes(&i.attrs, item),
            Item::Impl(i) => self.process_attributes(&i.attrs, item),
            Item::Macro(i) => self.process_attributes(&i.attrs, item),
            Item::Mod(i) => self.process_attributes(&i.attrs, item),
            Item::Static(i) => self.process_attributes(&i.attrs, item),
            Item::Struct(i) => self.process_attributes(&i.attrs, item),
            Item::Trait(i) => self.process_attributes(&i.attrs, item),
            Item::TraitAlias(i) => self.process_attributes(&i.attrs, item),
            Item::Type(i) => self.process_attributes(&i.attrs, item),
            Item::Union(i) => self.process_attributes(&i.attrs, item),
            Item::Use(i) => self.process_attributes(&i.attrs, item),
            _ => {}
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_item_macro(&mut self, node: &'a ItemMacro) {
        self.add_macro_from_path(&node.mac, MacroKind::Functional);
        syn::visit::visit_item_macro(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'a ExprMacro) {
        self.add_macro_from_path(&node.mac, MacroKind::Functional);
        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        // Check for macro in various expression positions
        if let Expr::Macro(m) = expr {
            self.add_macro_from_path(&m.mac, MacroKind::Functional);
        }
        syn::visit::visit_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        // Handle statement-level macros (like println!(...);)
        if let Stmt::Macro(m) = stmt {
            self.add_macro_from_path(&m.mac, MacroKind::Functional);
        }
        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_stmt_macro(&mut self, node: &'a StmtMacro) {
        self.add_macro_from_path(&node.mac, MacroKind::Functional);
        syn::visit::visit_stmt_macro(self, node);
    }
}

/// Returns true if the given name is a compiler built-in attribute that does
/// not produce `-Z trace-macros` output and should be hidden from the TUI.
pub fn is_builtin_attribute(name: &str) -> bool {
    matches!(
        name,
        "doc"
            | "cfg"
            | "cfg_attr"
            | "allow"
            | "warn"
            | "deny"
            | "forbid"
            | "deprecated"
            | "must_use"
            | "repr"
            | "inline"
            | "cold"
            | "track_caller"
            | "link"
            | "link_name"
            | "link_section"
            | "no_mangle"
            | "used"
            | "path"
            | "non_exhaustive"
            | "automatically_derived"
            | "global_allocator"
            | "export_name"
            | "macro_use"
            | "macro_export"
    )
}

/// Returns true if the given name is a compiler built-in function-like macro
/// that does not produce `-Z trace-macros` output.
fn is_builtin_functional(name: &str) -> bool {
    matches!(
        name,
        "include_str"
            | "include_bytes"
            | "include"
            | "env"
            | "option_env"
            | "concat"
            | "stringify"
            | "line"
            | "column"
            | "file"
            | "module_path"
            | "cfg"
            | "compile_error"
            | "format_args"
            | "format_args_nl"
    )
}

/// Find all macro calls in the given Rust source code.
pub fn find_macros(source: &str) -> Vec<MacroCall> {
    let file = match syn::parse_file(source) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let mut visitor = MacroVisitor::new();
    visitor.visit_file(&file);

    // Sort by line number
    visitor.macros.sort_by_key(|m| m.line);

    // Filter out compiler built-in macros that don't produce trace output
    visitor.macros.retain(|m| match m.kind {
        MacroKind::Functional => !is_builtin_functional(&m.name),
        MacroKind::Attribute => !is_builtin_attribute(&m.name),
        MacroKind::Derive => true, // derive macros are always user-defined
    });

    // Deduplicate based on line and name
    let mut seen = std::collections::HashSet::new();
    visitor
        .macros
        .into_iter()
        .filter(|m| seen.insert((m.line, m.name.clone(), m.kind)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_functional_macros() {
        let source = r#"
fn main() {
    println!("Hello");
    vec![1, 2, 3];
}
"#;
        let macros = find_macros(source);
        let names: Vec<_> = macros.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"println"));
        assert!(names.contains(&"vec"));
    }

    #[test]
    fn test_functional_macro_columns() {
        // Test that col_start and col_end correctly capture the macro call bounds
        let source = "fn test() {\n    let v = vec![1, 2, 3];\n}";
        // Line 2: "    let v = vec![1, 2, 3];"
        //          0123456789012345678901234567
        //                      ^           ^
        //                   col_start=12  col_end=25
        let macros = find_macros(source);
        let vec_mac = macros.iter().find(|m| m.name == "vec").unwrap();

        assert_eq!(vec_mac.line, 2);
        assert_eq!(vec_mac.col_start, 12);
        assert_eq!(vec_mac.col_end, 25);
        assert_eq!(vec_mac.line_end, 2);

        // Verify we can slice the original line correctly
        let line = source.lines().nth(vec_mac.line - 1).unwrap();
        assert_eq!(&line[..vec_mac.col_start], "    let v = ");
        assert_eq!(&line[vec_mac.col_end..], ";");
    }

    #[test]
    fn test_find_attribute_macros() {
        let source = r#"
#[test]
fn my_test() {}

#[my_custom_attr]
fn custom() {}
"#;
        let macros = find_macros(source);
        let names: Vec<_> = macros.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"test"));
        assert!(names.contains(&"my_custom_attr"));
        // Built-in attributes like cfg, doc, allow should be filtered out
        let source2 = r#"
#[cfg(test)]
mod tests {}

#[doc = "hello"]
fn documented() {}

#[allow(unused)]
fn linted() {}
"#;
        let macros2 = find_macros(source2);
        let names2: Vec<_> = macros2.iter().map(|m| m.name.as_str()).collect();
        assert!(!names2.contains(&"cfg"), "cfg should be filtered as built-in");
        assert!(!names2.contains(&"doc"), "doc should be filtered as built-in");
        assert!(!names2.contains(&"allow"), "allow should be filtered as built-in");
    }

    #[test]
    fn test_find_derive_macros() {
        let source = r#"
#[derive(Debug, Clone, PartialEq)]
struct Foo {
    x: i32,
}
"#;
        let macros = find_macros(source);
        let names: Vec<_> = macros.iter().map(|m| m.name.as_str()).collect();
        assert!(!names.contains(&"derive"), "derive attribute itself should be skipped");
        assert!(names.contains(&"Debug"));
        assert!(names.contains(&"Clone"));
        assert!(names.contains(&"PartialEq"));
    }

    #[test]
    fn test_multiple_attribute_macros_on_one_item() {
        let source = r#"
#[attr_a]
#[attr_b(x, y)]
pub struct Multi {
    val: i32,
}
"#;
        let macros = find_macros(source);
        let attrs: Vec<_> = macros
            .iter()
            .filter(|m| m.kind == MacroKind::Attribute)
            .collect();

        assert_eq!(attrs.len(), 2, "Expected 2 attribute macros: {:?}", attrs);

        let a = attrs.iter().find(|m| m.name == "attr_a").unwrap();
        let b = attrs.iter().find(|m| m.name == "attr_b").unwrap();

        // Both should span from their attribute line to the item end (line 6: `}`)
        assert_eq!(a.line, 2);
        assert_eq!(a.item_line_end, 6);
        assert_eq!(b.line, 3);
        assert_eq!(b.item_line_end, 6);

        // attr_a's input should be item without #[attr_a] (but WITH #[attr_b])
        assert!(
            a.input.contains("attr_b"),
            "attr_a input should retain #[attr_b]: {}",
            a.input
        );
        assert!(
            !a.input.contains("attr_a"),
            "attr_a input should NOT contain #[attr_a]: {}",
            a.input
        );

        // attr_b's input should be item without #[attr_b] (but WITH #[attr_a])
        assert!(
            b.input.contains("attr_a"),
            "attr_b input should retain #[attr_a]: {}",
            b.input
        );
        assert!(
            !b.input.contains("attr_b"),
            "attr_b input should NOT contain #[attr_b]: {}",
            b.input
        );

        // attr_b should have arguments (syn adds space before comma)
        assert_eq!(b.arguments, "x , y");
        assert_eq!(a.arguments, "");
    }

    #[test]
    fn test_multiple_derives_one_attribute() {
        let source = r#"
#[derive(Debug, Clone, PartialEq)]
struct Bar {
    x: i32,
}
"#;
        let macros = find_macros(source);
        let derives: Vec<_> = macros
            .iter()
            .filter(|m| m.kind == MacroKind::Derive)
            .collect();

        assert_eq!(derives.len(), 3, "Expected 3 derive macros: {:?}", derives);

        // All derives should cover the same item span
        for d in &derives {
            assert_eq!(d.line, 2, "derive {} line", d.name);
            assert_eq!(d.item_line_end, 5, "derive {} item_line_end", d.name);
        }

        // Input should be the item WITHOUT #[derive(...)]
        let debug = derives.iter().find(|m| m.name == "Debug").unwrap();
        assert!(
            !debug.input.contains("derive"),
            "derive input should not contain #[derive]: {}",
            debug.input
        );
        assert!(
            debug.input.contains("struct Bar"),
            "derive input should contain the struct: {}",
            debug.input
        );
    }

    #[test]
    fn test_multiple_derives_two_attributes() {
        let source = r#"
#[derive(Debug)]
#[derive(Clone, PartialEq)]
struct Baz;
"#;
        let macros = find_macros(source);
        let derives: Vec<_> = macros
            .iter()
            .filter(|m| m.kind == MacroKind::Derive)
            .collect();

        assert_eq!(derives.len(), 3, "Expected 3 derive macros: {:?}", derives);

        // Debug is on line 2 (first derive attr)
        let debug = derives.iter().find(|m| m.name == "Debug").unwrap();
        assert_eq!(debug.line, 2);
        assert_eq!(debug.item_line_end, 4);

        // Clone and PartialEq are on line 3 (second derive attr)
        let clone = derives.iter().find(|m| m.name == "Clone").unwrap();
        let partialeq = derives.iter().find(|m| m.name == "PartialEq").unwrap();
        assert_eq!(clone.line, 3);
        assert_eq!(clone.item_line_end, 4);
        assert_eq!(partialeq.line, 3);
        assert_eq!(partialeq.item_line_end, 4);

        // Debug's input should exclude its own #[derive(Debug)] but keep #[derive(Clone, PartialEq)]
        assert!(
            !debug.input.contains("Debug"),
            "Debug input should not contain its own derive: {}",
            debug.input
        );
        assert!(
            debug.input.contains("Clone"),
            "Debug input should retain the other derive attr: {}",
            debug.input
        );

        // Clone's input should exclude #[derive(Clone, PartialEq)] but keep #[derive(Debug)]
        assert!(
            clone.input.contains("Debug"),
            "Clone input should retain #[derive(Debug)]: {}",
            clone.input
        );
        assert!(
            !clone.input.contains("Clone"),
            "Clone input should not contain its own derive: {}",
            clone.input
        );
    }
}
