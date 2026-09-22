//! Recovering proc-macro names and kinds from a dylib's `.rustc` metadata.
//!
//! From rustc 1.98 the `__rustc_proc_macro_decls_*` table is a bare slice of
//! `run` function pointers: the names and kinds that older layouts carried inline
//! moved into crate metadata. The pointers still resolve to symbols naming the
//! user's function (`…::expand1<krate::the_fn>`), so function names come from the
//! symbol table — but a derive's *trait* name (`#[proc_macro_derive(Foo)]` on
//! `fn some_other_name`) exists only here, as does the bang/derive distinction.
//!
//! Metadata is a compiler-internal format with no stability guarantee. This module
//! therefore decodes exactly the layout rustc 1.98, 1.99 and 1.100 write, checked
//! against `rustc_metadata/src/rmeta/{mod,encoder}.rs` at each of those releases,
//! and returns `None` for any other version or for any byte that does not match.
//! The caller then reports no names rather than wrong ones.
//!
//! # Layout
//!
//! The `.rustc` section is `METADATA_HEADER` (`rust\0\0\0` + format version 10),
//! a little-endian `u64` length, then the metadata blob. All positions below are
//! offsets into that blob, which starts at section offset 16.
//!
//! The blob repeats the header, then holds the `CrateRoot` position as a raw
//! little-endian `u64`, then the `rustc <version>` string. Everything else uses
//! the opaque encoder: integers are LEB128, a `String` is `<len> <bytes> 0xC1`,
//! an `Option` is a `0`/`1` byte, a derived enum's discriminant is one byte, a
//! `Symbol` is `0x00` + string, `0x01` + offset of an earlier string, or `0x02` +
//! predefined index.
//!
//! Metadata is a post-order tree. A `LazyValue`/`LazyArray` field does not hold
//! an absolute position: the first lazy field in a node stores the *backward*
//! distance from the node's start, each later one the *forward* distance from the
//! previous lazy field's target. `LazyArray` stores its element count first and
//! omits the distance when the count is zero.
//!
//! `CrateRoot` starts with `CrateHeader { triple, hash: Svh (16 raw bytes),
//! name: Symbol, is_proc_macro_crate, is_stub }`, then `extra_filename: String`,
//! `stable_crate_id` (8 raw bytes), `required_panic_strategy: Option<_>`,
//! `panic_in_drop_strategy`, `edition`, four `bool`s, a run of `LazyArray`s (15 on
//! 1.98, 16 from 1.99 — `canonical_symbols` was added), then
//! `proc_macro_data: Option<ProcMacroData { proc_macro_decls_static: DefIndex,
//! stability: Option<_>, macros: LazyArray<(DefIndex, LazyValue<ProcMacroKind>)> }>`.
//!
//! The `macros` array is the anchor: rustc fills it in the same loop that emits
//! the decls table (`proc_macro_harness::mk_decls` calls `declare_proc_macro` per
//! entry, and `encode_proc_macros` walks that list), so its order *is* the table
//! order, and each element points at a `ProcMacroKind` record:
//! `<kind> <name: String> [derive only: <helper count> <helper: String>…]`. The
//! trait name is stored as a full `String` even when rustc predefines the symbol
//! (`Display`, `Debug`, …); only the def-path echo elsewhere uses the `0x02`
//! form, and this module never reads that.

/// Macro kind discriminants, matching `ProcMacroKind`'s variant order.
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

/// `rust\0\0\0` followed by `METADATA_VERSION`, which is 10 on 1.98 through 1.100.
const METADATA_HEADER: &[u8; 8] = b"rust\0\0\0\x0a";
/// Terminates every encoded `str`; the decoder asserts on it.
const STR_SENTINEL: u8 = 0xC1;
const SYMBOL_STR: u8 = 0;
const SYMBOL_OFFSET: u8 = 1;
const SYMBOL_PREDEFINED: u8 = 2;
/// Header, then the raw `u64` root position: the version string starts here.
const BLOB_VERSION_POS: usize = 16;

/// The metadata blob inside a `.rustc` section, if the framing checks out.
fn blob(meta: &[u8]) -> Option<&[u8]> {
    if meta.get(..8)? != METADATA_HEADER {
        return None;
    }
    let len = u64::from_le_bytes(meta.get(8..16)?.try_into().ok()?);
    let end = 16usize.checked_add(usize::try_from(len).ok()?)?;
    let blob = meta.get(16..end)?;
    (blob.get(..8)? == METADATA_HEADER).then_some(blob)
}

/// The `CrateRoot`'s position within the blob.
fn root_pos(blob: &[u8]) -> Option<usize> {
    let pos = usize::try_from(u64::from_le_bytes(blob.get(8..16)?.try_into().ok()?)).ok()?;
    (pos >= BLOB_VERSION_POS && pos < blob.len()).then_some(pos)
}

fn blob_version(blob: &[u8]) -> Option<&str> {
    Reader::at(blob, BLOB_VERSION_POS)?.str()
}

/// The `rustc <version>` string recorded at the head of a `.rustc` section.
pub fn version_string(meta: &[u8]) -> Option<String> {
    if let Some(v) = blob(meta).and_then(blob_version) {
        return Some(v.to_string());
    }
    // Older layouts, or a header this module does not know: the string is still
    // length-prefixed and starts with `rustc `, which is enough to report it.
    let at = meta.windows(6).position(|w| w == b"rustc ")?;
    let len = usize::from(*meta.get(at.checked_sub(1)?)?);
    let s = std::str::from_utf8(meta.get(at..at.checked_add(len)?)?).ok()?;
    s.bytes()
        .all(|b| (0x20..0x7f).contains(&b))
        .then(|| s.to_string())
}

/// How many `LazyArray` fields sit between the scalar prefix of `CrateRoot` and
/// `proc_macro_data`, per rustc version.
///
/// This is the one place the three supported releases differ: 1.99 inserted
/// `canonical_symbols`. Anything else is unknown and decodes to nothing — a later
/// rustc that moves a field would otherwise be read as garbage, and while the
/// structural checks downstream would almost certainly reject it, "almost" is
/// not the standard here.
fn lazy_arrays_before_proc_macro_data(version: &str) -> Option<usize> {
    let rest = version.strip_prefix("rustc 1.")?;
    let digits = rest.find(|c: char| !c.is_ascii_digit())?;
    match rest[..digits].parse::<u32>().ok()? {
        98 => Some(15),
        99 | 100 => Some(16),
        _ => None,
    }
}

/// Bounds-checked cursor over the opaque encoding.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn at(bytes: &'a [u8], pos: usize) -> Option<Self> {
        (pos <= bytes.len()).then_some(Reader { bytes, pos })
    }

    fn u8(&mut self) -> Option<u8> {
        let b = *self.bytes.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }

    /// A `bool` or `Option` tag: anything but `0`/`1` means the layout moved.
    fn bool(&mut self) -> Option<bool> {
        match self.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    fn skip(&mut self, n: usize) -> Option<()> {
        let end = self.pos.checked_add(n)?;
        (end <= self.bytes.len()).then(|| self.pos = end)
    }

    /// Unsigned LEB128, as `emit_usize`/`emit_u64`/`emit_u32` write it.
    fn leb(&mut self) -> Option<usize> {
        let mut value: u64 = 0;
        for shift in (0..64).step_by(7) {
            let byte = self.u8()?;
            let low = u64::from(byte & 0x7f);
            // The tenth byte can only carry the top bit.
            if shift == 63 && low > 1 {
                return None;
            }
            value |= low << shift;
            if byte & 0x80 == 0 {
                return usize::try_from(value).ok();
            }
        }
        None
    }

    /// `<len> <utf-8 bytes> 0xC1`.
    fn str(&mut self) -> Option<&'a str> {
        let len = self.leb()?;
        let start = self.pos;
        let end = start.checked_add(len)?;
        let s = std::str::from_utf8(self.bytes.get(start..end)?).ok()?;
        self.pos = end;
        (self.u8()? == STR_SENTINEL).then_some(s)
    }

    /// Skip a `Symbol` in any of its three encodings.
    fn symbol(&mut self) -> Option<()> {
        match self.u8()? {
            SYMBOL_STR => self.str().map(|_| ()),
            SYMBOL_OFFSET => (self.leb()? < self.bytes.len()).then_some(()),
            SYMBOL_PREDEFINED => (self.leb()? <= u32::MAX as usize).then_some(()),
            _ => None,
        }
    }
}

/// Lazy-distance bookkeeping for one metadata node.
struct Node {
    start: usize,
    prev: Option<usize>,
}

impl Node {
    fn new(start: usize) -> Self {
        Node { start, prev: None }
    }

    /// The absolute position a lazy field's distance denotes. Distances start at
    /// 1 — zero-length nodes do not exist — and every target lies before the node
    /// that refers to it.
    fn resolve(&mut self, distance: usize) -> Option<usize> {
        if distance == 0 {
            return None;
        }
        let pos = match self.prev {
            None => self.start.checked_sub(distance)?,
            Some(prev) => prev.checked_add(distance)?,
        };
        if pos >= self.start {
            return None;
        }
        self.prev = Some(pos);
        Some(pos)
    }
}

/// Position and element count of `ProcMacroData::macros`.
struct MacrosArray {
    pos: usize,
    len: usize,
}

/// Walk `CrateRoot` from its start to `proc_macro_data.macros`.
fn macros_array(blob: &[u8], root: usize, n_arrays: usize) -> Option<MacrosArray> {
    let mut r = Reader::at(blob, root)?;
    let mut node = Node::new(root);

    // CrateHeader.
    match r.u8()? {
        // TargetTuple::TargetTuple(tuple)
        0 => {
            r.str()?;
        }
        // TargetTuple::TargetJson { tuple, contents } — the path is not encoded.
        1 => {
            r.str()?;
            r.str()?;
        }
        _ => return None,
    }
    r.skip(16)?; // hash: Svh, a Fingerprint written as raw bytes
    r.symbol()?; // name
    if !r.bool()? {
        return None; // is_proc_macro_crate: this is not a proc-macro crate
    }
    if r.bool()? {
        return None; // is_stub: the real metadata lives in a separate file
    }

    // CrateRoot scalars.
    r.str()?; // extra_filename
    r.skip(8)?; // stable_crate_id: Hash64 written as raw bytes
    if r.bool()? {
        r.u8()?; // required_panic_strategy: Some(PanicStrategy)
    }
    r.u8()?; // panic_in_drop_strategy
    r.u8()?; // edition
    for _ in 0..4 {
        r.bool()?; // has_global_allocator … has_default_lib_allocator
    }

    // The LazyArray fields. Their contents are irrelevant, but each non-empty one
    // moves the lazy cursor that the `macros` distance is measured from.
    for _ in 0..n_arrays {
        if r.leb()? > 0 {
            node.resolve(r.leb()?)?;
        }
    }

    // proc_macro_data: Option<ProcMacroData>.
    if !r.bool()? {
        return None;
    }
    r.leb()?; // proc_macro_decls_static: DefIndex
    if r.bool()? {
        // stability: Some(..) needs `#![feature(staged_api)]`, so only the
        // standard library's own proc-macro crates would get here. Its encoding
        // is involved enough not to be worth decoding blind.
        return None;
    }
    let len = r.leb()?;
    if len == 0 {
        return None;
    }
    let pos = node.resolve(r.leb()?)?;
    Some(MacrosArray { pos, len })
}

/// The `ProcMacroKind` record position of each `macros` element, in table order.
fn record_positions(blob: &[u8], array: &MacrosArray) -> Option<Vec<usize>> {
    let mut r = Reader::at(blob, array.pos)?;
    let mut node = Node::new(array.pos);
    let mut out = Vec::with_capacity(array.len);
    for _ in 0..array.len {
        r.leb()?; // DefIndex
        out.push(node.resolve(r.leb()?)?);
    }
    Some(out)
}

/// Whether `s` could be a macro, trait or helper-attribute name.
fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c == '_' || c.is_alphabetic())
        && chars.all(|c| c == '_' || c.is_alphanumeric())
}

/// Decode one `ProcMacroKind` record.
fn record_at(blob: &[u8], pos: usize) -> Option<ProcMacroEntry> {
    let mut r = Reader::at(blob, pos)?;
    let kind = r.u8()?;
    if !matches!(kind, KIND_DERIVE | KIND_ATTR | KIND_BANG) {
        return None;
    }
    let name = r.str()?;
    if !is_identifier(name) {
        return None;
    }
    if kind == KIND_DERIVE {
        // attributes: Vec<String> — the helper attributes.
        let helpers = r.leb()?;
        if helpers > blob.len() {
            return None;
        }
        for _ in 0..helpers {
            if !is_identifier(r.str()?) {
                return None;
            }
        }
    }
    Some(ProcMacroEntry {
        kind,
        name: name.to_string(),
    })
}

/// Proc-macro `(kind, name)` pairs in decls-table order.
///
/// `fn_names` must be the macro functions' own names, in table order, as taken
/// from the symbol table. They are not used to find anything — the metadata is
/// decoded on its own terms — but the result must agree with them: same count,
/// and every bang or attribute macro named after its function.
pub fn proc_macro_entries(meta: &[u8], fn_names: &[String]) -> Option<Vec<ProcMacroEntry>> {
    if fn_names.is_empty() {
        return None;
    }
    let blob = blob(meta)?;
    let n_arrays = lazy_arrays_before_proc_macro_data(blob_version(blob)?)?;
    let root = root_pos(blob)?;
    let array = macros_array(blob, root, n_arrays)?;
    if array.len != fn_names.len() {
        return None;
    }
    let entries = record_positions(blob, &array)?
        .into_iter()
        .map(|pos| record_at(blob, pos))
        .collect::<Option<Vec<_>>>()?;
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

    /// One macro in a synthetic crate.
    struct Macro {
        kind: u8,
        name: &'static str,
        helpers: &'static [&'static str],
        /// A `#[doc]` string rustc would store right before the record. It encodes
        /// as `0x00 <len> <text> 0xC1` — byte-identical to a derive record header,
        /// which is what fooled the previous identifier-scanning approach.
        doc: Option<&'static str>,
    }

    fn derive(name: &'static str, helpers: &'static [&'static str]) -> Macro {
        Macro {
            kind: KIND_DERIVE,
            name,
            helpers,
            doc: None,
        }
    }

    fn attr(name: &'static str) -> Macro {
        Macro {
            kind: KIND_ATTR,
            name,
            helpers: &[],
            doc: None,
        }
    }

    fn bang(name: &'static str) -> Macro {
        Macro {
            kind: KIND_BANG,
            name,
            helpers: &[],
            doc: None,
        }
    }

    /// A synthetic `.rustc` section in the real layout: the framing, the version
    /// string, per-macro records and def-path echoes, the `macros` array, and a
    /// `CrateRoot` whose lazy distances are computed the way the encoder does.
    struct Synth {
        version: &'static str,
        n_arrays: usize,
        /// How many of the leading `LazyArray`s to make non-empty, so the
        /// forward-distance bookkeeping gets exercised. Real proc-macro crates
        /// leave every one of them empty.
        filled_arrays: usize,
        json_triple: bool,
        predefined_crate_name: bool,
        is_proc_macro: bool,
        stability: bool,
        macros: Vec<Macro>,
    }

    impl Synth {
        fn new(version: &'static str, n_arrays: usize, macros: Vec<Macro>) -> Self {
            Synth {
                version,
                n_arrays,
                filled_arrays: 0,
                json_triple: false,
                predefined_crate_name: false,
                is_proc_macro: true,
                stability: false,
                macros,
            }
        }

        /// The section bytes and each record's *section* offset, in order.
        fn build(&self) -> (Vec<u8>, Vec<usize>) {
            fn leb(v: &mut Vec<u8>, mut n: usize) {
                loop {
                    let byte = (n & 0x7f) as u8;
                    n >>= 7;
                    if n == 0 {
                        v.push(byte);
                        return;
                    }
                    v.push(byte | 0x80);
                }
            }
            fn string(v: &mut Vec<u8>, s: &str) {
                leb(v, s.len());
                v.extend_from_slice(s.as_bytes());
                v.push(STR_SENTINEL);
            }

            let mut b = Vec::new();
            b.extend_from_slice(METADATA_HEADER);
            b.extend_from_slice(&[0u8; 8]); // root position, patched below
            string(&mut b, self.version);

            // Filler resembling the def-path table entries that precede the
            // records in a real blob: `01 00 06 <symbol>` per function name.
            for m in &self.macros {
                b.extend_from_slice(&[1, 0, 6, SYMBOL_STR]);
                string(&mut b, &format!("fn_{}", m.name));
            }

            let mut records = Vec::new();
            for (i, m) in self.macros.iter().enumerate() {
                if let Some(doc) = m.doc {
                    b.extend_from_slice(&[0x00, 0x14, 0, 0, 0]);
                    string(&mut b, doc);
                }
                records.push(b.len());
                b.push(m.kind);
                string(&mut b, m.name);
                if m.kind == KIND_DERIVE {
                    leb(&mut b, m.helpers.len());
                    for h in m.helpers {
                        string(&mut b, h);
                    }
                }
                // The def-key echo: alternate between the string form and the
                // predefined form so both appear.
                b.extend_from_slice(&[1, 0, 7]);
                if i % 2 == 0 {
                    b.push(SYMBOL_STR);
                    string(&mut b, m.name);
                } else {
                    b.push(SYMBOL_PREDEFINED);
                    leb(&mut b, 1000 + i);
                }
                b.push(0);
            }

            // Non-empty leading arrays, if requested: three opaque bytes each.
            let mut filled = Vec::new();
            for k in 0..self.filled_arrays {
                filled.push(b.len());
                b.extend_from_slice(&[0x2a, 0x2b, k as u8]);
            }

            // macros: LazyArray<(DefIndex, LazyValue<ProcMacroKind>)>.
            let array = b.len();
            for (i, &rec) in records.iter().enumerate() {
                leb(&mut b, 4 + i); // DefIndex
                let dist = if i == 0 {
                    array - rec
                } else {
                    rec - records[i - 1]
                };
                leb(&mut b, dist);
            }

            // CrateRoot.
            let root = b.len();
            if self.json_triple {
                b.push(1);
                string(&mut b, "custom-target");
                string(&mut b, "{\"llvm-target\":\"x\"}");
            } else {
                b.push(0);
                string(&mut b, "x86_64-unknown-linux-gnu");
            }
            b.extend_from_slice(&[0xab; 16]); // Svh
            if self.predefined_crate_name {
                b.push(SYMBOL_PREDEFINED);
                leb(&mut b, 4242);
            } else {
                b.push(SYMBOL_STR);
                string(&mut b, "synth");
            }
            b.push(self.is_proc_macro as u8);
            b.push(0); // is_stub
            string(&mut b, "-0123456789abcdef");
            b.extend_from_slice(&[0xcd; 8]); // stable_crate_id
            b.push(0); // required_panic_strategy: None
            b.push(0); // panic_in_drop_strategy
            b.push(2); // edition
            b.extend_from_slice(&[0, 0, 0, 0]);
            // The filled arrays come first, then the empty remainder.
            let mut prev: Option<usize> = None;
            for &pos in &filled {
                leb(&mut b, 3);
                let dist = match prev {
                    None => root - pos,
                    Some(p) => pos - p,
                };
                leb(&mut b, dist);
                prev = Some(pos);
            }
            for _ in filled.len()..self.n_arrays {
                leb(&mut b, 0);
            }
            b.push(1); // proc_macro_data: Some
            leb(&mut b, 13); // proc_macro_decls_static
            if self.stability {
                b.extend_from_slice(&[1, 0, 0, 0, 0]);
            } else {
                b.push(0);
            }
            leb(&mut b, records.len());
            let dist = match prev {
                None => root - array,
                Some(p) => array - p,
            };
            leb(&mut b, dist);
            b.extend_from_slice(b"\x01\x00rust-end-file");

            b[8..16].copy_from_slice(&(root as u64).to_le_bytes());

            let mut section = Vec::new();
            section.extend_from_slice(METADATA_HEADER);
            section.extend_from_slice(&(b.len() as u64).to_le_bytes());
            section.extend_from_slice(&b);
            (section, records.iter().map(|r| r + 16).collect())
        }
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn entries(v: &[(u8, &str)]) -> Vec<ProcMacroEntry> {
        v.iter()
            .map(|(k, n)| ProcMacroEntry {
                kind: *k,
                name: n.to_string(),
            })
            .collect()
    }

    /// The probe crate every real-blob test below was built from, in source order.
    /// Function names differ from trait names, `Display` is a symbol rustc
    /// predefines, `Serialize` carries a helper attribute, `TwoHelpers` two, and
    /// the doc comments, `#[deprecated(note)]` and `#[doc(alias)]` all put
    /// derive-record lookalikes into the blob. The bang comes last so that a
    /// phantom entry would push it off the end.
    const PROBE_FNS: [&str; 7] = [
        "derive_frob_impl",
        "derive_display_impl",
        "derive_serialize",
        "derive_two",
        "derive_old",
        "tag_it",
        "last_bang",
    ];
    const PROBE_ENTRIES: [(u8, &str); 7] = [
        (KIND_DERIVE, "Frobnicate"),
        (KIND_DERIVE, "Display"),
        (KIND_DERIVE, "Serialize"),
        (KIND_DERIVE, "TwoHelpers"),
        (KIND_DERIVE, "OldDerive"),
        (KIND_ATTR, "tag_it"),
        (KIND_BANG, "last_bang"),
    ];

    fn probe_macros() -> Vec<Macro> {
        vec![
            Macro {
                doc: Some(" A derive whose function name differs from its trait name."),
                ..derive("Frobnicate", &[])
            },
            derive("Display", &[]),
            derive("Serialize", &["serde"]),
            Macro {
                doc: Some(" PhantomDeriveName lurking in the docs."),
                ..derive("TwoHelpers", &["alpha", "beta"])
            },
            derive("OldDerive", &[]),
            attr("tag_it"),
            bang("last_bang"),
        ]
    }

    #[test]
    fn reads_the_version_string() {
        let (sec, _) = Synth::new(
            "rustc 1.100.0-nightly (cea272fa3 2026-09-07)",
            16,
            vec![bang("m1")],
        )
        .build();
        assert_eq!(
            version_string(&sec).as_deref(),
            Some("rustc 1.100.0-nightly (cea272fa3 2026-09-07)")
        );
        // A section in a layout this module does not decode still reports its
        // version through the fallback scan.
        let mut old = b"rust\0\0\0\x09\0\0\0\0\0\0\0\0".to_vec();
        old.push(20);
        old.extend_from_slice(b"rustc 1.97.0 (abcdef)");
        assert_eq!(
            version_string(&old).as_deref(),
            Some("rustc 1.97.0 (abcdef")
        );
        assert_eq!(version_string(b"nothing here"), None);
    }

    #[test]
    fn layout_is_selected_by_version() {
        assert_eq!(
            lazy_arrays_before_proc_macro_data("rustc 1.98.0 (88d9e12ae 2026-08-18)"),
            Some(15)
        );
        assert_eq!(
            lazy_arrays_before_proc_macro_data("rustc 1.98.1 (48a229cea 2026-09-01)"),
            Some(15)
        );
        assert_eq!(
            lazy_arrays_before_proc_macro_data("rustc 1.99.0-beta.7 (aa0593682 2026-09-19)"),
            Some(16)
        );
        assert_eq!(
            lazy_arrays_before_proc_macro_data("rustc 1.100.0-nightly (cea272fa3 2026-09-07)"),
            Some(16)
        );
        // Neither older nor newer releases are decoded: fail closed.
        assert_eq!(
            lazy_arrays_before_proc_macro_data("rustc 1.97.0 (2d8144b78 2026-07-07)"),
            None
        );
        assert_eq!(
            lazy_arrays_before_proc_macro_data("rustc 1.101.0-nightly (0 2026-10-01)"),
            None
        );
        assert_eq!(lazy_arrays_before_proc_macro_data("rustc 2.0.0"), None);
        assert_eq!(lazy_arrays_before_proc_macro_data("clang 1.98.0"), None);
    }

    /// The case the whole exercise exists for: derives whose trait names differ
    /// from their function names, next to the attributes that used to produce
    /// phantom records — on both root layouts.
    #[test]
    fn recovers_derive_trait_names_and_kinds() {
        for (version, n_arrays) in [
            ("rustc 1.98.0 (88d9e12ae 2026-08-18)", 15),
            ("rustc 1.99.0-beta.7 (aa0593682 2026-09-19)", 16),
            ("rustc 1.100.0-nightly (cea272fa3 2026-09-07)", 16),
        ] {
            let (sec, _) = Synth::new(version, n_arrays, probe_macros()).build();
            assert_eq!(
                proc_macro_entries(&sec, &names(&PROBE_FNS)),
                Some(entries(&PROBE_ENTRIES)),
                "{version}"
            );
        }
    }

    /// A real proc-macro crate leaves every `LazyArray` before `proc_macro_data`
    /// empty, so the forward-distance rule only gets exercised synthetically.
    #[test]
    fn follows_lazy_distances_past_non_empty_arrays() {
        let mut synth = Synth::new("rustc 1.99.0 (0 2026-10-29)", 16, probe_macros());
        synth.filled_arrays = 3;
        let (sec, _) = synth.build();
        assert_eq!(
            proc_macro_entries(&sec, &names(&PROBE_FNS)),
            Some(entries(&PROBE_ENTRIES))
        );
    }

    /// The header's crate name may be a predefined symbol and the triple a custom
    /// JSON target; neither is needed, but both have to be stepped over correctly.
    #[test]
    fn steps_over_every_header_encoding() {
        let mut synth = Synth::new("rustc 1.98.0 (88d9e12ae 2026-08-18)", 15, probe_macros());
        synth.predefined_crate_name = true;
        synth.json_triple = true;
        let (sec, _) = synth.build();
        assert_eq!(
            proc_macro_entries(&sec, &names(&PROBE_FNS)),
            Some(entries(&PROBE_ENTRIES))
        );
    }

    #[test]
    fn refuses_rather_than_guessing_when_nothing_lines_up() {
        let fns = names(&PROBE_FNS);
        let good = Synth::new("rustc 1.98.0 (88d9e12ae 2026-08-18)", 15, probe_macros());
        let (sec, records) = good.build();
        assert!(proc_macro_entries(&sec, &fns).is_some());

        // Not metadata at all, or no functions to check against.
        assert_eq!(proc_macro_entries(b"\0\0\0\0garbage", &fns), None);
        assert_eq!(proc_macro_entries(&sec, &[]), None);

        // A version this module has no layout for.
        let (sec97, _) =
            Synth::new("rustc 1.97.0 (2d8144b78 2026-07-07)", 15, probe_macros()).build();
        assert_eq!(proc_macro_entries(&sec97, &fns), None);
        // The right version string over the other layout: the walk lands on the
        // wrong bytes and must notice.
        let (wrong, _) =
            Synth::new("rustc 1.98.0 (88d9e12ae 2026-08-18)", 16, probe_macros()).build();
        assert_eq!(proc_macro_entries(&wrong, &fns), None);

        // Count mismatch in either direction.
        assert_eq!(proc_macro_entries(&sec, &fns[..6]), None);
        let mut more = fns.clone();
        more.push("extra".to_string());
        assert_eq!(proc_macro_entries(&sec, &more), None);

        // A bang or attribute macro is invoked by its function's name; a mismatch
        // means the table and the metadata disagree.
        let mut swapped = fns.clone();
        swapped.swap(5, 6);
        assert_eq!(proc_macro_entries(&sec, &swapped), None);
        let mut renamed = fns.clone();
        renamed[6] = "something_else".to_string();
        assert_eq!(proc_macro_entries(&sec, &renamed), None);

        // Corruption at any record: an unknown kind, a missing string sentinel, a
        // name that is not an identifier.
        for (i, &rec) in records.iter().enumerate() {
            let mut bad = sec.clone();
            bad[rec] = 3;
            assert_eq!(proc_macro_entries(&bad, &fns), None, "kind of record {i}");
            let mut bad = sec.clone();
            let len = usize::from(bad[rec + 1]);
            bad[rec + 2 + len] = 0xC2;
            assert_eq!(
                proc_macro_entries(&bad, &fns),
                None,
                "sentinel of record {i}"
            );
            let mut bad = sec.clone();
            bad[rec + 2] = b'1';
            assert_eq!(
                proc_macro_entries(&bad, &fns),
                None,
                "identifier of record {i}"
            );
        }

        // Header claims: not a proc-macro crate, or a stability entry this module
        // does not decode.
        let mut not_pm = Synth::new("rustc 1.98.0 (88d9e12ae 2026-08-18)", 15, probe_macros());
        not_pm.is_proc_macro = false;
        assert_eq!(proc_macro_entries(&not_pm.build().0, &fns), None);
        let mut stable = Synth::new("rustc 1.98.0 (88d9e12ae 2026-08-18)", 15, probe_macros());
        stable.stability = true;
        assert_eq!(proc_macro_entries(&stable.build().0, &fns), None);

        // Truncation anywhere: the declared length no longer fits, or the root
        // position points past the end.
        for cut in [sec.len() - 1, sec.len() - 40, sec.len() / 2, 20, 16, 8] {
            assert_eq!(proc_macro_entries(&sec[..cut], &fns), None, "cut at {cut}");
        }
        let mut short_len = sec.clone();
        short_len[8..16].copy_from_slice(&40u64.to_le_bytes());
        assert_eq!(proc_macro_entries(&short_len, &fns), None);
        let mut bad_root = sec.clone();
        bad_root[24..32].copy_from_slice(&(u64::MAX).to_le_bytes());
        assert_eq!(proc_macro_entries(&bad_root, &fns), None);
        bad_root[24..32].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(proc_macro_entries(&bad_root, &fns), None);
    }

    fn from_hex(lines: &[&str]) -> Vec<u8> {
        lines
            .iter()
            .flat_map(|l| l.as_bytes().chunks(2))
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    /// The `.rustc` section of the probe crate built with rustc 1.98.0, captured
    /// with `objcopy --dump-section .rustc=…`.
    const PROBE_SECTION_1_98: &[&str] = &[
        "727573740000000a5309000000000000727573740000000ace0700000000000023727573746320312e39382e30202838",
        "386439653132616520323032362d30382d313829c100000001000600106465726976655f66726f625f696d706cc10001",
        "000600136465726976655f646973706c61795f696d706cc10001000600106465726976655f73657269616c697a65c100",
        "010006000a6465726976655f74776fc100010006000a6465726976655f6f6c64c10001000600067461675f6974c10001",
        "000600096c6173745f62616e67c1000000fc00a909000014010000fc00380000352050726f62652063726174653a2065",
        "7665727920736861706520746861742062726f6b6520746865206f6c64207363616e6e65722ec100000a46726f626e69",
        "63617465c1000014000000fc573d00003a2041206465726976652077686f73652066756e6374696f6e206e616d652064",
        "6966666572732066726f6d20697473207472616974206e616d652ec1010007000a46726f626e6963617465c100fcb601",
        "36000f05000007446973706c6179c1000014000000fc85022400002120507265646566696e65642073796d626f6c2061",
        "73207472616974206e616d652ec1010007027500fcc80239000f0500000953657269616c697a65c101057365726465c1",
        "0014000000fc9a0327000024205468652073657264652073686170653a2068656c70657220617474726962757465732e",
        "c1010007000953657269616c697a65c100fcf50336000f0500000a54776f48656c70657273c10205616c706861c10462",
        "657461c10014000000fcc4044a0000472054776f48656c7065727320776974682074776f2068656c7065722061747472",
        "69627574657320616e64206120646f6320636f6d6d656e7420746861742068617320746578742ec10014000000fc8f05",
        "2a000027205068616e746f6d4465726976654e616d65206c75726b696e6720696e2074686520646f63732ec101000700",
        "0a54776f48656c70657273c100fcf40530000f050000094f6c64446572697665c1000011030100214465707265636174",
        "65644e6f74652075736520736f6d657468696e6720656c7365c1fcd106230000fcbd06390001000700094f6c64446572",
        "697665c100fc970730000f050001067461675f6974c10013dce00700010009416c6961734e616d65c15cee0700000000",
        "000000000000000000000000000000000001000701a60100fc94083f000f050002096c6173745f62616e67c101000701",
        "b30100fcec0833000f0500048305056d0647075d08bc0109580a43c7fb1afe903d4fde00000000000000000000000000",
        "00000000000000000000002bb3f9f49ea33f5c5816f64d6049aa0ff26d552de55b3ac60257c97fa8f78930cccb6e34ce",
        "cffbdbf1d92c63ece9985346b45492be80dc25c60100000000000000000000160101800101d00101340202e201023601",
        "037c0003010000002d2d2d2d2d2c2b07010000000000007401bb011802d4022c036f038a03c1000000000000007201b9",
        "011602d2022a036d03880300000000000000006d01b4011102cd02250368038303bf35000000000000005c01ae010102",
        "bc02150361037c0300000006000f02000201000000000000000000000000000000000000000000000078048904d0034f",
        "444854010804200e00000000000000200000000000000000000002b7de00005816f64d6049aa0f050000006449824b41",
        "a9a78102000000a1efc72f603a2a050b000000000000000000000000000000000000000000000000000000f26d552de5",
        "5b3ac6060000004ec7a628664903230c0000000000000000000000000000000257c97fa8f78930070000000000000000",
        "00000000000000000000000000000000000000000000000000000000000000f1d92c63ece99853090000000000000000",
        "00000000000000cccb6e34cecffbdb08000000000000000000000000000000c7fb1afe903d4fde0000000076169729b1",
        "37ee6e030000000000000000000000000000000000000000000000000000002d390cdbf421a6e20d0000000000000000",
        "000000000000000000000000000000000000008238990bb76dd077010000000000000000000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000046b45492be80dc250a0000002bb3f9f49ea33f5c04000000074002ffff6311ff18ffffff29ff6dff6f",
        "37ffff71ffff3bffffffffffff122e074002ffff6311ff18ffffff29ff6dff00000a7372632f6c69622e7273c1732f74",
        "6d702f636c617564652d313030302f2d686f6d652d796173756f2d6768712d6769746875622d636f6d2d796173756f2d",
        "6f7a752d6d616372612f34313033633638342d613039652d343837622d383138312d3765303866626463356662312f73",
        "6372617463687061642f706d70726f6265c17e2f746d702f636c617564652d313030302f2d686f6d652d796173756f2d",
        "6768712d6769746875622d636f6d2d796173756f2d6f7a752d6d616372612f34313033633638342d613039652d343837",
        "622d383138312d3765303866626463356662312f736372617463687061642f706d70726f62652f7372632f6c69622e72",
        "73c10000203ad42c4e5d0ce2ac3eb68693755c05280000000000000000000000000000000000aa09aa091e01391d013e",
        "214e01251e510128334e014b2b3a48013a2048011c1849010e00d5298c9c318690afbdf8c2fd2c0f1ba200006f060018",
        "7838365f36342d756e6b6e6f776e2d6c696e75782d676e75c1b773a71dfb72aeb6c59be8d50edb18340007706d70726f",
        "6265c10100112d33643138646337326233393139366662c1c7fb1afe903d4fde00000200000000000000000000000000",
        "000000000000010d0007c308000010000000000000000000080b00000058000000000000000000000000000000000000",
        "000000000000000000000000000000000000000000000000030b00000021000000010b00020b0b020b16020b16000016",
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "000000010100000001000000000000020b00000016000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000021020100020102020201af0600000000000000000100727573742d656e642d66",
        "696c65",
    ];

    /// The same probe crate built with rustc 1.100.0-nightly (cea272fa3
    /// 2026-09-07): the 16-array root layout shared with 1.99.
    const PROBE_SECTION_1_100: &[&str] = &[
        "727573740000000a6909000000000000727573740000000ad7070000000000002c727573746320312e3130302e302d6e",
        "696768746c79202863656132373266613320323032362d30392d303729c100000001000600106465726976655f66726f",
        "625f696d706cc10001000600136465726976655f646973706c61795f696d706cc10001000600106465726976655f7365",
        "7269616c697a65c100010006000a6465726976655f74776fc100010006000a6465726976655f6f6c64c1000100060006",
        "7461675f6974c10001000600096c6173745f62616e67c1000000fc00a909000014010000fc00380000352050726f6265",
        "2063726174653a20657665727920736861706520746861742062726f6b6520746865206f6c64207363616e6e65722ec1",
        "00000a46726f626e6963617465c1000014000000fc573d00003a2041206465726976652077686f73652066756e637469",
        "6f6e206e616d6520646966666572732066726f6d20697473207472616974206e616d652ec1010007000a46726f626e69",
        "63617465c100fcb60136000f05000007446973706c6179c1000014000000fc85022400002120507265646566696e6564",
        "2073796d626f6c206173207472616974206e616d652ec1010007027700fcc80239000f0500000953657269616c697a65",
        "c101057365726465c10014000000fc9a0327000024205468652073657264652073686170653a2068656c706572206174",
        "74726962757465732ec1010007000953657269616c697a65c100fcf50336000f0500000a54776f48656c70657273c102",
        "05616c706861c10462657461c10014000000fcc4044a0000472054776f48656c7065727320776974682074776f206865",
        "6c706572206174747269627574657320616e64206120646f6320636f6d6d656e7420746861742068617320746578742e",
        "c10014000000fc8f052a000027205068616e746f6d4465726976654e616d65206c75726b696e6720696e207468652064",
        "6f63732ec1010007000a54776f48656c70657273c100fcf40530000f050000094f6c64446572697665c1000011030100",
        "21446570726563617465644e6f74652075736520736f6d657468696e6720656c7365c1fcd106230000fcbd0639000100",
        "0700094f6c64446572697665c100fc970730000f050001067461675f6974c10013dce00700010009416c6961734e616d",
        "65c15cee0700000000000000000000000000000000000000000001000701af0100fc94083f000f050002096c6173745f",
        "62616e67c101000701bc0100fcec0833000f0500048305056d0647075d08bc0109580a43f61d0fd29eb0b49b00000000",
        "00000000000000000000000000000000000000001300060bcf2b36bf75b2afa1d5038920a8317fd5da1ba30c6be79652",
        "a799188bf7970dae0f80a75e89ed069a103076b63ab8676c88b5c7c3cf01000000000000000000001f0101890101d901",
        "013d0202eb01023f0103850003010000002c2c2c2c2c2b2a10010000000000007d01c4012102dd02350378039303ca00",
        "0000000000007b01c2011f02db0233037603910300000000000000007601bd011a02d6022e0371038c03c83e00000000",
        "0000006501b7010a02c5021e036a03850300000006000f02000201000000000000000000000000000000000000000000",
        "000081049204d0034f444854010804200e00000000000000200000000000000000000002b7de00000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "0000000094eac3fe058ce3c6010000000000000000000000000000006be79652a799188b070000003ab8676c88b5c7c3",
        "0a000000000000000000000000000000000000000000000000000000000000000000000000000000e7beefd1acad3d44",
        "02000000000000000000000000000000185870f14ee190540c0000001300060bcf2b36bf04000000f7970dae0f80a75e",
        "0800000089ed069a103076b6090000000a21fcee31fb87f20b0000000000000000000000000000000000000000000000",
        "0000000075b2afa1d5038920050000000000000000000000000000000000000000000000000000000000000000000000",
        "00000000000000000000000000000000a8317fd5da1ba30c0600000000000000000000000000000049ebeefbbc149bc7",
        "0d000000000000000000000000000000f61d0fd29eb0b49b00000000e9813f0b9f724be403000000ffffffffff63ff45",
        "61ffffff22ff2a5f2f5b79ffff10ffffffff06ff63ff4d72ffffffffff63ff4561ffffff22ff2a5f00000a7372632f6c",
        "69622e7273c1732f746d702f636c617564652d313030302f2d686f6d652d796173756f2d6768712d6769746875622d63",
        "6f6d2d796173756f2d6f7a752d6d616372612f34313033633638342d613039652d343837622d383138312d3765303866",
        "626463356662312f736372617463687061642f706d70726f6265c17e2f746d702f636c617564652d313030302f2d686f",
        "6d652d796173756f2d6768712d6769746875622d636f6d2d796173756f2d6f7a752d6d616372612f3431303363363834",
        "2d613039652d343837622d383138312d3765303866626463356662312f736372617463687061642f706d70726f62652f",
        "7372632f6c69622e7273c10000203ad42c4e5d0ce2ac3eb68693755c05280000000000000000000000000000000000aa",
        "09aa091e01391d013e214e01251e510128334e014b2b3a48013a2048011c1849010e0096124fdb837656222066463edd",
        "7699ce0000780600187838365f36342d756e6b6e6f776e2d6c696e75782d676e75c1e0b3be1eb3640345027eab0d548a",
        "aefa0007706d70726f6265c10100112d37363462633463373139386232393763c1f61d0fd29eb0b49b00000200000000",
        "00000000000000000000000000000000010d0007c308000010000000000000000000080b000000580000000000000000",
        "00000000000000000000000000000000000000000000000000000000000000000000000000030b00000021000000010b",
        "00020b0b020b16020b160000160000000000000000000000000000000000000000000000000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "00000000000000000000000000000000010100000001000000000000020b000000160000000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000000000000000000021020100020102020201",
        "af0600000000000000000100727573742d656e642d66696c65",
    ];

    /// Bytes captured from real dylibs, one per root layout.
    #[test]
    fn decodes_sections_captured_from_real_dylibs() {
        for (name, hex, version) in [
            (
                "1.98.0",
                PROBE_SECTION_1_98,
                "rustc 1.98.0 (88d9e12ae 2026-08-18)",
            ),
            (
                "1.100.0-nightly",
                PROBE_SECTION_1_100,
                "rustc 1.100.0-nightly (cea272fa3 2026-09-07)",
            ),
        ] {
            let sec = from_hex(hex);
            assert_eq!(version_string(&sec).as_deref(), Some(version), "{name}");

            // The blob really does contain the strings that used to read as
            // derive records — a doc comment, a `#[deprecated(note)]`, a
            // `#[doc(alias)]` — and the def-path echo for `Display` really is a
            // predefined symbol with no string, so the record must be the source.
            let blob = &sec[16..];
            for needle in [
                &b"PhantomDeriveName"[..],
                b"DeprecatedNote",
                b"AliasName",
                b"A derive whose function name",
            ] {
                assert!(
                    blob.windows(needle.len()).any(|w| w == needle),
                    "{name}: {:?}",
                    std::str::from_utf8(needle)
                );
            }
            assert_eq!(
                blob.windows(7).filter(|w| w == b"Display").count(),
                1,
                "{name}"
            );

            assert_eq!(
                proc_macro_entries(&sec, &names(&PROBE_FNS)),
                Some(entries(&PROBE_ENTRIES)),
                "{name}"
            );

            // Any disagreement with the symbol table still fails closed.
            let mut swapped = names(&PROBE_FNS);
            swapped.swap(5, 6);
            assert_eq!(proc_macro_entries(&sec, &swapped), None, "{name}");
            assert_eq!(
                proc_macro_entries(&sec, &names(&PROBE_FNS[..6])),
                None,
                "{name}"
            );
        }
    }

    /// Validates the scan against a `.rustc` section dumped from any dylib. Ignored
    /// by default because it needs the file; run with
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
}
