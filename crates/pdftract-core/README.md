# pdftract-core

[![docs.rs](https://docs.rs/pdftract-core/badge.svg)](https://docs.rs/pdftract-core)

The core Rust library for PDF text extraction. This crate provides the parsing, layout analysis, font encoding recovery, and text extraction primitives used by the CLI (`pdftract-cli`) and Python bindings (`pdftract-py`).

## Cargo.lock Policy

This workspace checks in `Cargo.lock` at the repository root. This is unconventional for library crates—the Cargo Book historically suggested that only binary crates should check in lockfiles, allowing library consumers to resolve their own dependency versions.

pdftract departs from this convention for **release reproducibility**:

1. **SLSA Level 3 provenance** requires that every milestone tag produces byte-identical artifacts across builds. Without a checked-in lockfile, two runs of `cargo build` on the same commit can resolve different transitive dependency versions, producing different binary hashes.

2. **Multi-output artifacts**—this workspace produces Rust crates (`pdftract-core`, `pdftract-cli`), Python wheels (`pdftract-py`), and Docker images. All must be built from the same dependency tree.

3. **Supply-chain security**—the lockfile pins checksums for all transitive dependencies, enabling `cargo audit` to detect yanked or compromised crates.

4. **Downstream consumers** can still ignore the lockfile if needed. Cargo allows `cargo build --frozen` with a local lockfile override, or consumers can vendor the crate with their own dependency resolution.

The tradeoff—occasional merge conflicts when PRs update overlapping dependencies—is worth the guarantee of reproducible releases. See `CONTRIBUTING.md` for the lockfile-update workflow.

## Modules

- `parser`: PDF spec parsing (xref, trailer, object streams, indirect references)
- `font`: Font encoding recovery, glyph name lookup, fingerprinting
- `layout`: Page layout analysis, region segmentation, reading order
- `extract`: Text extraction with provenance (bounding boxes, confidence scores)
- `ocr`: Tesseract integration for raster pages

## Testing

### SDK Conformance Tests

The `conformance` integration test validates that `pdftract-core`'s public API satisfies the SDK contract shared across all language implementations. The test rig runs shared conformance cases from `tests/sdk-conformance/cases.json` and verifies correct behavior for all 9 SDK contract methods.

```bash
# Run the conformance suite
cargo test --test conformance

# Run with specific features
cargo test --test conformance --features ocr,profiles,remote,receipts
```

The conformance suite covers:
- `extract` — Full extraction with structured Document output
- `extract_text` — Plain text extraction
- `extract_markdown` — Markdown-formatted extraction with tables and headings
- `extract_stream` — Streaming NDJSON extraction for large documents
- `search` — Pattern search with regex and case-insensitive options
- `get_metadata` — PDF metadata (page count, title, author, creator)
- `hash` — Content fingerprinting (SHA256) with fast hash variant
- `classify` — Document classification with category and confidence
- `verify_receipt` — Receipt verification against signed metadata

Each test case validates expected results with numeric tolerances for bounding boxes and confidence scores. Feature-gated tests (OCR, decryption, classification, receipts, remote) skip automatically when the corresponding feature is not compiled.

See `CONTRIBUTING.md` for more details on the conformance suite and adding new test cases.

## Usage

```rust
use pdftract_core::{extract_text, ExtractOptions};

let options = ExtractOptions::default();
let result = extract_text("document.pdf", &options)?;
println!("{}", result.text);
```
