# cargo-macra [![Latest Version]][crates.io] [![Documentation]][docs.rs] [![GitHub Actions]][actions]

[Latest Version]: https://img.shields.io/crates/v/cargo-macra.svg
[crates.io]: https://crates.io/crates/cargo-macra
[Documentation]: https://img.shields.io/docsrs/cargo-macra
[docs.rs]: https://docs.rs/cargo-macra/latest/cargo_macra/
[GitHub Actions]: https://github.com/yasuo-ozu/macra/actions/workflows/rust.yml/badge.svg
[actions]: https://github.com/yasuo-ozu/macra/actions/workflows/rust.yml

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
  -h, --help                           Print help
```

## TUI Keys

- `j` / `k`, `Up` / `Down`: Move cursor
- `g` / `G`, `Home` / `End`: Jump top/bottom
- `n` / `N`: Jump next/previous macro
- `Enter`: Expand/collapse macro or enter `mod` file
- `Backspace`: Return to parent module
- `Tab` / `Shift+Tab`: Move tree selection
- `Space`: Toggle child visibility in macro tree
- `r`: Reload trace data
- `q` / `Esc`: Quit

## Development

```bash
cargo build
cargo test
```

## License

MIT (stub files included in this repository; fill in copyright owner/year).
