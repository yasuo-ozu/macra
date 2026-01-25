mod macro_finder;

use std::io::{self, stdout};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use macra::parse_trace::{MacroExpansion, MacroExpansionKind};
use macra::trace_macros::{MacroExpansionIter, TraceMacros};
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
    /// For derive macros: snapshot of sibling derive nodes' state before this expansion,
    /// used to restore their state on undo.
    /// Vec of (node_id, original_lines, line, line_end, item_line_end)
    derive_sibling_snapshot: Vec<(usize, Vec<String>, usize, usize, usize)>,
}

struct CacheInner {
    expansions: Vec<MacroExpansion>,
    current_idx: usize,
    done: bool,
    error: Option<String>,
}

struct ExpansionCache {
    inner: Arc<(Mutex<CacheInner>, Condvar)>,
}

impl ExpansionCache {
    fn new(iter: MacroExpansionIter) -> Self {
        let inner = Arc::new((
            Mutex::new(CacheInner {
                expansions: Vec::new(),
                current_idx: 0,
                done: false,
                error: None,
            }),
            Condvar::new(),
        ));

        let bg_inner = Arc::clone(&inner);
        thread::spawn(move || {
            let (ref mutex, ref condvar) = *bg_inner;
            for result in iter {
                let mut cache = mutex.lock().unwrap();
                match result {
                    Ok(exp) => {
                        cache.expansions.push(exp);
                        condvar.notify_all();
                    }
                    Err(e) => {
                        cache.error = Some(format!("{}", e));
                        cache.done = true;
                        condvar.notify_all();
                        return;
                    }
                }
            }
            let mut cache = mutex.lock().unwrap();
            cache.done = true;
            condvar.notify_all();
        });

        Self { inner }
    }

    /// Normalize tokens for comparison.
    ///
    /// - Removes spaces adjacent to punctuation (e.g., `a :: b` → `a::b`)
    /// - Collapses remaining whitespace to single space
    /// - Normalizes bracket types (`{}`, `[]` → `()`)
    fn normalize_tokens(s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut result = String::with_capacity(chars.len());

        fn is_punct(c: char) -> bool {
            !c.is_alphanumeric() && c != '_' && c != '"' && c != '\'' && !c.is_whitespace()
        }

        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c.is_whitespace() {
                // Look at prev (non-whitespace) and next (non-whitespace) to decide
                let prev = result.chars().last();
                // Skip all whitespace
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                let next = chars.get(i).copied();
                // Drop space if adjacent to punctuation on either side
                let prev_is_punct = prev.map_or(true, is_punct);
                let next_is_punct = next.map_or(true, is_punct);
                if !prev_is_punct && !next_is_punct {
                    result.push(' ');
                }
            } else {
                match c {
                    '{' | '[' => result.push('('),
                    '}' | ']' => result.push(')'),
                    _ => result.push(c),
                }
                i += 1;
            }
        }
        result
    }

    /// Calculate similarity score between two strings (0.0 to 1.0)
    fn similarity_score(a: &str, b: &str) -> f64 {
        let a_norm = Self::normalize_tokens(a);
        let b_norm = Self::normalize_tokens(b);

        if a_norm.is_empty() || b_norm.is_empty() {
            return 0.0;
        }

        let a_chars: Vec<char> = a_norm.chars().collect();
        let b_chars: Vec<char> = b_norm.chars().collect();
        let m = a_chars.len();
        let n = b_chars.len();

        let mut prev = vec![0usize; n + 1];
        let mut curr = vec![0usize; n + 1];

        for i in 1..=m {
            for j in 1..=n {
                if a_chars[i - 1] == b_chars[j - 1] {
                    curr[j] = prev[j - 1] + 1;
                } else {
                    curr[j] = curr[j - 1].max(prev[j]);
                }
            }
            std::mem::swap(&mut prev, &mut curr);
            curr.fill(0);
        }

        let lcs_len = prev[n];
        let max_len = m.max(n);
        lcs_len as f64 / max_len as f64
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
    fn expansion_matches(
        exp: &MacroExpansion,
        input: &str,
        arguments: &str,
        name: &str,
        kind: MacroKind,
    ) -> bool {
        let macro_name = name.rsplit("::").next().unwrap_or(name).trim();
        let exp_name = exp.name.rsplit("::").next().unwrap_or(&exp.name).trim();
        let input_matches = if exp.input.is_empty() {
            // rustc may truncate very large macro invocations in trace output
            // and emit only `name!` without arguments.
            exp.kind == MacroExpansionKind::Bang && exp.expanding.trim_end().ends_with('!')
        } else {
            Self::normalize_tokens(&exp.input) == Self::normalize_tokens(input)
        };
        exp_name == macro_name
            && exp.kind == Self::to_expansion_kind(kind)
            && input_matches
            && Self::normalize_tokens(&exp.arguments) == Self::normalize_tokens(arguments)
    }

    /// Search cached expansions for a matching trace. Returns the index if found.
    /// Log messages are collected into `logs` for display in the TUI.
    fn search_expansions(
        inner: &CacheInner,
        input: &str,
        arguments: &str,
        name: &str,
        kind: MacroKind,
        logs: &mut Vec<String>,
    ) -> Option<usize> {
        logs.push(format!(
            "input: {}, arguments: {}, name: {}, kind: {}",
            input,
            arguments,
            name,
            kind.as_str()
        ));
        // Search from current_idx forward
        for idx in inner.current_idx..inner.expansions.len() {
            let exp = &inner.expansions[idx];
            logs.push(format!(
                "  [{}] name={}, kind={:?}, input={}, args={}",
                idx, exp.name, exp.kind, exp.input, exp.arguments
            ));
            if Self::expansion_matches(exp, input, arguments, name, kind) {
                logs.push(format!(
                    "match at idx {}, name: {name}, arguments: {arguments}, input: {input}",
                    idx
                ));
                return Some(idx);
            }
        }
        logs.push("wrap around".to_string());
        // Wrap around: search from beginning to current_idx
        for idx in 0..inner.current_idx {
            let exp = &inner.expansions[idx];
            logs.push(format!(
                "  [{}] name={}, kind={:?}, input={}, args={}",
                idx, exp.name, exp.kind, exp.input, exp.arguments
            ));
            if Self::expansion_matches(exp, input, arguments, name, kind) {
                logs.push(format!(
                    "match at idx {}, name: {name}, arguments: {arguments}, input: {input}",
                    idx
                ));
                return Some(idx);
            }
        }
        logs.push(format!(
            "not match name: {name}, arguments: {arguments}, input: {input}",
        ));

        None
    }

    /// Find an expansion that matches the given macro input/arguments.
    /// Blocks until a match is found or the iterator is exhausted.
    /// Returns `(expansion_text, log_messages)`.
    fn find_trace_for_tokens(
        &self,
        input: &str,
        arguments: &str,
        name: &str,
        kind: MacroKind,
    ) -> (Option<String>, Vec<String>) {
        let (ref mutex, ref condvar) = *self.inner;
        let mut inner = mutex.lock().unwrap();
        let mut logs = Vec::new();

        loop {
            if let Some(idx) =
                Self::search_expansions(&inner, input, arguments, name, kind, &mut logs)
            {
                let result = inner.expansions[idx].to.clone();
                inner.current_idx = idx + 1;
                return (Some(result), logs);
            }

            if inner.done {
                return (None, logs);
            }

            // Wait for more data from the background thread
            inner = condvar.wait(inner).unwrap();
        }
    }

    /// Get nearest matching traces sorted by similarity.
    /// Returns (scored_traces, total_trace_count).
    fn get_nearest_traces(&self, tokens: &str, limit: usize) -> (Vec<(String, f64)>, usize) {
        let (ref mutex, _) = *self.inner;
        let inner = mutex.lock().unwrap();

        let total = inner.expansions.len();
        let mut scored: Vec<(String, f64)> = inner
            .expansions
            .iter()
            .map(|exp| {
                let score = Self::similarity_score(tokens, &exp.expanding);
                let display = if exp.expanding.len() > 60 {
                    format!("{}...", &exp.expanding[..57])
                } else {
                    exp.expanding.clone()
                };
                (display, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        (scored.into_iter().take(limit).collect(), total)
    }

    /// Take the stored error (if any), clearing it.
    fn take_error(&self) -> Option<String> {
        let (ref mutex, _) = *self.inner;
        let mut inner = mutex.lock().unwrap();
        inner.error.take()
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
    scroll_offset: u16,
    cursor_line: usize,
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
    scroll_offset: u16,
    /// Current cursor line in the source view (1-indexed). This can be on any line,
    /// not just macro lines.
    cursor_line: usize,
    /// Height of the source view area (updated each frame)
    source_view_height: u16,
    /// Status message to display
    status: String,
    /// Error message to display (shown until user presses Enter)
    error_message: Option<String>,
    /// Reusable `TraceMacros` for reloading trace data
    trace_macros: TraceMacros,
    /// Path of the currently loaded source file
    file_path: PathBuf,
    /// Module path segments (e.g., ["crate", "foo", "bar"])
    module_path: Vec<String>,
    /// Stack of saved module states for returning to parent modules
    module_stack: Vec<ModuleState>,
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
        let line_origins: Vec<Option<usize>> = (1..=source_lines.len()).map(|n| Some(n)).collect();
        let macros = find_macros(&source);

        let mut nodes = Vec::new();
        let mut next_id = 0;

        // Track which items already have a top-level attribute/derive macro.
        // Key: item_line_end, Value: true if first attr/derive already added.
        let mut item_first_attr: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        // Create root-level nodes for each macro found
        for mac in macros {
            // For attr/derive macros, only show the first non-built-in one as top-level.
            // Subsequent attrs/derives on the same item will appear as children
            // when the first one is expanded.
            if matches!(mac.kind, MacroKind::Attribute | MacroKind::Derive) {
                // Skip built-in attributes entirely at top-level; they'll appear
                // as children when a non-built-in sibling is expanded.
                if mac.kind == MacroKind::Attribute && is_builtin_attribute(&mac.name) {
                    continue;
                }
                if !item_first_attr.insert(mac.item_line_end) {
                    // Already have a top-level attr/derive for this item; skip
                    continue;
                }
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

            nodes.push(MacroNode {
                call: mac,
                id: next_id,
                parent_id: None,
                depth: 0,
                expanded: false,
                expansion_failed: false,
                original_lines,
                expanded_content: None,
                children: Vec::new(),
                children_visible: true,
                derive_sibling_snapshot: Vec::new(),
            });
            next_id += 1;
        }

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
            source_view_height: 20,
            status,
            error_message: None,
            trace_macros,
            file_path,
            module_path,
            module_stack: Vec::new(),
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
            self.sync_selection_to_cursor();
        }
    }

    /// Move the source cursor up by one line
    fn cursor_up(&mut self) {
        if self.cursor_line > 1 {
            self.cursor_line -= 1;
            self.ensure_cursor_visible();
            self.sync_selection_to_cursor();
        }
    }

    /// Move the source cursor down by one line
    fn cursor_down(&mut self) {
        if self.cursor_line < self.source_lines.len() {
            self.cursor_line += 1;
            self.ensure_cursor_visible();
            self.sync_selection_to_cursor();
        }
    }

    /// Ensure the cursor line is visible in the scroll viewport
    fn ensure_cursor_visible(&mut self) {
        let view_h = self.source_view_height.saturating_sub(2) as usize; // account for borders
        if view_h == 0 {
            return;
        }
        let top = self.scroll_offset as usize;
        let bottom = top + view_h;
        let total = self.source_lines.len();

        if self.cursor_line.saturating_sub(1) < top {
            self.scroll_offset = self.cursor_line.saturating_sub(1) as u16;
        } else if self.cursor_line > bottom {
            self.scroll_offset = (self.cursor_line - view_h) as u16;
        }

        // Clamp scroll so viewport doesn't extend past the last line
        let max_scroll = total.saturating_sub(view_h);
        if (self.scroll_offset as usize) > max_scroll {
            self.scroll_offset = max_scroll as u16;
        }
    }

    /// Sync the macro list selection to the macro at cursor_line (if any).
    /// If the cursor is not on any macro's line range, deselect.
    /// When multiple nodes overlap (parent covering children), prefer the deepest match.
    fn sync_selection_to_cursor(&mut self) {
        if self.visible_nodes.is_empty() {
            self.list_state.select(None);
            return;
        }
        let mut best: Option<(usize, usize)> = None; // (visible_idx, depth)
        for (i, &nid) in self.visible_nodes.iter().enumerate() {
            if let Some(node) = self.get_node(nid) {
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
                if self.cursor_line >= start && self.cursor_line <= end {
                    if best.map_or(true, |(_, d)| node.depth > d) {
                        best = Some((i, node.depth));
                    }
                }
            }
        }
        match best {
            Some((idx, _)) => {
                self.selected_idx = idx;
                self.list_state.select(Some(idx));
            }
            None => {
                self.list_state.select(None);
            }
        }
    }

    fn update_scroll(&mut self) {
        if let Some(node) = self.selected_node() {
            self.cursor_line = node.call.line;
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
            .filter(|n| n.parent_id.is_none())
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
        self.visible_nodes.push(node_id);

        // Get children and visibility
        let (children, children_visible) = {
            let node = self.nodes.iter().find(|n| n.id == node_id);
            match node {
                Some(n) => (n.children.clone(), n.children_visible),
                None => return,
            }
        };

        if children_visible {
            for child_id in children {
                self.collect_visible_nodes(child_id);
            }
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

        // Get node info
        let (
            name,
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
        ) = {
            let node = match self.get_node(node_id) {
                Some(n) => n,
                None => return,
            };
            (
                node.call.name.clone(),
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
            )
        };

        if already_expanded {
            self.status = format!("'{}' already expanded. Press Enter to undo.", name);
            return;
        }

        // Find matching trace (blocks until found or iterator exhausted)
        let (trace_result, _logs) = self
            .expansion_cache
            .find_trace_for_tokens(&input, &arguments, &name, kind);
        let expanded_text = match trace_result {
            Some(text) => text,
            None => {
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

                let (nearest, total) = self.expansion_cache.get_nearest_traces(&input, 8);
                let available_str = if nearest.is_empty() {
                    "No trace data available.\n\
                     Make sure -Z trace-macros is producing output.\n\
                     Some macros (built-in attributes, compiler intrinsics) don't produce traces."
                        .to_string()
                } else {
                    let matches: Vec<String> = nearest
                        .iter()
                        .map(|(s, score)| format!("{:.0}% {}", score * 100.0, s))
                        .collect();
                    format!(
                        "Nearest matches ({} total traces):\n  {}",
                        total,
                        matches.join("\n  ")
                    )
                };
                self.error_message = Some(format!(
                    "Expansion Error: No trace found for '{}' (type: {})\n\
                     Tokens: {}\n\n\
                     {}\n\n\
                     Press Enter to dismiss.",
                    name,
                    kind.as_str(),
                    if input.len() > 50 {
                        format!("{}...", &input[..47])
                    } else {
                        input.clone()
                    },
                    available_str
                ));
                return;
            }
        };

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
                            && n.call.line == line
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

        // For functional macros, replace only the macro call, keeping surrounding code
        let (formatted_lines, lines_removed) = if kind == MacroKind::Functional && line == line_end
        {
            // Single-line functional macro: replace only the macro call
            let orig_line = self.source_lines.get(line_idx).cloned().unwrap_or_default();

            // Extract parts before and after the macro call
            let before_macro = if col_start < orig_line.len() {
                &orig_line[..col_start]
            } else {
                ""
            };
            let after_macro = if col_end < orig_line.len() {
                orig_line[col_end..].trim_start()
            } else {
                ""
            };

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

            let before_macro = if col_start < orig_line.len() {
                &orig_line[..col_start]
            } else {
                ""
            };
            let after_macro = if col_end < end_orig_line.len() {
                end_orig_line[col_end..].trim_start()
            } else {
                ""
            };

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

        // Update line numbers for all nodes that come after this line
        if lines_added != 0 {
            for node in &mut self.nodes {
                if node.id != node_id && node.call.line > line {
                    node.call.line = (node.call.line as isize + lines_added) as usize;
                    node.call.line_end = (node.call.line_end as isize + lines_added) as usize;
                    node.call.item_line_end =
                        (node.call.item_line_end as isize + lines_added) as usize;
                }
            }
        }

        // For derive macros: snapshot and update existing sibling derive nodes
        // (only relevant for child-level derives where siblings already exist as nodes)
        let mut derive_sibling_snapshot = Vec::new();
        if kind == MacroKind::Derive {
            let has_remaining_line = !remaining_derives.is_empty();

            // Snapshot sibling state before modification (for undo)
            for node in &self.nodes {
                if node.id != node_id
                    && node.call.kind == MacroKind::Derive
                    && node.call.line == line
                    && !node.expanded
                {
                    derive_sibling_snapshot.push((
                        node.id,
                        node.original_lines.clone(),
                        node.call.line,
                        node.call.line_end,
                        node.call.item_line_end,
                    ));
                }
            }

            // Update sibling derives
            for node in &mut self.nodes {
                if node.id != node_id
                    && node.call.kind == MacroKind::Derive
                    && node.call.line == line
                    && !node.expanded
                {
                    if has_remaining_line {
                        // The remaining #[derive(...)] line is the last line in formatted_lines
                        let remaining_line_pos = line_idx + num_expanded_lines - 1; // 0-indexed
                        node.call.line = remaining_line_pos + 1; // 1-indexed
                        node.call.line_end = remaining_line_pos + 1;
                        // Update original_lines to the new remaining derive line
                        if let Some(new_line) = self.source_lines.get(remaining_line_pos) {
                            node.original_lines = vec![new_line.clone()];
                        }
                    }
                    // Adjust item_line_end by the net line change
                    node.call.item_line_end =
                        (node.call.item_line_end as isize + lines_added) as usize;
                }
            }
        }

        // Mark node as expanded and store expanded content
        if let Some(node) = self.get_node_mut(node_id) {
            node.expanded = true;
            node.expanded_content = Some(expanded_content.clone());
            node.children_visible = true;
            node.derive_sibling_snapshot = derive_sibling_snapshot;
        }

        // Parse expanded content to find child macros (use content without markers)
        let child_macros = find_macros(&content_for_parsing);

        // Create child nodes
        let mut child_ids = Vec::new();
        for child_mac in child_macros {
            let child_id = self.next_id;
            self.next_id += 1;

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
                call: MacroCall {
                    name: child_mac.name,
                    kind: child_mac.kind,
                    line: adjusted_line,
                    col_start: child_mac.col_start,
                    col_end: child_mac.col_end,
                    line_end: adjusted_line + (child_mac.line_end - child_mac.line),
                    item_line_end: adjusted_line + (child_mac.item_line_end - child_mac.line),
                    input: child_mac.input,
                    arguments: child_mac.arguments,
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
                derive_sibling_snapshot: Vec::new(),
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

        let (name, line, kind, expanded, original_lines, derive_sibling_snapshot) = {
            let node = match self.get_node(node_id) {
                Some(n) => n,
                None => return,
            };
            (
                node.call.name.clone(),
                node.call.line,
                node.call.kind,
                node.expanded,
                node.original_lines.clone(),
                node.derive_sibling_snapshot.clone(),
            )
        };

        if !expanded {
            self.status = format!("'{}' is not expanded", name);
            return;
        }

        // Compute actual line count accounting for expanded children
        let num_expanded_lines = self.actual_expanded_line_count(node_id);
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
            // Insert the original lines back with their original line numbers
            for (i, orig) in original_lines.iter().enumerate() {
                self.source_lines.insert(line_idx + i, orig.clone());
                // Restore the original line number (1-indexed)
                self.line_origins.insert(line_idx + i, Some(line + i));
            }
        }

        // Update line numbers for all nodes that come after this line
        if lines_delta != 0 {
            for node in &mut self.nodes {
                if node.id != node_id && node.call.line > line {
                    node.call.line = (node.call.line as isize - lines_delta) as usize;
                    node.call.line_end = (node.call.line_end as isize - lines_delta) as usize;
                    node.call.item_line_end =
                        (node.call.item_line_end as isize - lines_delta) as usize;
                }
            }
        }

        // For derive macros: restore sibling derive nodes from snapshot
        if kind == MacroKind::Derive {
            for (sib_id, sib_original_lines, sib_line, sib_line_end, sib_item_line_end) in
                &derive_sibling_snapshot
            {
                if let Some(sib_node) = self.get_node_mut(*sib_id) {
                    sib_node.original_lines = sib_original_lines.clone();
                    sib_node.call.line = *sib_line;
                    sib_node.call.line_end = *sib_line_end;
                    sib_node.call.item_line_end = *sib_item_line_end;
                }
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
            node.derive_sibling_snapshot.clear();
        }

        self.rebuild_visible_nodes();
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

        // Touch source files to force recompilation (cargo skips unchanged crates)
        if let Some(ref manifest_path) = self.trace_macros.args().manifest_path {
            let manifest = PathBuf::from(manifest_path);
            if let Some(dir) = manifest.parent() {
                let src_dir = dir.join("src");
                if src_dir.is_dir() {
                    // Touch all .rs files in src/
                    if let Ok(entries) = std::fs::read_dir(&src_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                                let _ = filetime::set_file_mtime(&path, filetime::FileTime::now());
                            }
                        }
                    }
                }
            }
        }

        match self.trace_macros.run() {
            Ok(iter) => {
                self.expansion_cache = ExpansionCache::new(iter);
                self.status = "Reloaded trace data.".to_string();
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
        let rest = if let Some(rest) = trimmed.strip_prefix("mod ") {
            rest
        } else if let Some(after_pub) = trimmed.strip_prefix("pub ") {
            // Handle `pub mod`, `pub(crate) mod`, `pub(super) mod`, etc.
            if let Some(rest) = after_pub.strip_prefix("mod ") {
                rest
            } else if after_pub.starts_with('(') {
                // Find closing paren then look for `mod `
                if let Some(close) = after_pub.find(')') {
                    after_pub[close + 1..].trim_start().strip_prefix("mod ")?
                } else {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
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
            Ok(s) => s,
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
            file_path: self.file_path.clone(),
            module_path: self.module_path.clone(),
        };
        self.module_stack.push(saved);

        // Set up new module state
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        let line_origins: Vec<Option<usize>> = (1..=source_lines.len()).map(|n| Some(n)).collect();
        let macros = find_macros(&source);

        let mut nodes = Vec::new();
        let mut next_id = 0;
        let mut item_first_attr: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        for mac in macros {
            if matches!(mac.kind, MacroKind::Attribute | MacroKind::Derive) {
                if !item_first_attr.insert(mac.item_line_end) {
                    continue;
                }
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

            nodes.push(MacroNode {
                call: mac,
                id: next_id,
                parent_id: None,
                depth: 0,
                expanded: false,
                expansion_failed: false,
                original_lines,
                expanded_content: None,
                children: Vec::new(),
                children_visible: true,
                derive_sibling_snapshot: Vec::new(),
            });
            next_id += 1;
        }

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
    let metadata = cmd.exec().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("cargo metadata failed: {}", e),
        )
    })?;

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

/// Find the macra-hook shared library
fn find_hook_lib() -> Option<PathBuf> {
    let lib_name = if cfg!(target_os = "macos") {
        "libmacra_hook.dylib"
    } else if cfg!(target_os = "windows") {
        "macra_hook.dll"
    } else {
        "libmacra_hook.so"
    };

    // Try to find in the same directory as cargo-macra
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let hook_lib = dir.join(lib_name);
            if hook_lib.exists() {
                return Some(hook_lib);
            }
        }
    }

    // Try common locations
    let paths = [
        PathBuf::from(format!("./target/debug/{}", lib_name)),
        PathBuf::from(format!("./target/release/{}", lib_name)),
    ];

    for path in paths {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn build_trace_macros(args: &Args) -> TraceMacros {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let tm_args = macra::trace_macros::Args {
        package: args.package.clone(),
        bin: args.bin.clone(),
        lib: args.lib,
        test: args.test.clone(),
        example: args.example.clone(),
        manifest_path: args.manifest_path.clone(),
        cargo_args: args.cargo_args.clone(),
        hook_lib: find_hook_lib(),
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

    loop {
        terminal.draw(|frame| ui(frame, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // If error message is displayed, dismiss it on Enter
                    if app.error_message.is_some() {
                        if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                            app.error_message = None;
                        }
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down | KeyCode::Char('j') => app.cursor_down(),
                        KeyCode::Up | KeyCode::Char('k') => app.cursor_up(),
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
                        KeyCode::Char('r') => app.reload_trace(),
                        KeyCode::Char('n') => app.jump_to_next_macro(),
                        KeyCode::Char('N') => app.jump_to_prev_macro(),
                        KeyCode::Tab => app.next(),
                        KeyCode::BackTab => app.previous(),
                        KeyCode::Char(' ') => app.toggle_children(),
                        KeyCode::PageDown => {
                            for _ in 0..10 {
                                app.cursor_down();
                            }
                        }
                        KeyCode::PageUp => {
                            for _ in 0..10 {
                                app.cursor_up();
                            }
                        }
                        KeyCode::Home | KeyCode::Char('g') => {
                            app.cursor_line = 1;
                            app.ensure_cursor_visible();
                            app.sync_selection_to_cursor();
                        }
                        KeyCode::End | KeyCode::Char('G') => {
                            app.cursor_line = app.source_lines.len().max(1);
                            app.ensure_cursor_visible();
                            app.sync_selection_to_cursor();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
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
    // Collect node data first to avoid borrow issues
    let node_data: Vec<_> = app
        .visible_nodes
        .iter()
        .filter_map(|&id| app.get_node(id).cloned())
        .collect();

    let items: Vec<ListItem> = node_data
        .iter()
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
                Span::styled(&node.call.name, name_style),
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

    let source_lines: Vec<Line> = app
        .source_lines
        .iter()
        .zip(app.line_origins.iter())
        .enumerate()
        .map(|(i, (line, origin))| {
            let display_idx = i + 1; // 1-indexed position in the display
            let is_cursor = display_idx == cursor_line;
            let is_expanded = origin.is_none();

            // Format line number: show original number or blank for expanded lines
            let line_num_str = match origin {
                Some(n) => format!("{:4} │ ", n),
                None => "     │ ".to_string(),
            };

            let line_num_style = if is_cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::DarkGray)
                    .bold()
            } else if is_expanded {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let content_style = if is_cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::DarkGray)
                    .bold()
            } else if is_expanded {
                Style::default().fg(Color::Green)
            } else {
                colorize_line(line)
            };

            Line::from(vec![
                Span::styled(line_num_str, line_num_style),
                Span::styled(line.as_str(), content_style),
            ])
        })
        .collect();

    let mod_display = app.module_path_display();
    let title = if let Some(node) = app.selected_node() {
        format!(
            " {} - {} at line {} (depth {}) ",
            mod_display, node.call.name, node.call.line, node.depth
        )
    } else {
        format!(" {} ", mod_display)
    };

    let paragraph = Paragraph::new(source_lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((app.scroll_offset, 0));

    frame.render_widget(paragraph, main_chunks[1]);

    // Bottom status bar with key guide
    let key_guide = " j/k=↑↓  g/G=top/bottom  n/N=next/prev macro  Enter=expand/mod  BS=back  r=reload  q=quit ";
    let status_text = if app.status.is_empty() {
        key_guide.to_string()
    } else {
        format!("{} | {}", app.status, key_guide)
    };
    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(status, chunks[1]);

    // Error popup (if any)
    if let Some(ref error_msg) = app.error_message {
        let area = frame.area();
        let popup_width = (area.width * 80 / 100).min(80);
        let popup_height = (area.height * 60 / 100).min(20);
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

/// Simple syntax coloring for Rust code.
fn colorize_line(line: &str) -> Style {
    let trimmed = line.trim();

    // Comments (highest priority)
    if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
        return Style::default().fg(Color::DarkGray);
    }

    // Attributes
    if trimmed.starts_with("#[") || trimmed.starts_with("#![") {
        return Style::default().fg(Color::Yellow);
    }

    // Functional macro invocations (check before keywords so `let x = vec![...]` gets cyan)
    if trimmed.contains("!(")
        || trimmed.contains("! (")
        || trimmed.contains("!{")
        || trimmed.contains("![")
    {
        return Style::default().fg(Color::Cyan);
    }

    // External module declarations (`mod foo;` / `pub mod foo;`) — navigable with Enter
    if is_mod_declaration(trimmed) {
        return Style::default().fg(Color::Cyan).bold();
    }

    // Keywords
    if trimmed.starts_with("fn ")
        || trimmed.starts_with("pub ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("const ")
        || trimmed.starts_with("static ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("trait ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("mod ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("where ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("else ")
        || trimmed.starts_with("match ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("loop ")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("async ")
        || trimmed.starts_with("await ")
    {
        return Style::default().fg(Color::Blue);
    }

    // Strings
    if trimmed.contains('"') {
        return Style::default().fg(Color::Green);
    }

    Style::default().fg(Color::White)
}

/// Check if a trimmed line is an external module declaration (`mod foo;`).
fn is_mod_declaration(trimmed: &str) -> bool {
    let rest = if let Some(rest) = trimmed.strip_prefix("mod ") {
        rest
    } else if let Some(after_pub) = trimmed.strip_prefix("pub ") {
        if let Some(rest) = after_pub.strip_prefix("mod ") {
            rest
        } else if after_pub.starts_with('(') {
            match after_pub.find(')') {
                Some(close) => match after_pub[close + 1..].trim_start().strip_prefix("mod ") {
                    Some(rest) => rest,
                    None => return false,
                },
                None => return false,
            }
        } else {
            return false;
        }
    } else {
        return false;
    };
    let rest = rest.trim();
    rest.ends_with(';') && !rest.contains('{')
}

/// Resolve a module path (e.g., "foo::bar") relative to the top-level source file.
/// Returns the resolved file path and the full module path segments including "crate".
fn resolve_module_path(
    top_level: &PathBuf,
    module_str: &str,
) -> io::Result<(PathBuf, Vec<String>)> {
    let segments: Vec<&str> = module_str.split("::").filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Ok((top_level.clone(), vec!["crate".to_string()]));
    }

    let mut current_file = top_level.clone();
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
    let source = std::fs::read_to_string(&src_path)?;

    eprintln!("Running cargo with -Z trace-macros...");
    let tm = build_trace_macros(&args);
    let iter = tm.run()?;

    if args.show_expansion {
        let expansions: Vec<_> = iter.collect::<io::Result<Vec<_>>>()?;
        print_expansions(&expansions);
        return Ok(());
    }

    let cache = ExpansionCache::new(iter);
    run_app(source, src_path, module_path, cache, tm)
}

/// Print all macro expansions to stdout in a human-readable format.
///
/// Each expansion is printed as:
///   == caller_pattern ==
///   input tokens
///   ---
///   output tokens
fn print_expansions(expansions: &[macra::parse_trace::MacroExpansion]) {
    use macra::parse_trace::MacroExpansionKind;

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

        println!("== {} ==", caller);
        if !expansion.input.is_empty() {
            println!("{}", expansion.input);
        }
        println!("---");
        println!("{}", expansion.to);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_matches_falls_back_for_truncated_bang_input() {
        let exp = MacroExpansion {
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
            MacroKind::Functional
        ));
    }

    #[test]
    fn expansion_matches_keeps_strict_input_when_present() {
        let exp = MacroExpansion {
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
            MacroKind::Functional
        ));
    }
}
