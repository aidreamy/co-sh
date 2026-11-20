# morpheus-sdk

[![Crates.io](https://img.shields.io/crates/v/morpheus-sdk.svg)](https://crates.io/crates/morpheus-sdk)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/aidreamy/morpheus/blob/main/LICENSE)

SDK crate for [Morpheus](https://github.com/aidreamy/morpheus), a coding agent for the terminal.

`morpheus-sdk` consolidates the reusable building blocks of a coding agent into a single library: a content-hash file patch format with session rollback, tolerant tool-call extraction, an embedded LSP client stack, and AST/code intelligence over 56 tree-sitter grammars. It is a plain library with no dependency on the Morpheus app, so you can build your own agent on it alone.

The multi-provider LLM connector is not part of this crate. It lives in the separate [`aiface`](https://crates.io/crates/aiface) crate, which this SDK does not depend on.

## Highlights

- **The hashline patch format**: content-hash-tagged multi-file edits with all-or-nothing preflight, stale-tag recovery, CRLF/BOM tolerance and streaming parse.
- **Session rollback**: bounded file-version history per session; restore by hash, step back, or undo a restore.
- **Tolerant tool-call extraction**: turns raw LLM output (streamed or batch, inline JSON or native tool calls) into validated tool calls, with rejection messages designed to teach the model to retry.
- **Embedded LSP stack**: tokio-native client with a curated 26-server catalog (rust-analyzer, gopls, pyright, clangd, and others), lazy spawn, backpressure, and model-facing diagnostic formatting.
- **Code intelligence**: ast-grep structural search/rewrite across 56 tree-sitter grammars, LRU-cached incremental parsing, and syntax highlighting.

## Usage

```rust
use morpheus_sdk::hashline::format::{compute_file_hash, format_hashline_header};

let source = "fn main() {}\n";
let tag = compute_file_hash(source);
println!("{}", format_hashline_header("src/main.rs", &tag));
```

Modules: `hashline` (patch engine), `rollback` (file versioning), `extract_action` (tool-call parsing), `lsp` (language servers), `ast` / `tree_sitter` / `find` (code search and parsing).

More runnable examples in [`examples/`](https://github.com/aidreamy/morpheus/tree/main/crates/morpheus-sdk/examples); run them with `cargo run -p morpheus-sdk --example hashline-patcher` (also `rollback`, `find`, `ast-ops`, `tree-sitter`, `extract-action`).

## Requirements

- Rust 2024 edition (≥ 1.85)
- LSP servers installed on `PATH` separately (no auto-install)

Note: the tree-sitter grammar dependencies (56 crates) are the most significant compile-time cost. If you only need the patch engine, compiling is faster on a stable toolchain cache.

## Related crates

- [`morpheus-tools`](https://crates.io/crates/morpheus-tools): the reference tool implementations (fs, bash, web, computer control, and others)
- [`morpheus-recall`](https://crates.io/crates/morpheus-recall): memory and context recall
- [`morpheus-tui`](https://crates.io/crates/morpheus-tui): terminal interface

## License

Apache-2.0. See [LICENSE](https://github.com/aidreamy/morpheus/blob/main/LICENSE).
