# pdftract

[![crates.io](https://img.shields.io/badge/crates.io-coming--soon-orange)](https://github.com/jedarden/pdftract/blob/main/docs/plan/plan.md)
[![PyPI](https://img.shields.io/badge/PyPI-coming--soon-orange)](https://github.com/jedarden/pdftract/blob/main/docs/plan/plan.md)
[![docs.rs](https://img.shields.io/badge/docs.rs-coming--soon-orange)](#documentation)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.78-orange)](https://blog.rust-lang.org/2024/05/02/Rust-1.78.0.html)

**pdftract** is a pure-Rust PDF text extraction library built for the cases where other tools give up: scanned documents, unusual font encodings, multi-column layouts, footnotes, mixed-mode pages, and encrypted files. Where most extractors treat PDF text extraction as a coordinate sort, pdftract runs a full reading-order pipeline — segmenting layout regions, recovering broken font encodings, routing each page to the right extraction mode (vector, OCR, or hybrid), and emitting structured JSON with per-span provenance. If your PDFs are academic papers, legal filings, financial reports, or anything else that wasn't typeset in a word processor, pdftract is what you want.

> **⚠️ Development status (evidence-audited 2026-09-15).** pdftract is pre-release and mid-stabilization (tracked in bead `pdftract-f19fd721`). The tables below describe the *designed* capability set; what is actually verified today:
>
> - **Extraction is broken at HEAD** — `pdftract extract` fails on the fixture corpus with `No /Root reference in trailer` (a page-tree parse regression). Every extraction-dependent path is blocked behind this fix. Evidence: `crates/pdftract-py/conformance-report.json` (2026-09-05: 32 conformance cases, **0 passing**); re-confirmed at HEAD on 2026-09-15 by direct CLI run.
> - **`pdftract serve` is broken at HEAD** — every request, including `GET /health`, returns HTTP 500 (missing axum `ConnectInfo` wiring; re-confirmed at HEAD on 2026-09-15).
> - **CI is not wired to git events yet** — nothing runs automatically on push/PR (bead `pdftract-a8d7bd1d`, open), and no completed end-to-end CI run has been retained — the CI cluster holds zero pdftract workflow runs (the stabilization baseline records "no retained end-to-end CI proof").
> - **Nothing is published** — `pdftract-core` is on neither crates.io nor docs.rs; no wheels, images, or release archives exist yet (the single `v0.1.0-test` tag exists for release-cascade testing only).

## How it compares

| Capability | pdftract | pdfplumber | pypdf | pdfminer |
|---|---|---|---|---|
| Multi-column reading order | 🚧 Full layout segmentation¹ | ⚠ Heuristic | ❌ | ⚠ Partial |
| Footnotes & sidebars | 🚧¹ | ❌ | ❌ | ❌ |
| Font encoding recovery | 🚧 Glyph name → fingerprint → shape¹ | ⚠ ToUnicode only | ⚠ ToUnicode only | ⚠ ToUnicode only |
| Scanned / mixed PDF (OCR) | 🚧 Per-page hybrid routing² | ❌ | ❌ | ❌ |
| PDF/UA structure tree | 🚧³ | ❌ | ⚠ Partial | ❌ |
| PDF decryption (RC4/AES) | 🚧 (`decrypt` feature)⁴ | ⚠ Partial | ⚠ Partial | ⚠ Partial |
| Per-span bounding boxes + confidence | 🚧¹ | ✅ | ❌ | ⚠ Partial |
| Streaming extraction (large files) | 🚧⁵ | ❌ | ❌ | ❌ |
| CJK scripts | ❌⁶ (`cjk` feature) | ⚠ | ⚠ | ⚠ |
| HTTP microservice mode | ❌⁷ (`serve`) | ❌ | ❌ | ❌ |
| Language | Rust + Python + C ABI | Python | Python | Python |

🚧 = implemented in the source tree, not yet verified end-to-end · ❌ = not working at HEAD · third-party columns unchanged

¹ Implemented, but unverifiable until the page-tree parse regression is fixed (see development status above): `pdftract extract` currently fails on all fixtures, including these paths.
² Page classification/routing implemented; the OCR acceptance corpus and its WER <3% gate are still open (bead `bf-33zjo`).
³ Structure-tree parser implemented and tagged-PDF corpus ready (bead `bf-5pyzm`); end-to-end verification pending the parse fix.
⁴ RC4/AES-128/AES-256 implemented with unit-test coverage (bead `pdftract-4mdfv`: 217/217 parser tests at close, 2026-06-03); the end-to-end encrypted-file path is blocked by the parse regression.
⁵ Lazy per-page decode implemented (bead `bf-2y2rp`); bounded-memory behavior on real documents is unverifiable until the parse fix.
⁶ Feature flag, corpus, and acceptance tests all exist; the acceptance tests fail in the most recent recorded run (0/5, `notes/bf-1wczm-cjk_encoding-run.log`).
⁷ Every request returns HTTP 500 at HEAD, including `GET /health` (see development status above).

## Platform Support

| Platform | Status |
|----------|--------|
| Linux x86_64 | Primary development platform — the test suite runs here during development; automated PR CI not yet wired (see development status above) |
| Linux aarch64 | Cross-compile build target in the CI pipeline; tests are not executed on-device; CI trigger wiring pending |
| macOS x86_64 | Cross-compile target defined in CI; no completed CI build recorded yet; manual smoke procedure documented for first release |
| macOS aarch64 | Cross-compile target defined in CI; no completed CI build recorded yet; manual smoke procedure documented for first release |
| Windows x86_64 | Cross-compile target defined in CI; no completed CI build recorded yet; manual smoke procedure documented for first release |

See [docs/operations/manual-platform-smoke.md](docs/operations/manual-platform-smoke.md) for the per-release smoke procedure (no release has shipped yet).

## Installation

**🚧 Public releases coming soon** — pdftract is preparing for its first public release (v0.1.0). The install commands below are planned but not yet available. For now, use the development installation instructions below.

**Minimum Supported Rust Version (MSRV):** 1.78

### Development installation

Build from source:

```bash
# Clone the repository
git clone https://github.com/jedarden/pdftract.git
cd pdftract

# Build the CLI
cargo build --release

# Install the CLI locally
cargo install --path .
```

### Planned release channels (coming soon)

#### Cargo

```bash
# Add as a library dependency
cargo add pdftract-core

# Or install the CLI
cargo install pdftract
```

*Status: not yet published to crates.io.*

#### pip

```bash
pip install pdftract
```

*Status: not yet published to PyPI (verified 2026-09-15: the package does not exist on PyPI yet).*

#### Docker

Pin to a released semver tag — never `:latest`:

```bash
docker pull ronaldraygun/pdftract:X.Y.Z          # default build
docker pull ronaldraygun/pdftract:ocr-X.Y.Z      # with OCR
docker pull ronaldraygun/pdftract:full-X.Y.Z     # all features
```

For reproducible consumption (CI, deployment manifests), pin the immutable
digest instead of the tag:

```bash
# Resolve the digest for a tag first
docker buildx imagetools inspect ronaldraygun/pdftract:X.Y.Z

# Then pull by digest — this is what you record in your Dockerfile / manifest
docker pull ronaldraygun/pdftract@sha256:<digest>
```

Images are published as cosign-signed, version-tagged multi-arch (amd64 +
arm64) manifest lists; no floating `:latest` tag exists. The image digest
above is the integrity pin for the container itself. The aggregate
`SHA256SUMS` checksum file (covering the binary archives, wheels, sdist, and
SBOM) plus its cosign signature `SHA256SUMS.sig` are published alongside each
release and verify in one shot (generation is implemented — bead
`pdftract-1wfp`, closed — but no release has shipped yet, so no file exists
yet):

```bash
cosign verify-blob --signature SHA256SUMS.sig SHA256SUMS
```

*Status: not yet published to Docker Hub (verified 2026-09-15: the repository does not exist on Docker Hub yet).*

#### Homebrew

```bash
brew install jedarden/tap/pdftract
```

*Status: tap live at [jedarden/homebrew-tap](https://github.com/jedarden/homebrew-tap). Formulas are generated from the versioned release archives and SHA256SUMS and pushed by the `pdftract-homebrew-publish` step of the release cascade (`.ci/argo-workflows/pdftract-homebrew-publish.yaml`), which also verifies `brew install` and `pdftract --version` in a pinned Homebrew container for each release. The first formula lands with the first milestone release. A homebrew-core submission for plain `brew install pdftract` can follow once the project meets homebrew-core notability criteria.*

## Quickstart

### Rust

```rust
use pdftract_core::{extract_pdf, ExtractionOptions};
use std::path::Path;

let opts = ExtractionOptions::default();
let doc = extract_pdf(Path::new("report.pdf"), &opts)?;

for page in &doc.pages {
    println!("Page {}: {} spans", page.page_number, page.spans.len());
}
```

Streaming extraction for large files (callback-based; return `false` to stop early):

```rust
use pdftract_core::extract_pdf_streaming;

extract_pdf_streaming(Path::new("large.pdf"), &opts, |page| {
    process(page);
    true // keep going
})?;
```

NDJSON output (one JSON object per page on stdout):

```rust
pdftract_core::extract_pdf_ndjson(Path::new("report.pdf"), &opts, std::io::stdout())?;
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
# NOTE: broken at HEAD — see the development-status note above
pdftract serve --bind 127.0.0.1:8080

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

The full extraction pipeline compiles from source out of the box; extraction *correctness* is mid-stabilization — see the development-status note above for what is currently verified. Optional features unlock heavier dependencies:

| Feature | What it adds | Enable with |
|---|---|---|
| `ocr` | Tesseract/Leptonica OCR for scanned and mixed pages | `cargo add pdftract-core --features ocr` |
| `decrypt` | RC4, AES-128, AES-256 PDF decryption | `cargo add pdftract-core --features decrypt` |
| `cjk` | CJK script support (Chinese, Japanese, Korean) | `cargo add pdftract-core --features cjk` |
| `full-render` | Full-page rasterization for assisted OCR and inspect UI | `cargo add pdftract-core --features full-render` |

When the Python wheels and Docker image ship, they will come with `ocr`, `decrypt`, and `cjk` pre-enabled (no wheels or images exist yet — see the development-status note above).

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
| Rust library | [`pdftract-core`](https://crates.io/crates/pdftract-core) on crates.io | Primary API — **🚧 coming soon** |
| CLI binary | [`pdftract`](https://crates.io/crates/pdftract) on crates.io | Wraps the library — **🚧 coming soon** |
| Python bindings | [`pdftract`](https://pypi.org/project/pdftract/) on PyPI | PyO3-based, wheels for Linux/macOS/Windows — **🚧 coming soon** |
| C shared library | `libpdftract` | Stable C ABI; use `pdftract codegen` to generate FFI headers for your language — **🚧 coming soon** |
| Docker image | [`ronaldraygun/pdftract`](https://hub.docker.com/r/ronaldraygun/pdftract) | Includes `serve` mode HTTP microservice — **🚧 coming soon** |
| HTTP microservice | `pdftract serve` | REST API for language-agnostic integration (build from source) — ⚠ broken at HEAD, see status note above |

Additional language SDK packages (Go, Node.js, Ruby) are in progress, built on top of the C ABI.

## Documentation

- **User guide:** [pdftract.com](https://pdftract.com) — **🚧 coming soon, live after the first tagged release** (until then, see [docs/user-docs/src/](docs/user-docs/src/) for the full user guide)
- **API reference:** docs.rs/pdftract-core — 🚧 coming soon (crate not yet published)
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

---

Part of [jedarden.com](https://jedarden.com)

*This GitHub repo is a read-only mirror of git.ardenone.com/jedarden/pdftract — issues and PRs are welcome here either way.*
