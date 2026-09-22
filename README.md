# cargo-macra [![Latest Version]][crates.io] [![Documentation]][docs.rs] [![Check]][check] [![1.86-1.98]][versions] [![Nightly]][nightly] [![MSRV]][msrv] [![License]][license]

<p align="center">
  <img src="https://raw.githubusercontent.com/yasuo-ozu/macra/refs/heads/main/logo.png" alt="cargo-macra logo" width="320" />
</p>

[Latest Version]: https://img.shields.io/crates/v/cargo-macra.svg
[crates.io]: https://crates.io/crates/cargo-macra
[Documentation]: https://img.shields.io/docsrs/cargo-macra
[docs.rs]: https://docs.rs/cargo-macra/latest/cargo_macra/
[Check]: https://github.com/yasuo-ozu/macra/actions/workflows/check.yml/badge.svg
[check]: https://github.com/yasuo-ozu/macra/actions/workflows/check.yml
[1.86-1.98]: https://github.com/yasuo-ozu/macra/actions/workflows/1.86-1.98.yml/badge.svg
[versions]: https://github.com/yasuo-ozu/macra/actions/workflows/1.86-1.98.yml
[Nightly]: https://github.com/yasuo-ozu/macra/actions/workflows/nightly.yml/badge.svg
[nightly]: https://github.com/yasuo-ozu/macra/actions/workflows/nightly.yml
[MSRV]: https://img.shields.io/badge/MSRV-1.86-blue.svg
[msrv]: https://github.com/yasuo-ozu/macra/blob/main/Cargo.toml
[License]: https://img.shields.io/crates/l/cargo-macra.svg
[license]: https://github.com/yasuo-ozu/macra/blob/main/LICENSE


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
Usage: cargo macra [OPTIONS] [MODULE] [CARGO_ARGS]...

Arguments:
  [MODULE]         Module path to open (e.g., "foo::bar")
  [CARGO_ARGS]...  Additional arguments to pass to cargo

Options:
  -p, --package <PACKAGE>              Package to check
      --bin <BIN>                      Build only the specified binary
      --lib                            Build only the specified library
      --test <TEST>                    Build only the specified test target
      --example <EXAMPLE>              Build only the specified example
      --manifest-path <MANIFEST_PATH>  Path to Cargo.toml
      --show-expansion                 Print expansions and exit
      --color <WHEN>                   Coloring of printed expansions
                                       [default: auto] [possible values: auto, always, never]
  -h, --help                           Print help
```

Printed expansions are pretty-printed with `prettyplease` and syntax-highlighted.
With `--color auto` (the default) colors are emitted only when stdout is a
terminal; `NO_COLOR` and `TERM=dumb` disable them as well.

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
original source on the left and the macro output on the right. Code outside those
ranges stays full width, so only what actually changed is split.

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

Both columns are syntax highlighted; the original is dimmed to keep the expansion
in focus. Nested expansions do not nest columns — only the outermost expanded range
of a nest is split, since inner expansions already sit inside its output.

## Development

```bash
cargo build
cargo test
```

## License

MIT
