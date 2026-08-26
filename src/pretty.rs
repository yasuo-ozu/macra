//! Pretty-printing and ANSI syntax highlighting for expanded macro source.
//!
//! Macro expansions arrive as `TokenStream`-style text (everything on a few
//! very long lines, with spaces around every punctuation token).  This module
//! runs that text through `syn` + `prettyplease` to obtain idiomatic Rust
//! formatting, and then colorizes it with a small hand-written Rust lexer so
//! the result is readable on a terminal.

use std::io::IsTerminal;

/// Placeholder substituted for `$crate` so that `syn` can parse expansions
/// that still contain macro-transcriber tokens.
const DOLLAR_CRATE_PLACEHOLDER: &str = "__macra_dollar_crate__";

/// Name of the dummy function used to wrap expression/statement fragments.
const WRAPPER_FN: &str = "__macra_fmt_wrapper";

/// When to emit ANSI color escapes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum ColorChoice {
    /// Colorize only when stdout is a terminal and `NO_COLOR` is unset.
    #[default]
    Auto,
    /// Always colorize.
    Always,
    /// Never colorize.
    Never,
}

impl ColorChoice {
    /// Resolve the choice against the environment (TTY, `NO_COLOR`, `TERM`).
    pub fn resolve(self) -> bool {
        match self {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => {
                std::io::stdout().is_terminal()
                    && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
                    && std::env::var_os("TERM").is_none_or(|t| t != "dumb")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Pretty-print Rust source produced by a macro expansion.
///
/// Tries to parse the text as a sequence of items first, then as a function
/// body (which covers expression and statement fragments).  If neither parses
/// — expansions are not always syntactically complete — the input is returned
/// unchanged.
pub fn format_source(src: &str) -> String {
    let escaped = src.replace("$crate", DOLLAR_CRATE_PLACEHOLDER);
    match format_items(&escaped).or_else(|| format_fragment(&escaped)) {
        Some(formatted) => formatted.replace(DOLLAR_CRATE_PLACEHOLDER, "$crate"),
        None => src.to_string(),
    }
}

/// Format `src` interpreted as a sequence of top-level items.
fn format_items(src: &str) -> Option<String> {
    let file = syn::parse_file(src).ok()?;
    // An empty parse means the text was not items at all (e.g. an expression
    // fragment); let the caller fall through to the fragment path.
    if file.items.is_empty() && !src.trim().is_empty() {
        return None;
    }
    Some(prettyplease::unparse(&file))
}

/// Format `src` interpreted as an expression or statement sequence by wrapping
/// it in a dummy function, then stripping the wrapper back off.
fn format_fragment(src: &str) -> Option<String> {
    let wrapped = format!("fn {}() {{\n{}\n}}\n", WRAPPER_FN, src);
    let file = syn::parse_file(&wrapped).ok()?;
    let printed = prettyplease::unparse(&file);

    let body = printed
        .strip_prefix(&format!("fn {}() {{\n", WRAPPER_FN))
        .and_then(|rest| rest.trim_end().strip_suffix('}'))?;
    Some(dedent(body))
}

/// Remove one level (four spaces) of indentation from every line.
fn dedent(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for line in s.lines() {
        out.push_str(line.strip_prefix("    ").unwrap_or(line));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Highlighting
// ---------------------------------------------------------------------------

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

/// Lexical categories recognized by the highlighter.
///
/// Produced by [`tokenize`] and consumed both by [`highlight`] (which maps them
/// to ANSI escapes for stdout) and by the TUI (which maps them to ratatui
/// styles), so the two renderers always agree on what a token is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Plain,
    Comment,
    Attribute,
    Keyword,
    MacroName,
    Type,
    Literal,
    Number,
    Lifetime,
}

impl TokenKind {
    /// ANSI SGR sequence for this category, or `None` to use the default color.
    ///
    /// The TUI mirrors this palette with ratatui colors.
    fn ansi(self) -> Option<&'static str> {
        match self {
            TokenKind::Plain => None,
            TokenKind::Comment => Some("\x1b[90m"),
            TokenKind::Attribute => Some("\x1b[33m"),
            TokenKind::Keyword => Some("\x1b[34m"),
            TokenKind::MacroName => Some("\x1b[36m"),
            TokenKind::Type => Some("\x1b[35m"),
            TokenKind::Literal => Some("\x1b[32m"),
            TokenKind::Number => Some("\x1b[93m"),
            TokenKind::Lifetime => Some("\x1b[35m"),
        }
    }
}

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "union", "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final",
    "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Split `src` into consecutive, non-overlapping syntax tokens.
///
/// The returned ranges are **byte** offsets into `src` (so they can be used to
/// slice it directly), always start at 0, and always tile the whole input, so
/// concatenating `&src[range]` for every token reproduces `src` exactly.
///
/// This is the single lexer shared by the stdout highlighter ([`highlight`])
/// and the TUI source pane.
pub fn tokenize(src: &str) -> Vec<(TokenKind, std::ops::Range<usize>)> {
    let chars: Vec<char> = src.chars().collect();
    // Byte offset of every character, plus a sentinel for one-past-the-end, so
    // char indices from the scanner can be turned back into byte offsets.
    let mut byte_of: Vec<usize> = Vec::with_capacity(chars.len() + 1);
    byte_of.extend(src.char_indices().map(|(b, _)| b));
    byte_of.push(src.len());

    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let kind = scan_token(&chars, &mut i);
        debug_assert!(i > start, "lexer must always make progress");
        // A scanner may run past the end when consuming an escape at EOF.
        let end = i.min(chars.len());
        i = end;
        tokens.push((kind, byte_of[start]..byte_of[end]));
    }
    tokens
}

/// Colorize Rust source with ANSI escapes.
pub fn highlight(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    for (kind, range) in tokenize(src) {
        push_styled(&mut out, &src[range], kind);
    }
    out
}

/// Append `text` to `out`, wrapped in the escape sequence for `kind`.
fn push_styled(out: &mut String, text: &str, kind: TokenKind) {
    match kind.ansi() {
        Some(code) => {
            out.push_str(code);
            out.push_str(text);
            out.push_str(RESET);
        }
        None => out.push_str(text),
    }
}

/// Consume one token starting at `*i` and return its category.
fn scan_token(chars: &[char], i: &mut usize) -> TokenKind {
    let c = chars[*i];
    let next = chars.get(*i + 1).copied();

    // Comments
    if c == '/' && next == Some('/') {
        while *i < chars.len() && chars[*i] != '\n' {
            *i += 1;
        }
        return TokenKind::Comment;
    }
    if c == '/' && next == Some('*') {
        *i += 2;
        let mut depth = 1usize;
        while *i < chars.len() && depth > 0 {
            if chars[*i] == '/' && chars.get(*i + 1) == Some(&'*') {
                depth += 1;
                *i += 2;
            } else if chars[*i] == '*' && chars.get(*i + 1) == Some(&'/') {
                depth -= 1;
                *i += 2;
            } else {
                *i += 1;
            }
        }
        return TokenKind::Comment;
    }

    // Attributes: `#[...]` and `#![...]`, consumed as one span.
    if c == '#' && (next == Some('[') || (next == Some('!') && chars.get(*i + 2) == Some(&'['))) {
        let bracket = if next == Some('[') { *i + 1 } else { *i + 2 };
        *i = bracket;
        let mut depth = 0usize;
        while *i < chars.len() {
            match chars[*i] {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        *i += 1;
                        break;
                    }
                }
                '"' => {
                    scan_string(chars, i);
                    continue;
                }
                _ => {}
            }
            *i += 1;
        }
        return TokenKind::Attribute;
    }

    // Raw / byte string and byte char literals.
    if c == 'r' && matches!(next, Some('"') | Some('#')) && scan_raw_string(chars, i) {
        return TokenKind::Literal;
    }
    if c == 'b'
        && next == Some('r')
        && matches!(chars.get(*i + 2), Some('"') | Some('#'))
        && scan_raw_string(chars, i)
    {
        return TokenKind::Literal;
    }
    if c == 'b' && matches!(next, Some('"') | Some('\'')) {
        *i += 1;
        if chars[*i] == '"' {
            scan_string(chars, i);
        } else {
            scan_char(chars, i);
        }
        return TokenKind::Literal;
    }

    // String literals
    if c == '"' {
        scan_string(chars, i);
        return TokenKind::Literal;
    }

    // Char literals and lifetimes
    if c == '\'' {
        // `'a'` is a char, `'a` is a lifetime.
        let is_char = next == Some('\\')
            || (next.is_some() && chars.get(*i + 2) == Some(&'\''))
            || !next.is_some_and(|n| n.is_alphabetic() || n == '_');
        if is_char {
            scan_char(chars, i);
        } else {
            *i += 1;
            while *i < chars.len() && (chars[*i].is_alphanumeric() || chars[*i] == '_') {
                *i += 1;
            }
            return TokenKind::Lifetime;
        }
        return TokenKind::Literal;
    }

    // Numbers
    if c.is_ascii_digit() {
        while *i < chars.len() {
            let d = chars[*i];
            // A `.` continues the literal only when followed by a digit, so
            // that ranges like `1..2` are not swallowed.
            let part = d.is_alphanumeric()
                || d == '_'
                || (d == '.' && chars.get(*i + 1).is_some_and(|n| n.is_ascii_digit()));
            if !part {
                break;
            }
            *i += 1;
        }
        return TokenKind::Number;
    }

    // Identifiers
    if c.is_alphabetic() || c == '_' {
        let start = *i;
        while *i < chars.len() && (chars[*i].is_alphanumeric() || chars[*i] == '_') {
            *i += 1;
        }
        let word: String = chars[start..*i].iter().collect();

        // `name!` (possibly with a space before `!`, as token streams emit)
        // is a macro invocation; `name != x` is not.
        let mut j = *i;
        while chars.get(j) == Some(&' ') {
            j += 1;
        }
        if chars.get(j) == Some(&'!') && chars.get(j + 1) != Some(&'=') {
            *i = j + 1;
            return TokenKind::MacroName;
        }

        if KEYWORDS.contains(&word.as_str()) {
            return TokenKind::Keyword;
        }
        if word.starts_with(char::is_uppercase) {
            return TokenKind::Type;
        }
        return TokenKind::Plain;
    }

    // Anything else: punctuation, whitespace.
    *i += 1;
    TokenKind::Plain
}

/// Consume a `"…"` string literal starting at `*i`.
fn scan_string(chars: &[char], i: &mut usize) {
    *i += 1;
    while *i < chars.len() {
        match chars[*i] {
            '\\' => *i += 2,
            '"' => {
                *i += 1;
                return;
            }
            _ => *i += 1,
        }
    }
}

/// Consume a `'…'` char literal starting at `*i`.
fn scan_char(chars: &[char], i: &mut usize) {
    *i += 1;
    while *i < chars.len() {
        match chars[*i] {
            '\\' => *i += 2,
            '\'' | '\n' => {
                *i += 1;
                return;
            }
            _ => *i += 1,
        }
    }
}

/// Try to consume a raw string (`r"…"`, `r#"…"#`, `br#"…"#`) starting at `*i`.
/// Returns `false` (leaving `*i` untouched) if the text is not a raw string.
fn scan_raw_string(chars: &[char], i: &mut usize) -> bool {
    let mut j = *i;
    if chars.get(j) == Some(&'b') {
        j += 1;
    }
    if chars.get(j) != Some(&'r') {
        return false;
    }
    j += 1;
    let hashes_start = j;
    while chars.get(j) == Some(&'#') {
        j += 1;
    }
    let hashes = j - hashes_start;
    if chars.get(j) != Some(&'"') {
        return false;
    }
    j += 1;
    while j < chars.len() {
        if chars[j] == '"' {
            let closing = j + 1;
            if chars[closing..]
                .iter()
                .take(hashes)
                .filter(|c| **c == '#')
                .count()
                == hashes
            {
                *i = closing + hashes;
                return true;
            }
        }
        j += 1;
    }
    // Unterminated: consume the rest so the lexer still makes progress.
    *i = chars.len();
    true
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

/// Format `src` and, when `color` is set, colorize it.
///
/// The returned string always ends with exactly one newline.
pub fn render(src: &str, color: bool) -> String {
    let formatted = format_source(src);
    let trimmed = formatted.trim_end();
    let mut out = if color {
        highlight(trimmed)
    } else {
        trimmed.to_string()
    };
    out.push('\n');
    out
}

/// Style a section header (e.g. `== foo! ==`).
pub fn header(text: &str, color: bool) -> String {
    if color {
        format!("{}{}{}", BOLD, text, RESET)
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_items() {
        let out = format_source("pub struct Foo { pub value : i32, }");
        assert_eq!(out, "pub struct Foo {\n    pub value: i32,\n}\n");
    }

    #[test]
    fn formats_expression_fragment() {
        let out = format_source("(get_answer(), get_answer())");
        assert_eq!(out.trim_end(), "(get_answer(), get_answer())");
    }

    #[test]
    fn formats_statement_fragment() {
        let out = format_source(r#"println! (concat! ("a", stringify! (B)));"#);
        assert_eq!(out.trim_end(), r#"println!(concat!("a", stringify!(B)));"#);
    }

    #[test]
    fn preserves_dollar_crate() {
        let out = format_source("impl $crate::Trait for Foo { }");
        assert!(out.contains("$crate::Trait"), "got: {}", out);
    }

    #[test]
    fn unparsable_input_is_returned_verbatim() {
        let src = "this is ! not @ rust >>";
        assert_eq!(format_source(src), src);
    }

    /// Concatenating every token's slice must reproduce the input exactly, and
    /// the ranges must tile it without gaps or overlaps.
    fn assert_tokens_tile(src: &str) -> Vec<(TokenKind, std::ops::Range<usize>)> {
        let tokens = tokenize(src);
        let mut pos = 0;
        let mut rebuilt = String::new();
        for (_, range) in &tokens {
            assert_eq!(range.start, pos, "gap/overlap in {:?}: {:?}", src, tokens);
            assert!(range.end > range.start, "empty token in {:?}", src);
            rebuilt.push_str(&src[range.clone()]);
            pos = range.end;
        }
        assert_eq!(pos, src.len(), "tokens do not cover {:?}", src);
        assert_eq!(rebuilt, src);
        tokens
    }

    #[test]
    fn tokenize_tiles_the_input() {
        for src in [
            "",
            "fn main() { let s = \"hi\"; /* c */ let l: &'static str = s; }",
            "#[derive(Greet, Describe)]\npub struct Foo;",
            "let a = r#\"x\"y\"#; let b: &'a T; let c = b'\\n'; let d = 1_000.5f64;",
            "// trailing comment",
            "let x = '\\",
            "let s = \"unterminated",
        ] {
            assert_tokens_tile(src);
        }
    }

    #[test]
    fn tokenize_handles_multibyte_utf8() {
        let src = "let 日本語 = \"漢字\"; // コメント";
        let tokens = assert_tokens_tile(src);
        // Ranges must land on character boundaries (slicing above would have
        // panicked otherwise) and the categories must still be right.
        assert!(
            tokens
                .iter()
                .any(|(k, r)| *k == TokenKind::Literal && &src[r.clone()] == "\"漢字\"")
        );
        assert!(
            tokens
                .iter()
                .any(|(k, r)| *k == TokenKind::Comment && src[r.clone()].starts_with("//"))
        );
        assert_eq!(tokens[0].0, TokenKind::Keyword);
    }

    #[test]
    fn tokenize_classifies_kinds() {
        let src = "#[inline]\npub fn f<'a>(x: Foo) -> u32 { foo!(1); }";
        let kinds: Vec<(TokenKind, &str)> = tokenize(src)
            .into_iter()
            .map(|(k, r)| (k, &src[r]))
            .filter(|(k, _)| *k != TokenKind::Plain)
            .collect();
        assert!(kinds.contains(&(TokenKind::Attribute, "#[inline]")));
        assert!(kinds.contains(&(TokenKind::Keyword, "pub")));
        assert!(kinds.contains(&(TokenKind::Keyword, "fn")));
        assert!(kinds.contains(&(TokenKind::Lifetime, "'a")));
        assert!(kinds.contains(&(TokenKind::Type, "Foo")));
        assert!(kinds.contains(&(TokenKind::MacroName, "foo!")));
        assert!(kinds.contains(&(TokenKind::Number, "1")));
    }

    #[test]
    fn highlight_is_lossless_without_escapes() {
        let src = "fn main() { let s = \"hi\"; /* c */ let l: &'static str = s; }";
        let colored = highlight(src);
        let stripped: String = strip_ansi(&colored);
        assert_eq!(stripped, src);
    }

    #[test]
    fn highlight_marks_expected_kinds() {
        let colored = highlight("#[derive(Greet)]\npub fn f() { foo!(1); }");
        assert!(
            colored.contains("\x1b[33m#[derive(Greet)]"),
            "{:?}",
            colored
        );
        assert!(colored.contains("\x1b[34mpub"), "{:?}", colored);
        assert!(colored.contains("\x1b[36mfoo!"), "{:?}", colored);
        assert!(colored.contains("\x1b[93m1"), "{:?}", colored);
    }

    #[test]
    fn highlight_handles_raw_strings_and_lifetimes() {
        let src = "let a = r#\"x\"y\"#; let b: &'a T;";
        let colored = highlight(src);
        assert_eq!(strip_ansi(&colored), src);
        assert!(colored.contains("\x1b[35m'a"), "{:?}", colored);
    }

    #[test]
    fn not_equal_is_not_a_macro() {
        let colored = highlight("if a != b {}");
        assert!(!colored.contains("\x1b[36m"), "{:?}", colored);
    }

    #[test]
    fn render_never_colors_when_disabled() {
        let out = render("pub struct Foo;", false);
        assert!(!out.contains('\x1b'));
        assert!(out.ends_with('\n') && !out.ends_with("\n\n"));
    }

    #[test]
    fn color_choice_resolution() {
        assert!(ColorChoice::Always.resolve());
        assert!(!ColorChoice::Never.resolve());
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }
}
