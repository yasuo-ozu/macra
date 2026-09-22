use std::io::{BufRead, BufReader, Read};

/// The kind of macro invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroExpansionKind {
    /// Function-like macro: `name!(...)` / `name![...]` / `name!{...}`
    Bang,
    /// Attribute macro: `#[name]` or `#[name(...)]`
    Attribute,
    /// Derive macro: `#[derive(Name)]`
    Derive,
}

/// A single macro expansion pair: the "expanding" text and the "to" text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroExpansion {
    pub expanding: String,
    pub arguments: String,
    pub to: String,
    /// Macro name (e.g. `"println"`, `"derive"`, `"test"`).
    pub name: String,
    /// Crate that defines the macro, when known. Empty for `-Z trace-macros`
    /// output, which does not report it. Deliberately separate from `name`: two
    /// same-named macros from different crates are told apart by this, but a derive
    /// is routinely invoked through a re-export — `#[derive(serde::Serialize)]` for
    /// a macro defined in `serde_derive` — so it can never be a match requirement.
    pub krate: String,
    /// Kind of macro invocation.
    pub kind: MacroExpansionKind,
    /// Raw input token stream that the macro receives.
    pub input: String,
}

/// A group of macro expansions from a single `note: trace_macro` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceGroup {
    pub expansions: Vec<MacroExpansion>,
}

/// Iterator over trace groups parsed from macro tracing output.
pub struct TraceParser<R: Read> {
    reader: BufReader<R>,
    current_line: String,
    peeked_line: Option<String>,
}

impl<R: Read> TraceParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            current_line: String::new(),
            peeked_line: None,
        }
    }

    fn strip_ansi_escape_sequences(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                // Skip CSI sequences: ESC [ ... final-byte
                if chars.peek() == Some(&'[') {
                    let _ = chars.next();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                continue;
            }
            out.push(ch);
        }
        out
    }

    fn read_line(&mut self) -> Option<String> {
        if let Some(line) = self.peeked_line.take() {
            return Some(line);
        }
        self.current_line.clear();
        match self.reader.read_line(&mut self.current_line) {
            Ok(0) => None,
            Ok(_) => {
                // Also drop a '\r' so a CRLF-converted capture still ends its
                // lines in the closing backtick.
                let line = self
                    .current_line
                    .trim_end_matches('\n')
                    .trim_end_matches('\r');
                Some(Self::strip_ansi_escape_sequences(line))
            }
            Err(_) => None,
        }
    }

    fn peek_line(&mut self) -> Option<&str> {
        if self.peeked_line.is_none() {
            self.peeked_line = self.read_line();
        }
        self.peeked_line.as_deref()
    }

    /// Extract the backtick-delimited text of a `= note: expanding` / `= note: to`
    /// note, consuming continuation lines as needed.  `first_line` is the line
    /// holding the `= note:` prefix; it has already been consumed.
    ///
    /// rustc wraps the text in backticks and the text may itself contain
    /// backticks: a `macro_rules!` that emits a documented item produces
    ///
    ///   = note: to `/// Wraps a [`Vec`] of items; see also [`String`] and [`Option`] for the general idea.
    ///           pub struct Foo
    ///           { ... }
    ///           impl Foo { pub const N : u32 = inner! (); }`
    ///
    /// so neither "the last backtick on the first line" nor "the first line that
    /// ends in a backtick" locates the closing delimiter: the first line ends in
    /// one and the note goes on for three more.  What is reliable is the shape
    /// of rustc's emitter output: every continuation line of a multi-line note
    /// is padded with spaces up to the column where the note text starts, i.e.
    /// right after `= note: ` (10 spaces for a one-digit line-number gutter, 11
    /// for two digits, 13 for four; verified on 1.86, 1.97 and nightly).  That
    /// padding is applied even to an empty line inside a raw string literal.
    /// Nothing that can follow a note is indented that deeply: the next
    /// `= note:` sits 8 columns to the left, `note: trace_macro`, `warning:` and
    /// `error:` start at column 0, and cargo's right-aligned status words
    /// (`    Finished`, `     Running`) never exceed 7 spaces.
    ///
    /// So a line ends the note iff it ends with a backtick *and* the next line
    /// is not a continuation line (or there is no next line).  A line that does
    /// not end in a backtick is never terminal, whatever follows it, which keeps
    /// the behaviour for ordinary notes identical to before.  EOF before the
    /// closing backtick yields `None`.
    fn extract_backtick_content(&mut self, first_line: &str) -> Option<String> {
        // Find the opening backtick.  It is ASCII, so `start_idx + 1` is a char
        // boundary.
        let start_idx = first_line.find('`')?;

        // Column at which rustc pads continuation lines: the text column of the
        // note, i.e. just past `= note: `.  Callers only pass lines that contain
        // `= note: `, so the fallback is never hit in practice; the opening
        // backtick's column is the closest stand-in if it ever is.
        let pad = first_line
            .find("note: ")
            .map(|i| i + "note: ".len())
            .unwrap_or(start_idx + 1);

        let mut content = String::new();
        let mut line = first_line[start_idx + 1..].to_string();
        loop {
            if line.ends_with('`') && !self.next_line_is_continuation(pad) {
                // '`' is one byte, so `len() - 1` is a char boundary.
                content.push_str(&line[..line.len() - 1]);
                return Some(content);
            }
            content.push_str(&line);
            content.push('\n');
            line = self.read_line()?;
        }
    }

    /// Whether the next line (without consuming it) is a continuation line of
    /// the current note, i.e. starts with at least `pad` spaces.  EOF is not a
    /// continuation.  Compares bytes, so multibyte content after the padding is
    /// irrelevant.
    fn next_line_is_continuation(&mut self, pad: usize) -> bool {
        match self.peek_line() {
            Some(next) => {
                let bytes = next.as_bytes();
                bytes.len() >= pad && bytes[..pad].iter().all(|&b| b == b' ')
            }
            None => false,
        }
    }

    /// Extract macro name from an `expanding` string.
    ///
    /// The expanding string has the form `name! { args }` (or with `()` / `[]`).
    /// Returns the name part before `!`.
    fn extract_macro_name(expanding: &str) -> String {
        if let Some(bang_pos) = expanding.find('!') {
            expanding[..bang_pos].trim().to_string()
        } else {
            expanding.to_string()
        }
    }

    /// Extract macro arguments from an `expanding` string.
    ///
    /// The expanding string has the form `name! { args }` (or with `()` / `[]`).
    /// Returns the content between the outermost delimiters, trimmed.
    fn extract_arguments(expanding: &str) -> String {
        // Find the `!` that separates macro name from arguments
        let bang_pos = match expanding.find('!') {
            Some(p) => p,
            None => return String::new(),
        };
        let after_bang = expanding[bang_pos + 1..].trim_start();
        let first_char = match after_bang.chars().next() {
            Some(c) => c,
            None => return String::new(),
        };
        let (open, close) = match first_char {
            '{' => ('{', '}'),
            '(' => ('(', ')'),
            '[' => ('[', ']'),
            _ => return String::new(),
        };
        let mut depth = 0i32;
        let mut start = None;
        let mut end = None;
        for (i, ch) in after_bang.char_indices() {
            if ch == open {
                depth += 1;
                if start.is_none() {
                    start = Some(i + ch.len_utf8());
                }
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
        }
        match (start, end) {
            (Some(s), Some(e)) => after_bang[s..e].trim().to_string(),
            _ => String::new(),
        }
    }

    fn parse_trace_group(&mut self) -> Option<TraceGroup> {
        let mut expansions = Vec::new();

        while let Some(l) = self.peek_line() {
            let line = l.to_string();

            if line.starts_with("note: trace_macro") {
                // Next trace group starts
                if !expansions.is_empty() {
                    break;
                }
                // Consume the "note: trace_macro" line
                self.read_line();
                continue;
            }

            if line.contains("= note: expanding `") {
                self.read_line(); // consume the line
                let expanding = self.extract_backtick_content(&line)?;

                // Now look for the corresponding "to" line
                loop {
                    let to_line = self.peek_line()?.to_string();

                    if to_line.contains("= note: to `") {
                        self.read_line(); // consume the line
                        let to = self.extract_backtick_content(&to_line)?;
                        let input = Self::extract_arguments(&expanding);
                        let name = Self::extract_macro_name(&expanding);
                        expansions.push(MacroExpansion {
                            // `-Z trace-macros` does not report a defining crate.
                            krate: String::new(),
                            expanding,
                            arguments: String::new(),
                            to,
                            name,
                            kind: MacroExpansionKind::Bang,
                            input,
                        });
                        break;
                    } else if to_line.starts_with("note: trace_macro")
                        || to_line.contains("= note: expanding `")
                    {
                        // Unexpected: got another expanding before to
                        break;
                    } else {
                        // Skip other lines (location info, source code, etc.)
                        self.read_line();
                    }
                }
            } else if line.trim().is_empty()
                || line.starts_with("   -->")
                || line.starts_with("    |")
                || line.starts_with("   |")
                || line.starts_with("  -->")
                || line.starts_with("...")
                || line.contains("= note: this note originates")
            {
                // Skip location/formatting lines
                self.read_line();
            } else if !expansions.is_empty() {
                // Non-trace content after we have some expansions means end of group
                break;
            } else {
                // Skip unrelated lines before finding any expansions
                self.read_line();
            }
        }

        if expansions.is_empty() {
            None
        } else {
            Some(TraceGroup { expansions })
        }
    }
}

impl<R: Read> Iterator for TraceParser<R> {
    type Item = TraceGroup;

    fn next(&mut self) -> Option<Self::Item> {
        // Skip lines until we find "note: trace_macro"
        loop {
            match self.peek_line() {
                Some(line) if line.starts_with("note: trace_macro") => {
                    return self.parse_trace_group();
                }
                Some(_) => {
                    self.read_line();
                }
                None => return None,
            }
        }
    }
}

/// Parse macro tracing output from a reader.
///
/// Takes any `std::io::Read` and returns an iterator over `TraceGroup`s.
/// Each `TraceGroup` contains one or more `MacroExpansion` pairs from
/// a single `note: trace_macro` block.
pub fn parse_trace<R: Read>(reader: R) -> TraceParser<R> {
    TraceParser::new(reader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_expansion() {
        let input = r#"note: trace_macro
  --> src/lib.rs:15:17
   |
15 |             if !matches!(segment.arguments, PathArguments::None) {
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: expanding `matches! { segment.arguments, PathArguments::None }`
   = note: to `match segment.arguments { PathArguments::None => true, _ => false }`
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 1);
        assert_eq!(
            groups[0].expansions[0].expanding,
            "matches! { segment.arguments, PathArguments::None }"
        );
        assert_eq!(
            groups[0].expansions[0].to,
            "match segment.arguments { PathArguments::None => true, _ => false }"
        );
    }

    #[test]
    fn test_multiple_expansions_in_group() {
        let input = r#"note: trace_macro
  --> macro/lib.rs:16:17
   |
16 |                 abort!(segment, "Path arguments are not allowed");
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: expanding `abort! { segment, "message" }`
   = note: to `diagnostic!(segment, Error, "message").abort()`
   = note: expanding `diagnostic! { segment, Error, "message" }`
   = note: to `Diagnostic::new(segment, Error, "message")`
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 2);
        assert_eq!(
            groups[0].expansions[0].expanding,
            "abort! { segment, \"message\" }"
        );
        assert_eq!(
            groups[0].expansions[0].to,
            "diagnostic!(segment, Error, \"message\").abort()"
        );
        assert_eq!(
            groups[0].expansions[1].expanding,
            "diagnostic! { segment, Error, \"message\" }"
        );
        assert_eq!(
            groups[0].expansions[1].to,
            "Diagnostic::new(segment, Error, \"message\")"
        );
    }

    #[test]
    fn test_multiline_to_content() {
        let input = r#"note: trace_macro
  --> src/lib.rs:1:1
   |
   = note: expanding `vec! { 1, 2, 3 }`
   = note: to `{
       let mut v = Vec::new();
       v.push(1);
       v.push(2);
       v.push(3);
       v
   }`
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 1);
        assert_eq!(groups[0].expansions[0].expanding, "vec! { 1, 2, 3 }");
        assert_eq!(
            groups[0].expansions[0].to,
            r#"{
       let mut v = Vec::new();
       v.push(1);
       v.push(2);
       v.push(3);
       v
   }"#
        );
    }

    #[test]
    fn test_multiple_trace_groups() {
        let input = r#"note: trace_macro
  --> src/lib.rs:1:1
   |
   = note: expanding `println! { "hello" }`
   = note: to `print!("hello\n")`

note: trace_macro
  --> src/lib.rs:2:1
   |
   = note: expanding `dbg! { x }`
   = note: to `{ eprintln!("{}", x); x }`
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].expansions[0].expanding, "println! { \"hello\" }");
        assert_eq!(groups[1].expansions[0].expanding, "dbg! { x }");
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert!(groups.is_empty());
    }

    #[test]
    fn test_no_trace_macro() {
        let input = r#"   Compiling myproject v0.1.0
    Finished dev profile
"#;
        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert!(groups.is_empty());
    }

    #[test]
    fn test_real_rustc_output() {
        // Real output from `RUSTFLAGS="-Z trace-macros" cargo +nightly check`
        let input = r#"note: trace_macro
  --> macro/lib.rs:15:17
   |
15 |             if !matches!(segment.arguments, PathArguments::None) {
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: expanding `matches! { segment.arguments, PathArguments::None }`
   = note: to `#[allow(non_exhaustive_omitted_patterns)] match segment.arguments
           { PathArguments::None => true, _ => false }`

note: trace_macro
  --> macro/lib.rs:16:17
   |
16 |                 abort!(segment, "Path arguments are not allowed");
   |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: expanding `abort! { segment, "Path arguments are not allowed" }`
   = note: to `$crate :: diagnostic!
           (segment, $crate :: Level :: Error, "Path arguments are not allowed").abort()`
   = note: expanding `diagnostic! { segment, $crate :: Level :: Error, "Path arguments are not allowed" }`
   = note: to `{
               #[allow(unused_imports)] use $crate :: __export ::
               {
                   ToTokensAsSpanRange, Span2AsSpanRange, SpanAsSpanRange,
                   SpanRangeAsSpanRange
               }; use $crate :: DiagnosticExt; let span_range =
               (&
               segment).FIRST_ARG_MUST_EITHER_BE_Span_OR_IMPLEMENT_ToTokens_OR_BE_SpanRange();
               $crate :: Diagnostic ::
               spanned_range(span_range, $crate :: Level :: Error,
               "Path arguments are not allowed".to_string())
           }`

note: trace_macro
  --> macro/lib.rs:43:41
   |
43 |     if input.peek(Ident) && input.peek2(Token![=]) {
   |                                         ^^^^^^^^^
   |
   = note: expanding `Token! { = }`
   = note: to `$crate :: token :: Eq`
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.37s
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 3);

        // First group: single expansion
        assert_eq!(groups[0].expansions.len(), 1);
        assert_eq!(
            groups[0].expansions[0].expanding,
            "matches! { segment.arguments, PathArguments::None }"
        );
        assert_eq!(
            groups[0].expansions[0].to,
            r#"#[allow(non_exhaustive_omitted_patterns)] match segment.arguments
           { PathArguments::None => true, _ => false }"#
        );

        // Second group: multiple expansions (abort! -> diagnostic!)
        assert_eq!(groups[1].expansions.len(), 2);
        assert_eq!(
            groups[1].expansions[0].expanding,
            "abort! { segment, \"Path arguments are not allowed\" }"
        );
        assert_eq!(
            groups[1].expansions[0].to,
            r#"$crate :: diagnostic!
           (segment, $crate :: Level :: Error, "Path arguments are not allowed").abort()"#
        );
        assert_eq!(
            groups[1].expansions[1].expanding,
            "diagnostic! { segment, $crate :: Level :: Error, \"Path arguments are not allowed\" }"
        );
        assert_eq!(
            groups[1].expansions[1].to,
            r#"{
               #[allow(unused_imports)] use $crate :: __export ::
               {
                   ToTokensAsSpanRange, Span2AsSpanRange, SpanAsSpanRange,
                   SpanRangeAsSpanRange
               }; use $crate :: DiagnosticExt; let span_range =
               (&
               segment).FIRST_ARG_MUST_EITHER_BE_Span_OR_IMPLEMENT_ToTokens_OR_BE_SpanRange();
               $crate :: Diagnostic ::
               spanned_range(span_range, $crate :: Level :: Error,
               "Path arguments are not allowed".to_string())
           }"#
        );

        // Third group: Token! macro
        assert_eq!(groups[2].expansions.len(), 1);
        assert_eq!(groups[2].expansions[0].expanding, "Token! { = }");
        assert_eq!(groups[2].expansions[0].to, "$crate :: token :: Eq");
    }

    /// Real output of `RUSTFLAGS="-Z trace-macros" cargo +1.97.0 check` for a
    /// `macro_rules!` that emits a documented item.  The first line of the `to`
    /// note contains intra-doc links, so it has backticks of its own; the closing
    /// backtick is on the fourth line.  Before the fix `to` was cut at the last
    /// backtick of the first line and the nested `inner!` expansion was dropped.
    #[test]
    fn test_first_line_backtick_does_not_end_multiline_to() {
        let input = r#"note: trace_macro
  --> src/lib.rs:39:1
   |
39 | documented!();
   | ^^^^^^^^^^^^^
   |
   = note: expanding `documented! {  }`
   = note: to `/// Wraps a [`Vec`] of items; see also [`String`] and [`Option`] for the general idea.
           pub struct Foo
           { pub items : Vec < u32 > , pub name : String, pub extra : Option < u64 > , }
           impl Foo { pub const N : u32 = inner! (); }`
   = note: expanding `inner! {  }`
   = note: to `42u32`

note: trace_macro
  --> src/lib.rs:40:1
   |
40 | trailing_tick!();
   | ^^^^^^^^^^^^^^^^
   |
   = note: expanding `trailing_tick! {  }`
   = note: to `pub struct Bar
           {
               pub a : u32, /// See [`Bar`]
               pub b : u32,
           }`

note: trace_macro
  --> src/lib.rs:41:1
   |
41 | one_line_tick!();
   | ^^^^^^^^^^^^^^^^
   |
   = note: expanding `one_line_tick! {  }`
   = note: to `/// a [`b`] c
           pub const C : u32 = 1;`

    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 3);

        // The nested `inner!` expansion must survive.
        assert_eq!(groups[0].expansions.len(), 2);
        assert_eq!(groups[0].expansions[0].expanding, "documented! {  }");
        assert_eq!(
            groups[0].expansions[0].to,
            r#"/// Wraps a [`Vec`] of items; see also [`String`] and [`Option`] for the general idea.
           pub struct Foo
           { pub items : Vec < u32 > , pub name : String, pub extra : Option < u64 > , }
           impl Foo { pub const N : u32 = inner! (); }"#
        );
        assert_eq!(groups[0].expansions[1].expanding, "inner! {  }");
        assert_eq!(groups[0].expansions[1].to, "42u32");

        // Backticks on an inner line only.
        assert_eq!(groups[1].expansions.len(), 1);
        assert_eq!(
            groups[1].expansions[0].to,
            r#"pub struct Bar
           {
               pub a : u32, /// See [`Bar`]
               pub b : u32,
           }"#
        );

        // Backticks on the first line, which does not itself end in one.
        assert_eq!(groups[2].expansions.len(), 1);
        assert_eq!(
            groups[2].expansions[0].to,
            r#"/// a [`b`] c
           pub const C : u32 = 1;"#
        );
    }

    /// Real output (cargo +1.97.0, identical on 1.86.0 and nightly) with a
    /// four-digit line-number gutter, so continuation lines carry 13 spaces of
    /// padding rather than 11.  Three shapes rustc actually produces:
    /// a `to` whose last content line ends in `";` right before the closing
    /// backtick; a `to` whose first line ends in `]` after an intra-doc link and
    /// whose closing backtick stands alone on the last line (the pretty-printer
    /// emits a newline after a trailing doc comment); and a nested expansion.
    #[test]
    fn test_wide_gutter_and_closing_backtick_alone_on_last_line() {
        let input = r#"note: trace_macro
    --> src/lib.rs:1025:1
     |
1025 | documented!();
     | ^^^^^^^^^^^^^
     |
     = note: expanding `documented! {  }`
     = note: to `/// Wraps a [`Vec`] of items; see also [`String`] and [`Option`] for the general idea.
             pub struct Foo
             { pub items : Vec < u32 > , pub name : String, pub extra : Option < u64 > }
             impl Foo { pub const N : u32 = inner! (); }`
     = note: expanding `inner! {  }`
     = note: to `42u32`

note: trace_macro
    --> src/lib.rs:1026:1
     |
1026 | raw_str!();
     | ^^^^^^^^^^
     |
     = note: expanding `raw_str! {  }`
     = note: to `pub const R : & str = r"first line
             second line ends in `tick`";`

note: trace_macro
    --> src/lib.rs:1027:1
     |
1027 | tail_doc!();
     | ^^^^^^^^^^^
     |
     = note: expanding `tail_doc! {  }`
     = note: to `pub struct Baz; /// dangling [`Baz`]
             `

error: could not compile `probe2` (lib) due to 3 previous errors
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 3);

        assert_eq!(groups[0].expansions.len(), 2);
        assert_eq!(
            groups[0].expansions[0].to,
            r#"/// Wraps a [`Vec`] of items; see also [`String`] and [`Option`] for the general idea.
             pub struct Foo
             { pub items : Vec < u32 > , pub name : String, pub extra : Option < u64 > }
             impl Foo { pub const N : u32 = inner! (); }"#
        );
        assert_eq!(groups[0].expansions[1].to, "42u32");

        assert_eq!(groups[1].expansions.len(), 1);
        assert_eq!(
            groups[1].expansions[0].to,
            r#"pub const R : & str = r"first line
             second line ends in `tick`";"#
        );

        assert_eq!(groups[2].expansions.len(), 1);
        assert_eq!(
            groups[2].expansions[0].to,
            "pub struct Baz; /// dangling [`Baz`]\n             "
        );
    }

    /// Real output (cargo +1.97.0) with a one-digit gutter: continuation lines
    /// carry only 10 spaces, and here it is the `expanding` note, not `to`,
    /// that spans two lines.
    #[test]
    fn test_one_digit_gutter_multiline_expanding() {
        let input = r#"note: trace_macro
 --> src/lib.rs:4:1
  |
4 | long_in!(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb [`tick`] cccc);
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: expanding `long_in! { aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
          bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb [tick] cccc }`
  = note: to `pub struct LongIn;`

"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 1);
        assert_eq!(
            groups[0].expansions[0].expanding,
            r#"long_in! { aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
          bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb [tick] cccc }"#
        );
        assert_eq!(groups[0].expansions[0].name, "long_in");
        assert_eq!(groups[0].expansions[0].to, "pub struct LongIn;");
    }

    /// Real output (cargo +1.97.0) for a raw string literal containing an empty
    /// line.  rustc pads the empty line to the note's text column instead of
    /// emitting a genuinely blank line, and the line before it ends in a
    /// backtick.  A "blank line ends the note" rule would cut the note there.
    #[test]
    fn test_padded_blank_line_inside_expansion() {
        let input = r#"note: trace_macro
 --> src/lib.rs:6:1
  |
6 | blank_inside!();
  | ^^^^^^^^^^^^^^^
  |
  = note: expanding `blank_inside! {  }`
  = note: to `pub const R : & str = r"code span `x`
          
          after blank";`

    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
"#;

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 1);
        assert_eq!(
            groups[0].expansions[0].to,
            "pub const R : & str = r\"code span `x`\n          \n          after blank\";"
        );
    }

    /// The same real notes when they are the very last thing in the input and
    /// there is no trailing newline: the single-line and the multi-line shape
    /// must both terminate at EOF, and a note cut off mid-way must be dropped
    /// rather than returned truncated.
    #[test]
    fn test_note_at_eof() {
        let single = "note: trace_macro\n  --> src/lib.rs:39:1\n   |\n   = note: expanding `inner! {  }`\n   = note: to `42u32`";
        let groups: Vec<_> = parse_trace(single.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 1);
        assert_eq!(groups[0].expansions[0].to, "42u32");

        let multi = "note: trace_macro\n  --> src/lib.rs:41:1\n   |\n   = note: expanding `one_line_tick! {  }`\n   = note: to `/// a [`b`] c\n           pub const C : u32 = 1;`";
        let groups: Vec<_> = parse_trace(multi.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 1);
        assert_eq!(
            groups[0].expansions[0].to,
            "/// a [`b`] c\n           pub const C : u32 = 1;"
        );

        let truncated = "note: trace_macro\n  --> src/lib.rs:41:1\n   |\n   = note: expanding `one_line_tick! {  }`\n   = note: to `/// a [`b`] c\n           pub const C : u32 = 1;";
        let groups: Vec<_> = parse_trace(truncated.as_bytes()).collect();
        assert!(groups.is_empty());
    }
    /// Real output of `cargo +1.97.0 check --color=always`, byte for byte.  The
    /// `= note:` prefix is split across CSI colour sequences that must be stripped
    /// before the prefix or the padding column can be recognised, the doc comment
    /// is multibyte text, and cargo's `Finished` line carries an OSC 8 hyperlink
    /// (`ESC ] 8 ; ;` ... `ESC \`), which is not a CSI sequence and is left in the
    /// line; it starts with four spaces so it can never be mistaken for a note
    /// continuation.
    #[test]
    fn test_colored_output_with_multibyte_content() {
        let input = "\u{1b}[1m\u{1b}[92mnote\u{1b}[0m\u{1b}[1m: trace_macro\u{1b}[0m\n\
 \u{1b}[1m\u{1b}[94m--> \u{1b}[0msrc/lib.rs:9:1\n\
  \u{1b}[1m\u{1b}[94m|\u{1b}[0m\n\
\u{1b}[1m\u{1b}[94m9\u{1b}[0m \u{1b}[1m\u{1b}[94m|\u{1b}[0m documented!();\n\
  \u{1b}[1m\u{1b}[94m|\u{1b}[0m \u{1b}[1m\u{1b}[92m^^^^^^^^^^^^^\u{1b}[0m\n\
  \u{1b}[1m\u{1b}[94m|\u{1b}[0m\n\
  \u{1b}[1m\u{1b}[94m= \u{1b}[0m\u{1b}[1mnote\u{1b}[0m: expanding `documented! {  }`\n\
  \u{1b}[1m\u{1b}[94m= \u{1b}[0m\u{1b}[1mnote\u{1b}[0m: to `/// 日本語の説明: [`Vec`] を包む。詳細は [`String`] と [`Option`] を参照。\n\
          pub struct Foo { pub items : Vec < u32 > } impl Foo\n\
          { pub const N : u32 = inner! (); }`\n\
  \u{1b}[1m\u{1b}[94m= \u{1b}[0m\u{1b}[1mnote\u{1b}[0m: expanding `inner! {  }`\n\
  \u{1b}[1m\u{1b}[94m= \u{1b}[0m\u{1b}[1mnote\u{1b}[0m: to `42u32`\n\
\n\
\u{1b}[1m\u{1b}[92m    Finished\u{1b}[0m \u{1b}]8;;https://doc.rust-lang.org/cargo/reference/profiles.html#default-profiles\u{1b}\\`dev` profile [unoptimized + debuginfo]\u{1b}]8;;\u{1b}\\ target(s) in 0.07s\n\
";

        let groups: Vec<_> = parse_trace(input.as_bytes()).collect();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].expansions.len(), 2);
        assert_eq!(groups[0].expansions[0].expanding, "documented! {  }");
        assert_eq!(
            groups[0].expansions[0].to,
            "/// 日本語の説明: [`Vec`] を包む。詳細は [`String`] と [`Option`] を参照。\n\
          pub struct Foo { pub items : Vec < u32 > } impl Foo\n\
          { pub const N : u32 = inner! (); }"
        );
        assert_eq!(groups[0].expansions[1].expanding, "inner! {  }");
        assert_eq!(groups[0].expansions[1].to, "42u32");
    }
}
