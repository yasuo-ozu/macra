mod macro_finder;
mod pretty;

use std::io::{self, stdout};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use cargo_macra::parse_trace::{MacroExpansion, MacroExpansionKind};
use cargo_macra::trace_macros::{MacroExpansionIter, TraceMacros};
use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use macro_finder::{MacroCall, MacroKind, find_macros, is_builtin_attribute};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
#[derive(Parser, Debug, Clone)]
#[command(name = "cargo-macra")]
#[command(bin_name = "cargo macra")]
#[command(about = "Interactive macro expansion viewer for Rust")]
struct Args {
    /// Subcommand name (when invoked as `cargo macra`)
    #[arg(hide = true)]
    _subcommand: Option<String>,

    /// Package to check
    #[arg(short, long)]
    package: Option<String>,

    /// Build only the specified binary
    #[arg(long)]
    bin: Option<String>,

    /// Build only the specified library
    #[arg(long)]
    lib: bool,

    /// Build only the specified test target
    #[arg(long)]
    test: Option<String>,

    /// Build only the specified example
    #[arg(long)]
    example: Option<String>,

    /// Path to Cargo.toml
    #[arg(long)]
    manifest_path: Option<String>,

    /// Print all macro expansions to stdout and exit without launching the TUI
    #[arg(long)]
    show_expansion: bool,

    /// Coloring of printed expansions
    #[arg(long, value_name = "WHEN", value_enum, default_value_t = pretty::ColorChoice::Auto)]
    color: pretty::ColorChoice,

    /// Module path to open (e.g., "foo::bar" opens the file for module `crate::foo::bar`)
    module: Option<String>,

    /// Additional arguments to pass to cargo
    #[arg(trailing_var_arg = true)]
    cargo_args: Vec<String>,
}

/// A node in the macro tree
#[derive(Debug, Clone)]
struct MacroNode {
    /// The macro call info
    call: MacroCall,
    /// Unique ID for this node
    id: usize,
    /// Parent node ID (None for root-level macros)
    parent_id: Option<usize>,
    /// Depth in the tree (0 for root level)
    depth: usize,
    /// Whether this macro has been expanded
    expanded: bool,
    /// Whether expansion failed for this macro
    expansion_failed: bool,
    /// The original lines before expansion (for undo)
    original_lines: Vec<String>,
    /// The expanded content (if expanded)
    expanded_content: Option<String>,
    /// Child node IDs
    children: Vec<usize>,
    /// Whether children are visible (collapsed/expanded in tree view)
    children_visible: bool,
    /// Identifies the derives of one `#[derive(A, B)]` on one item. Set to the id of
    /// the group's first derive, so it is unique and — unlike a line number — never
    /// moves. Matching siblings by line broke as soon as one of them was expanded and
    /// left its line, which made an already-expanded derive look unexpanded and get
    /// re-emitted in the remaining `#[derive(...)]`.
    derive_group: usize,
    /// Set when this node's source text was swallowed by an enclosing expansion (the
    /// id of that expansion's node), so its coordinates describe lines that no longer
    /// exist. Such nodes are hidden, and are un-hidden with their state intact when
    /// the consumer is undone — an already-expanded node included, so the buffer the
    /// undo puts back and the node agree. Which expansion swallowed a node decides
    /// whether it moves when the buffer changes elsewhere (see `shift_nodes`): it moves
    /// exactly when its consumer does. Applying a shift on the node's own stale line
    /// number instead wrapped it past `usize::MAX` for the consuming expansion and left
    /// it stranded inside another expansion's output for every later one.
    consumed_by: Option<usize>,
    /// Nodes this expansion swallowed, un-hidden on undo.
    consumed_ids: Vec<usize>,
    /// Nodes this expansion relocated onto its lifted tail, as they were beforehand.
    /// Undo restores them: reversing only the line shift left their columns rebased
    /// onto the tail line and their `original_lines` holding the tail's text, so they
    /// overlapped the macro they had shared a line with.
    tail_relocation_snapshot: Vec<RelocatedNode>,
    /// The `line_origins` entries this expansion replaced, restored verbatim on undo.
    /// Recomputing them from the node's `call.line` gets the gutter wrong as soon as an
    /// earlier expansion has shifted this node, and stamps real line numbers onto lines
    /// that are themselves expansion output (they must stay `None`).
    original_line_origins: Vec<Option<usize>>,
}

/// Non-blocking poll for a key that cancels a pending expansion lookup.
///
/// Called while waiting on the trace stream. Waiting used to be unbounded on the UI
/// thread, so a trace that never arrived (a hung `cargo check`, or a macro whose
/// expansion was never captured) froze the TUI completely — no redraw, no input, and
/// no Ctrl-C because raw mode swallows SIGINT.
/// Paint a one-line notice on the bottom row while a trace lookup blocks.
///
/// The lookup runs on the UI thread and ratatui cannot redraw during it, so without
/// this the screen simply freezes on the pre-keypress frame for as long as `cargo
/// check` takes — indistinguishable from a hang. Written straight to the alternate
/// screen with crossterm; the next `terminal.draw` paints over it.
fn draw_wait_notice(name: &str) {
    use crossterm::{
        QueueableCommand,
        cursor::MoveTo,
        style::{Attribute, Print, SetAttribute},
        terminal::{Clear, ClearType, size},
    };
    use std::io::Write;

    let Ok((w, h)) = size() else { return };
    if h == 0 || w == 0 {
        return;
    }
    let text = fit_to_width(
        &format!(" Expanding '{}' — waiting for cargo… (Esc cancels) ", name),
        w as usize,
    );
    let mut out = stdout();
    let _ = out.queue(MoveTo(0, h - 1));
    let _ = out.queue(Clear(ClearType::CurrentLine));
    let _ = out.queue(SetAttribute(Attribute::Reverse));
    let _ = out.queue(Print(text));
    let _ = out.queue(SetAttribute(Attribute::Reset));
    let _ = out.flush();
}

fn wait_cancelled() -> bool {
    while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if key.code == KeyCode::Esc
                || (key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')))
            {
                return true;
            }
        }
    }
    false
}

/// How long a lookup lets the trace stream settle after its first match before
/// deciding whether the match is ambiguous.
///
/// Entries for one macro are emitted together, so this is generous for spotting a
/// collision while still bounded — it never waits for the whole build.
const AMBIGUITY_SETTLE: std::time::Duration = std::time::Duration::from_millis(250);

/// Outcome of a trace lookup.
enum TraceLookup {
    /// Exactly one distinct expansion matched.
    Found(String),
    /// Several equally good but *differing* expansions matched, and nothing in the
    /// trace distinguishes them — the user has to say which one they meant.
    Ambiguous(Vec<TraceCandidate>),
    /// The expansion stream ran to completion without a match.
    Exhausted,
    /// The user cancelled while waiting.
    Aborted,
}

/// An ambiguous expansion awaiting the user's choice.
struct PendingChoice {
    /// The macro node being expanded.
    node_id: usize,
    /// Its name, for the popup title.
    name: String,
    candidates: Vec<TraceCandidate>,
    selected: usize,
}

/// A node's coordinates before it was relocated onto an expansion's lifted tail,
/// kept so undo can put it back exactly.
#[derive(Debug, Clone)]
struct RelocatedNode {
    id: usize,
    /// `call.line` of the expanded node at the time. The snapshot below is absolute,
    /// so undo restores it shifted by however far the expansion has moved since —
    /// restoring it verbatim lost every expansion made above it in between.
    anchor_line: usize,
    line: usize,
    line_end: usize,
    derive_line: usize,
    item_line_end: usize,
    col_start: usize,
    col_end: usize,
    original_lines: Vec<String>,
}

/// One option offered when a lookup is ambiguous.
#[derive(Clone)]
struct TraceCandidate {
    /// First non-blank line of `output`, as a one-line summary for the popup.
    ///
    /// Deliberately *not* `exp.expanding`: for a bang macro that field holds the
    /// invocation's input and for a derive it holds the macro name, and a collision
    /// requires both to be equal — so every candidate would carry the same label and
    /// the list could not be chosen from. The output is what actually differs.
    label: String,
    /// The crate that defines the macro this expansion came from, or empty when the
    /// hook could not resolve it (a `macro_rules!` trace, or a non-unix host).
    krate: String,
    /// The expansion this candidate would inline.
    output: String,
}

struct CacheInner {
    expansions: Vec<MacroExpansion>,
    /// `normalize_tokens` of each expansion's `input` and `arguments`, computed once
    /// when the entry is pushed; index-aligned with `expansions`.
    normalized: Vec<(String, String)>,
    done: bool,
    error: Option<String>,
    /// Stored build failure message (non-zero exit from cargo check).
    build_error: Option<String>,
}

struct ExpansionCache {
    inner: Arc<(Mutex<CacheInner>, Condvar)>,
    /// This run's `cargo check`. Killed explicitly on reload and by `Drop` on exit,
    /// so repeated `r` presses cannot stack concurrent cargo processes.
    child: Arc<Mutex<std::process::Child>>,
}

impl ExpansionCache {
    fn new(
        iter: MacroExpansionIter,
        check_result: std::sync::mpsc::Receiver<io::Result<cargo_macra::trace_macros::CheckResult>>,
        child: Arc<Mutex<std::process::Child>>,
    ) -> Self {
        let inner = Arc::new((
            Mutex::new(CacheInner {
                expansions: Vec::new(),
                normalized: Vec::new(),
                done: false,
                error: None,
                build_error: None,
            }),
            Condvar::new(),
        ));

        let bg_inner = Arc::clone(&inner);
        thread::spawn(move || {
            let (ref mutex, ref condvar) = *bg_inner;
            for result in iter {
                match result {
                    Ok(exp) => {
                        // Normalize outside the lock: entries are matched against this
                        // pre-normalized text, so it is computed once per entry ever
                        // instead of once per candidate per scan.
                        let normalized = (
                            cargo_macra::normalize_tokens(&exp.input),
                            cargo_macra::normalize_tokens(&exp.arguments),
                        );
                        let mut cache = mutex.lock().unwrap();
                        cache.expansions.push(exp);
                        cache.normalized.push(normalized);
                        condvar.notify_all();
                    }
                    Err(e) => {
                        let mut cache = mutex.lock().unwrap();
                        cache.error = Some(format!("{}", e));
                        cache.done = true;
                        condvar.notify_all();
                        break;
                    }
                }
            }
            // Check the build result after the expansion stream is exhausted.
            if let Some(msg) = Self::check_result_to_build_error(check_result.recv()) {
                let mut cache = mutex.lock().unwrap();
                cache.build_error = Some(msg);
            }
            let mut cache = mutex.lock().unwrap();
            cache.done = true;
            condvar.notify_all();
        });

        Self { inner, child }
    }

    /// Turn the `cargo check` completion message into a user-visible build error.
    ///
    /// A successful build produces `None`, and so does a disconnected channel (the run
    /// was killed on reload or exit). A failed build yields its compiler error lines.
    /// An `io::Error` from `wait()`ing on the child is surfaced too: silently dropping
    /// it degraded the user-visible message to a generic "No trace found".
    fn check_result_to_build_error(
        recv: Result<
            io::Result<cargo_macra::trace_macros::CheckResult>,
            std::sync::mpsc::RecvError,
        >,
    ) -> Option<String> {
        match recv {
            Ok(Ok(result)) => {
                if result.success {
                    return None;
                }
                // Extract compiler error lines from stderr (skip hook/trace noise).
                let errors: Vec<&str> = result
                    .stderr
                    .lines()
                    .filter(|l| l.starts_with("error"))
                    .collect();
                if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("\n"))
                }
            }
            Ok(Err(e)) => Some(format!("failed to wait for cargo check: {}", e)),
            Err(_) => None,
        }
    }

    /// Kill this run's `cargo check`.
    ///
    /// Safe to call at any time: if the child already exited and was reaped by the
    /// reader thread, `kill` returns an error that is deliberately ignored. Closing
    /// the pipes makes both reader threads see EOF and finish on their own.
    fn kill_child(&self) {
        let mut child = self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = child.kill();
    }

    /// Convert MacroKind to MacroExpansionKind for comparison.
    fn to_expansion_kind(kind: MacroKind) -> MacroExpansionKind {
        match kind {
            MacroKind::Functional => MacroExpansionKind::Bang,
            MacroKind::Attribute => MacroExpansionKind::Attribute,
            MacroKind::Derive => MacroExpansionKind::Derive,
        }
    }

    /// Check if a MacroExpansion matches the given input, arguments, name, and kind
    /// using normalized comparison for input and arguments.
    /// When `relaxed_name` is true, also matches mangled names that contain the
    /// source name (e.g. `__Parse_temporal_<hash>` matches source name `Parse`).
    /// Convenience form that normalizes on every call; the hot path in
    /// `search_expansions` calls `expansion_matches_pre` directly, with normalization
    /// done once per entry and once per query instead.
    #[cfg(test)]
    fn expansion_matches(
        exp: &MacroExpansion,
        input: &str,
        arguments: &str,
        name: &str,
        kind: MacroKind,
        relaxed_name: bool,
        strict_input: bool,
    ) -> bool {
        Self::expansion_matches_pre(
            exp,
            &cargo_macra::normalize_tokens(&exp.input),
            &cargo_macra::normalize_tokens(&exp.arguments),
            input,
            &cargo_macra::normalize_tokens(input),
            &cargo_macra::normalize_tokens(arguments),
            name,
            kind,
            relaxed_name,
            strict_input,
        )
    }

    /// Core of `expansion_matches`, taking pre-normalized token text.
    ///
    /// `exp_norm_input` / `exp_norm_arguments` must be `normalize_tokens` of
    /// `exp.input` / `exp.arguments` (stored in `CacheInner::normalized`), and
    /// `norm_input` / `norm_arguments` those of the query. The raw `input` is still
    /// needed for the truncated-invocation fallback below.
    #[allow(clippy::too_many_arguments)]
    fn expansion_matches_pre(
        exp: &MacroExpansion,
        exp_norm_input: &str,
        exp_norm_arguments: &str,
        input: &str,
        norm_input: &str,
        norm_arguments: &str,
        name: &str,
        kind: MacroKind,
        relaxed_name: bool,
        strict_input: bool,
    ) -> bool {
        let macro_name = name.rsplit("::").next().unwrap_or(name).trim();
        let exp_name = exp.name.rsplit("::").next().unwrap_or(&exp.name).trim();
        // Mangled helper macros are generated as `__<Name>_<hash>`, so the relaxed
        // pass has to stop at a `_` boundary. A bare prefix test also accepted
        // `__Parser_<hash>` for source name `Parse`, silently expanding a different
        // macro — and with only one such match there is no collision to prompt about.
        let name_matches = exp_name == macro_name
            || (relaxed_name
                && !macro_name.is_empty()
                && exp_name
                    .strip_prefix("__")
                    .and_then(|rest| rest.strip_prefix(macro_name))
                    .is_some_and(|tail| tail.is_empty() || tail.starts_with('_')));
        let input_matches = if kind == MacroKind::Attribute {
            // Hook-captured attribute macros serialize doc comments differently than
            // syn (`///` vs `#[doc = "..."]`), so the inputs legitimately differ and
            // this comparison cannot simply be required. But skipping it outright
            // reduces the key to name + arguments, and two bare `#[my_attr]`s on
            // different items then collide — expanding the second showed the first
            // one's output. So try the input first and only fall back to ignoring it.
            !strict_input || exp_norm_input == norm_input
        } else if exp.input.is_empty() {
            // Either both inputs are empty (normal match), or rustc may
            // truncate very large macro invocations and emit only `name!`
            // without arguments (fallback).
            input.is_empty()
                || (exp.kind == MacroExpansionKind::Bang && exp.expanding.trim_end().ends_with('!'))
        } else {
            exp_norm_input == norm_input
        };
        name_matches
            && exp.kind == Self::to_expansion_kind(kind)
            && input_matches
            && exp_norm_arguments == norm_arguments
    }

    /// The distinct expansions among `hits`, in stream order.
    ///
    /// Collisions that expand to the same text are not a real choice — the usual case
    /// being one macro invoked twice with identical arguments — so they collapse to a
    /// single candidate. Only genuinely differing outputs are worth asking about.
    fn distinct_candidates(inner: &CacheInner, hits: &[usize]) -> Vec<TraceCandidate> {
        let mut candidates: Vec<TraceCandidate> = Vec::new();
        for &idx in hits {
            let exp = &inner.expansions[idx];
            if candidates.iter().any(|c| c.output == exp.to) {
                continue;
            }
            let summary = exp
                .to
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("(empty expansion)");
            // Naming the defining crate is usually what tells two colliding macros
            // apart — the same derive name coming from two different crates.
            let label = if exp.krate.is_empty() {
                summary.to_string()
            } else {
                format!("[{}] {}", exp.krate, summary)
            };
            candidates.push(TraceCandidate {
                label,
                krate: exp.krate.clone(),
                output: exp.to.clone(),
            });
        }
        candidates
    }

    /// Narrow an ambiguous candidate list to the crate a path-qualified call named.
    ///
    /// `test_proc_macros::Greet` and a bare `Greet` from another crate expand
    /// differently, and the source says which one was meant — so a collision that the
    /// output alone cannot resolve often is resolved by the path.
    ///
    /// Deliberately a preference, not a filter. The hook reports the crate that
    /// *defines* a macro, while the path names the crate it was *reached through*, and
    /// those differ for every re-export — `serde::Serialize` is defined in
    /// `serde_derive`. So a qualifier that matches nothing leaves the list untouched
    /// and the user still gets the popup; it can only ever remove a wrong answer, never
    /// the only right one.
    fn narrow_by_crate(candidates: Vec<TraceCandidate>, krate: &str) -> Vec<TraceCandidate> {
        if krate.is_empty() || candidates.len() < 2 {
            return candidates;
        }
        let matching: Vec<TraceCandidate> = candidates
            .iter()
            .filter(|c| c.krate == krate)
            .cloned()
            .collect();
        if matching.len() == 1 {
            matching
        } else {
            candidates
        }
    }

    /// Every cached expansion matching this query at the most specific pass that
    /// matches anything, in stream order.
    ///
    /// Passes run most specific first — an exact name before a relaxed (mangled) one,
    /// and an input that actually matches before falling back to ignoring it — and the
    /// first pass to match anything wins outright. A less specific pass can never add
    /// candidates alongside a more specific one.
    ///
    /// `norm_input` / `norm_arguments` are `normalize_tokens` of the query, computed
    /// once by the caller. `min_idx` restricts the scan to entries at index >=
    /// `min_idx`: `find_trace_for_tokens` passes 0 for a lookup's first scan and, after
    /// each *unsuccessful* scan, the length it saw, so later wakes only test entries
    /// appended since. That is safe because entries are immutable and only ever
    /// appended, and matching is a pure function of entry and query — an entry rejected
    /// by every pass cannot start matching later. (A scan that does match returns
    /// immediately, so the watermark never hides a candidate from a later scan.)
    ///
    /// Note the `strict_input` dimension only affects attribute macros; for bang and
    /// derive macros both values behave identically, so those passes are skipped.
    #[allow(clippy::too_many_arguments)]
    fn search_expansions(
        inner: &CacheInner,
        input: &str,
        norm_input: &str,
        norm_arguments: &str,
        name: &str,
        kind: MacroKind,
        min_idx: usize,
    ) -> Vec<usize> {
        let strict_passes: &[bool] = if kind == MacroKind::Attribute {
            &[true, false]
        } else {
            &[true]
        };
        for &strict_input in strict_passes {
            for relaxed in [false, true] {
                let hits: Vec<usize> = (min_idx..inner.expansions.len())
                    .filter(|&idx| {
                        let (exp_norm_input, exp_norm_arguments) = &inner.normalized[idx];
                        Self::expansion_matches_pre(
                            &inner.expansions[idx],
                            exp_norm_input,
                            exp_norm_arguments,
                            input,
                            norm_input,
                            norm_arguments,
                            name,
                            kind,
                            relaxed,
                            strict_input,
                        )
                    })
                    .collect();
                if !hits.is_empty() {
                    return hits;
                }
            }
        }

        Vec::new()
    }

    /// Find an expansion that matches the given macro input/arguments.
    ///
    /// Waits for the background stream to produce a match, but never blocks the UI
    /// thread indefinitely: `should_abort` is polled between waits so the caller can
    /// let the user cancel. Without this the TUI freezes with no redraw and no key
    /// handling whenever a trace never arrives — and because raw mode swallows
    /// SIGINT, Ctrl-C cannot break out either.
    fn find_trace_for_tokens(
        &self,
        input: &str,
        arguments: &str,
        name: &str,
        krate: &str,
        kind: MacroKind,
        should_abort: &mut dyn FnMut() -> bool,
    ) -> TraceLookup {
        // Normalize the query once for the whole lookup, not once per candidate per wake.
        let norm_input = cargo_macra::normalize_tokens(input);
        let norm_arguments = cargo_macra::normalize_tokens(arguments);

        let (ref mutex, ref condvar) = *self.inner;
        let mut inner = mutex.lock().unwrap();

        // Entries below this watermark have already been tested against this query in
        // all four passes and rejected, so later wakes only scan what arrived since.
        // The watermark is read under the lock, and pushes also happen under the lock,
        // so an entry arriving during a wait always has an index >= the watermark.
        let mut scanned_len = 0;

        loop {
            let hits = Self::search_expansions(
                &inner,
                input,
                &norm_input,
                &norm_arguments,
                name,
                kind,
                scanned_len,
            );
            if !hits.is_empty() {
                // A colliding entry may still be a few pushes behind: the reader thread
                // appends one entry at a time and notifies per entry, so the first push
                // wakes us holding a single hit even when its twin is imminent.
                // Returning here would auto-pick it and never show the popup for any
                // expansion done while the build is still running. Give the stream a
                // short bounded window to settle instead — long enough for entries
                // emitted together, short enough not to wait on the whole build.
                if !inner.done {
                    let deadline = std::time::Instant::now() + AMBIGUITY_SETTLE;
                    while !inner.done && std::time::Instant::now() < deadline {
                        let (guard, _timeout) = condvar
                            .wait_timeout(inner, std::time::Duration::from_millis(25))
                            .unwrap();
                        inner = guard;
                        if should_abort() {
                            return TraceLookup::Aborted;
                        }
                    }
                }
                // Rescan from the start: the watermark would exclude the hits found
                // above, and every candidate has to be considered together.
                let hits = Self::search_expansions(
                    &inner,
                    input,
                    &norm_input,
                    &norm_arguments,
                    name,
                    kind,
                    0,
                );
                let mut candidates =
                    Self::narrow_by_crate(Self::distinct_candidates(&inner, &hits), krate);
                return match candidates.len() {
                    // Unreachable: entries are only ever appended, so the hits found
                    // above are still present.
                    0 => TraceLookup::Exhausted,
                    1 => TraceLookup::Found(candidates.pop().expect("just checked").output),
                    _ => TraceLookup::Ambiguous(candidates),
                };
            }
            scanned_len = inner.expansions.len();

            if inner.done {
                return TraceLookup::Exhausted;
            }

            // Wait in short slices so the abort check stays responsive.
            let (guard, _timeout) = condvar
                .wait_timeout(inner, std::time::Duration::from_millis(50))
                .unwrap();
            inner = guard;

            if should_abort() {
                return TraceLookup::Aborted;
            }
        }
    }

    /// Write a diagnostic log file for a failed expansion.
    /// Contains the macro info dump followed by all cached expansion traces.
    /// Returns the path to the log file.
    fn write_error_log(
        &self,
        name: &str,
        kind: MacroKind,
        input: &str,
        arguments: &str,
    ) -> Option<PathBuf> {
        use std::io::Write;

        let tmp_dir = std::env::temp_dir().join("macra");
        if std::fs::create_dir_all(&tmp_dir).is_err() {
            return None;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let log_path = tmp_dir.join(format!("expansion-error-{}.log", timestamp));

        let mut file = match std::fs::File::create(&log_path) {
            Ok(f) => f,
            Err(_) => return None,
        };

        // Dump macro info
        let _ = writeln!(file, "name: {}", name);
        let _ = writeln!(file, "kind: {}", kind.as_str());
        let _ = writeln!(file, "input: {}", input);
        let _ = writeln!(file, "arguments: {}", arguments);
        let _ = writeln!(file);

        // Dump all cached expansions (--show-expansion format)
        let (ref mutex, _) = *self.inner;
        let inner = mutex.lock().unwrap();

        if inner.expansions.is_empty() {
            let _ = writeln!(file, "No macro expansions found.");
        } else {
            let mut first = true;
            for expansion in &inner.expansions {
                if !first {
                    let _ = writeln!(file);
                }
                first = false;

                let caller = match expansion.kind {
                    MacroExpansionKind::Bang => format!("{}!", expansion.name),
                    MacroExpansionKind::Attribute => {
                        if expansion.arguments.is_empty() {
                            format!("#[{}]", expansion.name)
                        } else {
                            format!(
                                "#[{}({})]",
                                expansion.name,
                                expansion.arguments.replace('\n', " ")
                            )
                        }
                    }
                    MacroExpansionKind::Derive => format!("#[derive({})]", expansion.name),
                };

                let _ = writeln!(file, "== {} ==", caller);
                if !expansion.input.is_empty() {
                    let _ = writeln!(file, "{}", expansion.input);
                }
                let _ = writeln!(file, "---");
                let _ = writeln!(file, "{}", expansion.to);
            }
        }

        Some(log_path)
    }

    /// Take the stored error (if any), clearing it.
    fn take_error(&self) -> Option<String> {
        let (ref mutex, _) = *self.inner;
        let mut inner = mutex.lock().unwrap();
        inner.error.take()
    }

    /// Return the stored build error (if any).
    fn build_error(&self) -> Option<String> {
        let (ref mutex, _) = *self.inner;
        let inner = mutex.lock().unwrap();
        inner.build_error.clone()
    }
}

impl Drop for ExpansionCache {
    fn drop(&mut self) {
        // Quitting the TUI should not leave a `cargo check` running behind it.
        self.kill_child();
    }
}

/// Saved state when navigating into a submodule
struct ModuleState {
    source_lines: Vec<String>,
    line_origins: Vec<Option<usize>>,
    nodes: Vec<MacroNode>,
    next_id: usize,
    visible_nodes: Vec<usize>,
    selected_idx: usize,
    list_state: ListState,
    scroll_offset: usize,
    cursor_line: usize,
    cursor_col: usize,
    file_path: PathBuf,
    module_path: Vec<String>,
}

struct App {
    /// Current displayed source (line-based for easy manipulation)
    source_lines: Vec<String>,
    /// Tracks the original file line number for each source_lines entry.
    /// `Some(n)` means the line corresponds to original line `n` (1-indexed).
    /// `None` means the line is from an expansion (no line number shown).
    line_origins: Vec<Option<usize>>,
    /// All macro nodes (flat storage, tree structure via parent_id/children)
    nodes: Vec<MacroNode>,
    /// Next node ID
    next_id: usize,
    /// Background-cached expansion data
    expansion_cache: ExpansionCache,
    /// Flattened list of visible node IDs for display
    visible_nodes: Vec<usize>,
    /// Currently selected index in visible_nodes
    selected_idx: usize,
    list_state: ListState,
    scroll_offset: usize,
    /// Current cursor line in the source view (1-indexed). This can be on any line,
    /// not just macro lines.
    cursor_line: usize,
    /// Current cursor column (0-indexed) within `cursor_line`. Several macros can
    /// share one line — most importantly the derives of a single `#[derive(A, B)]`,
    /// which are order-independent and must each be selectable. Left/Right move the
    /// cursor between the macro spans on the current line.
    cursor_col: usize,
    /// Height of the source view area (updated each frame)
    source_view_height: u16,
    /// Status message to display
    status: String,
    /// Error message to display (shown until user presses Enter)
    error_message: Option<String>,
    /// Set when an expansion lookup matched several differing traces and is waiting
    /// for the user to pick one.
    pending_choice: Option<PendingChoice>,
    /// Set when something drew outside ratatui's buffer (the blocking-lookup notice),
    /// so the event loop knows the next frame has to be a full repaint.
    needs_full_redraw: bool,
    /// Reusable `TraceMacros` for reloading trace data
    trace_macros: TraceMacros,
    /// Path of the currently loaded source file
    file_path: PathBuf,
    /// Module path segments (e.g., ["crate", "foo", "bar"])
    module_path: Vec<String>,
    /// Stack of saved module states for returning to parent modules
    module_stack: Vec<ModuleState>,
    /// When set, each expanded range is rendered as a two-column block comparing the
    /// original source (left) with the expansion (right). Code outside those ranges
    /// stays full width. Toggled with `v`.
    split_view: bool,
}

/// A range of `source_lines` that holds macro output, paired with the source it
/// replaced, so the two can be shown side by side.
struct SplitRegion<'a> {
    /// 0-based index into `source_lines` where the expansion starts.
    start: usize,
    /// How many `source_lines` entries the expansion occupies.
    len: usize,
    /// The source lines that were replaced.
    original: &'a [String],
    /// The expansion's own lines as `(source index, text)`. The `// -- expanded: X --`
    /// / `// -- end X --` markers are dropped: the block header already names the
    /// macro. Keeping the source index means the cursor still lines up after the drop.
    right: Vec<(usize, &'a str)>,
    /// Name of the macro that produced the expansion.
    name: &'a str,
}

impl SplitRegion<'_> {
    /// Body rows in the block: the taller of the two columns.
    fn rows(&self) -> usize {
        self.right.len().max(self.original.len())
    }
}

/// Sort regions by position and drop any that sit inside another. A macro expanded
/// inside another macro's output already lies within that parent's range; splitting it
/// again would nest columns inside columns.
fn keep_outermost<'a>(mut regions: Vec<SplitRegion<'a>>) -> Vec<SplitRegion<'a>> {
    regions.sort_by_key(|r| (r.start, std::cmp::Reverse(r.len)));
    let mut out: Vec<SplitRegion> = Vec::new();
    for r in regions {
        match out.last() {
            Some(prev) if r.start < prev.start + prev.len => continue,
            _ => out.push(r),
        }
    }
    out
}

/// Is this one of the `// -- expanded: X --` / `// -- end X --` marker lines that
/// `expand_selected` wraps inlined output in?
fn is_expansion_marker(line: &str) -> bool {
    // Markers are generated as `// -- expanded: NAME --` and `// -- end NAME --`,
    // where NAME is a macro path and so never contains whitespace. Requiring the whole
    // shape — both delimiters and a whitespace-free name — keeps an ordinary comment
    // like `// -- end of section --` from being taken for a marker and silently
    // dropped from a split block. The old test accepted any `// -- end …--` line.
    let t = line.trim();
    let names_a_macro = |prefix: &str| {
        t.strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix("--"))
            .map(str::trim)
            .is_some_and(|name| !name.is_empty() && !name.contains(char::is_whitespace))
    };
    names_a_macro("// -- expanded:") || names_a_macro("// -- end ")
}

/// Tab stop width used when expanding `\t` in loaded source files.
///
/// 4 matches the rustfmt default indent, so tab-indented code lines up the way its
/// author most likely sees it.
const TAB_WIDTH: usize = 4;

/// Expand tabs to spaces, advancing to the next multiple of [`TAB_WIDTH`].
///
/// Must run on the loaded source *before* `find_macros` parses it: macro columns come
/// from proc-macro2 spans computed over this exact text, and ratatui drops control
/// characters from the buffer entirely, so the parsed text and the displayed text have
/// to be one and the same string. Columns are counted in characters (matching
/// proc-macro2 span columns and `byte_of_col`) and reset on every newline.
fn expand_tabs(source: &str) -> String {
    if !source.contains('\t') {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut col = 0usize;
    for ch in source.chars() {
        match ch {
            '\t' => {
                let n = TAB_WIDTH - col % TAB_WIDTH;
                for _ in 0..n {
                    out.push(' ');
                }
                col += n;
            }
            '\n' => {
                out.push('\n');
                col = 0;
            }
            _ => {
                out.push(ch);
                col += 1;
            }
        }
    }
    out
}

/// What `$crate` is rewritten to before expansion output is handed to `find_macros`:
/// `syn::parse_file` rejects the `$` token, but the placeholder is a plain identifier.
const DOLLAR_CRATE_PLACEHOLDER: &str = "__macra_dollar_crate__";

/// Map character column `col` of `safe_line` — a line of the placeholder-substituted
/// parse text — back onto the real text, where every placeholder ending at or before
/// `col` was a `$crate`. A span boundary never falls inside an identifier, so a
/// placeholder is either wholly before the column or not counted.
fn unsubstituted_col(safe_line: &str, col: usize) -> usize {
    let prefix: String = safe_line.chars().take(col).collect();
    let stretch = DOLLAR_CRATE_PLACEHOLDER.len() - "$crate".len();
    col - prefix.matches(DOLLAR_CRATE_PLACEHOLDER).count() * stretch
}

/// The node whose position stands for `id`'s: `id` itself while it is visible, else
/// the expansion that swallowed it — following the chain when that one was swallowed
/// in turn. A consumed node's own coordinates are frozen at the moment it was hidden,
/// so they say nothing about where its text will reappear; its consumer's do.
fn anchor_of(nodes: &[MacroNode], id: usize) -> usize {
    let mut cur = id;
    // A hidden node cannot be expanded, so it cannot consume, and the chain is
    // acyclic; the bound only keeps a corrupt one from spinning forever.
    for _ in 0..nodes.len() {
        match nodes
            .iter()
            .find(|n| n.id == cur)
            .and_then(|n| n.consumed_by)
        {
            Some(next) => cur = next,
            None => break,
        }
    }
    cur
}

/// Account for node `edited` replacing buffer lines `first..=last` (1-indexed) with
/// `delta` more (or, negative, fewer) lines.
///
/// `expand_node` and `undo_selected` both go through here, with opposite `delta`s,
/// so they cannot disagree about which nodes move. They used to: expansion skipped
/// every consumed node while undo skipped only its own, so a node swallowed by an
/// earlier expansion was never moved forward but was moved back, ending up on a
/// line inside another expansion's output once it was un-hidden.
///
/// A node moves when its anchor (`anchor_of`) starts after `first`. For a visible
/// node that is its own line. A consumed node moves exactly when its consumer does,
/// because that is where its text comes back on undo — and it stays put when the
/// edit happens *inside* its consumer's output, which the consumer's undo removes
/// wholesale. Nodes anchored on `edited` itself are the ones this edit swallowed;
/// `pinned` are those relocated onto the lifted tail, already at their final place.
///
/// An unexpanded attribute or derive whose item encloses the edited range does not
/// move, but its item grew or shrank with it, so `item_line_end` follows. Left stale,
/// expanding the attribute later removed too few lines and stranded the rest of its
/// item — `    baz();` ... `}` — after its end marker.
fn shift_nodes(
    nodes: &mut [MacroNode],
    edited: usize,
    first: usize,
    last: usize,
    delta: isize,
    pinned: &[usize],
) {
    if delta == 0 {
        return;
    }
    // Decide before moving anything: an anchor's line is itself about to change.
    let moves: Vec<bool> = nodes
        .iter()
        .map(|node| {
            let anchor = anchor_of(nodes, node.id);
            anchor != edited
                && !pinned.contains(&node.id)
                && nodes
                    .iter()
                    .find(|n| n.id == anchor)
                    .is_some_and(|a| a.call.line > first)
        })
        .collect();
    let shifted = |v: usize| (v as isize + delta) as usize;
    for (node, moves) in nodes.iter_mut().zip(moves) {
        if moves {
            node.call.line = shifted(node.call.line);
            node.call.line_end = shifted(node.call.line_end);
            node.call.item_line_end = shifted(node.call.item_line_end);
            // `derive_line` keys the column span of derive nodes and can differ from
            // `line` for a multi-line `#[derive(...)]`; leaving it behind silently
            // breaks h/l stepping and the selection highlight.
            node.call.derive_line = shifted(node.call.derive_line);
        } else if node.id != edited
            && node.consumed_by.is_none()
            && !node.expanded
            && node.call.kind != MacroKind::Functional
            && node.call.line <= first
            && node.call.item_line_end >= last
        {
            node.call.item_line_end = shifted(node.call.item_line_end);
        }
    }
}

/// Build the top-level macro nodes for one file.
///
/// Shared by the initial file and by `enter_submodule` so a module behaves exactly
/// like the root file.
fn build_root_nodes(source: &str, source_lines: &[String]) -> (Vec<MacroNode>, usize) {
    let macros = find_macros(source);

    let mut nodes = Vec::new();
    let mut next_id = 0;

    // Items whose (single) top-level attribute macro node has already been added.
    // Attribute macros are order-*dependent*: rustc expands them outside-in, and
    // each one's output contains the remaining attributes, so only the first can
    // be expanded from the original source. The rest appear as children later.
    let mut item_first_attr: std::collections::HashSet<usize> = std::collections::HashSet::new();
    // Items that carry a non-built-in attribute macro. Derives on such an item
    // cannot be expanded until that attribute has run, so they stay hidden.
    let attr_gated_items: std::collections::HashSet<usize> = macros
        .iter()
        .filter(|m| m.kind == MacroKind::Attribute && !is_builtin_attribute(&m.name))
        .map(|m| m.item_line_end)
        .collect();

    // Derives of one `#[derive(...)]` share both their attribute's line and their
    // item, so key the group on the pair — the group id is the first member's node
    // id, which is unique and never shifts. The item alone is not enough:
    // `#[derive(Debug)]` and `#[derive(Clone)]` stacked on one struct are two
    // attributes, and treating them as siblings made expanding one re-shift the
    // other's item end and restore it to stale coordinates on undo.
    let mut derive_groups: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();

    // Create root-level nodes for each macro found
    for mac in macros {
        match mac.kind {
            MacroKind::Attribute => {
                // Built-in attributes are not macros we can expand; they show up
                // as children when a non-built-in sibling is expanded.
                if is_builtin_attribute(&mac.name) {
                    continue;
                }
                if !item_first_attr.insert(mac.item_line_end) {
                    // Already have a top-level attribute macro for this item.
                    continue;
                }
            }
            MacroKind::Derive => {
                // Derives within one `#[derive(..)]` are order-independent: each
                // receives the same item and appends its own output. So every
                // derive gets its own top-level node and the user can expand them
                // in any order — provided no attribute macro has to run first.
                if attr_gated_items.contains(&mac.item_line_end) {
                    continue;
                }
            }
            MacroKind::Functional => {}
        }

        let line_idx = mac.line.saturating_sub(1);
        let effective_end = match mac.kind {
            MacroKind::Attribute => mac.item_line_end,
            MacroKind::Derive | MacroKind::Functional => mac.line_end,
        };
        let line_end_idx = effective_end.saturating_sub(1);
        let original_lines: Vec<String> = source_lines
            .get(line_idx..=line_end_idx.min(source_lines.len().saturating_sub(1)))
            .unwrap_or(&[])
            .to_vec();

        let derive_group = if mac.kind == MacroKind::Derive {
            *derive_groups
                .entry((mac.line, mac.item_line_end))
                .or_insert(next_id)
        } else {
            next_id
        };

        nodes.push(MacroNode {
            call: mac,
            id: next_id,
            derive_group,
            parent_id: None,
            depth: 0,
            expanded: false,
            expansion_failed: false,
            original_lines,
            expanded_content: None,
            children: Vec::new(),
            children_visible: true,
            original_line_origins: Vec::new(),
            tail_relocation_snapshot: Vec::new(),
            consumed_by: None,
            consumed_ids: Vec::new(),
        });
        next_id += 1;
    }

    (nodes, next_id)
}

impl App {
    fn new(
        source: String,
        file_path: PathBuf,
        module_path: Vec<String>,
        expansion_cache: ExpansionCache,
        trace_macros: TraceMacros,
    ) -> Self {
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        let line_origins: Vec<Option<usize>> = (1..=source_lines.len()).map(Some).collect();
        let (nodes, next_id) = build_root_nodes(&source, &source_lines);

        let visible_nodes: Vec<usize> = nodes.iter().map(|n| n.id).collect();
        let list_state = ListState::default();

        let status = format!("Found {} macros.", nodes.len(),);

        let mut app = Self {
            source_lines,
            line_origins,
            nodes,
            next_id,
            expansion_cache,
            visible_nodes,
            selected_idx: 0,
            list_state,
            scroll_offset: 0,
            cursor_line: 1,
            cursor_col: 0,
            source_view_height: 20,
            status,
            error_message: None,
            pending_choice: None,
            needs_full_redraw: false,
            trace_macros,
            file_path,
            module_path,
            module_stack: Vec::new(),
            split_view: false,
        };
        app.sync_selection_to_cursor();
        app
    }

    fn selected_node(&self) -> Option<&MacroNode> {
        self.list_state
            .selected()
            .and_then(|idx| self.visible_nodes.get(idx))
            .and_then(|&id| self.nodes.iter().find(|n| n.id == id))
    }

    fn selected_node_id(&self) -> Option<usize> {
        self.list_state
            .selected()
            .and_then(|idx| self.visible_nodes.get(idx).copied())
    }

    fn get_node(&self, id: usize) -> Option<&MacroNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    fn get_node_mut(&mut self, id: usize) -> Option<&mut MacroNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    fn next(&mut self) {
        if self.visible_nodes.is_empty() {
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % self.visible_nodes.len();
        self.list_state.select(Some(self.selected_idx));
        self.update_scroll();
    }

    fn previous(&mut self) {
        if self.visible_nodes.is_empty() {
            return;
        }
        self.selected_idx = if self.selected_idx == 0 {
            self.visible_nodes.len() - 1
        } else {
            self.selected_idx - 1
        };
        self.list_state.select(Some(self.selected_idx));
        self.update_scroll();
    }

    /// Jump the cursor forward to the next macro invocation in source order
    fn jump_to_next_macro(&mut self) {
        if self.visible_nodes.is_empty() {
            return;
        }
        // Collect macro start lines, sorted ascending
        let mut lines: Vec<usize> = self
            .visible_nodes
            .iter()
            .filter_map(|&nid| self.get_node(nid).map(|n| n.call.line))
            .collect();
        lines.sort();
        lines.dedup();
        // Find the first line strictly after cursor_line
        if let Some(&target) = lines.iter().find(|&&l| l > self.cursor_line) {
            self.cursor_line = target;
            self.ensure_cursor_visible();
            self.snap_cursor_col();
            self.sync_selection_to_cursor();
        }
    }

    /// Jump the cursor backward to the previous macro invocation in source order
    fn jump_to_prev_macro(&mut self) {
        if self.visible_nodes.is_empty() {
            return;
        }
        let mut lines: Vec<usize> = self
            .visible_nodes
            .iter()
            .filter_map(|&nid| self.get_node(nid).map(|n| n.call.line))
            .collect();
        lines.sort();
        lines.dedup();
        // Find the last line strictly before cursor_line
        if let Some(&target) = lines.iter().rev().find(|&&l| l < self.cursor_line) {
            self.cursor_line = target;
            self.ensure_cursor_visible();
            self.snap_cursor_col();
            self.sync_selection_to_cursor();
        }
    }

    /// Move the source cursor up by one line
    fn cursor_up(&mut self) {
        if self.cursor_line > 1 {
            self.cursor_line -= 1;
            self.ensure_cursor_visible();
            self.snap_cursor_col();
            self.sync_selection_to_cursor();
        }
    }

    /// Move the source cursor down by one line
    fn cursor_down(&mut self) {
        if self.cursor_line < self.source_lines.len() {
            self.cursor_line += 1;
            self.ensure_cursor_visible();
            self.snap_cursor_col();
            self.sync_selection_to_cursor();
        }
    }

    /// Ensure the cursor line is visible in the scroll viewport.
    ///
    /// `scroll_offset` stays in source-line space (the renderer converts it), but the
    /// viewport height is measured in *display* rows, which split blocks inflate. So
    /// the comparisons are done in display space, otherwise the cursor drifts off the
    /// bottom of a split region.
    fn ensure_cursor_visible(&mut self) {
        let view_h = self.source_view_height.saturating_sub(2) as usize; // account for borders
        if view_h == 0 {
            return;
        }
        let total = self.source_lines.len();
        // Only each region's geometry matters here. Collecting it into plain tuples
        // ends the borrow of `self` that the regions hold, so `scroll_offset` can be
        // assigned below.
        let regions: Vec<(usize, usize, usize)> = self
            .split_regions()
            .iter()
            .map(|r| (r.start, r.len, r.rows()))
            .collect();
        let display_row_of = |source_idx: usize| -> usize {
            source_idx
                + regions
                    .iter()
                    .filter(|&&(start, len, _)| start + len <= source_idx)
                    .map(|&(_, len, rows)| (rows + 2).saturating_sub(len))
                    .sum::<usize>()
        };

        let cursor_idx = self.cursor_line.saturating_sub(1);
        let cur_disp = display_row_of(cursor_idx);
        let top_disp = display_row_of(self.scroll_offset);

        if cur_disp < top_disp {
            self.scroll_offset = cursor_idx;
        } else if cur_disp >= top_disp + view_h {
            // Scroll down just enough to bring the cursor row into view.
            let want = cur_disp + 1 - view_h;
            let idx = (0..total).find(|&i| display_row_of(i) >= want).unwrap_or(0);
            self.scroll_offset = idx;
        }

        // Clamp scroll so the viewport doesn't extend past the last row.
        let total_disp = display_row_of(total);
        let max_disp = total_disp.saturating_sub(view_h);
        let max_scroll = (0..total)
            .find(|&i| display_row_of(i) >= max_disp)
            .unwrap_or(0);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
    }

    /// The column range a node occupies on `line`, if it has a meaningful one there.
    /// Only the node's starting line carries a column range; on continuation lines
    /// the whole line belongs to the node.
    fn node_col_span(node: &MacroNode, line: usize) -> Option<(usize, usize)> {
        let own_line = if node.call.kind == MacroKind::Derive {
            node.call.derive_line
        } else {
            node.call.line
        };
        if own_line != line {
            return None;
        }
        let end = if node.call.line_end == own_line || node.call.kind == MacroKind::Derive {
            node.call.col_end
        } else {
            // Multi-line invocation: it owns the rest of its first line.
            usize::MAX
        };
        Some((node.call.col_start, end.max(node.call.col_start + 1)))
    }

    /// The cursor position that puts the source cursor on `node` itself: its own line
    /// (`derive_line` for derives — the line `node_col_span` keys their span on, which
    /// differs from `call.line` in a multi-line `#[derive(...)]` list) and the start of
    /// its column span there.
    fn node_cursor_pos(node: &MacroNode) -> (usize, usize) {
        let own_line = if node.call.kind == MacroKind::Derive {
            node.call.derive_line
        } else {
            node.call.line
        };
        let col = Self::node_col_span(node, own_line)
            .map(|(start, _)| start)
            .unwrap_or(node.call.col_start);
        (own_line, col)
    }

    /// Visible-node indices of every macro that starts on `line`, ordered by column.
    /// This is what Left/Right steps through.
    fn macros_on_line(&self, line: usize) -> Vec<(usize, usize)> {
        let mut v: Vec<(usize, usize)> = self
            .visible_nodes
            .iter()
            .enumerate()
            .filter_map(|(i, &nid)| {
                let node = self.get_node(nid)?;
                let (start, _) = Self::node_col_span(node, line)?;
                Some((i, start))
            })
            .collect();
        v.sort_by_key(|&(i, col)| (col, i));
        v
    }

    /// Sync the macro list selection to the macro under the cursor (if any).
    /// If the cursor is not on any macro, deselect.
    ///
    /// Selection is column-aware: when several macros share a line — the derives of
    /// one `#[derive(A, B, C)]`, or `a!(); b!();` — the one whose column range covers
    /// `cursor_col` wins, so the user can address each independently. Ties fall back
    /// to the deepest node (a child nested inside a parent's range), then the
    /// narrowest span.
    fn sync_selection_to_cursor(&mut self) {
        let nodes: Vec<&MacroNode> = self
            .visible_nodes
            .iter()
            .filter_map(|&nid| self.get_node(nid))
            .collect();
        match Self::pick_node_at(&nodes, self.cursor_line, self.cursor_col) {
            Some(idx) => {
                self.selected_idx = idx;
                self.list_state.select(Some(idx));
            }
            None => {
                self.list_state.select(None);
            }
        }
    }

    /// Index into `nodes` of the macro under `(line, col)`, or `None` if there is
    /// none. Pure so the selection rules can be tested directly.
    fn pick_node_at(nodes: &[&MacroNode], line: usize, col: usize) -> Option<usize> {
        // (visible_idx, covers_column, depth, span_width)
        let mut best: Option<(usize, bool, usize, usize)> = None;
        for (i, node) in nodes.iter().enumerate() {
            let start = node.call.line;
            let end = if node.expanded {
                let num = node
                    .expanded_content
                    .as_ref()
                    .map(|c| c.lines().count())
                    .unwrap_or(1);
                start + num - 1
            } else {
                match node.call.kind {
                    MacroKind::Attribute => node.call.item_line_end,
                    _ => node.call.line_end,
                }
            };
            if line < start || line > end {
                continue;
            }
            let (covers, width) = match Self::node_col_span(node, line) {
                Some((cs, ce)) => (col >= cs && col < ce, ce.saturating_sub(cs)),
                None => (false, usize::MAX),
            };
            let better = match best {
                None => true,
                Some((_, b_covers, b_depth, b_width)) => {
                    (covers, node.depth, std::cmp::Reverse(width))
                        > (b_covers, b_depth, std::cmp::Reverse(b_width))
                }
            };
            if better {
                best = Some((i, covers, node.depth, width));
            }
        }
        best.map(|(i, ..)| i)
    }

    /// Move the cursor to the previous/next macro on the current line. Returns false
    /// when there is nothing to move to, so the caller can fall back to line movement.
    fn cursor_horizontal(&mut self, forward: bool) -> bool {
        let on_line = self.macros_on_line(self.cursor_line);
        if on_line.len() < 2 {
            return false;
        }
        let cur = on_line
            .iter()
            .position(|&(i, _)| Some(i) == self.list_state.selected());
        let next = match (cur, forward) {
            (Some(p), true) if p + 1 < on_line.len() => p + 1,
            (Some(p), false) if p > 0 => p - 1,
            (Some(_), _) => return false,
            (None, true) => 0,
            (None, false) => on_line.len() - 1,
        };
        self.cursor_col = on_line[next].1;
        self.sync_selection_to_cursor();
        true
    }

    /// Toggle the side-by-side comparison of expanded ranges.
    fn toggle_split_view(&mut self) {
        self.split_view = !self.split_view;
        let n = self.split_regions().len();
        self.status = if !self.split_view {
            "Inline view.".to_string()
        } else if n == 0 {
            "Split view: expand a macro to compare it with the original.".to_string()
        } else {
            format!("Split view: comparing {} expanded range(s).", n)
        };
        self.ensure_cursor_visible();
    }

    /// The expanded ranges to render side by side, in source order and never
    /// overlapping. Only the outermost expansion of a nest produces a region: a child
    /// expanded inside a parent's output already sits within the parent's range, and
    /// splitting it again would nest columns inside columns.
    fn split_regions(&self) -> Vec<SplitRegion<'_>> {
        if !self.split_view {
            return Vec::new();
        }
        // A node swallowed by an enclosing expansion keeps `expanded` (so undoing the
        // consumer brings it back as it was), but its coordinates point into text
        // that is not on screen; a region built from them would overlap the consumer's.
        let regions: Vec<SplitRegion> = self
            .nodes
            .iter()
            .filter(|n| n.expanded && n.consumed_by.is_none())
            .filter_map(|n| {
                n.expanded_content.as_ref()?;
                let start = n.call.line.saturating_sub(1);
                // Not `expanded_content.lines().count()`: a child macro expanded
                // inside this node's output grows the range it occupies, and a stale
                // length here makes the row accounting below underflow.
                let len = self.actual_expanded_line_count(n.id).max(1);
                let right = (start..start + len)
                    .filter_map(|idx| {
                        let text = self.source_lines.get(idx)?;
                        (!is_expansion_marker(text)).then_some((idx, text.as_str()))
                    })
                    .collect();
                Some(SplitRegion {
                    start,
                    len,
                    original: &n.original_lines,
                    right,
                    name: &n.call.name,
                })
            })
            .collect();
        keep_outermost(regions)
    }

    /// How far PageUp/PageDown move: one screenful, less a line of overlap so the
    /// reader keeps some context. Never zero, or the page keys become no-ops.
    fn page_step(&self) -> usize {
        (self.source_view_height.saturating_sub(3) as usize).max(1)
    }

    /// Place the column cursor on the first macro of the current line (if any), so
    /// that vertical movement always lands on a selectable macro.
    fn snap_cursor_col(&mut self) {
        if let Some(&(_, col)) = self.macros_on_line(self.cursor_line).first() {
            self.cursor_col = col;
        } else {
            self.cursor_col = 0;
        }
    }

    fn update_scroll(&mut self) {
        if let Some(node) = self.selected_node() {
            // Park the cursor on the selected node itself — line *and* column. Leaving
            // `cursor_col` behind meant the next j/k re-snapped it to the first macro on
            // the line and silently changed the selection; and a derive in a multi-line
            // `#[derive(...)]` list keys its span on `derive_line`, so parking on
            // `call.line` left the selection with no highlighted span in the source.
            let (own_line, col) = Self::node_cursor_pos(node);
            self.cursor_line = own_line;
            self.cursor_col = col;
            self.ensure_cursor_visible();
        }
    }

    /// Rebuild the visible_nodes list based on tree structure and visibility
    fn rebuild_visible_nodes(&mut self) {
        self.visible_nodes.clear();

        // Collect root nodes (no parent)
        let root_ids: Vec<usize> = self
            .nodes
            .iter()
            .filter(|n| n.parent_id.is_none() && n.consumed_by.is_none())
            .map(|n| n.id)
            .collect();

        // DFS traversal to build visible list
        for root_id in root_ids {
            self.collect_visible_nodes(root_id);
        }

        // Adjust selection if needed
        if self.selected_idx >= self.visible_nodes.len() {
            self.selected_idx = self.visible_nodes.len().saturating_sub(1);
        }
        self.list_state.select(if self.visible_nodes.is_empty() {
            None
        } else {
            Some(self.selected_idx)
        });
    }

    fn collect_visible_nodes(&mut self, node_id: usize) {
        // Get children and visibility
        let (children, children_visible) = {
            let node = self.nodes.iter().find(|n| n.id == node_id);
            match node {
                // A consumed child is as gone as a consumed root: its text is inside
                // another expansion's output. Listing it let Tab and `n` select it
                // and expand it at coordinates that slice that output.
                Some(n) if n.consumed_by.is_some() => return,
                Some(n) => (n.children.clone(), n.children_visible),
                None => return,
            }
        };
        self.visible_nodes.push(node_id);

        if children_visible {
            for child_id in children {
                self.collect_visible_nodes(child_id);
            }
        }
    }

    /// Relocate the macros that sat in the lifted trailing fragment of an expanded call
    /// onto the fragment's new line, rebasing their columns onto its text.
    ///
    /// `tail_src_line` is the line the fragment was cut from — the call's own line for a
    /// single-line invocation, its end line for a multi-line one. `tail_line` is the
    /// 1-indexed line the fragment now occupies (the last formatted line of the
    /// expansion), and `tail_text` that line's text. Returns the ids of the nodes moved,
    /// so the caller's generic shift loop can skip them: `tail_line` is already their
    /// final position.
    fn relocate_tail_nodes(
        nodes: &mut [MacroNode],
        source_lines: &[String],
        expanded_id: usize,
        tail_src_line: usize,
        tail_start_col: usize,
        indent: usize,
        tail_line: usize,
    ) -> Vec<RelocatedNode> {
        // Shift the whole span rather than pinning every line to `tail_line`: a
        // relocated macro can itself be multi-line, and collapsing its end onto its
        // start left its continuation lines outside it. The delta equals the
        // expansion's net line change, so this agrees with the generic shift loop
        // these nodes are excluded from.
        let delta = tail_line as isize - tail_src_line as isize;
        let shift = |v: usize| (v as isize + delta).max(1) as usize;
        let anchor_line = nodes
            .iter()
            .find(|n| n.id == expanded_id)
            .map(|n| n.call.line)
            .unwrap_or(tail_src_line);

        let mut relocated = Vec::new();
        for node in nodes.iter_mut() {
            // An already-expanded node's `call.line` points at its own marker line and
            // its `original_lines` hold the text it replaced. Rebasing either would
            // overwrite that with marker text and corrupt its undo. Skipping it here
            // leaves it to the generic shift loop, which moves it by the same delta.
            if node.id != expanded_id
                && !node.expanded
                && node.call.line == tail_src_line
                && node.call.col_start >= tail_start_col
            {
                let before = RelocatedNode {
                    id: node.id,
                    anchor_line,
                    line: node.call.line,
                    line_end: node.call.line_end,
                    derive_line: node.call.derive_line,
                    item_line_end: node.call.item_line_end,
                    col_start: node.call.col_start,
                    col_end: node.call.col_end,
                    original_lines: node.original_lines.clone(),
                };
                node.call.line = shift(node.call.line);
                node.call.line_end = shift(node.call.line_end);
                node.call.derive_line = shift(node.call.derive_line);
                node.call.item_line_end = shift(node.call.item_line_end);
                // Columns only describe the span's first line, which is the lifted
                // fragment; the continuation lines keep their own text.
                node.call.col_start = rebase_col(node.call.col_start, tail_start_col, indent);
                node.call.col_end = rebase_col(node.call.col_end, tail_start_col, indent);
                let first = node.call.line.saturating_sub(1);
                let last = node
                    .call
                    .line_end
                    .saturating_sub(1)
                    .max(first)
                    .min(source_lines.len().saturating_sub(1));
                node.original_lines = source_lines.get(first..=last).unwrap_or(&[]).to_vec();
                relocated.push(before);
            }
        }
        relocated
    }

    /// Move the highlight in the ambiguity popup, clamped at both ends.
    fn choice_move(&mut self, delta: isize) {
        if let Some(choice) = &mut self.pending_choice {
            let last = choice.candidates.len().saturating_sub(1);
            let next = choice.selected as isize + delta;
            choice.selected = next.clamp(0, last as isize) as usize;
        }
    }

    /// Expand using the highlighted candidate.
    fn choice_confirm(&mut self) {
        let Some(choice) = self.pending_choice.take() else {
            return;
        };
        let Some(candidate) = choice.candidates.get(choice.selected).cloned() else {
            return;
        };
        self.expand_node(choice.node_id, Some(candidate.output));
    }

    /// Dismiss the popup without expanding.
    fn choice_cancel(&mut self) {
        if let Some(choice) = self.pending_choice.take() {
            self.status = format!("Expansion of '{}' cancelled.", choice.name);
        }
    }

    /// Expand the currently selected macro
    fn expand_selected(&mut self) {
        let node_id = match self.selected_node_id() {
            Some(id) => id,
            None => {
                self.status = "No macro selected".to_string();
                return;
            }
        };
        self.expand_node(node_id, None);
    }

    /// Expand `node_id`, either resolving the trace now or using one the user already
    /// picked out of an ambiguity popup.
    fn expand_node(&mut self, node_id: usize, chosen: Option<String>) {
        // Get node info
        let (
            name,
            krate,
            input,
            arguments,
            kind,
            line,
            col_start,
            col_end,
            line_end,
            item_line_end,
            depth,
            already_expanded,
            sibling_derives,
            derive_group,
        ) = {
            let node = match self.get_node(node_id) {
                Some(n) => n,
                None => return,
            };
            (
                node.call.name.clone(),
                node.call.krate.clone(),
                node.call.input.clone(),
                node.call.arguments.clone(),
                node.call.kind,
                node.call.line,
                node.call.col_start,
                node.call.col_end,
                node.call.line_end,
                node.call.item_line_end,
                node.depth,
                node.expanded,
                node.call.sibling_derives.clone(),
                node.derive_group,
            )
        };

        if already_expanded {
            self.status = format!("'{}' already expanded. Press Enter to undo.", name);
            return;
        }

        // Find the matching trace. This waits on the background stream, so poll for
        // a cancel key while it runs — see `find_trace_for_tokens`.
        // `should_abort` is only reached once the lookup has actually had to wait, so a
        // trace that is already cached still expands without any flicker.
        let lookup = match chosen {
            // Resuming from the ambiguity popup: the user already picked.
            Some(text) => TraceLookup::Found(text),
            None => {
                // `Cell` so the closure can record that it painted without borrowing
                // `self`, which the lookup call already borrows.
                let notified = std::cell::Cell::new(false);
                let mut wait = || {
                    if !notified.get() {
                        notified.set(true);
                        draw_wait_notice(&name);
                    }
                    wait_cancelled()
                };
                let lookup = self
                    .expansion_cache
                    .find_trace_for_tokens(&input, &arguments, &name, &krate, kind, &mut wait);
                // The notice went straight to the terminal, so ratatui's buffer does
                // not know that row changed and its diff would leave it on screen.
                if notified.get() {
                    self.needs_full_redraw = true;
                }
                lookup
            }
        };
        let expanded_text = match lookup {
            TraceLookup::Found(text) => text,
            TraceLookup::Ambiguous(candidates) => {
                // Several traces match this invocation and expand differently, and
                // nothing in the trace says which one is this call site's. Guessing
                // (the old rotating pointer did) silently shows the wrong expansion,
                // so ask instead.
                self.status = format!(
                    "'{}' matches {} different expansions — pick one.",
                    name,
                    candidates.len()
                );
                self.pending_choice = Some(PendingChoice {
                    node_id,
                    name: name.clone(),
                    candidates,
                    selected: 0,
                });
                return;
            }
            TraceLookup::Aborted => {
                self.status = format!("Expansion of '{}' cancelled.", name);
                return;
            }
            TraceLookup::Exhausted => {
                // Mark as failed and show error
                if let Some(node) = self.get_node_mut(node_id) {
                    node.expansion_failed = true;
                }

                // For derive macros with siblings, try the next sibling
                if kind == MacroKind::Derive && sibling_derives.len() > 1 {
                    let next_sibling_id = self
                        .nodes
                        .iter()
                        .find(|n| {
                            n.call.kind == MacroKind::Derive
                                && n.call.line == line
                                && n.id != node_id
                                && !n.expanded
                                && !n.expansion_failed
                        })
                        .map(|n| n.id);
                    if let Some(next_id) = next_sibling_id {
                        // Select the next sibling derive in the visible nodes list
                        if let Some(vis_idx) =
                            self.visible_nodes.iter().position(|&id| id == next_id)
                        {
                            self.list_state.select(Some(vis_idx));
                            self.status = format!("'{}' failed, trying next derive...", name);
                            self.expand_selected();
                            return;
                        }
                    }
                }

                // Check for stream error
                if let Some(err) = self.expansion_cache.take_error() {
                    self.error_message = Some(format!(
                        "Expansion Stream Error\n\n{}\n\nPress Enter to dismiss.",
                        err
                    ));
                    return;
                }

                // Check if the build failed — that explains why the trace is missing.
                if let Some(build_err) = self.expansion_cache.build_error() {
                    self.error_message = Some(format!(
                        "Build Error: cargo check failed, so '{}' was not expanded.\n\n\
                         {}\n\n\
                         Press Enter to dismiss.",
                        name, build_err,
                    ));
                    return;
                }

                // Write diagnostic log file
                let log_info = match self
                    .expansion_cache
                    .write_error_log(&name, kind, &input, &arguments)
                {
                    Some(path) => format!("Log: {}", path.display()),
                    None => "Failed to write log file.".to_string(),
                };
                self.error_message = Some(format!(
                    "Expansion Error: No trace found for '{}' (type: {})\n\n\
                     {}\n\n\
                     Press Enter to dismiss.",
                    name,
                    kind.as_str(),
                    log_info,
                ));
                return;
            }
        };

        // Traces come back as a single line of `TokenStream`-style text; run it
        // through the pretty-printer so the inlined expansion is readable.
        let expanded_text = pretty::format_source(&expanded_text);

        let line_idx = line.saturating_sub(1);

        // Get indentation from original line
        let base_indent = if line_idx < self.source_lines.len() {
            let orig = &self.source_lines[line_idx];
            let trimmed_len = orig.trim_start().len();
            orig.len() - trimmed_len
        } else {
            0
        };
        let base_indent_str: String = " ".repeat(base_indent);

        // Find the minimum indentation in the expanded text (to preserve relative indentation)
        let min_indent = expanded_text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        // Format expanded text with proper indentation (preserving relative indentation)
        let content_lines: Vec<String> = expanded_text
            .lines()
            .map(|l| {
                if l.trim().is_empty() {
                    String::new()
                } else {
                    let current_indent = l.len() - l.trim_start().len();
                    let relative_indent = current_indent.saturating_sub(min_indent);
                    format!(
                        "{}{}{}",
                        base_indent_str,
                        " ".repeat(relative_indent),
                        l.trim()
                    )
                }
            })
            .collect();

        // For derive macros: compute remaining derives (siblings not yet expanded)
        let remaining_derives: Vec<String> = if kind == MacroKind::Derive {
            sibling_derives
                .iter()
                .filter(|d| {
                    if *d == &name {
                        return false;
                    }
                    let sibling_node = self.nodes.iter().find(|n| {
                        n.call.kind == MacroKind::Derive
                            && n.call.name == **d
                            && n.derive_group == derive_group
                            && n.id != node_id
                    });
                    match sibling_node {
                        Some(n) => !n.expanded,
                        None => true,
                    }
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        // Set when a trailing fragment of an expanded line is moved onto its own line,
        // as (source line the fragment came from, first character column of the
        // fragment on that line, indent of the new line). The fragment comes from
        // `line` for a single-line call and from `line_end` for a multi-line one.
        // Other macros that sat in that fragment have to follow it.
        let mut tail_rebase: Option<(usize, usize, usize)> = None;
        // The item text the derive branch rescued from the attribute's own line, when
        // an attribute and its item share a line. Non-empty shifts where the remaining
        // `#[derive(...)]` line ends up, and it has to reach child discovery too.
        let mut derive_tail = String::new();

        // For functional macros, replace only the macro call, keeping surrounding code
        let (formatted_lines, lines_removed) = if kind == MacroKind::Functional && line == line_end
        {
            // Single-line functional macro: replace only the macro call
            let orig_line = self.source_lines.get(line_idx).cloned().unwrap_or_default();

            // Extract parts before and after the macro call
            // `col_start`/`col_end` are character columns, so they have to be
            // converted before they can index into the line's bytes.
            let (before_macro, _, after_macro) = split_at_cols(&orig_line, col_start, col_end);
            let trimmed = after_macro.trim_start();
            if !trimmed.is_empty() {
                // `trim_start` skipped this many characters past `col_end`.
                let skipped = after_macro.chars().count() - trimmed.chars().count();
                tail_rebase = Some((line, col_end + skipped, base_indent));
            }
            let after_macro = trimmed;

            let mut lines = Vec::new();
            // Line with code before macro and start marker
            lines.push(format!("{}// -- expanded: {} --", before_macro, name));
            // Expanded content
            lines.extend(content_lines.clone());
            // End marker
            lines.push(format!("{}// -- end {} --", base_indent_str, name));
            // Code after macro on its own line (if non-empty)
            if !after_macro.is_empty() {
                lines.push(format!("{}{}", base_indent_str, after_macro));
            }

            (lines, 1) // removing 1 original line
        } else if kind == MacroKind::Functional && line != line_end {
            // Multi-line functional macro
            let orig_line = self.source_lines.get(line_idx).cloned().unwrap_or_default();
            let end_line_idx = line_end.saturating_sub(1);
            let end_orig_line = self
                .source_lines
                .get(end_line_idx)
                .cloned()
                .unwrap_or_default();

            // Character columns again — see the single-line branch above.
            let before_macro = &orig_line[..byte_of_col(&orig_line, col_start)];
            let after_macro = &end_orig_line[byte_of_col(&end_orig_line, col_end)..];
            let trimmed = after_macro.trim_start();
            if !trimmed.is_empty() {
                // The tail is lifted off the macro's *end* line, so macros that have to
                // follow it sit on `line_end`, past `col_end` plus the whitespace
                // `trim_start` skipped.
                let skipped = after_macro.chars().count() - trimmed.chars().count();
                tail_rebase = Some((line_end, col_end + skipped, base_indent));
            }
            let after_macro = trimmed;

            let mut lines = Vec::new();
            lines.push(format!("{}// -- expanded: {} --", before_macro, name));
            lines.extend(content_lines.clone());
            lines.push(format!("{}// -- end {} --", base_indent_str, name));
            if !after_macro.is_empty() {
                lines.push(format!("{}{}", base_indent_str, after_macro));
            }

            let num_lines_removed = line_end - line + 1;
            (lines, num_lines_removed)
        } else if kind == MacroKind::Derive {
            // Derive macros: replace only the #[derive(...)] attribute line,
            // keep the item intact below.
            let mut lines = Vec::new();
            lines.push(format!("{}// -- expanded: {} --", base_indent_str, name));
            lines.extend(content_lines.clone());
            lines.push(format!("{}// -- end {} --", base_indent_str, name));

            if !remaining_derives.is_empty() {
                lines.push(format!(
                    "{}#[derive({})]",
                    base_indent_str,
                    remaining_derives.join(", ")
                ));
            }

            // `#[derive(Debug)] struct S;` puts the item on the attribute's own line,
            // and that whole line is about to be removed. Carry the item text over, or
            // it disappears from the view until the expansion is undone.
            let last_removed = self
                .source_lines
                .get(line_end.saturating_sub(1))
                .cloned()
                .unwrap_or_default();
            // A list spanning several lines — `#[derive(\n    A,\n)] struct S;` — closes
            // on a line that has no `#[` of its own, so the attribute's own start is no
            // help there; the item is whatever follows the bracket that closes it.
            let tail = if line == line_end {
                attribute_tail(&last_removed, col_start)
            } else {
                attribute_close_tail(&last_removed)
            };
            if !tail.is_empty() {
                lines.push(format!("{}{}", base_indent_str, tail));
                derive_tail = tail;
            }

            // Only remove the #[derive(...)] attribute line(s), NOT the item
            let num_lines_removed = line_end - line + 1;
            (lines, num_lines_removed)
        } else {
            // Attribute macros: replace attribute + entire target item
            let mut lines = Vec::new();
            lines.push(format!("{}// -- expanded: {} --", base_indent_str, name));
            lines.extend(content_lines.clone());
            lines.push(format!("{}// -- end {} --", base_indent_str, name));

            let num_lines_removed = item_line_end - line + 1;
            (lines, num_lines_removed)
        };

        // expanded_content stores full output including markers (for correct undo line count)
        // content_for_parsing is just the code (for finding child macros)
        // For derive macros, include expansion + remaining derives + remaining attrs + item
        // so that find_macros() discovers remaining derives and other attrs as children.
        // Build content_for_parsing: the text given to find_macros() to discover
        // child macros. For derive macros we must include the end marker so that
        // child line numbers align with source_lines (which also has the marker).
        let content_for_parsing = if kind == MacroKind::Derive {
            let mut parts = content_lines.clone();
            // Include the end marker so line offsets match source_lines
            parts.push(format!("{}// -- end {} --", base_indent_str, name));
            // Add the remaining #[derive(remaining)] line if any
            if !remaining_derives.is_empty() {
                parts.push(format!(
                    "{}#[derive({})]",
                    base_indent_str,
                    remaining_derives.join(", ")
                ));
            }
            // The item rescued off the attribute's own line is real code in the buffer
            // now, so child macros inside it have to be discoverable — and it has to
            // sit here in the same position as in `formatted_lines` for child line
            // offsets to line up.
            if !derive_tail.is_empty() {
                parts.push(format!("{}{}", base_indent_str, derive_tail));
            }
            // Add remaining source lines between the attribute and the item end
            // (other attrs + the item body itself)
            let after_attr_idx = line_end; // line_end (1-indexed) used as 0-index = next line
            let item_end_idx = item_line_end.saturating_sub(1);
            for idx in after_attr_idx..=item_end_idx.min(self.source_lines.len().saturating_sub(1))
            {
                parts.push(self.source_lines[idx].clone());
            }
            parts.join("\n")
        } else {
            content_lines.join("\n")
        };
        let expanded_content = formatted_lines.join("\n");
        let num_expanded_lines = formatted_lines.len();
        let lines_added = (num_expanded_lines as isize) - (lines_removed as isize);

        // Snapshot the gutter entries about to be replaced, so undo can put back
        // exactly what was there (see `original_line_origins`).
        let replaced_origins: Vec<Option<usize>> = self
            .line_origins
            .get(line_idx..(line_idx + lines_removed).min(self.line_origins.len()))
            .unwrap_or(&[])
            .to_vec();
        // Likewise the text. The `original_lines` taken when the node was discovered
        // predate any expansion made *inside* its range since — a `println!` expanded
        // inside a `#[tokio::main] fn` — so undoing the attribute would put back the
        // pre-`println!` text while the `println!` node still claims to be expanded.
        // What undo has to restore is exactly what this expansion removes.
        let replaced_lines: Vec<String> = self
            .source_lines
            .get(line_idx..(line_idx + lines_removed).min(self.source_lines.len()))
            .unwrap_or(&[])
            .to_vec();

        // Update source and line_origins: replace the lines with expanded lines
        if line_idx < self.source_lines.len() {
            // Remove original lines from both source_lines and line_origins
            for _ in 0..lines_removed {
                if line_idx < self.source_lines.len() {
                    self.source_lines.remove(line_idx);
                    self.line_origins.remove(line_idx);
                }
            }
            // Insert new lines — all expanded lines have no original line number
            for (i, formatted_line) in formatted_lines.iter().enumerate() {
                self.source_lines
                    .insert(line_idx + i, formatted_line.clone());
                self.line_origins.insert(line_idx + i, None);
            }
        }

        // Macros that sat *after* the expanded call on a removed line moved with the
        // trailing fragment onto its own line — the fragment of `line` itself for a
        // single-line call, the fragment of `line_end` for a multi-line one. Without
        // this they keep pointing at a line that is now expansion output (or gone
        // entirely), and expanding one of them would slice that text at stale columns.
        // The relocated ids are excluded from the generic shift loop below: `tail_line`
        // is already their final position, and shifting them again would overshoot by
        // `lines_added`.
        let mut tail_relocation: Vec<RelocatedNode> = Vec::new();
        if let Some((tail_src_line, tail_start_col, indent)) = tail_rebase {
            let tail_line = line_idx + num_expanded_lines; // 1-indexed
            tail_relocation = Self::relocate_tail_nodes(
                &mut self.nodes,
                &self.source_lines,
                node_id,
                tail_src_line,
                tail_start_col,
                indent,
                tail_line,
            );
        }

        // Nodes whose own text sat inside the range this expansion replaced describe
        // lines that no longer exist. Shifting them is meaningless and, when the
        // expansion is shorter than what it replaced (an attribute macro that strips
        // its item), the negative shift wrapped their line numbers past `usize::MAX` —
        // after which `n` threw the cursor outside the buffer. Hide them instead; the
        // expansion re-discovers whatever survived as children, and undo brings them
        // back as they were — an expanded one included (see `consumed_by`).
        //
        // The first line is the one exception: a functional call keeps the text before
        // it on the marker line, so a macro there is untouched, and one after it is
        // relocated with the tail. An attribute or derive expansion replaces that line
        // whole, so `#[my_attr] fn f() { q!(1) }` loses `q` too — left visible, it
        // pointed into `// -- expanded: my_attr --` and expanding it corrupted that
        // line. The same rule swallows the other derives of a `#[derive(A, B)]`: the
        // remaining `#[derive(B)]` line is part of this expansion's output, and the
        // child discovery below finds `B` there with the right columns. Moving the
        // root `B` onto that line as well produced two `B` nodes, one of them with
        // its old columns.
        let mut consumed_ids: Vec<usize> = Vec::new();
        let removed_end = line + lines_removed;
        for node in &mut self.nodes {
            let inside = node.call.line > line && node.call.line < removed_end
                || node.call.line == line && kind != MacroKind::Functional;
            if node.id != node_id
                && node.consumed_by.is_none()
                && inside
                && !tail_relocation.iter().any(|r| r.id == node.id)
            {
                node.consumed_by = Some(node_id);
                consumed_ids.push(node.id);
            }
        }

        // Move everything after the replaced range, and grow the item of an attribute
        // that encloses it. The relocated nodes are already at their final position.
        let relocated_ids: Vec<usize> = tail_relocation.iter().map(|r| r.id).collect();
        shift_nodes(
            &mut self.nodes,
            node_id,
            line,
            removed_end - 1,
            lines_added,
            &relocated_ids,
        );

        // Mark node as expanded and store expanded content
        if let Some(node) = self.get_node_mut(node_id) {
            node.expanded = true;
            // A previous attempt may have failed because the trace had not streamed in
            // yet. Leaving the flag set kept the `!` marker on a node that is now
            // expanded, and permanently excluded it from the derive-sibling retry.
            node.expansion_failed = false;
            node.expanded_content = Some(expanded_content.clone());
            node.original_lines = replaced_lines;
            node.original_line_origins = replaced_origins;
            node.tail_relocation_snapshot = tail_relocation;
            node.consumed_ids = consumed_ids;
            node.children_visible = true;
        }

        // Parse expanded content to find child macros (use content without markers).
        // Replace $crate with a valid identifier so syn::parse_file() can parse
        // expanded macro output that contains $crate tokens.
        let content_for_parsing_safe =
            content_for_parsing.replace("$crate", DOLLAR_CRATE_PLACEHOLDER);
        let child_macros = find_macros(&content_for_parsing_safe);
        // The columns `find_macros` reports index the substituted text, but the
        // buffer holds the real `$crate`, which is shorter than the placeholder; a
        // child after a `$crate` on its line has to be mapped back or its span is
        // skewed by the difference — the highlight sat on the wrong text and
        // expanding it left the invocation in place. `input`, `arguments` and `krate`
        // are mapped back below for the same reason.
        let safe_lines: Vec<&str> = content_for_parsing_safe.lines().collect();
        let real_col = |line_no: usize, col: usize| {
            safe_lines
                .get(line_no.saturating_sub(1))
                .map_or(col, |text| unsubstituted_col(text, col))
        };

        // Create child nodes
        let mut child_ids = Vec::new();
        // Same grouping as `build_root_nodes`, over the children of this expansion.
        let mut child_derive_groups: std::collections::HashMap<(usize, usize), usize> =
            std::collections::HashMap::new();
        for child_mac in child_macros {
            let child_id = self.next_id;
            self.next_id += 1;
            let child_derive_group = if child_mac.kind == MacroKind::Derive {
                *child_derive_groups
                    .entry((child_mac.line, child_mac.item_line_end))
                    .or_insert(child_id)
            } else {
                child_id
            };
            // A derive's columns sit on `derive_line`; anything else spans from its
            // first line to its last.
            let (col_line, col_end_line) = if child_mac.kind == MacroKind::Derive {
                (child_mac.derive_line, child_mac.derive_line)
            } else {
                (child_mac.line, child_mac.line_end)
            };

            // Adjust child line number: account for the marker comment line at the start
            let adjusted_line = line + child_mac.line; // +1 for marker, child_mac.line is 1-based

            let child_line_start = child_mac.line.saturating_sub(1);
            // For attr child macros, use item_line_end for original_lines span
            let child_effective_end = match child_mac.kind {
                MacroKind::Attribute => child_mac.item_line_end,
                MacroKind::Derive | MacroKind::Functional => child_mac.line_end,
            };
            let child_line_end_idx = child_effective_end.saturating_sub(1);
            let child_original_lines: Vec<String> = content_for_parsing
                .lines()
                .skip(child_line_start)
                .take(child_line_end_idx - child_line_start + 1)
                .map(|s| s.to_string())
                .collect();

            self.nodes.push(MacroNode {
                derive_group: child_derive_group,
                call: MacroCall {
                    name: child_mac.name,
                    // `$crate::m!()` parses as a path rooted at the placeholder, which
                    // would otherwise be reported as a crate literally named
                    // `__macra_dollar_crate__`. `$crate` names the defining crate,
                    // which the call site cannot know, so it qualifies nothing.
                    krate: if child_mac.krate == DOLLAR_CRATE_PLACEHOLDER {
                        String::new()
                    } else {
                        child_mac.krate
                    },
                    kind: child_mac.kind,
                    line: adjusted_line,
                    col_start: real_col(col_line, child_mac.col_start),
                    col_end: real_col(col_end_line, child_mac.col_end),
                    derive_line: adjusted_line + (child_mac.derive_line - child_mac.line),
                    line_end: adjusted_line + (child_mac.line_end - child_mac.line),
                    item_line_end: adjusted_line + (child_mac.item_line_end - child_mac.line),
                    input: child_mac.input.replace(DOLLAR_CRATE_PLACEHOLDER, "$crate"),
                    arguments: child_mac
                        .arguments
                        .replace(DOLLAR_CRATE_PLACEHOLDER, "$crate"),
                    sibling_derives: child_mac.sibling_derives,
                },
                id: child_id,
                parent_id: Some(node_id),
                depth: depth + 1,
                expanded: false,
                expansion_failed: false,
                original_lines: child_original_lines,
                expanded_content: None,
                children: Vec::new(),
                children_visible: true,
                original_line_origins: Vec::new(),
                tail_relocation_snapshot: Vec::new(),
                consumed_by: None,
                consumed_ids: Vec::new(),
            });

            child_ids.push(child_id);
        }

        // Update parent's children list
        if let Some(node) = self.get_node_mut(node_id) {
            node.children = child_ids.clone();
        }

        self.rebuild_visible_nodes();

        self.status = format!(
            "Expanded '{}' -> {} child macros found",
            name,
            child_ids.len()
        );
    }

    /// Compute the actual number of lines a node currently occupies in source_lines,
    /// accounting for expanded children that may have added extra lines.
    fn actual_expanded_line_count(&self, node_id: usize) -> usize {
        let node = match self.get_node(node_id) {
            Some(n) => n,
            None => return 0,
        };
        let base_count = node
            .expanded_content
            .as_ref()
            .map(|c| c.lines().count().max(1))
            .unwrap_or(1);

        let children = node.children.clone();
        let child_delta: isize = children
            .iter()
            .filter_map(|&cid| self.get_node(cid))
            .filter(|c| c.expanded)
            .map(|c| {
                let actual = self.actual_expanded_line_count(c.id) as isize;
                let original = c.original_lines.len().max(1) as isize;
                actual - original
            })
            .sum();

        (base_count as isize + child_delta) as usize
    }

    /// Undo expansion of the selected macro
    fn undo_selected(&mut self) {
        let node_id = match self.selected_node_id() {
            Some(id) => id,
            None => {
                self.status = "No macro selected".to_string();
                return;
            }
        };

        let (
            name,
            line,
            expanded,
            original_lines,
            original_line_origins,
            tail_relocation,
            consumed_ids,
        ) = {
            let node = match self.get_node(node_id) {
                Some(n) => n,
                None => return,
            };
            (
                node.call.name.clone(),
                node.call.line,
                node.expanded,
                node.original_lines.clone(),
                node.original_line_origins.clone(),
                node.tail_relocation_snapshot.clone(),
                node.consumed_ids.clone(),
            )
        };

        if !expanded {
            self.status = format!("'{}' is not expanded", name);
            return;
        }

        // Compute actual line count accounting for expanded children
        let num_expanded_lines = self.actual_expanded_line_count(node_id);

        // `actual_expanded_line_count` only walks descendants, so an expanded node that
        // is *not* a descendant but sits inside this range — a macro relocated onto
        // the lifted tail — is not counted, and undoing would remove the wrong number
        // of lines and strand its output. Ask the user to collapse it first rather
        // than corrupting the buffer. A node this expansion swallowed is different:
        // its output is part of the text `original_lines` puts back, and it is
        // hidden, so asking to collapse it first would leave the undo impossible.
        let descendants = self.descendant_ids(node_id);
        let blocker = self
            .nodes
            .iter()
            .find(|n| {
                n.expanded
                    && n.id != node_id
                    && !descendants.contains(&n.id)
                    && !consumed_ids.contains(&n.id)
                    && n.call.line >= line
                    && n.call.line < line + num_expanded_lines
            })
            .map(|n| n.call.name.clone());
        if let Some(blocker) = blocker {
            self.status = format!(
                "Collapse '{}' first — it is expanded inside '{}'.",
                blocker, name
            );
            return;
        }
        let num_original_lines = original_lines.len().max(1);
        let lines_delta = num_expanded_lines as isize - num_original_lines as isize;

        // Restore original lines in source (remove expanded lines, insert originals)
        let line_idx = line.saturating_sub(1);
        if line_idx < self.source_lines.len() {
            // Remove the expanded lines from both source_lines and line_origins
            for _ in 0..num_expanded_lines {
                if line_idx < self.source_lines.len() {
                    self.source_lines.remove(line_idx);
                    self.line_origins.remove(line_idx);
                }
            }
            // Put back the exact gutter entries this expansion replaced. `line` is the
            // node's *current* position, which an earlier expansion may have shifted,
            // so deriving the numbers from it here produced a scrambled gutter — and
            // stamped line numbers onto a child's lines that are expansion output and
            // must render as `+`.
            for (i, orig) in original_lines.iter().enumerate() {
                self.source_lines.insert(line_idx + i, orig.clone());
                let origin = original_line_origins.get(i).copied().unwrap_or(None);
                self.line_origins.insert(line_idx + i, origin);
            }
        }

        // Move everything after this block back, and shrink the item of an attribute
        // that encloses it — the exact reverse of the expansion's `shift_nodes` call.
        let relocated_ids: Vec<usize> = tail_relocation.iter().map(|r| r.id).collect();
        shift_nodes(
            &mut self.nodes,
            node_id,
            line,
            line + num_expanded_lines - 1,
            -lines_delta,
            &relocated_ids,
        );

        // The nodes this expansion swallowed are visible again. Their coordinates
        // moved with this node in the meantime (`shift_nodes` anchors them on it), so
        // they describe the text just put back.
        for id in &consumed_ids {
            if let Some(node) = self.get_node_mut(*id) {
                node.consumed_by = None;
            }
        }

        // Put back the nodes this expansion had relocated onto its lifted tail. Undoing
        // only the line shift left their columns rebased onto the tail and their
        // `original_lines` holding the tail's text, so they overlapped the macro they
        // had shared a line with — and expanding one then sliced the wrong text. The
        // snapshot is absolute, taken when this node sat on `r.anchor_line`; whatever
        // has moved this node since moved the relocated node by as much, so the
        // restored coordinates have to follow — restoring the snapshot verbatim put
        // `b` of `a!(); b!();` back onto a line inside an expansion made above them.
        for r in &tail_relocation {
            let since = line as isize - r.anchor_line as isize;
            let back = |v: usize| (v as isize + since).max(1) as usize;
            if let Some(node) = self.get_node_mut(r.id) {
                node.call.line = back(r.line);
                node.call.line_end = back(r.line_end);
                node.call.derive_line = back(r.derive_line);
                node.call.item_line_end = back(r.item_line_end);
                node.call.col_start = r.col_start;
                node.call.col_end = r.col_end;
                node.original_lines = r.original_lines.clone();
            }
        }

        // Remove all descendant nodes recursively
        self.remove_descendants(node_id);

        // Reset node state
        if let Some(node) = self.get_node_mut(node_id) {
            node.expanded = false;
            node.expanded_content = None;
            node.children.clear();
            node.children_visible = true;
            node.original_line_origins.clear();
            node.tail_relocation_snapshot.clear();
            node.consumed_ids.clear();
        }

        self.rebuild_visible_nodes();

        // Collapsing is the only mutation that shrinks the buffer. Without re-clamping,
        // a cursor that was sitting inside the expansion is left past the end: the pane
        // renders blank and `j`/`k` stop responding until the user presses `G`.
        self.cursor_line = self.cursor_line.min(self.source_lines.len().max(1));
        self.ensure_cursor_visible();
        self.snap_cursor_col();
        self.sync_selection_to_cursor();

        self.status = format!("Undid expansion of '{}'", name);
    }

    /// Toggle expansion of the selected macro (expand if not expanded, undo if expanded)
    fn toggle_expansion(&mut self) {
        let is_expanded = self.selected_node().map(|n| n.expanded).unwrap_or(false);

        if is_expanded {
            self.undo_selected();
        } else {
            self.expand_selected();
        }
    }

    /// Ids of every node below `parent_id`, at any depth.
    fn descendant_ids(&self, parent_id: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack: Vec<usize> = self
            .get_node(parent_id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        while let Some(id) = stack.pop() {
            out.push(id);
            if let Some(node) = self.get_node(id) {
                stack.extend(node.children.iter().copied());
            }
        }
        out
    }

    /// Recursively remove all descendant nodes
    fn remove_descendants(&mut self, parent_id: usize) {
        let children: Vec<usize> = self
            .nodes
            .iter()
            .find(|n| n.id == parent_id)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        for child_id in children {
            self.remove_descendants(child_id);
        }

        // Remove direct children
        self.nodes.retain(|n| n.parent_id != Some(parent_id));
    }

    /// Toggle visibility of children for selected node
    fn toggle_children(&mut self) {
        let node_id = match self.selected_node_id() {
            Some(id) => id,
            None => return,
        };

        if let Some(node) = self.get_node_mut(node_id) {
            if !node.children.is_empty() {
                node.children_visible = !node.children_visible;
            }
        }

        self.rebuild_visible_nodes();
    }

    /// Reload trace data
    fn reload_trace(&mut self) {
        self.status = "Reloading trace data...".to_string();

        // Kill the previous run first: repeated `r` presses must not stack concurrent
        // cargo processes, and the old one has to release the target-directory lock
        // before the new one can make progress.
        self.expansion_cache.kill_child();

        // Re-read the file. The user reloads precisely because it changed on disk;
        // keeping the old lines would leave the display and the fresh traces
        // permanently out of sync, so every expansion would fail to match.
        let source = match std::fs::read_to_string(&self.file_path) {
            Ok(s) => expand_tabs(&s),
            Err(e) => {
                self.status = format!("Failed to re-read {}: {}", self.file_path.display(), e);
                return;
            }
        };

        // Touch the target source file to force recompilation.
        let _ = filetime::set_file_mtime(&self.file_path, filetime::FileTime::now());

        match self.trace_macros.run() {
            Ok(run) => {
                self.expansion_cache = ExpansionCache::new(run.iter, run.check_result, run.child);

                // Rebuild every per-file state from the fresh source. Expanded nodes
                // are dropped on purpose: their content, undo snapshots and line ranges
                // all describe the previous run of the previous file.
                self.source_lines = source.lines().map(|s| s.to_string()).collect();
                self.line_origins = (1..=self.source_lines.len()).map(Some).collect();
                let (nodes, next_id) = build_root_nodes(&source, &self.source_lines);
                self.visible_nodes = nodes.iter().map(|n| n.id).collect();
                self.nodes = nodes;
                self.next_id = next_id;
                self.selected_idx = 0;
                self.list_state = ListState::default();
                // The file may have shrunk under the cursor.
                self.cursor_line = self.cursor_line.min(self.source_lines.len()).max(1);
                self.ensure_cursor_visible();
                self.snap_cursor_col();
                self.sync_selection_to_cursor();

                // Parent modules on the stack still hold their pre-edit source. Backing
                // out into one would show stale lines whose macros no longer match the
                // traces this reload just generated, so refresh them the same way.
                for saved in &mut self.module_stack {
                    let Ok(text) = std::fs::read_to_string(&saved.file_path) else {
                        continue;
                    };
                    let text = expand_tabs(&text);
                    saved.source_lines = text.lines().map(|s| s.to_string()).collect();
                    saved.line_origins = (1..=saved.source_lines.len()).map(Some).collect();
                    let (nodes, next_id) = build_root_nodes(&text, &saved.source_lines);
                    saved.visible_nodes = nodes.iter().map(|n| n.id).collect();
                    saved.nodes = nodes;
                    saved.next_id = next_id;
                    saved.selected_idx = 0;
                    saved.list_state = ListState::default();
                    // Without this the tree comes back with nothing highlighted until
                    // the cursor moves.
                    saved
                        .list_state
                        .select((!saved.visible_nodes.is_empty()).then_some(0));
                    saved.cursor_line = saved.cursor_line.min(saved.source_lines.len()).max(1);
                    saved.cursor_col = 0;
                    saved.scroll_offset = saved
                        .scroll_offset
                        .min(saved.source_lines.len().saturating_sub(1));
                }

                self.status = format!("Reloaded trace data. Found {} macros.", self.nodes.len());
            }
            Err(e) => {
                self.status = format!("Failed to reload trace: {}", e);
            }
        }
    }

    /// Try to parse a `mod <name>;` declaration from the current cursor line.
    /// Returns the module name if the line is a `mod` declaration without a body.
    fn parse_mod_declaration_at_cursor(&self) -> Option<String> {
        let line = self.source_lines.get(self.cursor_line.saturating_sub(1))?;
        let trimmed = line.trim();
        // Match patterns: `mod foo;`, `pub mod foo;`, `pub(crate) mod foo;`, etc.
        let rest = match trimmed.strip_prefix("mod ") {
            Some(rest) => rest,
            // Handle `pub mod`, `pub(crate) mod`, `pub(super) mod`, etc.
            None => {
                let after_pub = trimmed.strip_prefix("pub ")?;
                match after_pub.strip_prefix("mod ") {
                    Some(rest) => rest,
                    // A visibility qualifier: skip past its closing paren.
                    None => after_pub
                        .strip_prefix('(')?
                        .split_once(')')?
                        .1
                        .trim_start()
                        .strip_prefix("mod ")?,
                }
            }
        };
        // `rest` should be `name;` or `name ;`
        let rest = rest.trim();
        if !rest.ends_with(';') {
            return None; // has a body block, not an external module
        }
        let name = rest.trim_end_matches(';').trim();
        if name.is_empty() || name.contains('{') {
            return None;
        }
        Some(name.to_string())
    }

    /// Resolve the file path for a submodule given its name.
    /// Follows Rust module resolution rules:
    /// - If current file is `mod.rs`, `lib.rs`, or `main.rs`: look in the same directory
    /// - Otherwise: look in a subdirectory named after the current file (without extension)
    fn resolve_submodule_path(&self, mod_name: &str) -> Option<PathBuf> {
        let file_name = self.file_path.file_stem()?.to_str()?;
        let parent_dir = self.file_path.parent()?;

        // Determine the base directory for submodule search
        let base_dir = if file_name == "mod" || file_name == "lib" || file_name == "main" {
            parent_dir.to_path_buf()
        } else {
            parent_dir.join(file_name)
        };

        // Try `base_dir/mod_name.rs` first, then `base_dir/mod_name/mod.rs`
        let candidate1 = base_dir.join(format!("{}.rs", mod_name));
        if candidate1.exists() {
            return Some(candidate1);
        }
        let candidate2 = base_dir.join(mod_name).join("mod.rs");
        if candidate2.exists() {
            return Some(candidate2);
        }
        None
    }

    /// Enter a submodule: save current state and load the submodule file.
    fn enter_submodule(&mut self) {
        let mod_name = match self.parse_mod_declaration_at_cursor() {
            Some(name) => name,
            None => return, // not on a mod declaration
        };

        let sub_path = match self.resolve_submodule_path(&mod_name) {
            Some(p) => p,
            None => {
                self.status = format!("Cannot find module file for '{}'", mod_name);
                return;
            }
        };

        let source = match std::fs::read_to_string(&sub_path) {
            Ok(s) => expand_tabs(&s),
            Err(e) => {
                self.status = format!("Failed to read {}: {}", sub_path.display(), e);
                return;
            }
        };

        // Save current state
        let saved = ModuleState {
            source_lines: std::mem::take(&mut self.source_lines),
            line_origins: std::mem::take(&mut self.line_origins),
            nodes: std::mem::take(&mut self.nodes),
            next_id: self.next_id,
            visible_nodes: std::mem::take(&mut self.visible_nodes),
            selected_idx: self.selected_idx,
            list_state: std::mem::take(&mut self.list_state),
            scroll_offset: self.scroll_offset,
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
            file_path: self.file_path.clone(),
            module_path: self.module_path.clone(),
        };
        self.module_stack.push(saved);

        // Set up new module state
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        let line_origins: Vec<Option<usize>> = (1..=source_lines.len()).map(Some).collect();
        // Same rules as the top-level file. This used to dedupe derives per item as
        // well as attributes, which left every derive after the first in a
        // `#[derive(A, B)]` with no node at all — unreachable by Tab, h/l, or cursor.
        let (nodes, next_id) = build_root_nodes(&source, &source_lines);

        let visible_nodes: Vec<usize> = nodes.iter().map(|n| n.id).collect();
        let list_state = ListState::default();

        self.source_lines = source_lines;
        self.line_origins = line_origins;
        self.nodes = nodes;
        self.next_id = next_id;
        self.visible_nodes = visible_nodes;
        self.selected_idx = 0;
        self.list_state = list_state;
        self.scroll_offset = 0;
        self.cursor_line = 1;
        self.snap_cursor_col();
        self.module_path.push(mod_name.clone());
        self.file_path = sub_path;
        self.sync_selection_to_cursor();
        self.status = format!(
            "Entered module '{}'. Found {} macros. Press Backspace to return.",
            mod_name,
            self.nodes.len(),
        );
    }

    /// Return to the parent module, restoring saved state.
    fn return_to_parent_module(&mut self) {
        let saved = match self.module_stack.pop() {
            Some(s) => s,
            None => {
                self.status = "Already at the top-level module".to_string();
                return;
            }
        };

        self.source_lines = saved.source_lines;
        self.line_origins = saved.line_origins;
        self.nodes = saved.nodes;
        self.next_id = saved.next_id;
        self.visible_nodes = saved.visible_nodes;
        self.selected_idx = saved.selected_idx;
        self.list_state = saved.list_state;
        self.scroll_offset = saved.scroll_offset;
        self.cursor_line = saved.cursor_line;
        self.cursor_col = saved.cursor_col;
        self.file_path = saved.file_path;
        self.module_path = saved.module_path;
        self.status = format!(
            "Returned to module '{}'",
            self.module_path.last().unwrap_or(&"crate".to_string()),
        );
    }

    /// Get the current module path as a display string (e.g., "crate::foo::bar")
    fn module_path_display(&self) -> String {
        self.module_path.join("::")
    }
}

/// Find the top-level source file for the target, using `cargo metadata`.
fn find_source_file(args: &Args) -> io::Result<PathBuf> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(ref manifest_path) = args.manifest_path {
        cmd.manifest_path(manifest_path);
    }
    let metadata = cmd
        .exec()
        .map_err(|e| io::Error::other(format!("cargo metadata failed: {}", e)))?;

    // Determine the package name to look up.
    // If -p/--package is set, use that; otherwise use workspace_default_members.
    let package = if let Some(ref pkg_name) = args.package {
        metadata
            .packages
            .iter()
            .find(|p| p.name == *pkg_name)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("package '{}' not found in workspace", pkg_name),
                )
            })?
    } else {
        // Use workspace_default_members to find the default package
        let default_id = metadata.workspace_default_members.first().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no workspace_default_members found; use -p to specify a package",
            )
        })?;
        metadata
            .packages
            .iter()
            .find(|p| &p.id == default_id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("default member package not found: {}", default_id),
                )
            })?
    };

    use cargo_metadata::TargetKind;

    // Find the matching target within the package.
    let target = if let Some(ref bin_name) = args.bin {
        package
            .targets
            .iter()
            .find(|t| t.is_kind(TargetKind::Bin) && t.name == *bin_name)
    } else if args.lib {
        package
            .targets
            .iter()
            .find(|t| t.is_kind(TargetKind::Lib) || t.is_kind(TargetKind::ProcMacro))
    } else if let Some(ref test_name) = args.test {
        // --test: first try kind=test with matching name, then fall back to lib
        package
            .targets
            .iter()
            .find(|t| t.is_kind(TargetKind::Test) && t.name == *test_name)
            .or_else(|| {
                package
                    .targets
                    .iter()
                    .find(|t| t.is_kind(TargetKind::Lib) || t.is_kind(TargetKind::ProcMacro))
            })
    } else if let Some(ref example_name) = args.example {
        package
            .targets
            .iter()
            .find(|t| t.is_kind(TargetKind::Example) && t.name == *example_name)
    } else {
        // Default: prefer lib, then bin
        package
            .targets
            .iter()
            .find(|t| t.is_kind(TargetKind::Lib) || t.is_kind(TargetKind::ProcMacro))
            .or_else(|| package.targets.iter().find(|t| t.is_kind(TargetKind::Bin)))
    };

    let target = target.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no matching target found in package '{}'", package.name),
        )
    })?;

    let src_path = target.src_path.clone().into_std_path_buf();
    if !src_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("source file not found: {}", src_path.display()),
        ));
    }

    Ok(src_path)
}

fn build_trace_macros(args: &Args) -> TraceMacros {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let tm_args = cargo_macra::trace_macros::Args {
        package: args.package.clone(),
        bin: args.bin.clone(),
        lib: args.lib,
        test: args.test.clone(),
        example: args.example.clone(),
        manifest_path: args.manifest_path.clone(),
        cargo_args: args.cargo_args.clone(),
        hook_lib: cargo_macra::find_hook_lib(std::env::current_exe().ok().as_deref())
            .unwrap_or_default(),
    };
    TraceMacros::new(std::path::Path::new(&cargo), &tm_args)
}

fn run_app(
    source: String,
    file_path: PathBuf,
    module_path: Vec<String>,
    expansion_cache: ExpansionCache,
    trace_macros: TraceMacros,
) -> io::Result<()> {
    // Restore the terminal even if the TUI panics — otherwise the shell is left in
    // raw mode on the alternate screen, which looks exactly like a hang.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    install_signal_handler();

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = App::new(
        source,
        file_path,
        module_path,
        expansion_cache,
        trace_macros,
    );

    // Run the loop in a closure so that an I/O error takes the same exit path as a
    // clean quit and always restores the terminal.
    let result = (|| -> io::Result<()> {
        loop {
            if app.needs_full_redraw {
                app.needs_full_redraw = false;
                terminal.clear()?;
            }
            terminal.draw(|frame| ui(frame, &mut app))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                let ev = event::read()?;
                if let Event::Resize(_, _) = ev {
                    // Shrinking the terminal can leave the cursor below the new
                    // viewport; nothing else pulls it back until the next j/k.
                    app.ensure_cursor_visible();
                }
                if let Event::Key(key) = ev {
                    if key.kind == KeyEventKind::Press {
                        // Raw mode swallows SIGINT, so Ctrl-C arrives as a plain key
                        // event; without this the only way out of a wedged TUI is to
                        // kill the terminal. Checked before anything else so it works
                        // even while an error dialog is up.
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
                        {
                            break;
                        }

                        // The ambiguity popup is modal: it owns the keyboard until the
                        // user picks a candidate or backs out.
                        if app.pending_choice.is_some() {
                            match key.code {
                                KeyCode::Down | KeyCode::Char('j') => app.choice_move(1),
                                KeyCode::Up | KeyCode::Char('k') => app.choice_move(-1),
                                KeyCode::Enter => app.choice_confirm(),
                                KeyCode::Esc | KeyCode::Char('q') => app.choice_cancel(),
                                _ => {}
                            }
                            continue;
                        }

                        // If error message is displayed, dismiss it on Enter
                        if app.error_message.is_some() {
                            if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                                app.error_message = None;
                            }
                            continue;
                        }

                        match key.code {
                            KeyCode::Char('q') => break,
                            // Deliberately not a quit key: Esc cancels a pending
                            // expansion, and an Esc arriving just after the trace
                            // landed used to exit the application instead.
                            KeyCode::Esc => {
                                app.status = "Press 'q' to quit.".to_string();
                            }
                            KeyCode::Down | KeyCode::Char('j') => app.cursor_down(),
                            KeyCode::Up | KeyCode::Char('k') => app.cursor_up(),
                            // Step between macros that share the current line (the
                            // derives of one `#[derive(A, B)]`, `a!(); b!();`, ...).
                            // A no-op when the line holds nothing further, rather
                            // than silently moving the cursor somewhere unexpected.
                            KeyCode::Right | KeyCode::Char('l') => {
                                app.cursor_horizontal(true);
                            }
                            KeyCode::Left | KeyCode::Char('h') => {
                                app.cursor_horizontal(false);
                            }
                            KeyCode::Enter => {
                                // If cursor is on a `mod foo;` declaration, enter that submodule.
                                // Otherwise, toggle macro expansion.
                                if app.parse_mod_declaration_at_cursor().is_some() {
                                    app.enter_submodule();
                                } else {
                                    app.toggle_expansion();
                                }
                            }
                            KeyCode::Backspace => app.return_to_parent_module(),
                            KeyCode::Char('v') => app.toggle_split_view(),
                            KeyCode::Char('r') => app.reload_trace(),
                            KeyCode::Char('n') => app.jump_to_next_macro(),
                            KeyCode::Char('N') => app.jump_to_prev_macro(),
                            KeyCode::Tab => app.next(),
                            KeyCode::BackTab => app.previous(),
                            KeyCode::Char(' ') => app.toggle_children(),
                            KeyCode::PageDown => {
                                for _ in 0..app.page_step() {
                                    app.cursor_down();
                                }
                            }
                            KeyCode::PageUp => {
                                for _ in 0..app.page_step() {
                                    app.cursor_up();
                                }
                            }
                            KeyCode::Home | KeyCode::Char('g') => {
                                app.cursor_line = 1;
                                app.ensure_cursor_visible();
                                app.snap_cursor_col();
                                app.sync_selection_to_cursor();
                            }
                            KeyCode::End | KeyCode::Char('G') => {
                                app.cursor_line = app.source_lines.len().max(1);
                                app.ensure_cursor_visible();
                                app.snap_cursor_col();
                                app.sync_selection_to_cursor();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    })();

    restore_terminal();
    let _ = std::panic::take_hook();

    result
}

/// Put the terminal back into its normal state. Safe to call more than once.
/// Restore the terminal if we are killed from outside.
///
/// Raw mode swallows Ctrl-C (it arrives as a key event instead), so the signals that
/// actually reach this process come from `kill`, a closing terminal emulator, or a CI
/// timeout. The default disposition terminates without unwinding, so neither the panic
/// hook nor the normal exit path runs, and the user is left staring at a shell in raw
/// mode on the alternate screen.
#[cfg(unix)]
fn install_signal_handler() {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    for &sig in &[SIGTERM, SIGINT, SIGHUP] {
        // Safety: `restore_terminal` only issues terminal escape sequences and an
        // ioctl; it allocates nothing and takes no locks that a signal could interrupt.
        let registered = unsafe {
            signal_hook::low_level::register(sig, move || {
                restore_terminal();
                let _ = signal_hook::low_level::emulate_default_handler(sig);
            })
        };
        let _ = registered;
    }
}

#[cfg(not(unix))]
fn install_signal_handler() {}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = stdout().execute(LeaveAlternateScreen);
}

fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(chunks[0]);

    // Left panel: macro tree
    // Borrow each node for the duration of the map; only the name is cloned into its
    // span, so the items own their text and the borrow of `app` ends at `collect`.
    // Cloning whole nodes here copied each macro's entire input text every frame.
    let items: Vec<ListItem> = app
        .visible_nodes
        .iter()
        .filter_map(|&id| app.get_node(id))
        .map(|node| {
            let indent = "  ".repeat(node.depth);

            // Tree branch characters
            let branch = if node.depth > 0 { "├─ " } else { "" };

            // Collapse/expand indicator
            let collapse_indicator = if !node.children.is_empty() {
                if node.children_visible { "v " } else { "> " }
            } else {
                "  "
            };

            // Status marker: ✓ for expanded, ! for failed, space for pending
            let (status_marker, status_style) = if node.expanded {
                ("✓", Style::default().fg(Color::Green))
            } else if node.expansion_failed {
                ("!", Style::default().fg(Color::Red).bold())
            } else {
                (" ", Style::default())
            };

            let kind_style = match node.call.kind {
                MacroKind::Functional => Style::default().fg(Color::Cyan),
                MacroKind::Attribute => Style::default().fg(Color::Yellow),
                MacroKind::Derive => Style::default().fg(Color::Magenta),
            };

            let name_style = if node.expanded {
                Style::default().fg(Color::Green).bold()
            } else if node.expansion_failed {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White).bold()
            };

            let line = Line::from(vec![
                Span::raw(indent),
                Span::raw(branch),
                Span::raw(collapse_indicator),
                Span::styled(format!("[{}] ", node.call.kind.as_str()), kind_style),
                Span::styled(node.call.name.clone(), name_style),
                Span::styled(
                    format!(" L{}", node.call.line),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!(" {}", status_marker), status_style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Macros [{}] ", app.module_path_display())),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).bold())
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, main_chunks[0], &mut app.list_state);

    // Update source_view_height for scroll calculations
    app.source_view_height = main_chunks[1].height;

    // Right panel: source code with highlighting
    let cursor_line = app.cursor_line;
    // Column range of the selected macro on the cursor line. Several macros can share
    // a line — the derives of one `#[derive(A, B, C)]` above all — so highlighting the
    // whole line is not enough to show which one Enter will expand.
    let sel_span: Option<(usize, usize)> = app
        .selected_node()
        .and_then(|n| App::node_col_span(n, cursor_line));

    let regions = app.split_regions();
    // Inner width of the source pane, minus the 7-char line-number gutter.
    let content_w = main_chunks[1].width.saturating_sub(2).saturating_sub(7) as usize;

    // Display rows the split blocks add before source line `idx`. Same arithmetic as
    // `ensure_cursor_visible`, but over the regions already built above rather than
    // re-deriving (and re-cloning) them.
    let extra_before = |idx: usize| -> usize {
        regions
            .iter()
            .filter(|r| r.start + r.len <= idx)
            .map(|r| (r.rows() + 2).saturating_sub(r.len))
            .sum()
    };

    // Build only the rows that fit in the viewport. Rendering the whole buffer meant
    // re-tokenizing and re-allocating every line ~10x/second: on a 5k-line buffer a
    // single frame took longer than the event loop's poll interval, so keystrokes
    // lagged by nearly a second.
    //
    // `scroll_offset` is in source-line space; `top_disp` is the display row the
    // viewport starts at — the same value the old `Paragraph::scroll` offset used, so
    // scrolling behaves identically.
    let view_h = main_chunks[1].height.saturating_sub(2) as usize;
    let top_src = app.scroll_offset;
    let top_disp = top_src + extra_before(top_src);

    // Start the walk at the beginning of the split region containing the top source
    // line — its block may be partially scrolled off the top — or at the top line
    // itself when it sits outside every region.
    let start_i = regions
        .iter()
        .find(|r| r.start <= top_src && top_src < r.start + r.len)
        .map(|r| r.start)
        .unwrap_or(top_src)
        .min(app.source_lines.len());

    let mut source_lines: Vec<Line> = Vec::with_capacity(view_h);
    let mut i = start_i; // source-line index
    let mut row = start_i + extra_before(start_i); // display row of source line `i`
    while i < app.source_lines.len() && row < top_disp + view_h {
        if let Some(region) = regions.iter().find(|r| r.start == i) {
            let block_rows = region.rows() + 2;
            if row + block_rows > top_disp {
                // Only the slice of the block that intersects the viewport.
                let first = top_disp.saturating_sub(row);
                let last = (top_disp + view_h - row).min(block_rows);
                render_split_block_rows(
                    &mut source_lines,
                    region,
                    content_w,
                    cursor_line,
                    first..last,
                );
            }
            row += block_rows;
            i += region.len;
            continue;
        }
        if row >= top_disp {
            source_lines.push(render_plain_line(
                &app.source_lines[i],
                app.line_origins[i],
                i + 1,
                cursor_line,
                sel_span,
            ));
        }
        row += 1;
        i += 1;
    }

    let mod_display = app.module_path_display();
    let title = if let Some(node) = app.selected_node() {
        format!(
            " {} - {} at line {} (depth {}) ",
            mod_display, node.call.name, node.call.line, node.depth
        )
    } else {
        format!(" {} ", mod_display)
    };

    // `source_lines` already starts at the viewport's first display row and holds at
    // most one screenful, so the widget needs no scroll of its own.
    let paragraph =
        Paragraph::new(source_lines).block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(paragraph, main_chunks[1]);

    // Bottom status bar with key guide
    let key_guide = " j/k=↑↓  h/l=←→ pick macro  g/G=top/bottom  n/N=next/prev  Enter=expand/mod  v=split/inline  BS=back  r=reload  q=quit ";
    let status_text = if app.status.is_empty() {
        key_guide.to_string()
    } else {
        format!("{} | {}", app.status, key_guide)
    };
    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(status, chunks[1]);

    // Ambiguity popup: several traces matched and expand differently.
    if let Some(ref choice) = app.pending_choice {
        let area = frame.area();
        let popup_width = (area.width as u32 * 80 / 100).min(90) as u16;
        let popup_height = (area.height as u32 * 70 / 100).min(24) as u16;
        let popup_area = Rect::new(
            (area.width - popup_width) / 2,
            (area.height - popup_height) / 2,
            popup_width,
            popup_height,
        );
        frame.render_widget(ratatui::widgets::Clear, popup_area);

        let inner_w = popup_width.saturating_sub(2) as usize;
        let inner_h = popup_height.saturating_sub(2) as usize;
        let mut lines: Vec<Line> = Vec::new();

        // The list is windowed around the highlight and the preview always keeps a
        // couple of rows: with many candidates an unwindowed list scrolled the
        // highlight off the bottom (`Paragraph` does not scroll itself) and starved
        // the preview of its entire budget.
        let total = choice.candidates.len();
        let list_rows = inner_h.saturating_sub(4).clamp(1, total.max(1)).min(total);
        let first = choice
            .selected
            .saturating_sub(list_rows.saturating_sub(1))
            .min(total.saturating_sub(list_rows));
        if first > 0 {
            lines.push(Line::from(Span::styled(
                format!("  … {} above", first),
                Style::default().fg(Color::DarkGray),
            )));
        }
        for (i, cand) in choice
            .candidates
            .iter()
            .enumerate()
            .skip(first)
            .take(list_rows)
        {
            let selected = i == choice.selected;
            let marker = if selected { "> " } else { "  " };
            let style = if selected {
                Style::default().fg(Color::White).bg(Color::DarkGray).bold()
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{}{}. ", marker, i + 1), style),
                Span::styled(fit_to_width(&cand.label, inner_w.saturating_sub(6)), style),
            ]));
        }
        let shown = first + list_rows;
        if shown < total {
            lines.push(Line::from(Span::styled(
                format!("  … {} below", total - shown),
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Preview of the highlighted candidate, so the labels alone don't have to
        // carry the decision.
        if let Some(cand) = choice.candidates.get(choice.selected) {
            let budget = inner_h.saturating_sub(lines.len() + 1);
            if budget > 0 {
                lines.push(Line::from(Span::styled(
                    "expands to:",
                    Style::default().fg(Color::DarkGray),
                )));
                for text in cand.output.lines().take(budget) {
                    lines.push(Line::from(highlight_owned(
                        &fit_to_width(text, inner_w),
                        Style::default(),
                    )));
                }
            }
        }

        let popup = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(format!(" Which '{}'? ", choice.name))
                .title_style(Style::default().fg(Color::Yellow).bold()),
        );
        frame.render_widget(popup, popup_area);
    }

    // Error popup (if any)
    if let Some(ref error_msg) = app.error_message {
        let area = frame.area();
        // Widen in u32: `area.width * 80` overflows a u16 past 819 columns, which
        // panics in debug builds the moment an error popup is shown.
        let popup_width = (area.width as u32 * 80 / 100).min(80) as u16;
        let popup_height = (area.height as u32 * 60 / 100).min(20) as u16;
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the popup area
        frame.render_widget(ratatui::widgets::Clear, popup_area);

        let error_paragraph = Paragraph::new(error_msg.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .title(" Error ")
                    .title_style(Style::default().fg(Color::Red).bold()),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false });

        frame.render_widget(error_paragraph, popup_area);
    }
}

/// Byte offset of character column `col` in `line`, clamped to the end of the line.
///
/// Columns come from proc-macro2 spans and are character-based; slicing a `str` with
/// one directly panics on any line containing multibyte text. Every conversion from a
/// span column to a byte index must go through here.
fn byte_of_col(line: &str, col: usize) -> usize {
    line.char_indices()
        .nth(col)
        .map(|(b, _)| b)
        .unwrap_or(line.len())
}

/// Where a macro that sat at character column `col` lands after the text from
/// `tail_start_col` onward is lifted onto its own line indented by `indent`.
fn rebase_col(col: usize, tail_start_col: usize, indent: usize) -> usize {
    indent + col.saturating_sub(tail_start_col)
}

/// Whatever follows the `#[...]` attribute containing character column `col` on
/// `line`, trimmed.
///
/// Used to rescue the item text when an attribute and its item share a line, as in
/// `#[derive(Debug)] struct S;`. Returns an empty string when the attribute ends the
/// line (the usual formatting), so the common case is unaffected.
fn attribute_tail(line: &str, col: usize) -> String {
    let anchor = byte_of_col(line, col);
    let Some(open) = line[..anchor].rfind("#[") else {
        return String::new();
    };
    text_after_closing_bracket(line, open + 2)
}

/// `attribute_tail` for the last line of an attribute that spans several lines: the
/// `#[` was opened on an earlier line, so the item is whatever follows the first `]`
/// on this one that is not closing a `[` of its own. With `rfind("#[")` on this line
/// the `struct S;` of `#[derive(\n    A,\n)] struct S;` vanished until undo.
fn attribute_close_tail(line: &str) -> String {
    text_after_closing_bracket(line, 0)
}

/// The trimmed text after the `]` that closes a `[` opened before byte `from`, or an
/// empty string when the bracket does not close on this line.
fn text_after_closing_bracket(line: &str, from: usize) -> String {
    // `[` and `]` are ASCII, so scanning bytes cannot land inside a multibyte char.
    let bytes = line.as_bytes();
    let mut depth = 0usize;
    for idx in from..bytes.len() {
        match bytes[idx] {
            b'[' => depth += 1,
            b']' => {
                if depth == 0 {
                    return line[idx + 1..].trim().to_string();
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    String::new()
}

/// Split `line` at the character columns `[start, end)`, returning the three pieces.
/// Columns come from proc-macro2 spans and are character-based, so they must not be
/// used as byte offsets.
fn split_at_cols(line: &str, start: usize, end: usize) -> (&str, &str, &str) {
    let s = byte_of_col(line, start);
    let e = byte_of_col(line, end.max(start));
    (&line[..s], &line[s..e], &line[e..])
}

/// Ratatui style for one syntax token kind.
///
/// Mirrors the ANSI palette that `pretty::highlight` uses for `--show-expansion`
/// so the TUI and stdout output look the same.
/// Truncate to `w` *display columns* (with an ellipsis) or pad with spaces to exactly
/// `w`. Never byte- or char-based: a CJK glyph counts as one char but occupies two
/// cells, and ratatui lays the buffer out by display width. Measuring in chars made the
/// split view's separator drift right by one cell per wide glyph on that row.
fn fit_to_width(s: &str, w: usize) -> String {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    let width = s.width();
    if width <= w {
        let mut out = s.to_string();
        out.extend(std::iter::repeat_n(' ', w - width));
        return out;
    }
    if w == 0 {
        return String::new();
    }
    // Keep one column for the ellipsis. A wide glyph straddling the boundary is
    // dropped whole, so the result can fall a column short — pad it back below.
    let mut out = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if used + cw > w - 1 {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    used += 1;
    out.extend(std::iter::repeat_n(' ', w.saturating_sub(used)));
    out
}

/// Syntax-highlight `text` into spans that own their content, so callers can pass
/// temporaries (padded/truncated cells) rather than borrowing from `source_lines`.
fn highlight_owned(text: &str, base: Style) -> Vec<Span<'static>> {
    highlight_spans(text, base, None)
        .into_iter()
        .map(|s| Span::styled(s.content.into_owned(), s.style))
        .collect()
}

/// Render one ordinary, full-width source line.
fn render_plain_line(
    line: &str,
    origin: Option<usize>,
    display_idx: usize,
    cursor_line: usize,
    sel_span: Option<(usize, usize)>,
) -> Line<'static> {
    let is_cursor = display_idx == cursor_line;
    let is_expanded = origin.is_none();

    // Expanded lines have no original number; a green `+` marks them as macro output
    // so they stay distinguishable now that their content is syntax highlighted like
    // everything else.
    let line_num_str = match origin {
        Some(n) => format!("{:4} │ ", n),
        None => "   + │ ".to_string(),
    };

    let line_num_style = if is_cursor {
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::DarkGray)
            .bold()
    } else if is_expanded {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // The cursor line keeps its dark-gray background (and bold) while the syntax
    // colors show through on top of it.
    let base_style = if is_cursor {
        Style::default().bg(Color::DarkGray).bold()
    } else {
        Style::default()
    };

    // On the cursor line, pick out the selected macro's own columns.
    let sel = sel_span.filter(|_| is_cursor);
    let mut spans = vec![Span::styled(line_num_str, line_num_style)];
    spans.extend(
        highlight_spans(line, base_style, sel)
            .into_iter()
            .map(|s| Span::styled(s.content.into_owned(), s.style)),
    );
    Line::from(spans)
}

/// Render one expanded range as a two-column block: the source it replaced on the
/// left, the macro output on the right, framed by divider rows. The UI renders through
/// `render_split_block_rows`; the tests exercise whole blocks through this.
#[cfg(test)]
fn render_split_block(
    out: &mut Vec<Line<'static>>,
    region: &SplitRegion,
    content_w: usize,
    cursor_line: usize,
) {
    render_split_block_rows(out, region, content_w, cursor_line, 0..region.rows() + 2);
}

/// Render only the given slice of a split block's display rows, so a block partially
/// scrolled out of the viewport costs only its visible rows.
///
/// Block-row space: row 0 is the header, rows `1..=region.rows()` are the body (body
/// row `k` is block row `k + 1`), and row `region.rows() + 1` is the footer. `rows` is
/// clamped to that space; an empty or out-of-range slice emits nothing.
fn render_split_block_rows(
    out: &mut Vec<Line<'static>>,
    region: &SplitRegion,
    content_w: usize,
    cursor_line: usize,
    rows: std::ops::Range<usize>,
) {
    let total = region.rows() + 2;
    let rows = rows.start.min(total)..rows.end.min(total);
    if rows.is_empty() {
        return;
    }

    let gutter = Style::default().fg(Color::DarkGray);
    let rule = Style::default().fg(Color::DarkGray);
    // Two columns plus the " │ " separator between them.
    let sep = " │ ";
    let left_w = content_w.saturating_sub(sep.chars().count()) / 2;
    let right_w = content_w.saturating_sub(left_w + sep.chars().count());

    let head = |label: &str, w: usize| -> String {
        use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
        let text = format!("─ {} ", label);
        // Macro names can be non-ASCII, so measure the label in display columns.
        if text.width() >= w {
            let mut out = String::new();
            let mut used = 0usize;
            for ch in text.chars() {
                let cw = ch.width().unwrap_or(0);
                if used + cw > w {
                    break;
                }
                out.push(ch);
                used += cw;
            }
            out.extend(std::iter::repeat_n('─', w - used));
            out
        } else {
            let pad = w - text.width();
            let mut s = text;
            s.extend(std::iter::repeat_n('─', pad));
            s
        }
    };

    // Frame geometry, in cells from the left edge of the row. A body row is
    // `"     │ "` (7) + left_w + `" │ "` (3) + right_w, so its separator bar sits at
    // cell `left_w + 8`. The header/footer gutter is `"     ├"` (6), so the rule to
    // the tee must be `left_w + 2` wide and the rule after it `right_w + 1` — the
    // frame used to be two cells narrow, putting ┬/┴ left of the bar they cap.
    let head_left_w = left_w + 2;
    let head_right_w = right_w + 1;

    // Header: ├─ original ──┬─ expanded (name) ──
    if rows.contains(&0) {
        out.push(Line::from(vec![
            Span::styled("     ├", gutter),
            Span::styled(head("original", head_left_w), rule),
            Span::styled("┬", rule),
            Span::styled(
                head(&format!("expanded: {}", region.name), head_right_w),
                rule,
            ),
        ]));
    }

    // Body: block rows `1..=region.rows()`, clipped to the requested slice.
    for block_row in rows.start.max(1)..rows.end.min(total - 1) {
        let k = block_row - 1;
        let left = region.original.get(k).map(String::as_str).unwrap_or("");
        // The cursor moves over `source_lines`, which inside a split region are the
        // expanded lines — so it belongs to the right column.
        let is_cursor = region
            .right
            .get(k)
            .is_some_and(|(idx, _)| idx + 1 == cursor_line);
        let right_base = if is_cursor {
            Style::default().bg(Color::DarkGray).bold()
        } else {
            Style::default()
        };

        let mut spans = vec![Span::styled("     │ ", gutter)];
        spans.extend(highlight_owned(
            &fit_to_width(left, left_w),
            Style::default().add_modifier(Modifier::DIM),
        ));
        spans.push(Span::styled(sep, rule));
        match region.right.get(k) {
            Some((_, text)) => {
                spans.extend(highlight_owned(&fit_to_width(text, right_w), right_base))
            }
            None => spans.push(Span::styled(" ".repeat(right_w), right_base)),
        }
        out.push(Line::from(spans));
    }

    // Footer: ├────┴────
    if rows.contains(&(total - 1)) {
        out.push(Line::from(vec![
            Span::styled("     ├", gutter),
            Span::styled("─".repeat(head_left_w), rule),
            Span::styled("┴", rule),
            Span::styled("─".repeat(head_right_w), rule),
        ]));
    }
}

fn token_style(kind: pretty::TokenKind) -> Style {
    use pretty::TokenKind as K;
    let color = match kind {
        K::Plain => Color::White,
        K::Comment => Color::DarkGray,
        K::Attribute => Color::Yellow,
        K::Keyword => Color::Blue,
        K::MacroName => Color::Cyan,
        K::Type => Color::Magenta,
        K::Literal => Color::Green,
        K::Number => Color::LightYellow,
        K::Lifetime => Color::Magenta,
    };
    Style::default().fg(color)
}

/// Syntax-highlight one source line into per-token spans.
///
/// `base` supplies the backdrop (background / modifiers) that every token
/// inherits; each token then patches its own foreground on top.  When `sel` is
/// given, that half-open range of *character* columns is instead rendered as the
/// selected-macro marker (black on yellow), splitting tokens if it lands inside
/// one.
fn highlight_spans<'a>(line: &'a str, base: Style, sel: Option<(usize, usize)>) -> Vec<Span<'a>> {
    let sel_style = base.patch(Style::default().fg(Color::Black).bg(Color::Yellow).bold());
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut push = |text: &'a str, style: Style| {
        if !text.is_empty() {
            spans.push(Span::styled(text, style));
        }
    };

    let mut col = 0usize; // character column of the token about to be emitted
    for (kind, range) in pretty::tokenize(line) {
        let text = &line[range];
        let len = text.chars().count();
        let style = base.patch(token_style(kind));
        match sel {
            // Clamp the selection into this token's own column space; the
            // helper is character-based, so multi-byte text is safe.
            Some((cs, ce)) if ce > cs && ce > col && cs < col + len => {
                let (a, b, c) = split_at_cols(
                    text,
                    cs.saturating_sub(col).min(len),
                    ce.saturating_sub(col).min(len),
                );
                push(a, style);
                push(b, sel_style);
                push(c, style);
            }
            _ => push(text, style),
        }
        col += len;
    }
    spans
}

/// Resolve a module path (e.g., "foo::bar") relative to the top-level source file.
/// Returns the resolved file path and the full module path segments including "crate".
fn resolve_module_path(top_level: &Path, module_str: &str) -> io::Result<(PathBuf, Vec<String>)> {
    let segments: Vec<&str> = module_str.split("::").filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Ok((top_level.to_path_buf(), vec!["crate".to_string()]));
    }

    let mut current_file = top_level.to_path_buf();
    let mut module_path = vec!["crate".to_string()];

    for segment in &segments {
        let file_name = current_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let parent_dir = current_file.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "cannot determine parent directory")
        })?;

        let base_dir = if file_name == "mod" || file_name == "lib" || file_name == "main" {
            parent_dir.to_path_buf()
        } else {
            parent_dir.join(file_name)
        };

        // Try segment.rs first, then segment/mod.rs
        let candidate1 = base_dir.join(format!("{}.rs", segment));
        let candidate2 = base_dir.join(segment).join("mod.rs");

        if candidate1.exists() {
            current_file = candidate1;
        } else if candidate2.exists() {
            current_file = candidate2;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "cannot find module '{}' (tried {} and {})",
                    segment,
                    candidate1.display(),
                    candidate2.display()
                ),
            ));
        }
        module_path.push(segment.to_string());
    }

    Ok((current_file, module_path))
}

fn main() -> io::Result<()> {
    let mut args = Args::parse();

    // When invoked via `cargo run -- symbol` (without "macra" subcommand),
    // _subcommand consumes the module path. Detect and fix this.
    if args.module.is_none() {
        if let Some(ref sub) = args._subcommand {
            if sub != "macra" {
                args.module = Some(sub.clone());
            }
        }
    }

    eprintln!("Finding source file via cargo metadata...");
    let top_level_path = match find_source_file(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error finding source file: {}", e);
            std::process::exit(1);
        }
    };

    // Resolve module path if specified
    let (src_path, module_path) = if let Some(ref module_str) = args.module {
        match resolve_module_path(&top_level_path, module_str) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error resolving module path '{}': {}", module_str, e);
                std::process::exit(1);
            }
        }
    } else {
        (top_level_path, vec!["crate".to_string()])
    };

    eprintln!("Loading source from {}", src_path.display());
    let source = expand_tabs(&std::fs::read_to_string(&src_path)?);

    // Touch the source file to invalidate cargo's cache and force recompilation.
    // This ensures the macra-hook (LD_PRELOAD) can intercept proc-macro loading,
    // which is necessary because cargo caches stderr output and replays it on
    // subsequent runs — if a previous compilation ran without the hook, the cached
    // stderr won't contain proc-macro expansion data.
    let _ = filetime::set_file_mtime(&src_path, filetime::FileTime::now());

    eprintln!("Running cargo with -Z trace-macros...");
    let tm = build_trace_macros(&args);
    let run = tm.run()?;

    if args.show_expansion {
        let expansions: Vec<_> = run.iter.collect::<io::Result<Vec<_>>>()?;
        print_expansions(&expansions, args.color.resolve());
        return Ok(());
    }

    let cache = ExpansionCache::new(run.iter, run.check_result, run.child);
    run_app(source, src_path, module_path, cache, tm)
}

/// Print all macro expansions to stdout in a human-readable format.
///
/// The token streams are pretty-printed with `prettyplease` and, when `color`
/// is set, colorized with ANSI escapes.
///
/// Each expansion is printed as:
///   == caller_pattern ==
///   input source
///   ---
///   output source
fn print_expansions(expansions: &[cargo_macra::parse_trace::MacroExpansion], color: bool) {
    use cargo_macra::parse_trace::MacroExpansionKind;

    if expansions.is_empty() {
        println!("No macro expansions found.");
        return;
    }

    let mut first = true;
    for expansion in expansions {
        if !first {
            println!();
        }
        first = false;

        // Format caller pattern based on kind
        let caller = match expansion.kind {
            MacroExpansionKind::Bang => format!("{}!", expansion.name),
            MacroExpansionKind::Attribute => {
                if expansion.arguments.is_empty() {
                    format!("#[{}]", expansion.name)
                } else {
                    format!(
                        "#[{}({})]",
                        expansion.name,
                        expansion.arguments.replace('\n', " ")
                    )
                }
            }
            MacroExpansionKind::Derive => format!("#[derive({})]", expansion.name),
        };

        println!("{}", pretty::header(&format!("== {} ==", caller), color));
        if !expansion.input.is_empty() {
            print!("{}", pretty::render(&expansion.input, color));
        }
        println!("{}", pretty::header("---", color));
        print!("{}", pretty::render(&expansion.to, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(kind: MacroKind, line: usize, col_start: usize, col_end: usize) -> MacroNode {
        MacroNode {
            call: MacroCall {
                name: "X".to_string(),
                krate: String::new(),
                kind,
                line,
                col_start,
                col_end,
                derive_line: line,
                line_end: line,
                item_line_end: line,
                input: String::new(),
                arguments: String::new(),
                sibling_derives: Vec::new(),
            },
            id: 0,
            derive_group: 0,
            parent_id: None,
            depth: 0,
            expanded: false,
            expansion_failed: false,
            original_lines: Vec::new(),
            expanded_content: None,
            children: Vec::new(),
            children_visible: true,
            original_line_origins: Vec::new(),
            tail_relocation_snapshot: Vec::new(),
            consumed_by: None,
            consumed_ids: Vec::new(),
        }
    }

    /// `#[derive(Greet, Describe)]` on line 92, as `find_macros` reports it.
    fn two_derives_on_one_line() -> Vec<MacroNode> {
        let mut greet = node(MacroKind::Derive, 92, 9, 14);
        greet.call.name = "Greet".into();
        greet.id = 1;
        let mut describe = node(MacroKind::Derive, 92, 16, 24);
        describe.call.name = "Describe".into();
        describe.id = 2;
        vec![greet, describe]
    }

    #[test]
    fn each_derive_on_a_shared_line_is_selectable_by_column() {
        let nodes = two_derives_on_one_line();
        let refs: Vec<&MacroNode> = nodes.iter().collect();

        // Cursor inside `Greet`.
        assert_eq!(App::pick_node_at(&refs, 92, 9), Some(0));
        assert_eq!(App::pick_node_at(&refs, 92, 13), Some(0));
        // Cursor inside `Describe` — previously unreachable: the old line-only
        // rule always returned the first node on the line.
        assert_eq!(App::pick_node_at(&refs, 92, 16), Some(1));
        assert_eq!(App::pick_node_at(&refs, 92, 23), Some(1));
    }

    #[test]
    fn column_outside_every_span_still_selects_something_on_the_line() {
        // Enter should not become a no-op just because the column sits on a comma.
        let nodes = two_derives_on_one_line();
        let refs: Vec<&MacroNode> = nodes.iter().collect();
        assert!(App::pick_node_at(&refs, 92, 15).is_some());
        assert!(App::pick_node_at(&refs, 91, 0).is_none());
    }

    #[test]
    fn nested_child_wins_over_its_enclosing_parent() {
        // A child macro inside a parent's expanded range must stay reachable.
        let mut parent = node(MacroKind::Functional, 10, 0, 40);
        parent.call.line_end = 20;
        parent.id = 1;
        let mut child = node(MacroKind::Functional, 12, 4, 12);
        child.depth = 1;
        child.id = 2;
        let nodes = [parent, child];
        let refs: Vec<&MacroNode> = nodes.iter().collect();
        assert_eq!(App::pick_node_at(&refs, 12, 4), Some(1));
        // Off the child's line, the parent still owns the range.
        assert_eq!(App::pick_node_at(&refs, 15, 0), Some(0));
    }

    fn region(start: usize, len: usize, original: usize) -> SplitRegion<'static> {
        SplitRegion {
            start,
            len,
            original: vec!["orig".to_string(); original].leak(),
            right: (start..start + len)
                .map(|i| (i, format!("line{}", i).leak() as &str))
                .collect(),
            name: "M",
        }
    }

    /// Any slice of a split block must reproduce exactly those rows of the full
    /// render — the viewport renderer relies on this to clip blocks at the screen
    /// edges without changing what is shown.
    #[test]
    fn split_block_row_slices_match_the_full_block() {
        let reg = region(0, 3, 5); // rows() = 5, so 7 block rows with the frame
        let total = reg.rows() + 2;

        let mut full = Vec::new();
        render_split_block(&mut full, &reg, 60, 2);
        assert_eq!(full.len(), total);

        for start in 0..=total {
            for end in start..=total + 1 {
                let mut sliced = Vec::new();
                render_split_block_rows(&mut sliced, &reg, 60, 2, start..end);
                let want = &full[start..end.min(total)];
                assert_eq!(sliced, want, "slice {}..{} diverged", start, end);
            }
        }
    }

    #[test]
    fn split_block_is_as_tall_as_its_taller_column() {
        assert_eq!(region(0, 6, 2).rows(), 6);
        assert_eq!(region(0, 2, 6).rows(), 6);
    }

    /// Every row of a split block — header, body, footer — must be the same number of
    /// display cells wide, or the ┬/┴ caps drift away from the separator bar they cap.
    #[test]
    fn split_block_rows_all_have_the_same_width() {
        use unicode_width::UnicodeWidthStr;

        let row_width =
            |line: &Line<'static>| -> usize { line.spans.iter().map(|s| s.content.width()).sum() };

        for content_w in [20usize, 41, 80, 81] {
            let mut out = Vec::new();
            render_split_block(&mut out, &region(0, 3, 2), content_w, 1);
            let widths: Vec<usize> = out.iter().map(row_width).collect();
            assert!(
                widths.windows(2).all(|w| w[0] == w[1]),
                "content_w={} produced ragged rows: {:?}",
                content_w,
                widths
            );
        }
    }

    /// The separator bar and the caps must land on the same cell.
    #[test]
    fn split_block_caps_sit_on_the_separator_bar() {
        let cell_of = |line: &Line<'static>, needle: char| -> Option<usize> {
            use unicode_width::UnicodeWidthChar;
            let mut cell = 0usize;
            for span in &line.spans {
                for ch in span.content.chars() {
                    if ch == needle {
                        return Some(cell);
                    }
                    cell += ch.width().unwrap_or(0);
                }
            }
            None
        };

        let mut out = Vec::new();
        render_split_block(&mut out, &region(0, 3, 2), 60, 999);
        let header = &out[0];
        let body = &out[1];
        let footer = out.last().unwrap();

        // The body's bar is the second `│` on the row (the first is the gutter).
        let bar = {
            use unicode_width::UnicodeWidthChar;
            let mut cell = 0usize;
            let mut seen = 0;
            let mut found = None;
            for span in &body.spans {
                for ch in span.content.chars() {
                    if ch == '│' {
                        seen += 1;
                        if seen == 2 {
                            found = Some(cell);
                        }
                    }
                    cell += ch.width().unwrap_or(0);
                }
            }
            found.expect("body row has a separator bar")
        };

        assert_eq!(cell_of(header, '┬'), Some(bar), "header cap off the bar");
        assert_eq!(cell_of(footer, '┴'), Some(bar), "footer cap off the bar");
    }

    #[test]
    fn expand_tabs_expands_leading_indentation() {
        assert_eq!(expand_tabs("\tfoo!();"), "    foo!();");
        assert_eq!(expand_tabs("\t\tx"), "        x");
    }

    #[test]
    fn expand_tabs_advances_to_tab_stops_mid_line() {
        // A tab is not a fixed run of spaces: it advances to the next stop.
        assert_eq!(expand_tabs("a\tb"), "a   b");
        assert_eq!(expand_tabs("abcd\tb"), "abcd    b");
    }

    #[test]
    fn expand_tabs_resets_the_column_at_newlines() {
        assert_eq!(expand_tabs("ab\n\tx"), "ab\n    x");
    }

    #[test]
    fn expand_tabs_counts_columns_in_characters() {
        // Multibyte chars are one column each, matching proc-macro2 span columns.
        assert_eq!(expand_tabs("日\tx"), "日   x");
    }

    #[test]
    fn expand_tabs_leaves_tabless_source_untouched() {
        assert_eq!(expand_tabs("fn main() {}\n"), "fn main() {}\n");
    }

    #[test]
    fn find_macros_columns_index_the_tab_expanded_text() {
        // The load path expands tabs and then parses the very same string, so the
        // span columns must slice the expanded lines correctly.
        let src = expand_tabs("fn main() {\n\tlet v = vec![1, 2, 3];\n}\n");
        let macros = find_macros(&src);
        let m = macros.iter().find(|m| m.name == "vec").unwrap();
        let line = src.lines().nth(m.line - 1).unwrap();
        assert_eq!(&line[m.col_start..m.col_end], "vec![1, 2, 3]");
    }

    /// `a!(1); b!(2);` — expanding `a!` lifts `b!(2);` onto its own indented line, and
    /// `b`'s columns have to follow it or a later expansion slices the wrong text.
    #[test]
    fn columns_rebase_onto_the_lifted_tail_line() {
        // "    a!(1); b!(2);" — `b` spans columns 11..13, the tail starts at column 11.
        let line = "    a!(1); b!(2);";
        assert_eq!(line.chars().nth(11), Some('b'));
        // Lifted onto its own line with the original 4-space indent, `b` starts at 4.
        assert_eq!(rebase_col(11, 11, 4), 4);
        // A macro further along the tail keeps its relative offset.
        assert_eq!(rebase_col(13, 11, 4), 6);
        // Never underflows if a column somehow precedes the tail.
        assert_eq!(rebase_col(2, 11, 4), 4);
    }

    /// `#[derive(Debug)] struct S;` shares one line between attribute and item, and
    /// expanding the derive removes that whole line — the item text has to be carried
    /// over or it vanishes from the view.
    #[test]
    fn attribute_tail_rescues_an_item_sharing_the_attribute_line() {
        // Column 9 is inside `Debug`.
        assert_eq!(attribute_tail("#[derive(Debug)] struct S;", 9), "struct S;");
        // Nested brackets must not end the attribute early.
        assert_eq!(
            attribute_tail("#[derive(Debug)] struct S([u8; 4]);", 9),
            "struct S([u8; 4]);"
        );
        // The usual formatting: attribute alone on its line, nothing to rescue.
        assert_eq!(attribute_tail("#[derive(Debug)]", 9), "");
        assert_eq!(attribute_tail("    #[derive(Debug)]   ", 13), "");
        // Not an attribute line at all.
        assert_eq!(attribute_tail("struct S;", 3), "");
        // Multibyte before the attribute must not break the byte scan.
        assert_eq!(
            attribute_tail("#[derive(Debug)] struct 挨拶;", 9),
            "struct 挨拶;"
        );
    }

    /// Regression: expansion columns are character-based, so a line containing
    /// multibyte text used to slice at a non-char boundary and panic the whole TUI.
    #[test]
    fn columns_convert_to_byte_offsets_on_multibyte_lines() {
        let line = "let 挨拶 = vec![1, 2];";
        // `vec` starts at character column 9, but byte 9 lands inside `拶` — slicing
        // the line with the raw column is what used to blow up the TUI.
        assert_eq!(line.chars().nth(9), Some('v'));
        assert!(!line.is_char_boundary(9));

        let start = byte_of_col(line, 9);
        assert_eq!(&line[..start], "let 挨拶 = ");
        assert_eq!(&line[start..], "vec![1, 2];");

        // Past the end clamps rather than panicking.
        assert_eq!(byte_of_col(line, 9_999), line.len());
        let (before, mid, after) = split_at_cols(line, 9, 12);
        assert_eq!(before, "let 挨拶 = ");
        assert_eq!(mid, "vec");
        assert_eq!(after, "![1, 2];");
    }

    #[test]
    fn nested_expansions_do_not_produce_nested_split_blocks() {
        // A child expanded inside its parent's output lies within the parent's range.
        let kept = keep_outermost(vec![region(10, 8, 1), region(12, 3, 1), region(30, 2, 1)]);
        let spans: Vec<(usize, usize)> = kept.iter().map(|r| (r.start, r.len)).collect();
        assert_eq!(spans, vec![(10, 8), (30, 2)]);
    }

    #[test]
    fn adjacent_regions_are_both_kept() {
        // Ending exactly where the next starts is not an overlap.
        let kept = keep_outermost(vec![region(0, 5, 1), region(5, 5, 1)]);
        assert_eq!(kept.len(), 2);
    }

    /// The derives of one `#[derive(A, B)]` share a group, and derives on a different
    /// item do not. Siblings used to be matched by line number, which broke as soon as
    /// one of them was expanded and left its line — the expanded derive then looked
    /// unexpanded and was re-emitted into the remaining `#[derive(...)]`.
    #[test]
    fn derives_of_one_attribute_share_a_group() {
        let source = "\
#[derive(Clone, Debug)]
struct A;

#[derive(Clone, Debug)]
struct B;
";
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        let (nodes, _) = build_root_nodes(source, &source_lines);

        let derives: Vec<&MacroNode> = nodes
            .iter()
            .filter(|n| n.call.kind == MacroKind::Derive)
            .collect();
        assert_eq!(derives.len(), 4, "two derives on each of two items");

        // Grouped by item, not by name and not by line.
        assert_eq!(derives[0].derive_group, derives[1].derive_group);
        assert_eq!(derives[2].derive_group, derives[3].derive_group);
        assert_ne!(derives[0].derive_group, derives[2].derive_group);
        // The group id is a node id, so it is stable and unique rather than a position.
        assert_eq!(derives[0].derive_group, derives[0].id);
    }

    #[test]
    fn expansion_markers_are_recognised() {
        assert!(is_expansion_marker("    // -- expanded: Describe --"));
        assert!(is_expansion_marker("// -- end Describe --"));
        assert!(is_expansion_marker("// -- end foo::bar --"));
        // Real code that merely mentions the words must not be swallowed.
        assert!(!is_expansion_marker("let x = 1; // -- end of the line"));
        assert!(!is_expansion_marker("impl Foo {"));
        // A comment shaped like a marker but naming several words is prose, not a
        // macro — this one used to match and vanish from the split view.
        assert!(!is_expansion_marker("// -- end of section --"));
        assert!(!is_expansion_marker("    // -- expanded: see below --"));
        // Both delimiters are required.
        assert!(!is_expansion_marker("// -- expanded: Describe"));
        assert!(!is_expansion_marker("// -- end Describe"));
        assert!(!is_expansion_marker("// -- end  --"));
    }

    #[test]
    fn fit_to_width_pads_and_truncates_by_display_columns() {
        use unicode_width::UnicodeWidthStr;
        assert_eq!(fit_to_width("ab", 5), "ab   ");
        assert_eq!(fit_to_width("abcdef", 4), "abc…");
        assert_eq!(fit_to_width("abc", 3), "abc");
        assert_eq!(fit_to_width("abc", 0), "");
        // Wide glyphs occupy two cells each: measuring them in chars is what made the
        // split view's separator drift. Every result must be exactly `w` cells wide.
        assert_eq!(fit_to_width("日本語", 5).width(), 5);
        assert_eq!(fit_to_width("日本語です", 4).width(), 4);
        assert_eq!(fit_to_width("日本", 6).width(), 6);
        assert_eq!(fit_to_width("日本", 4), "日本");
        // A wide glyph straddling the cut is dropped whole, then padded back.
        assert_eq!(fit_to_width("日本語", 4).width(), 4);
    }

    /// Expanding a multi-line `foo!(\n ... \n); bar!(9);` lifts the fragment after
    /// foo's closing paren — which lives on foo's END line — onto its own new line.
    /// `bar` must follow it there: keying the relocation on the start line missed it
    /// entirely and left it pointing into removed text.
    #[test]
    fn tail_macros_on_a_multiline_calls_end_line_follow_the_lifted_tail() {
        // 2: "    foo!("        — foo spans lines 2..=4, its span ends at col 5 of line 4
        // 3: "        1,"
        // 4: "    ); bar!(9);"  — bar (cols 7..14) sits after foo on foo's end line
        let mut foo = node(MacroKind::Functional, 2, 4, 5);
        foo.call.line_end = 4;
        foo.id = 1;
        let mut bar = node(MacroKind::Functional, 4, 7, 14);
        bar.id = 2;
        // A macro nested inside foo's last line, before the tail: must not move.
        let mut inner = node(MacroKind::Functional, 4, 4, 5);
        inner.id = 3;
        let mut nodes = vec![foo, bar, inner];

        // foo's 3 removed lines became 5 formatted lines whose last is the lifted
        // tail "    ; bar!(9);" — 1-indexed line 6, indented 4.
        // foo's 3 lines (2..=4) became 5 formatted lines (2..=6), the last being the
        // lifted tail.
        let source_lines: Vec<String> = vec![
            "fn main() {".into(),
            "    // -- expanded: foo --".into(),
            "    ()".into(),
            "    ()".into(),
            "    // -- end foo --".into(),
            "    ; bar!(9);".into(),
        ];
        let relocated = App::relocate_tail_nodes(&mut nodes, &source_lines, 1, 4, 5, 4, 6);

        assert_eq!(relocated.iter().map(|r| r.id).collect::<Vec<_>>(), vec![2]);
        let bar = &nodes[1];
        assert_eq!((bar.call.line, bar.call.line_end), (6, 6));
        // "    ; bar!(9);" — `bar!(9)` now spans character columns 6..13.
        assert_eq!((bar.call.col_start, bar.call.col_end), (6, 13));
        assert_eq!(bar.original_lines, vec!["    ; bar!(9);".to_string()]);
        // The expanded macro itself and the nested node keep their coordinates.
        assert_eq!(nodes[0].call.line, 2);
        assert_eq!(nodes[2].call.line, 4);
    }

    /// `relocate_tail_nodes` returns each moved node's prior coordinates so undo can
    /// restore them; reversing only the line shift left them overlapping their
    /// neighbour with columns rebased onto the tail.
    #[test]
    fn relocation_reports_the_pre_move_coordinates() {
        let mut bar = node(MacroKind::Functional, 2, 10, 14);
        bar.id = 2;
        let mut nodes = vec![bar];
        let source_lines: Vec<String> = vec![String::new(); 6];

        let moved = App::relocate_tail_nodes(&mut nodes, &source_lines, 1, 2, 10, 4, 5);

        assert_eq!(moved.len(), 1);
        let before = &moved[0];
        assert_eq!(before.id, 2);
        // The snapshot holds where it was, not where it went.
        assert_eq!((before.line, before.col_start, before.col_end), (2, 10, 14));
        assert_eq!(
            (nodes[0].call.line, nodes[0].call.col_start),
            (5, 4),
            "the node itself moved"
        );
    }

    /// An already-expanded node must not be relocated: its `original_lines` hold the
    /// text it replaced, and rebasing would overwrite that with marker text.
    #[test]
    fn an_expanded_node_is_left_alone_by_relocation() {
        let mut b = node(MacroKind::Functional, 2, 10, 14);
        b.id = 2;
        b.expanded = true;
        b.original_lines = vec!["    a!(); b!();".to_string()];
        let mut nodes = vec![b];
        let source_lines: Vec<String> = vec![String::new(); 6];

        let moved = App::relocate_tail_nodes(&mut nodes, &source_lines, 1, 2, 10, 4, 5);

        assert!(moved.is_empty());
        assert_eq!(nodes[0].call.col_start, 10);
        assert_eq!(nodes[0].original_lines, vec!["    a!(); b!();".to_string()]);
    }

    /// A relocated macro can itself span several lines. Pinning its end to the tail
    /// line put its own continuation lines outside it; the span has to shift whole.
    #[test]
    fn a_relocated_multiline_macro_keeps_its_extent() {
        let mut spanning = node(MacroKind::Functional, 4, 7, 14);
        spanning.call.line_end = 5;
        spanning.call.item_line_end = 5;
        spanning.id = 2;
        let mut nodes = vec![spanning];

        let source_lines: Vec<String> = vec![String::new(); 8];
        let relocated = App::relocate_tail_nodes(&mut nodes, &source_lines, 1, 4, 5, 4, 6);

        assert_eq!(relocated.iter().map(|r| r.id).collect::<Vec<_>>(), vec![2]);
        let n = &nodes[0];
        // Shifted by tail_line - tail_src_line = 2, so the two-line span is preserved
        // rather than collapsed onto line 6.
        assert_eq!(n.call.line, 6);
        assert_eq!(n.call.line_end, 7);
        assert_eq!(n.call.item_line_end, 7);
        assert_eq!(n.original_lines.len(), 2);
    }

    /// Tab moves the tree selection without going through the cursor, so the cursor
    /// must be parked on the selected node's own line *and* column — otherwise the
    /// next j/k re-snaps the column to the first macro on the line and silently
    /// changes the selection (`#[derive(A, B)]`: Tab onto `B`, then j, k → `A`).
    #[test]
    fn tab_parks_the_cursor_on_the_selected_nodes_own_span() {
        let nodes = two_derives_on_one_line();
        // The second derive's cursor lands on its own column, not the line's first.
        assert_eq!(App::node_cursor_pos(&nodes[1]), (92, 16));

        // A derive in a multi-line derive list lives on `derive_line`, not on the
        // attribute's first line; the cursor must follow it there or the selected
        // node has no highlighted span in the source.
        let mut wrapped = node(MacroKind::Derive, 10, 4, 9);
        wrapped.call.derive_line = 11;
        wrapped.call.line_end = 12;
        assert_eq!(App::node_cursor_pos(&wrapped), (11, 4));
        // The reported position is exactly where `node_col_span` keys the span, so
        // cursor, highlight, and selection all agree.
        assert_eq!(App::node_col_span(&wrapped, 11), Some((4, 9)));
        assert_eq!(App::node_col_span(&wrapped, 10), None);
    }

    #[test]
    fn col_span_is_reported_only_on_the_macros_own_line() {
        let n = node(MacroKind::Derive, 3, 9, 14);
        assert_eq!(App::node_col_span(&n, 3), Some((9, 14)));
        assert_eq!(App::node_col_span(&n, 4), None);
    }

    #[test]
    fn multiline_invocation_owns_the_rest_of_its_first_line() {
        let mut n = node(MacroKind::Functional, 2, 4, 7);
        n.call.line_end = 5;
        // The end column belongs to line 5, so on line 2 the macro extends to EOL.
        assert_eq!(App::node_col_span(&n, 2), Some((4, usize::MAX)));
    }

    #[test]
    fn split_at_cols_is_character_based() {
        // Byte slicing would panic or mis-split on multi-byte characters.
        let line = "let x = \"日本語\"; foo!();";
        let start = line.chars().position(|c| c == 'f').unwrap();
        let (a, b, c) = split_at_cols(line, start, start + 4);
        assert_eq!(b, "foo!");
        assert_eq!(a, "let x = \"日本語\"; ");
        assert_eq!(c, "();");
    }

    #[test]
    fn split_at_cols_clamps_past_end_of_line() {
        let (a, b, c) = split_at_cols("abc", 1, usize::MAX);
        assert_eq!((a, b, c), ("a", "bc", ""));
    }

    /// The concatenated span contents must always reproduce the line.
    fn spans_text(spans: &[Span<'_>]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn highlight_spans_colors_each_token_separately() {
        let line = "    pub fn f() { foo!(\"s\"); } // c";
        let spans = highlight_spans(line, Style::default(), None);
        assert_eq!(spans_text(&spans), line);
        let colored = |text: &str| {
            spans
                .iter()
                .find(|s| s.content == text)
                .unwrap_or_else(|| panic!("no span {:?} in {:?}", text, spans))
                .style
                .fg
        };
        assert_eq!(colored("pub"), Some(Color::Blue));
        assert_eq!(colored("fn"), Some(Color::Blue));
        assert_eq!(colored("foo!"), Some(Color::Cyan));
        assert_eq!(colored("\"s\""), Some(Color::Green));
        assert_eq!(colored("// c"), Some(Color::DarkGray));
        // More than one distinct color: this is what "actually highlighted" means.
        let mut colors: Vec<_> = spans.iter().filter_map(|s| s.style.fg).collect();
        colors.sort_by_key(|c| format!("{:?}", c));
        colors.dedup();
        assert!(colors.len() >= 4, "{:?}", colors);
    }

    #[test]
    fn highlight_spans_keeps_the_base_background() {
        let base = Style::default().bg(Color::DarkGray).bold();
        let spans = highlight_spans("let x = 1;", base, None);
        assert!(spans.iter().all(|s| s.style.bg == Some(Color::DarkGray)));
        assert!(spans.iter().any(|s| s.style.fg == Some(Color::Blue)));
    }

    #[test]
    fn highlight_spans_marks_the_selected_macro_columns() {
        let line = "#[derive(Greet, Describe)]";
        let start = line.find("Describe").unwrap();
        let spans = highlight_spans(line, Style::default(), Some((start, start + 8)));
        assert_eq!(spans_text(&spans), line);
        let marked: Vec<&str> = spans
            .iter()
            .filter(|s| s.style.bg == Some(Color::Yellow))
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(marked, vec!["Describe"]);
        assert!(
            spans
                .iter()
                .filter(|s| s.style.bg == Some(Color::Yellow))
                .all(|s| s.style.fg == Some(Color::Black))
        );
    }

    #[test]
    fn highlight_spans_selection_is_character_based() {
        // The attribute is one token, so the selection has to split inside it —
        // with multi-byte text before the range, byte offsets would mis-split.
        let line = "let s = \"日本語\"; foo!();";
        let start = line.chars().position(|c| c == 'f').unwrap();
        let spans = highlight_spans(line, Style::default(), Some((start, start + 4)));
        assert_eq!(spans_text(&spans), line);
        let marked: String = spans
            .iter()
            .filter(|s| s.style.bg == Some(Color::Yellow))
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(marked, "foo!");
    }

    #[test]
    fn expansion_matches_falls_back_for_truncated_bang_input() {
        let exp = MacroExpansion {
            krate: String::new(),
            expanding: "impl_char!".to_string(),
            arguments: String::new(),
            to: "impl ...".to_string(),
            name: "impl_char".to_string(),
            kind: MacroExpansionKind::Bang,
            input: String::new(),
        };

        assert!(ExpansionCache::expansion_matches(
            &exp,
            "$ _a (a) @ 'a'",
            "",
            "impl_char",
            MacroKind::Functional,
            false,
            true
        ));
    }

    #[test]
    fn expansion_matches_keeps_strict_input_when_present() {
        let exp = MacroExpansion {
            krate: String::new(),
            expanding: "foo! { a }".to_string(),
            arguments: String::new(),
            to: "b".to_string(),
            name: "foo".to_string(),
            kind: MacroExpansionKind::Bang,
            input: "a".to_string(),
        };

        assert!(!ExpansionCache::expansion_matches(
            &exp,
            "x",
            "",
            "foo",
            MacroKind::Functional,
            false,
            true
        ));
    }

    #[test]
    fn expansion_matches_empty_input_bang_macro() {
        // rustc normalizes `mystruct_hello!()` to `mystruct_hello! { }` in trace,
        // so expanding doesn't end with `!`. Both inputs are empty → should match.
        let exp = MacroExpansion {
            krate: String::new(),
            expanding: "mystruct_hello! { }".to_string(),
            arguments: String::new(),
            to: "println!(\"hello\");".to_string(),
            name: "mystruct_hello".to_string(),
            kind: MacroExpansionKind::Bang,
            input: String::new(),
        };

        assert!(ExpansionCache::expansion_matches(
            &exp,
            "",
            "",
            "mystruct_hello",
            MacroKind::Functional,
            false,
            true
        ));
    }

    /// Two bare `#[my_attr]`s differ only by the item they wrap. The strict pass has
    /// to tell them apart; the lenient pass still has to accept the doc-comment
    /// divergence the hook produces (`///` vs `#[doc = "..."]`).
    #[test]
    fn attribute_input_is_compared_strictly_first_then_ignored() {
        let exp = MacroExpansion {
            krate: String::new(),
            expanding: "#[my_attr] fn a() {}".to_string(),
            arguments: String::new(),
            to: "fn a() {}".to_string(),
            name: "my_attr".to_string(),
            kind: MacroExpansionKind::Attribute,
            input: "fn a() {}".to_string(),
        };

        let matches = |input: &str, strict: bool| {
            ExpansionCache::expansion_matches(
                &exp,
                input,
                "",
                "my_attr",
                MacroKind::Attribute,
                false,
                strict,
            )
        };

        // Strict: only the item this entry actually wrapped.
        assert!(matches("fn a() {}", true));
        assert!(!matches("fn b() {}", true));
        // Lenient fallback: input ignored, as before.
        assert!(matches("fn b() {}", false));
    }

    fn bang_expansion(name: &str, input: &str, to: &str) -> MacroExpansion {
        MacroExpansion {
            krate: String::new(),
            expanding: format!("{}! {{ {} }}", name, input),
            arguments: String::new(),
            to: to.to_string(),
            name: name.to_string(),
            kind: MacroExpansionKind::Bang,
            input: input.to_string(),
        }
    }

    fn cache_inner_of(expansions: Vec<MacroExpansion>) -> CacheInner {
        let normalized = expansions
            .iter()
            .map(|e| {
                (
                    cargo_macra::normalize_tokens(&e.input),
                    cargo_macra::normalize_tokens(&e.arguments),
                )
            })
            .collect();
        CacheInner {
            expansions,
            normalized,
            done: false,
            error: None,
            build_error: None,
        }
    }

    fn search(inner: &CacheInner, min_idx: usize) -> Vec<usize> {
        let norm_input = cargo_macra::normalize_tokens("a");
        let norm_arguments = cargo_macra::normalize_tokens("");
        ExpansionCache::search_expansions(
            inner,
            "a",
            &norm_input,
            &norm_arguments,
            "foo",
            MacroKind::Functional,
            min_idx,
        )
    }

    #[test]
    fn narrow_by_crate_resolves_a_collision_the_output_cannot() {
        let cand = |krate: &str, output: &str| TraceCandidate {
            label: format!("[{krate}] {output}"),
            krate: krate.to_string(),
            output: output.to_string(),
        };
        let two = vec![cand("alpha", "fn a() {}"), cand("beta", "fn b() {}")];

        // The path named one of them: that one wins outright, no popup.
        let picked = ExpansionCache::narrow_by_crate(two.clone(), "beta");
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].output, "fn b() {}");

        // A qualifier naming a crate that defines none of them changes nothing. This
        // is the re-export case (`serde::Serialize` is defined in `serde_derive`), and
        // narrowing to zero would turn a popup into a wrong answer.
        assert_eq!(
            ExpansionCache::narrow_by_crate(two.clone(), "serde").len(),
            2
        );

        // No qualifier, or nothing to choose between: untouched.
        assert_eq!(ExpansionCache::narrow_by_crate(two.clone(), "").len(), 2);
        assert_eq!(
            ExpansionCache::narrow_by_crate(vec![cand("alpha", "fn a() {}")], "beta").len(),
            1
        );

        // Two candidates from the *same* crate stay ambiguous: the crate does not
        // distinguish them, so the user still has to choose.
        let same = vec![cand("alpha", "fn a() {}"), cand("alpha", "fn b() {}")];
        assert_eq!(ExpansionCache::narrow_by_crate(same, "alpha").len(), 2);

        // An unresolved crate (non-unix host, or a `macro_rules!` trace) never matches
        // a qualifier, so those candidates are never silently preferred away.
        let unknown = vec![cand("", "fn a() {}"), cand("", "fn b() {}")];
        assert_eq!(ExpansionCache::narrow_by_crate(unknown, "alpha").len(), 2);
    }

    #[test]
    fn search_expansions_prefers_exact_name_and_resumes_from_min_idx() {
        // The mangled helper sits at a lower index, but the exact-name pass runs first
        // over all entries, so only the exact match is returned — a less specific pass
        // never contributes candidates alongside a more specific one.
        let inner = cache_inner_of(vec![
            bang_expansion("other", "a", "0"),
            bang_expansion("__foo_mangled", "a", "relaxed"),
            bang_expansion("foo", "a", "exact"),
        ]);
        assert_eq!(search(&inner, 0), vec![2]);
        // A resumed scan only considers entries appended since the last scan: with
        // min_idx past the end nothing is re-examined...
        assert!(search(&inner, 3).is_empty());
        // ...and every pass still runs over the new suffix, so a relaxed-name entry
        // appended after the watermark is found.
        let inner = cache_inner_of(vec![
            bang_expansion("foo", "a", "already scanned"),
            bang_expansion("__foo_mangled", "a", "new arrival"),
        ]);
        assert_eq!(search(&inner, 1), vec![1]);
    }

    /// Colliding entries are all returned, in stream order, rather than one being
    /// handed out per lookup by a rotating pointer.
    #[test]
    fn search_expansions_returns_every_equally_specific_match() {
        let inner = cache_inner_of(vec![
            bang_expansion("foo", "a", "first"),
            bang_expansion("other", "a", "unrelated"),
            bang_expansion("foo", "a", "second"),
        ]);
        assert_eq!(search(&inner, 0), vec![0, 2]);
    }

    /// Two invocations that expand to the same text are not a choice worth making, so
    /// they collapse; genuinely differing outputs each become a candidate.
    #[test]
    fn identical_outputs_collapse_but_differing_ones_do_not() {
        let same = cache_inner_of(vec![
            bang_expansion("foo", "a", "same output"),
            bang_expansion("foo", "a", "same output"),
        ]);
        let collapsed = ExpansionCache::distinct_candidates(&same, &[0, 1]);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].output, "same output");

        let differing = cache_inner_of(vec![
            bang_expansion("foo", "a", "one"),
            bang_expansion("foo", "a", "two"),
        ]);
        let both = ExpansionCache::distinct_candidates(&differing, &[0, 1]);
        assert_eq!(both.len(), 2);
        // The label summarises the *output*: for a bang macro `expanding` holds the
        // input and for a derive the macro name, and a collision requires those to be
        // equal — labelling with them gave every candidate the same text.
        assert_eq!(both[0].label, "one");
        assert_eq!(both[1].label, "two");
        assert_ne!(both[0].label, both[1].label);

        // When the defining crate is known it leads the label, which is what
        // actually distinguishes a collision between two same-named derives.
        let mut from_crates = cache_inner_of(vec![
            bang_expansion("foo", "a", "same text"),
            bang_expansion("foo", "a", "same text"),
        ]);
        from_crates.expansions[0].krate = "alpha".to_string();
        from_crates.expansions[1].krate = "beta".to_string();
        let labelled = ExpansionCache::distinct_candidates(&from_crates, &[0, 1]);
        // Identical output still collapses: the crate is shown, not matched on.
        assert_eq!(labelled.len(), 1);
        assert_eq!(labelled[0].label, "[alpha] same text");
        assert_eq!(
            both.iter().map(|c| c.output.as_str()).collect::<Vec<_>>(),
            vec!["one", "two"]
        );
    }

    /// The first non-blank line is used, and a whitespace-only expansion still gets a
    /// label rather than an empty row.
    #[test]
    fn candidate_labels_skip_blank_lines() {
        let inner = cache_inner_of(vec![
            bang_expansion("foo", "a", "\n\n   fn generated() {}\n"),
            bang_expansion("foo", "a", "   \n"),
        ]);
        let candidates = ExpansionCache::distinct_candidates(&inner, &[0, 1]);
        let labels: Vec<&str> = candidates.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["fn generated() {}", "(empty expansion)"]);
    }

    #[test]
    fn check_wait_error_is_surfaced_as_build_error() {
        use cargo_macra::trace_macros::CheckResult;

        // An io::Error from wait()ing on cargo must reach the user instead of being
        // swallowed into a generic "No trace found".
        let err = std::io::Error::other("waitpid failed");
        let msg = ExpansionCache::check_result_to_build_error(Ok(Err(err)))
            .expect("wait error should surface");
        assert!(msg.contains("waitpid failed"));

        // A failed build still extracts only the compiler error lines...
        let failed = CheckResult {
            success: false,
            stdout: String::new(),
            stderr: "warning: noise\nerror[E0308]: mismatched types\n".to_string(),
        };
        assert_eq!(
            ExpansionCache::check_result_to_build_error(Ok(Ok(failed))).as_deref(),
            Some("error[E0308]: mismatched types")
        );

        // ...and a success or a killed run (disconnected channel) reports nothing.
        let ok = CheckResult {
            success: true,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert_eq!(
            ExpansionCache::check_result_to_build_error(Ok(Ok(ok))),
            None
        );
        assert_eq!(
            ExpansionCache::check_result_to_build_error(Err(std::sync::mpsc::RecvError)),
            None
        );
    }

    #[test]
    fn expansion_matches_mangled_name_relaxed() {
        // Proc-macro crates like decycle generate mangled helper macros
        // (e.g. `__Parse_temporal_<hash>!`). Relaxed matching should find them.
        let exp = MacroExpansion {
            krate: String::new(),
            expanding: "__Parse_temporal_9874485626140785372! { args }".to_string(),
            arguments: String::new(),
            to: "expanded".to_string(),
            name: "__Parse_temporal_9874485626140785372".to_string(),
            kind: MacroExpansionKind::Bang,
            input: "args".to_string(),
        };

        // Exact match fails
        assert!(!ExpansionCache::expansion_matches(
            &exp,
            "args",
            "",
            "Parse",
            MacroKind::Functional,
            false,
            true
        ));
        // A different macro that merely starts with the same letters must not match,
        // or `Parse` silently expands `__Parser_<hash>`.
        assert!(!ExpansionCache::expansion_matches(
            &MacroExpansion {
                krate: String::new(),
                name: "__Parser_9874485626140785372".to_string(),
                ..exp.clone()
            },
            "args",
            "",
            "Parse",
            MacroKind::Functional,
            true,
            true
        ));
        // Relaxed match succeeds
        assert!(ExpansionCache::expansion_matches(
            &exp,
            "args",
            "",
            "Parse",
            MacroKind::Functional,
            true,
            true
        ));
    }

    // ---- Line-bookkeeping regressions -------------------------------------------
    //
    // These drive `expand_node`/`undo_selected` on a real `App` whose trace lookup is
    // bypassed (`expand_node(id, Some(text))` takes the popup path, which never
    // touches the cache), so the buffer and the node coordinates can be compared
    // after each step.

    /// An `App` over `source` with an idle, empty expansion cache.
    fn test_app(source: &str) -> App {
        let child = std::process::Command::new("true").spawn().unwrap();
        let cache = ExpansionCache {
            inner: Arc::new((
                Mutex::new(CacheInner {
                    expansions: Vec::new(),
                    normalized: Vec::new(),
                    done: true,
                    error: None,
                    build_error: None,
                }),
                Condvar::new(),
            )),
            child: Arc::new(Mutex::new(child)),
        };
        let tm = TraceMacros::new(
            Path::new("cargo"),
            &cargo_macra::trace_macros::Args::default(),
        );
        App::new(
            source.to_string(),
            PathBuf::from("/dev/null"),
            vec!["crate".into()],
            cache,
            tm,
        )
    }

    /// Select `id` the way Tab would, so `undo_selected` acts on it.
    fn select(app: &mut App, id: usize) {
        let idx = app
            .visible_nodes
            .iter()
            .position(|&n| n == id)
            .unwrap_or_else(|| panic!("node {} is not visible", id));
        app.selected_idx = idx;
        app.list_state.select(Some(idx));
    }

    fn node_named(app: &App, name: &str) -> usize {
        app.nodes
            .iter()
            .find(|n| n.call.name == name)
            .unwrap_or_else(|| panic!("no node named {}", name))
            .id
    }

    /// 1-indexed line of the first buffer line containing `needle`.
    fn line_of(app: &App, needle: &str) -> usize {
        app.source_lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("{:?} is not in the buffer", needle))
            + 1
    }

    /// The `#[tokio::main]` workflow: expand a `println!` inside the function, then
    /// the attribute. The attribute's `item_line_end` has to follow the expansion
    /// inside its item, or the attribute expansion removes too few lines and strands
    /// the tail of the function after its end marker; and the swallowed-but-expanded
    /// inner node must come back intact on undo instead of blocking it while hidden.
    #[test]
    fn enclosing_attribute_follows_an_expansion_inside_its_item() {
        let mut app = test_app("#[my_attr]\nfn f() {\n    foo!();\n}\n");
        let attr = node_named(&app, "my_attr");
        let foo = node_named(&app, "foo");

        app.expand_node(foo, Some("bar();\nbaz();".into()));
        let item_end = line_of(&app, "}");
        assert_eq!(app.get_node(attr).unwrap().call.item_line_end, item_end);

        app.expand_node(attr, Some("fn f() {}\nfn g() {}\nfn h() {}".into()));
        assert_eq!(
            app.source_lines.last().map(String::as_str),
            Some("// -- end my_attr --"),
            "nothing may survive past the attribute's end marker: {:#?}",
            app.source_lines
        );
        let inner = app.get_node(foo).unwrap();
        assert!(inner.expanded && inner.consumed_by == Some(attr));
        assert!(!app.visible_nodes.contains(&foo));

        // Undo the attribute: the buffer is back to the state with `foo` expanded.
        select(&mut app, attr);
        app.undo_selected();
        assert!(!app.get_node(attr).unwrap().expanded, "{}", app.status);
        assert_eq!(app.source_lines.len(), item_end);
        assert_eq!(app.source_lines[2], "    // -- expanded: foo --");
        assert_eq!(app.source_lines[0], "#[my_attr]");
        let inner = app.get_node(foo).unwrap();
        assert!(inner.expanded && inner.consumed_by.is_none());
        assert!(app.visible_nodes.contains(&foo));

        // And undoing `foo` restores the original file.
        select(&mut app, foo);
        app.undo_selected();
        assert_eq!(
            app.source_lines,
            vec!["#[my_attr]", "fn f() {", "    foo!();", "}"]
        );
        assert_eq!(app.get_node(attr).unwrap().call.item_line_end, 4);
    }

    /// A node swallowed by one expansion must move with that expansion when something
    /// above them both is expanded, or it is un-hidden on undo pointing at a line
    /// inside the other expansion's output.
    #[test]
    fn a_consumed_node_moves_with_its_consumer() {
        let mut app = test_app("top!();\n#[my_attr]\nfn f() {\n    foo!();\n}\n");
        let top = node_named(&app, "top");
        let attr = node_named(&app, "my_attr");
        let foo = node_named(&app, "foo");

        app.expand_node(attr, Some("fn f() {}".into()));
        app.expand_node(top, Some("x();\ny();".into()));
        select(&mut app, attr);
        app.undo_selected();

        assert_eq!(app.get_node(foo).unwrap().call.line, line_of(&app, "foo!"));
    }

    /// A node relocated onto an expansion's lifted tail is put back *relative to where
    /// the expansion now is*: restoring the absolute coordinates recorded at expansion
    /// time loses every shift applied above it in between.
    #[test]
    fn a_relocated_node_is_restored_relative_to_the_current_position() {
        let mut app = test_app("top!();\nfn f() {\n    a!(); b!();\n}\n");
        let top = node_named(&app, "top");
        let a = node_named(&app, "a");
        let b = node_named(&app, "b");

        app.expand_node(a, Some("x();".into()));
        app.expand_node(top, Some("y();\nz();".into()));
        select(&mut app, a);
        app.undo_selected();

        let b = app.get_node(b).unwrap();
        assert_eq!(b.call.line, line_of(&app, "b!"));
        assert_eq!((b.call.col_start, b.call.col_end), (10, 14));
        assert_eq!(app.source_lines[b.call.line - 1], "    a!(); b!();");
    }

    /// Two separate `#[derive]` attributes on one item are two groups, not one: the
    /// sibling-derive bookkeeping is for the names inside a single `#[derive(A, B)]`.
    /// Treating `#[derive(Clone)]` as a sibling of `#[derive(Debug)]` shifted it twice
    /// and, on undo, left it pointing past the end of a three-line buffer.
    #[test]
    fn separate_derive_attributes_on_one_item_are_separate_groups() {
        let mut app = test_app("#[derive(Debug)]\n#[derive(Clone)]\nstruct S;\n");
        let debug = node_named(&app, "Debug");
        let clone = node_named(&app, "Clone");
        assert_ne!(
            app.get_node(debug).unwrap().derive_group,
            app.get_node(clone).unwrap().derive_group
        );

        app.expand_node(debug, Some("impl Debug for S {}\nimpl X for S {}".into()));
        let c = app.get_node(clone).unwrap();
        assert_eq!(c.call.line, line_of(&app, "#[derive(Clone)]"));
        assert_eq!(c.call.item_line_end, line_of(&app, "struct S;"));

        select(&mut app, debug);
        app.undo_selected();
        let c = app.get_node(clone).unwrap();
        assert_eq!((c.call.line, c.call.item_line_end), (2, 3));
    }

    /// Child macros are located by parsing text in which `$crate` was replaced by a
    /// longer placeholder; their columns have to be mapped back onto the real text or
    /// every child after a `$crate` on its line is skewed by the length difference.
    #[test]
    fn child_columns_are_mapped_back_from_the_dollar_crate_placeholder() {
        let mut app = test_app("fn main() {\n    foo!(1);\n}\n");
        app.expand_node(
            0,
            Some("const X: u32 = $crate::compute!(1) + baz!(3);".into()),
        );
        let baz = node_named(&app, "baz");
        let (line, col_start, col_end) = {
            let n = app.get_node(baz).unwrap();
            (n.call.line, n.call.col_start, n.call.col_end)
        };
        let text = &app.source_lines[line - 1];
        assert_eq!(
            split_at_cols(text, col_start, col_end).1,
            "baz!(3)",
            "in {:?}",
            text
        );

        app.expand_node(baz, Some("9".into()));
        let marker = &app.source_lines[line - 1];
        assert!(marker.ends_with("// -- expanded: baz --"), "{:?}", marker);
        assert!(!marker.contains("baz!(3)"), "{:?}", marker);
    }

    /// A consumed *child* must leave `visible_nodes` like a consumed root does, or Tab
    /// and `n` still reach it with coordinates that point into another node's output.
    #[test]
    fn consumed_children_are_not_visible() {
        let mut app = test_app("#[outer]\nfn f() {}\n");
        app.expand_node(0, Some("#[inner_attr]\nfn g() {\n    q!(1);\n}".into()));
        let inner = node_named(&app, "inner_attr");
        let q = node_named(&app, "q");

        app.expand_node(inner, Some("fn g() {}".into()));

        assert_eq!(app.get_node(q).unwrap().consumed_by, Some(inner));
        assert!(!app.visible_nodes.contains(&q));
    }

    /// An attribute expansion replaces its whole first line, so a macro sharing that
    /// line with the attribute is gone too — unlike the text before a functional call,
    /// which stays on the marker line.
    #[test]
    fn a_macro_on_the_attributes_own_line_is_consumed() {
        let mut app = test_app("#[my_attr] fn f() { q!(1) }\n");
        let attr = node_named(&app, "my_attr");
        let q = node_named(&app, "q");

        app.expand_node(attr, Some("fn f() {}".into()));

        assert_eq!(app.get_node(q).unwrap().consumed_by, Some(attr));
        assert!(!app.visible_nodes.contains(&q));
    }

    /// Expanding `A` of `#[derive(A, B)]` leaves exactly one `B` to pick, sitting on the
    /// remaining `#[derive(B)]` line with columns that cover its name there.
    #[test]
    fn a_remaining_sibling_derive_is_offered_exactly_once() {
        let mut app = test_app("#[derive(A, B)]\nstruct S;\n");
        app.expand_node(0, Some("impl A for S {}".into()));

        let bs: Vec<&MacroNode> = app
            .visible_nodes
            .iter()
            .filter_map(|&id| app.get_node(id))
            .filter(|n| n.call.name == "B")
            .collect();
        assert_eq!(bs.len(), 1, "{:?}", bs);
        let b = bs[0];
        let text = &app.source_lines[b.call.derive_line - 1];
        assert_eq!(text.trim(), "#[derive(B)]");
        assert_eq!(split_at_cols(text, b.call.col_start, b.call.col_end).1, "B");
    }

    /// A multi-line `#[derive(\n ...\n)] struct S;` closes on the item's line, which
    /// has no `#[` of its own; the item still has to be carried over.
    #[test]
    fn an_item_after_a_multiline_derive_list_is_kept() {
        let mut app = test_app("#[derive(\n    A,\n    B,\n)] struct S;\n");
        app.expand_node(0, Some("impl A for S {}".into()));

        assert!(
            app.source_lines.iter().any(|l| l == "struct S;"),
            "{:#?}",
            app.source_lines
        );
    }
}
