use std::io::{BufRead, BufReader, Read};

/// A single macro expansion pair: the "expanding" text and the "to" text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroExpansion {
    pub expanding: String,
    pub to: String,
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

    fn read_line(&mut self) -> Option<String> {
        if let Some(line) = self.peeked_line.take() {
            return Some(line);
        }
        self.current_line.clear();
        match self.reader.read_line(&mut self.current_line) {
            Ok(0) => None,
            Ok(_) => Some(self.current_line.trim_end_matches('\n').to_string()),
            Err(_) => None,
        }
    }

    fn peek_line(&mut self) -> Option<&str> {
        if self.peeked_line.is_none() {
            self.peeked_line = self.read_line();
        }
        self.peeked_line.as_deref()
    }

    /// Extract content between backticks, handling multi-line content.
    /// Returns the content and consumes lines as needed.
    fn extract_backtick_content(&mut self, first_line: &str) -> Option<String> {
        // Find the opening backtick
        let start_idx = first_line.find('`')?;
        let after_backtick = &first_line[start_idx + 1..];

        // Check if content ends on the same line
        if let Some(end_idx) = after_backtick.find('`') {
            return Some(after_backtick[..end_idx].to_string());
        }

        // Multi-line content: collect until closing backtick
        let mut content = after_backtick.to_string();

        loop {
            let line = self.read_line()?;
            if let Some(end_idx) = line.find('`') {
                content.push('\n');
                content.push_str(&line[..end_idx]);
                break;
            } else {
                content.push('\n');
                content.push_str(&line);
            }
        }

        Some(content)
    }

    fn parse_trace_group(&mut self) -> Option<TraceGroup> {
        let mut expansions = Vec::new();

        loop {
            let line = match self.peek_line() {
                Some(l) => l.to_string(),
                None => break,
            };

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
                    let to_line = match self.peek_line() {
                        Some(l) => l.to_string(),
                        None => return None,
                    };

                    if to_line.contains("= note: to `") {
                        self.read_line(); // consume the line
                        let to = self.extract_backtick_content(&to_line)?;
                        expansions.push(MacroExpansion { expanding, to });
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
        assert_eq!(groups[0].expansions[0].expanding, "abort! { segment, \"message\" }");
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
}
