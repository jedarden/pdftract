# pdftract Python SDK

**pdftract** is a pure-Rust PDF text extraction library built for the cases where other tools give up: scanned documents, unusual font encodings, multi-column layouts, footnotes, mixed-mode pages, and encrypted files. This Python package provides idiomatic Python bindings with full access to the pdftract-core extraction pipeline.

## Installation

```bash
pip install pdftract
```

**Requirements:** Python 3.11 or later

The Python package includes platform-specific wheels for Linux, macOS, and Windows. No Rust toolchain is required for installation.

## Quickstart

```python
import pdftract

# Basic extraction
doc = pdftract.extract("report.pdf")
print(f"Extracted {len(doc.pages)} pages")

# Access text with layout information
for page in doc.pages:
    for span in page.spans:
        print(f"{span.text} at {span.bbox} (confidence: {span.confidence})")

# Plain text extraction
text = pdftract.extract_text("document.pdf")

# Markdown extraction
markdown = pdftract.extract_markdown("document.pdf", anchors=True)

# Streaming extraction for large PDFs
for page in pdftract.extract_stream("large.pdf"):
    print(f"Page {page.page_index}: {len(page.spans)} spans")

# Search across pages
for match in pdftract.search("document.pdf", r"invoice\s+#\d+"):
    print(f"Found '{match.text}' on page {match.page_number}")

# Get metadata without full extraction
metadata = pdftract.get_metadata("document.pdf")
print(f"Title: {metadata.title}, Pages: {metadata.page_count}")

# Compute structural fingerprint
fingerprint = pdftract.hash("document.pdf")
print(f"Fingerprint: {fingerprint.value}")

# Classify page types
classification = pdftract.classify("document.pdf")
print(f"Vector: {classification.vector_pages}, Scanned: {classification.scanned_pages}")
```

## Exception Hierarchy

The SDK provides a structured exception hierarchy for handling different error conditions:

```python
from pdftract import (
    PdftractError,           # Base exception for all pdftract errors
    CorruptPdfError,         # PDF file is corrupted or malformed
    EncryptionError,         # PDF is encrypted and password is missing/wrong
    SourceUnreachableError,  # File or URL is unreachable
    RemoteFetchInterruptedError,  # Network download interrupted
    TlsError,                # TLS/SSL certificate failure
    ReceiptVerifyError,      # Receipt verification failed
    UnsupportedOperationError,  # Operation not supported
)

try:
    doc = pdftract.extract("document.pdf")
except EncryptionError:
    print("PDF is encrypted - provide password")
except CorruptPdfError:
    print("PDF file is corrupted")
except SourceUnreachableError:
    print("Cannot read the file or URL")
```

## Extraction Options

All extraction functions accept keyword options:

```python
doc = pdftract.extract("document.pdf",
    ocr=True,                          # Enable OCR for scanned pages
    ocr_language=["eng", "fra"],       # OCR languages
    include_invisible=True,            # Include invisible text
    extract_forms=True,                # Extract form fields
    extract_attachments=True,          # Extract embedded attachments
    readability_threshold=0.7,         # Readability threshold (0.0-1.0)
    password="secret",                 # PDF password
    max_decompress_gb=4,              # Max decompressed GB per stream
    full_render=True,                 # Enable full rendering
)

markdown = pdftract.extract_markdown("document.pdf",
    anchors=True,                      # Include anchor links
)
```

## Typed Data Structures

The SDK uses Python dataclasses for type-safe access to extraction results:

```python
from pdftract import Document, Page, Span, Block, Metadata, Fingerprint

doc: Document = pdftract.extract("document.pdf")

# Document properties
metadata = doc.metadata              # Metadata object
pages = doc.pages                    # List[Page]

# Page properties
page = doc.pages[0]
page_number = page.page_index        # 0-based index
bbox = page.bbox                     # Page bounding box
rotation = page.rotation            # Page rotation in degrees
spans = page.spans                   # List[Span]
blocks = page.blocks                 # List[Block] (semantic structure)

# Span properties (text fragment with position)
span = page.spans[0]
text = span.text                     # Extracted text
bbox = span.bbox                     # [x0, y0, x1, y1]
font = span.font                     # Font name
size = span.size                     # Font size
confidence = span.confidence         # 0.0-1.0

# Block properties (semantic region)
block = page.blocks[0]
block_type = block.block_type       # "text", "heading", "list", "table", "figure"
content = block.content              # List of child blocks/spans
```

## Async API

For async/await support, use the `asyncio` module:

```python
import pdftract.asyncio as pdftract

# Async extraction
doc = await pdftract.extract("document.pdf")

# Async streaming
async for page in pdftract.extract_stream("large.pdf"):
    print(f"Page {page.page_index}")

# Async search
async for match in pdftract.search("document.pdf", pattern):
    print(f"Match: {match.text}")
```

## Subprocess Fallback

The Python SDK automatically falls back to subprocess mode when the native PyO3 module cannot be loaded. This typically happens in:

- **musl-libc environments** (e.g., Alpine Linux) where glibc-linked native modules are incompatible
- **Missing native module** when the wheel doesn't include a platform-specific build
- **Development builds** where the native module hasn't been compiled yet

**How it works:** The fallback calls the `pdftract` CLI binary via `subprocess.run()`, passing arguments and parsing JSON/NDJSON output. All public API functions work identically — the switch is transparent.

**Limitations:**
- Requires the `pdftract` CLI binary in `PATH` (install via `cargo install pdftract` or your package manager)
- Slight performance overhead from subprocess spawning and JSON serialization
- Stream operations yield pages from NDJSON rather than direct native iteration

**Checking which mode is active:**

```python
import pdftract

if pdftract._using_fallback:
    print("Using subprocess fallback (CLI binary)")
elif pdftract._native_available:
    print("Using native PyO3 module")
else:
    print("Neither mode available - ImportError will be raised")
```

When fallback activates, you'll see a `RuntimeWarning` at import time:

```
RuntimeWarning: Native module failed to import: <error details>.
Using subprocess fallback. Performance will be significantly degraded.
```

**Smoke test the fallback:**

```bash
# Force ImportError by renaming the native module
cd crates/pdftract-py/python/pdftract
mv _native.abi3.so _native.abi3.so.backup

# Test that fallback works (ensure CLI binary is in PATH first)
python3 -c "
import sys
sys.path.insert(0, 'crates/pdftract-py/python')
import pdftract
print(f'Fallback active: {pdftract._using_fallback}')
print(f'Native available: {pdftract._native_available}')
"

# Restore the native module
mv _native.abi3.so.backup _native.abi3.so
```

Or run the automated smoke test:

```bash
cd crates/pdftract-py
python3 test_fallback_smoke.py
```

## Development Installation

For development from source:

```bash
# Clone the repository
git clone https://github.com/jedarden/pdftract.git
cd pdftract

# Build the CLI and native module
cargo build --release

# Install the CLI locally
cargo install --path .

# Install Python package in development mode
cd crates/pdftract-py
pip install -e .
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux x86_64 | Fully CI-tested |
| Linux aarch64 | Fully CI-tested |
| macOS x86_64 | Build-tested |
| macOS aarch64 | Build-tested |
| Windows x86_64 | Build-tested |

## Documentation

- **API Reference:** Full API documentation available in the `pdftract` module docstrings
- **Main Project:** [github.com/jedarden/pdftract](https://github.com/jedarden/pdftract)
- **Output Schema:** See the main project docs for JSON structure details

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)

at your option.
