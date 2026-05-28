# pdftract

[![crates.io](https://img.shields.io/crates/v/pdftract)](https://crates.io/crates/pdftract)
[![docs.rs](https://img.shields.io/docsrs/pdftract)](https://docs.rs/pdftract)
[![CI Status](https://custom-icon-badges.demolab.com/badge/CI-Argo%20Workflows-success?logo=argocd&logoColor=white)](https://github.com/jedarden/pdftract/blob/main/.ci/argo-workflows/pdftract-ci.yaml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

A PDF text extraction library that gets the hard parts right.

## Platform Support

| Platform | Status |
|----------|--------|
| Linux x86_64 | Fully CI-tested (gating CI on every PR) |
| Linux aarch64 | Fully CI-tested |
| macOS x86_64 | Build-tested; manually smoke-tested per release |
| macOS aarch64 | Build-tested; manually smoke-tested per release |
| Windows x86_64 | Build-tested; manually smoke-tested per release |

> **Note:** Linux is fully CI-tested; macOS and Windows are build-tested and manually smoke-tested per release. See [docs/operations/manual-platform-smoke.md](docs/operations/manual-platform-smoke.md) for the per-release smoke procedure.

## Installation

### cargo

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
let doc = extract_pdf("file.pdf", &opts)?;
println!("Extracted {} pages", doc.metadata.page_count);
```

### Python

```python
import pdftract

doc = pdftract.extract("file.pdf")
print(f"Extracted {doc['metadata']['page_count']} pages")
```

### CLI

```bash
pdftract extract file.pdf --json result.json   # JSON output
pdftract extract file.pdf --text -             # Plain text to stdout
pdftract serve --port 8080                     # HTTP microservice
```

## What it does

- **Correct reading order** — layout regions are segmented and sequenced before text is emitted, handling multi-column pages, sidebars, footnotes, and mixed-layout documents
- **Font encoding recovery** — when `ToUnicode` CMaps are absent, wrong, or incomplete, pdftract works through a layered recovery pipeline: glyph name lookup, font fingerprinting, and glyph outline shape matching
- **Structure tree extraction** — PDF/UA and PDF/A documents encode their logical structure; pdftract reads this directly when present
- **Per-page hybrid routing** — each page is independently classified and routed to the appropriate pipeline: vector text extraction, full OCR, or assisted OCR
- **Structured output with provenance** — the primary output is JSON carrying per-span bounding boxes, font name, size, and confidence score

## Documentation

- **User docs:** [docs/user-docs](docs/user-docs/) (mdBook)
- **API reference:** [docs.rs/pdftract](https://docs.rs/pdftract)
- **Contributing guide:** [CONTRIBUTING.md](CONTRIBUTING.md)
- **Security policy:** [SECURITY.md](SECURITY.md)
- **Changelog:** [CHANGELOG.md](CHANGELOG.md)
- **License:** [LICENSE-MIT](LICENSE-MIT) or [LICENSE-APACHE](LICENSE-APACHE)

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

at your option.
