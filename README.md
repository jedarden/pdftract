# pdftract

[![crates.io](https://img.shields.io/crates/v/pdftract)](https://crates.io/crates/pdftract)
[![PyPI](https://img.shields.io/pypi/v/pdftract)](https://pypi.org/project/pdftract/)
[![docs.rs](https://img.shields.io/docsrs/pdftract-core)](https://docs.rs/pdftract-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.78-orange)](https://blog.rust-lang.org/2024/05/02/Rust-1.78.0.html)

**pdftract** is a pure-Rust PDF text extraction library built for the cases where other tools give up: scanned documents, unusual font encodings, multi-column layouts, footnotes, mixed-mode pages, and encrypted files. Where most extractors treat PDF text extraction as a coordinate sort, pdftract runs a full reading-order pipeline — segmenting layout regions, recovering broken font encodings, routing each page to the right extraction mode (vector, OCR, or hybrid), and emitting structured JSON with per-span provenance. If your PDFs are academic papers, legal filings, financial reports, or anything else that wasn't typeset in a word processor, pdftract is what you want.

## How it compares

| Capability | pdftract | pdfplumber | pypdf | pdfminer |
|---|---|---|---|---|
| Multi-column reading order | ✅ Full layout segmentation | ⚠ Heuristic | ❌ | ⚠ Partial |
| Footnotes & sidebars | ✅ | ❌ | ❌ | ❌ |
| Font encoding recovery | ✅ Glyph name → fingerprint → shape | ⚠ ToUnicode only | ⚠ ToUnicode only | ⚠ ToUnicode only |
| Scanned / mixed PDF (OCR) | ✅ Per-page hybrid routing | ❌ | ❌ | ❌ |
| PDF/UA structure tree | ✅ | ❌ | ⚠ Partial | ❌ |
| PDF decryption (RC4/AES) | ✅ (`decrypt` feature) | ⚠ Partial | ⚠ Partial | ⚠ Partial |
| Per-span bounding boxes + confidence | ✅ | ✅ | ❌ | ⚠ Partial |
| Streaming extraction (large files) | ✅ | ❌ | ❌ | ❌ |
| CJK scripts | ✅ (`cjk` feature) | ⚠ | ⚠ | ⚠ |
| HTTP microservice mode | ✅ (`serve`) | ❌ | ❌ | ❌ |
| Language | Rust + Python + C ABI | Python | Python | Python |

## Platform Support

| Platform | Status |
|----------|--------|
| Linux x86_64 | Fully CI-tested on every PR |
| Linux aarch64 | Fully CI-tested on every PR |
| macOS x86_64 | Build-tested; manually smoke-tested per release |
| macOS aarch64 | Build-tested; manually smoke-tested per release |
| Windows x86_64 | Build-tested; manually smoke-tested per release |

See [docs/operations/manual-platform-smoke.md](docs/operations/manual-platform-smoke.md) for the per-release smoke procedure.

## Installation

**Minimum Supported Rust Version (MSRV):** 1.78

### Cargo

```bash
cargo add pdftract-core
```

Or install the CLI:

```bash
cargo install pdftract
```

### pip

```bash
pip install pdftract
```

### Docker

```bash
docker pull ronaldraygun/pdftract:latest
```

### Homebrew

```bash
brew install pdftract
```

## Quickstart

### Rust

```rust
use pdftract_core::{extract_pdf, ExtractionOptions};

let opts = ExtractionOptions::default();
let doc = extract_pdf("report.pdf", &opts)?;

for page in &doc.pages {
    println!("Page {}: {} spans", page.number, page.spans.len());
}
```

Streaming extraction for large files:

```rust
use pdftract_core::extract_pdf_streaming;

for page in extract_pdf_streaming("large.pdf", &opts)? {
    let page = page?;
    process(page);
}
```

NDJSON output (one JSON object per page on stdout):

```rust
pdftract_core::extract_pdf_ndjson("report.pdf", &opts, std::io::stdout())?;
```

### Python

```python
import pdftract

doc = pdftract.extract("report.pdf")
print(f"{doc['metadata']['page_count']} pages")

for page in doc["pages"]:
    for span in page["spans"]:
        print(span["text"], span["bbox"], span["confidence"])
```

### CLI

```bash
# Extract to JSON
pdftract extract report.pdf --json output.json

# Plain text to stdout
pdftract extract report.pdf --text -

# Markdown output
pdftract extract report.pdf --markdown -

# Run as an HTTP microservice (POST /extract, GET /health)
pdftract serve --port 8080

# Compare two PDFs structurally
pdftract compare original.pdf revised.pdf

# Interactive page inspector
pdftract inspect report.pdf --page 3

# Diagnose extraction problems on a file
pdftract doctor report.pdf

# Validate PDF/UA or PDF/A conformance
pdftract validate report.pdf

# Stable content hash (for dedup / cache keys)
pdftract hash report.pdf

# Search for a pattern across pages
pdftract grep "invoice number" report.pdf

# Print page count and dimensions
pdftract pages report.pdf

# Classify each page (vector / scanned / mixed)
pdftract classify report.pdf

# Manage the local extraction cache
pdftract cache --list
pdftract cache --clear

# Migrate the local cache schema
pdftract migrate

# Verify a previously issued extraction receipt
pdftract verify-receipt receipt.json

# Generate client bindings from the C ABI headers
pdftract codegen --lang python

# Start the MCP (Model Context Protocol) server
pdftract mcp
```

## Features

All extraction functionality works out of the box. Optional features unlock heavier dependencies:

| Feature | What it adds | Enable with |
|---|---|---|
| `ocr` | Tesseract/Leptonica OCR for scanned and mixed pages | `cargo add pdftract-core --features ocr` |
| `decrypt` | RC4, AES-128, AES-256 PDF decryption | `cargo add pdftract-core --features decrypt` |
| `cjk` | CJK script support (Chinese, Japanese, Korean) | `cargo add pdftract-core --features cjk` |
| `full-render` | Full-page rasterization for assisted OCR and inspect UI | `cargo add pdftract-core --features full-render` |

In the Python wheel and Docker image, `ocr`, `decrypt`, and `cjk` are pre-enabled.

## What it does

**Correct reading order.** Most PDF extractors sort glyphs by Y then X coordinate. That breaks on multi-column articles, legal documents with sidebars, academic papers with footnotes, and anything typeset in a non-linear flow. pdftract segments each page into layout regions first, orders the regions, then emits text within each region — so the output reads the way a human would.

**Font encoding recovery.** PDFs can legally omit `ToUnicode` CMaps and describe only glyph IDs. When that happens, other extractors emit garbage or question marks. pdftract works through a layered recovery pipeline: glyph name lookup (standard and Adobe glyph lists), font fingerprinting against a known-font database, and finally glyph outline shape matching. Most documents that trip up other tools extract cleanly.

**Per-page hybrid routing.** Each page is independently classified as vector text, fully scanned (image-only), or mixed. Vector pages go through the fast extraction path. Scanned pages go to full OCR. Mixed pages use assisted OCR — vector spans anchor the OCR so it doesn't drift. This means one call handles an entire document regardless of how it was produced.

**Structure tree extraction.** PDF/UA and PDF/A files carry a logical structure tree (headings, paragraphs, tables, lists) separate from the visual rendering. pdftract reads this directly when present, so accessible PDFs yield structured output without heuristics.

**Structured output with provenance.** The primary output format is JSON. Every text span carries its bounding box, font name, point size, and a confidence score. This makes pdftract suitable as a preprocessing step for LLM pipelines, document indexing, and data extraction workflows that need to trace output back to the source page.

**Streaming extraction.** For large files, `extract_pdf_streaming` yields one page at a time so memory usage stays bounded regardless of document length.

## Available SDKs

pdftract ships multiple integration surfaces from a single Rust core:

| SDK | Package | Notes |
|---|---|---|
| Rust library | [`pdftract-core`](https://crates.io/crates/pdftract-core) on crates.io | Primary API |
| CLI binary | [`pdftract`](https://crates.io/crates/pdftract) on crates.io | Wraps the library |
| Python bindings | [`pdftract`](https://pypi.org/project/pdftract/) on PyPI | PyO3-based, wheels for Linux/macOS/Windows |
| C shared library | `libpdftract` | Stable C ABI; use `pdftract codegen` to generate FFI headers for your language |
| Docker image | [`ronaldraygun/pdftract`](https://hub.docker.com/r/ronaldraygun/pdftract) | Includes `serve` mode HTTP microservice |
| HTTP microservice | `pdftract serve` | REST API for language-agnostic integration |

Additional language SDK packages (Go, Node.js, Ruby) are in progress, built on top of the C ABI.

## Documentation

- **User guide:** [pdftract.com](https://pdftract.com)
- **API reference:** [docs.rs/pdftract-core](https://docs.rs/pdftract-core)
- **Extraction output schema:** [docs/research/extraction-output-schema.md](docs/research/extraction-output-schema.md)
- **SDK architecture:** [docs/notes/sdk-architecture.md](docs/notes/sdk-architecture.md)
- **Changelog:** [CHANGELOG.md](CHANGELOG.md)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)
- **Security policy:** [SECURITY.md](SECURITY.md)
- **Releases:** [GitHub Releases](https://github.com/jedarden/pdftract/releases)

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

at your option.
