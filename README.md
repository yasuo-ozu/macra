# cargo-macra [![Latest Version]][crates.io] [![Documentation]][docs.rs] [![Check]][check-link] [![1.86-1.98]][versions] [![Nightly]][nightly-link] [![MSRV]][msrv-link] [![License]][license-link]

<p align="center">
  <img src="https://raw.githubusercontent.com/yasuo-ozu/macra/refs/heads/main/logo.png" alt="cargo-macra logo" width="320" />
</p>

[Latest Version]: https://img.shields.io/crates/v/cargo-macra.svg
[crates.io]: https://crates.io/crates/cargo-macra
[Documentation]: https://img.shields.io/docsrs/cargo-macra
[docs.rs]: https://docs.rs/cargo-macra/latest/cargo_macra/
[Check]: https://github.com/yasuo-ozu/macra/actions/workflows/check.yml/badge.svg
[check-link]: https://github.com/yasuo-ozu/macra/actions/workflows/check.yml
[1.86-1.98]: https://github.com/yasuo-ozu/macra/actions/workflows/1.86-1.98.yml/badge.svg
[versions]: https://github.com/yasuo-ozu/macra/actions/workflows/1.86-1.98.yml
[Nightly]: https://github.com/yasuo-ozu/macra/actions/workflows/nightly.yml/badge.svg
[nightly-link]: https://github.com/yasuo-ozu/macra/actions/workflows/nightly.yml
[MSRV]: https://img.shields.io/badge/MSRV-1.86-blue.svg
[msrv-link]: https://github.com/yasuo-ozu/macra/blob/main/Cargo.toml
[License]: https://img.shields.io/crates/l/cargo-macra.svg
[license-link]: https://github.com/yasuo-ozu/macra/blob/main/LICENSE


Interactive Rust macro expansion viewer with a terminal UI.

## Motivation

`cargo-macra` is focused on understanding and debugging macro-heavy Rust code incrementally.
Instead of dumping all expanded code at once, it lets you step through expansions in source
context, which is useful for:

- learning how macro expansion works
- debugging complex macro crates where one macro expands into more macro calls
- investigating specific macro invocations when tools like rust-analyzer `expandMacro` or
  `cargo expand` are too all-at-once for practical debugging

## Screenshot

[![asciicast](https://asciinema.org/a/8ZoXg8XHY8jnC8PW.svg)](https://asciinema.org/a/8ZoXg8XHY8jnC8PW)

## Supported rustc versions and platforms

| Category | Supported |
| --- | --- |
| rustc versions (CI) | `1.86` through `1.98`, plus `nightly` |
| Platforms (CI) | `ubuntu-latest`, `ubuntu-24.04-arm`, `windows-latest`, `windows-11-arm`, `macos-latest` (Apple Silicon), `macos-15-intel` (Intel) |

## Install

### From crates.io (when published)

```bash
cargo install cargo-macra
```

### From source

```bash
git clone https://github.com/yasuo-ozu/macra.git
cd macra
cargo install --path .
```

## Usage

As a cargo subcommand:

```bash
cargo macra --manifest-path /path/to/Cargo.toml
```

Direct binary invocation:

```bash
cargo-macra --manifest-path /path/to/Cargo.toml
```

Open a specific module first:

```bash
cargo macra --manifest-path /path/to/Cargo.toml foo::bar
```

Print traced expansions without launching TUI:

```bash
cargo macra --manifest-path /path/to/Cargo.toml --show-expansion
```

## CLI Options

```text
  cargo macra [MODULE]                          launch the TUI (default)
  cargo macra list   [MODULE] [--macro SEL]...  numbered listing of macros
  cargo macra expand [MODULE] [--macro SEL]...  expanded source on stdout

`--macro SEL` (repeatable) selects which macro(s) `list`/`expand` act on: a 1-based number from `list`'s own output, or a macro name matched on its last path segment. With none, `expand` expands everything reachable.

Arguments:
  [MODULE]
          Module path to open (e.g., "foo::bar" opens the file for module `crate::foo::bar`). A module literally named `list` or `expand` needs a leading `--` ahead of it (`cargo macra -- expand`) so it is not mistaken for that subcommand
  [CARGO_ARGS]...
          Additional arguments to pass to cargo, after a literal `--` (e.g. `cargo macra expand foo -- --release`)

Options:
  -p, --package <PACKAGE>
          Package to check
      --bin <BIN>
          Build only the specified binary
      --lib
          Build only the specified library
      --test <TEST>
          Build only the specified test target
      --example <EXAMPLE>
          Build only the specified example
      --manifest-path <MANIFEST_PATH>
          Path to Cargo.toml
      --show-expansion
          Print all macro expansions to stdout and exit without launching the TUI
      --color <WHEN>
          Coloring of printed expansions
      --macro <SELECTOR>
          Select which macro `list`/`expand` operate on: a 1-based number from `list`'s output, or a macro name match
```

## TUI Keys

- `j` / `k`, `Up` / `Down`: Move cursor
- `h` / `l`, `Left` / `Right`: Move between macros that share the current line
- `g` / `G`, `Home` / `End`: Jump top/bottom
- `n` / `N`: Jump next/previous macro
- `Enter`: Expand/collapse macro or enter `mod` file
- `Backspace`: Return to parent module
- `Tab` / `Shift+Tab`: Move tree selection
- `Space`: Toggle child visibility in macro tree
- `v`: Toggle split view (compare original vs expanded)
- `PageUp` / `PageDown`: Move a screenful
- `r`: Reload trace data
- `q`: Quit
- `Esc`: Cancel a pending expansion, or dismiss the expansion-choice popup
- `Ctrl-C` / `Ctrl-D`: Quit, and cancel a pending expansion

`Esc` deliberately does not quit: it is the cancel key for a pending expansion, and an
`Esc` arriving just after the trace landed would otherwise exit the application.

### Split view

By default an expansion is inlined in place of the code it replaced. Press `v` to
compare the two instead: every expanded range becomes a two-column block with the
original source on the left and the macro output on the right.

```text
  91 │
  92 │ #[derive(Greet, Describe)]
     ├─ original ─────────────────┬─ expanded: Describe ──────────────
     │ #[derive(Greet, Describe)]  │ impl MultiDeriveOneAttr {
     │                             │     pub fn describe() -> String {
     │                             │         format!("{} is a struct", ..)
     │                             │     }
     │                             │ }
     │                             │ #[derive(Greet)]
     ├─────────────────────────────┴──────────────────────────────────
  93 │ pub struct MultiDeriveOneAttr;
```

### Non-interactive: `list` and `expand`

`cargo macra expand` prints expanded source like `cargo-expand`, and with no
arguments it expands everything:

```bash
cargo macra expand
```

Unlike `cargo-expand`, you can expand *one macro at a time*. `cargo macra list`
numbers what can be expanded (compiler built-ins and derive helper attributes are
left out, since they have no expansion to show):

```bash
$ cargo macra list
  1. attr    add_hello_method  (line 21)
  2. fn      make_answer  (line 27)
  3. derive  Greet  (line 30)
```

Add `-C N` to see the surrounding source, so an entry can be recognised without
opening the file:

```bash
$ cargo macra list -C 2
  3. derive  Greet  (line 30)
          28 │ 
          29 │ // Derive macro (emits format! + stringify!)
      >   30 │ #[derive(Greet)]
          31 │ pub struct Greeter;
```

`--macro` then picks one, by number or by name (the first macro with that name):

```bash
cargo macra expand --macro 3        # print just that derive's expansion
cargo macra expand --macro Greet    # the same one, by name
```

With no `--macro`, `expand` prints the whole file like `cargo-expand`. With one, it
prints only that macro's own result.

`--macro` repeats to go *deeper*, because a macro's expansion usually contains
more macros. `list` shows what is inside a result, and `expand` shows the result
itself:

```bash
$ cargo macra list --macro Greet    # what appears inside Greet's expansion
  1. fn      format  (line 33)

$ cargo macra expand --macro Greet --macro 1    # ... and expand that too
```

So each `--macro` is one step down a path, and the numbers are always the ones
the matching `list` just printed. A `--` may separate the subcommand from a
module path, which is also how to reach a module whose name collides with a
subcommand:

```bash
cargo macra list -- foo::bar
cargo macra -- expand              # the module `expand`, not the subcommand
```

## Development

```bash
cargo build
cargo test
```

## License

MIT
