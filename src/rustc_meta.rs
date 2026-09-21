//! Recovering proc-macro names and kinds from a dylib's `.rustc` metadata.
//!
//! From rustc 1.100 the `__rustc_proc_macro_decls_*` table is a bare slice of
//! `run` function pointers: the names and kinds that older layouts carried inline
//! moved into crate metadata. The pointers still resolve to symbols naming the
//! user's function (`…::expand1<krate::the_fn>`), so function names come from the
//! symbol table — but a derive's *trait* name (`#[proc_macro_derive(Foo)]` on
//! `fn some_other_name`) exists only here, as does the bang/derive distinction.
//!
//! Metadata is a compiler-internal format with no stability guarantee, so this
//! deliberately does not decode it. It scans for length-prefixed identifiers and
//! then *verifies* what it found against the function names taken from the symbol
//! table. Anything that does not line up returns `None`, and the caller reports no
//! names rather than wrong ones.

/// Macro kind discriminants, matching `ProcMacro`'s variant order.
pub const KIND_DERIVE: u8 = 0;
pub const KIND_ATTR: u8 = 1;
pub const KIND_BANG: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcMacroEntry {
    pub kind: u8,
    /// The name the macro is invoked by: a derive's trait name, otherwise the
    /// function's own name.
    pub name: String,
}

/// The `rustc <version>` string recorded at the head of a `.rustc` section.
///
/// Layout is `"rust"`, a format-version byte, a length, then the string — stable
/// enough that other tools rely on it for compatibility checks.
pub fn version_string(meta: &[u8]) -> Option<String> {
    let at = meta.windows(4).position(|w| w == b"rust")?;
    // Skip the magic and the 8-byte length that follows the format version, then
    // look for the length-prefixed version string within the next few bytes.
    for i in at..meta.len().min(at + 64) {
        // Written without a let-chain: those are stable only from 1.88, and this
        // crate supports 1.86.
        if let Some(s) = ident_like_at(meta, i, |b| b != 0) {
            if s.starts_with("rustc ") {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// A length-prefixed run of bytes at `i`, if the length is plausible and every
/// byte satisfies `ok`.
fn ident_like_at(meta: &[u8], i: usize, ok: impl Fn(u8) -> bool) -> Option<&str> {
    let len = *meta.get(i)? as usize;
    if len == 0 {
        return None;
    }
    let bytes = meta.get(i + 1..i + 1 + len)?;
    if !bytes.iter().copied().all(ok) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

fn rust_ident_at(meta: &[u8], i: usize) -> Option<&str> {
    let s = ident_like_at(meta, i, |b| b == b'_' || b.is_ascii_alphanumeric())?;
    // Reject runs that merely look textual, e.g. a length byte landing in a digit
    // string. A Rust identifier never starts with a digit.
    let first = s.as_bytes()[0];
    (first == b'_' || first.is_ascii_alphabetic()).then_some(s)
}

/// Proc-macro `(kind, name)` pairs in declaration order.
///
/// `fn_names` must be the macro functions' own names, in table order, as taken
/// from the symbol table; they anchor the scan and validate the result.
pub fn proc_macro_entries(meta: &[u8], fn_names: &[String]) -> Option<Vec<ProcMacroEntry>> {
    if fn_names.is_empty() {
        return None;
    }

    // The function names appear first, in order. Find where that run ends so the
    // scan for kind-tagged entries starts past it.
    let mut cursor = 0usize;
    for want in fn_names {
        let mut found = None;
        for i in cursor..meta.len() {
            if rust_ident_at(meta, i) == Some(want.as_str()) {
                found = Some(i + 1 + want.len());
                break;
            }
        }
        cursor = found?;
    }

    // Past that, each macro is recorded as `<kind> <len> <name>`. Derives appear
    // again without a kind byte, so keep the first sighting of each name.
    let mut entries: Vec<ProcMacroEntry> = Vec::new();
    let mut i = cursor;
    while i + 1 < meta.len() && entries.len() < fn_names.len() {
        let kind = meta[i];
        // Not a let-chain: stable only from 1.88, and this crate supports 1.86.
        if matches!(kind, KIND_DERIVE | KIND_ATTR | KIND_BANG) {
            if let Some(name) = rust_ident_at(meta, i + 1) {
                if !entries.iter().any(|e| e.name == name) {
                    entries.push(ProcMacroEntry {
                        kind,
                        name: name.to_string(),
                    });
                    i += 2 + name.len();
                    continue;
                }
            }
        }
        i += 1;
    }

    validate(entries, fn_names)
}

/// Reject anything that does not agree with the symbol-derived function names.
///
/// A bang or attribute macro is invoked by its function's name, so those must
/// match exactly; only a derive may legitimately differ, because its trait name is
/// chosen separately. Failing here means the metadata encoding moved, and the
/// caller must report nothing rather than guess.
fn validate(entries: Vec<ProcMacroEntry>, fn_names: &[String]) -> Option<Vec<ProcMacroEntry>> {
    if entries.len() != fn_names.len() {
        return None;
    }
    for (entry, fn_name) in entries.iter().zip(fn_names) {
        match entry.kind {
            KIND_BANG | KIND_ATTR if &entry.name != fn_name => return None,
            KIND_DERIVE | KIND_BANG | KIND_ATTR => {}
            _ => return None,
        }
    }
    Some(entries)
}

/// The defining crate, the macro function's name, and whether it is an attribute
/// macro, taken from a demangled table-entry symbol.
///
/// Each entry in the 1.100 decls table points at a `selfless_reify` wrapper whose
/// symbol names the user's function, e.g.
/// `…::wrapper::<…Buffer, <…Client>::expand1<krate::the_fn>::{closure#0}>`.
/// `expand2` takes two token streams, so it is an attribute macro; `expand1` is a
/// bang or a derive and only the metadata can say which.
pub fn fn_name_from_symbol(demangled: &str) -> Option<(String, String, bool)> {
    let (at, is_attr) = match (demangled.find("::expand1<"), demangled.find("::expand2<")) {
        (Some(i), _) => (i + "::expand1<".len(), false),
        (_, Some(i)) => (i + "::expand2<".len(), true),
        _ => return None,
    };
    // Take the path up to the matching `>`, tracking nesting so a generic argument
    // inside it cannot end the span early.
    let rest = &demangled[at..];
    let mut depth = 0usize;
    let mut end = None;
    for (i, c) in rest.char_indices() {
        match c {
            '<' => depth += 1,
            '>' if depth == 0 => {
                end = Some(i);
                break;
            }
            '>' => depth -= 1,
            _ => {}
        }
    }
    let path = rest[..end?].trim();
    // A proc macro must be declared at its crate's root, so the path is always
    // `krate::function` — which makes the defining crate recoverable even though the
    // decls table and the metadata both record bare names.
    // v0 mangling carries a crate disambiguator, so a segment demangles as
    // `probe2[7119de0192789d9]`; the hash is not part of the name.
    fn strip_disambiguator(seg: &str) -> &str {
        seg.split('[').next().unwrap_or(seg).trim()
    }
    let mut segments = path.rsplit("::");
    let name = strip_disambiguator(segments.next()?);
    let krate = strip_disambiguator(segments.next().unwrap_or(""));
    let plausible = |s: &str| {
        s.as_bytes()
            .first()
            .is_some_and(|b| *b == b'_' || b.is_ascii_alphabetic())
            && s.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric())
    };
    if !plausible(name) || (!krate.is_empty() && !plausible(krate)) {
        return None;
    }
    Some((krate.to_string(), name.to_string(), is_attr))
}

/// The crate name in a proc-macro dylib's file name, e.g. `libserde_derive-9f3.so`.
///
/// Needed because only v0 mangling keeps the generic arguments that name the macro
/// function; a toolchain using the legacy scheme emits bare
/// `…::wrapper::h47303a70604cc30b` symbols with the crate erased. The file name is
/// the one place it survives on every toolchain.
pub fn crate_from_dylib_name(file_name: &str) -> Option<String> {
    let stem = file_name
        .rsplit_once('.')
        .map(|(stem, _ext)| stem)
        .unwrap_or(file_name);
    let stem = stem.strip_prefix("lib").unwrap_or(stem);
    // Cargo appends `-<metadata hash>`; the crate name itself may contain `_` but
    // never `-`, so the last such segment is the hash.
    let name = match stem.rsplit_once('-') {
        Some((name, hash)) if !hash.is_empty() && hash.bytes().all(|b| b.is_ascii_hexdigit()) => {
            name
        }
        _ => stem,
    };
    let plausible = name
        .bytes()
        .next()
        .is_some_and(|b| b == b'_' || b.is_ascii_alphabetic())
        && name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric());
    plausible.then(|| name.to_string())
}

/// Qualify a macro name with the crate that defines it, as `krate::Name`.
///
/// The existing matcher compares only the last segment, so this adds information
/// without changing what matches. It is deliberately *not* used to require a crate
/// match: a derive is routinely invoked through a re-export, as
/// `#[derive(serde::Serialize)]` for a macro that lives in `serde_derive`, so the
/// path written in source need not name the defining crate at all.
pub fn qualified(krate: &str, name: &str) -> String {
    if krate.is_empty() {
        name.to_string()
    } else {
        format!("{krate}::{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a blob shaped like the real section: a version header, the function
    /// names in order, then `<kind> <len> <name>` records.
    fn blob(fns: &[&str], macros: &[(u8, &str)]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"rust\0\0\0\x0a");
        v.extend_from_slice(&[0u8; 8]);
        let ver = "rustc 1.100.0-nightly (cea272fa3 2026-09-07)";
        v.push(ver.len() as u8);
        v.extend_from_slice(ver.as_bytes());
        for f in fns {
            v.extend_from_slice(&[0xc1, 0, 1, 0, 6, 0]);
            v.push(f.len() as u8);
            v.extend_from_slice(f.as_bytes());
        }
        for (kind, name) in macros {
            v.extend_from_slice(&[0xc1, 0, 0x0f, 0x05, 0]);
            v.push(*kind);
            v.push(name.len() as u8);
            v.extend_from_slice(name.as_bytes());
            // Derives are recorded a second time without a kind byte.
            if *kind == KIND_DERIVE {
                v.extend_from_slice(&[0xc1, 0, 1, 0, 7, 0]);
                v.push(name.len() as u8);
                v.extend_from_slice(name.as_bytes());
            }
        }
        v
    }

    #[test]
    fn reads_the_version_string() {
        let b = blob(&["m1"], &[(KIND_BANG, "m1")]);
        assert_eq!(
            version_string(&b).as_deref(),
            Some("rustc 1.100.0-nightly (cea272fa3 2026-09-07)")
        );
        assert_eq!(version_string(b"nothing here"), None);
    }

    /// The case the whole exercise exists for: a derive whose trait name differs
    /// from its function name, which the symbol table alone cannot give us.
    #[test]
    fn recovers_derive_trait_names_and_kinds() {
        let fns = ["bang_one", "derive_ser", "derive_other", "attr_one"];
        let macros = [
            (KIND_BANG, "bang_one"),
            (KIND_DERIVE, "SerializeLike"),
            (KIND_DERIVE, "OtherTrait"),
            (KIND_ATTR, "attr_one"),
        ];
        let fn_names: Vec<String> = fns.iter().map(|s| s.to_string()).collect();
        let got = proc_macro_entries(&blob(&fns, &macros), &fn_names).expect("should parse");

        assert_eq!(
            got,
            macros
                .iter()
                .map(|(k, n)| ProcMacroEntry {
                    kind: *k,
                    name: n.to_string()
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn refuses_rather_than_guessing_when_nothing_lines_up() {
        let fn_names = vec!["bang_one".to_string()];
        // Encoding moved: the function names are not there at all.
        assert_eq!(proc_macro_entries(b"\0\0\0\0garbage", &fn_names), None);
        // Fewer records than functions.
        let fns = ["a_one", "b_two"];
        let names: Vec<String> = fns.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            proc_macro_entries(&blob(&fns, &[(KIND_BANG, "a_one")]), &names),
            None
        );
        // A bang macro is invoked by its function's name; a mismatch means the
        // scan latched onto the wrong strings.
        assert_eq!(
            proc_macro_entries(
                &blob(&fns, &[(KIND_BANG, "a_one"), (KIND_BANG, "something_else")]),
                &names
            ),
            None
        );
        // No functions to anchor against.
        assert_eq!(proc_macro_entries(&blob(&[], &[]), &[]), None);
    }

    /// Validates the scan against a real `.rustc` section rather than a synthetic
    /// one. Ignored by default because it needs a proc-macro dylib's section dumped
    /// to a file; run with
    /// `MACRA_RUSTC_SECTION=<file> MACRA_RUSTC_FNS=a,b cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn parses_a_real_metadata_section() {
        let path = std::env::var("MACRA_RUSTC_SECTION").expect("MACRA_RUSTC_SECTION");
        let fns: Vec<String> = std::env::var("MACRA_RUSTC_FNS")
            .expect("MACRA_RUSTC_FNS")
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let bytes = std::fs::read(path).expect("section file");
        println!("version: {:?}", version_string(&bytes));
        let got = proc_macro_entries(&bytes, &fns);
        println!("entries: {:#?}", got);
        assert!(got.is_some(), "scan failed on a real section");
    }

    /// Real symbols taken from a proc-macro dylib built with 1.100.0-nightly.
    #[test]
    fn reads_the_function_name_out_of_a_table_symbol() {
        let bang = "proc_macro::bridge::selfless_reify::reify_to_extern_c_fn_hrt_bridge::\
                    wrapper::<proc_macro::bridge::buffer::Buffer, \
                    <proc_macro::bridge::client::Client>::expand1<probe::m1>::{closure#0}>";
        assert_eq!(
            fn_name_from_symbol(bang),
            Some(("probe".to_string(), "m1".to_string(), false))
        );

        // `expand2` takes two token streams, so it is an attribute macro.
        let attr = "proc_macro::bridge::selfless_reify::reify_to_extern_c_fn_hrt_bridge::\
                    wrapper::<proc_macro::bridge::buffer::Buffer, \
                    <proc_macro::bridge::client::Client>::expand2<probe::a1>::{closure#0}>";
        assert_eq!(
            fn_name_from_symbol(attr),
            Some(("probe".to_string(), "a1".to_string(), true))
        );

        // A derive still reports its *function* name here; the trait name comes
        // from the metadata scan.
        let derive = "…::expand1<probe2::derive_ser>::{closure#0}>";
        assert_eq!(
            fn_name_from_symbol(derive),
            Some(("probe2".to_string(), "derive_ser".to_string(), false))
        );
        // A derive reached through a re-export still reports the crate that defines
        // it, which is why the crate is informational rather than a match condition.
        assert_eq!(
            qualified("probe2", "SerializeLike"),
            "probe2::SerializeLike"
        );
        assert_eq!(qualified("", "SerializeLike"), "SerializeLike");

        // Real `rustc_demangle` output carries crate disambiguators, which are not
        // part of the crate or function name.
        let hashed = "proc_macro[602e4fb18ec88a0a]::bridge::selfless_reify::\
                      reify_to_extern_c_fn_hrt_bridge::wrapper::<…, \
                      <proc_macro[602e4fb18ec88a0a]::bridge::client::Client>::\
                      expand1<probe2[7119de0192789d9]::bang_one>::{closure#0}>";
        assert_eq!(
            fn_name_from_symbol(hashed),
            Some(("probe2".to_string(), "bang_one".to_string(), false))
        );

        // Legacy mangling drops the generic arguments entirely, so the crate has to
        // come from the file name instead.
        assert_eq!(
            fn_name_from_symbol(
                "proc_macro::bridge::selfless_reify::reify_to_extern_c_fn_hrt_bridge::\
                 wrapper::h47303a70604cc30b"
            ),
            None
        );
        assert_eq!(
            crate_from_dylib_name("libprobe2-c3da34014c3bf847.so").as_deref(),
            Some("probe2")
        );
        assert_eq!(
            crate_from_dylib_name("libserde_derive-9f3ab.so").as_deref(),
            Some("serde_derive")
        );
        // A dylib without cargo's hash suffix, and one that is not a crate at all.
        assert_eq!(
            crate_from_dylib_name("libprobe2.so").as_deref(),
            Some("probe2")
        );
        assert_eq!(crate_from_dylib_name("lib-.so"), None);

        // Unrelated symbols must not be mistaken for entries.
        assert_eq!(fn_name_from_symbol("core::ptr::drop_in_place<Foo>"), None);
        assert_eq!(fn_name_from_symbol(""), None);
        // Truncated generic span.
        assert_eq!(fn_name_from_symbol("x::expand1<probe::m1"), None);
    }

    #[test]
    fn a_derive_may_differ_but_a_bang_may_not() {
        let fns = ["derive_fn"];
        let names: Vec<String> = fns.iter().map(|s| s.to_string()).collect();
        let ok = proc_macro_entries(&blob(&fns, &[(KIND_DERIVE, "TraitName")]), &names);
        assert_eq!(ok.unwrap()[0].name, "TraitName");
    }
}
