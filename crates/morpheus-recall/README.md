# morpheus-recall

[![Crates.io](https://img.shields.io/crates/v/morpheus-recall.svg)](https://crates.io/crates/morpheus-recall)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/aidreamy/morpheus/blob/main/LICENSE)

Memory/recall crate for [Morpheus](https://github.com/aidreamy/morpheus), a coding agent for the terminal.

`morpheus-recall` is a small RAG (Retrieval-Augmented Generation) layer: embed text with a pluggable embedder, using local ONNX via [fastembed](https://crates.io/crates/fastembed) or a cloud provider via [`morpheus-sdk`](https://crates.io/crates/morpheus-sdk), and store/search the vectors in a [LanceDB](https://lancedb.github.io/lancedb/) table. The public surface is intentionally minimal: an `Embedder` enum, a high-level `Rag` index, and a lower-level `VecDb` wrapper.

## Highlights

- **Two embedder paths**: `Embedder::Local` (fastembed/ONNX, about 46 built-in models with auto-download from HuggingFace, CPU-only) or `Embedder::Cloud` (any provider reachable through the morpheus-sdk connector: OpenAI, Gemini, Ollama, and others).
- **Embedding-agnostic read path**: `VecDb::connect_readonly` takes a pre-computed query vector, so you can search with any embedder as long as the dimension matches the table schema.
- **Content-deduplicated ingest**: `ingest`/`ingest_batch` skip content that is already indexed.
- **Guardrails built in**: table-name and id validation, dimension checks, and typed errors (`RagError`, `VecDbError`).

## Usage

The API lives in `morpheus_recall::embed`, gated behind the `lancedb` feature (plus `fastembed` or `cloud` to construct an embedder):

```toml
[dependencies]
morpheus-recall = { version = "0.1.0", features = ["lancedb", "fastembed"] }
```

```rust
use morpheus_recall::embed::{Rag, Embedder};

// 1. Pick an embedder
let embedder = Embedder::try_new_local(EmbeddingModel::BGESmallENV15)?;

// 2. Open (or create) a LanceDB table
let rag = Rag::connect("data/lancedb", "my_docs", embedder).await?;

// 3. Ingest documents. Embedding and storage are automatic.
rag.ingest("doc-1", "Retrieval-Augmented Generation (RAG) combines ...").await?;

// 4. Search semantically
let results = rag.search("What is RAG?", 5).await?;
for entry in &results {
    println!("[{}] {}", entry.id, entry.content);
}
```

## Requirements

- Rust 2024 edition (≥ 1.85)
- `lancedb` feature: a local filesystem directory for the database URI
- `fastembed` feature: ONNX Runtime (via `ort`); models download on first use
- `cloud` feature: API keys for the chosen embedding provider

Note: with the `cloud` feature this crate depends on its sibling `morpheus-sdk` via a path dependency, so that combination is not yet resolvable from crates.io. Use it from the [repository](https://github.com/aidreamy/morpheus) directly.

## Related crates

- [`morpheus-sdk`](https://crates.io/crates/morpheus-sdk): agent harness SDK (powers the cloud embedder)
- [`morpheus-tools`](https://crates.io/crates/morpheus-tools): tool implementations (read-only `recall_search` tool)
- [`morpheus-tui`](https://crates.io/crates/morpheus-tui): terminal interface

## License

Apache-2.0. See [LICENSE](https://github.com/aidreamy/morpheus/blob/main/LICENSE).
