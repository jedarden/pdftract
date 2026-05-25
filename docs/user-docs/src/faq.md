# FAQ

Frequently asked questions about pdftract.

## Table of Contents

- [General](#general)
  - [What is pdftract?](#what-is-pdftract)
  - [What's the difference between extract and extract_text?](#whats-the-difference-between-extract-and-extract_text)
  - [Does pdftract execute JavaScript embedded in PDFs?](#does-pdftract-execute-javascript-embedded-in-pdfs)
  - [How do I cite an extracted snippet?](#how-do-i-cite-an-extracted-snippet)
- [Installation and Setup](#installation-and-setup)
  - [How do I install pdftract?](#how-do-i-install-pdftract)
  - [How do I run pdftract behind a corporate proxy?](#how-do-i-run-pdftract-behind-a-corporate-proxy)
  - [What are the system requirements?](#what-are-the-system-requirements)
- [Usage](#usage)
  - [Why is my PDF returning broken_vector?](#why-is-my-pdf-returning-broken_vector)
  - [Why is OCR slow?](#why-is-ocr-slow)
  - [How do I extract text from a specific page range?](#how-do-i-extract-text-from-a-specific-page-range)
  - [How do I extract images from a PDF?](#how-do-i-extract-images-from-a-pdf)
  - [Can I process multiple PDFs at once?](#can-i-process-multiple-pdfs-at-once)
- [Configuration](#configuration)
  - [How do I add a custom profile?](#how-do-i-add-a-custom-profile)
  - [How do I adjust OCR accuracy?](#how-do-i-adjust-ocr-accuracy)
  - [How do I disable OCR for faster processing?](#how-do-i-disable-ocr-for-faster-processing)
  - [What are confidence scores and how do I use them?](#what-are-confidence-scores-and-how-do-i-use-them)
- [Output and Formats](#output-and-formats)
  - [How do I get output in Markdown format?](#how-do-i-get-output-in-markdown-format)
  - [How do I preserve table structure?](#how-do-i-preserve-table-structure)
  - [Can I extract metadata from PDFs?](#can-i-extract-metadata-from-pdfs)
  - [How do I handle password-protected PDFs?](#how-do-i-handle-password-protected-pdfs)
- [Troubleshooting](#troubleshooting)
  - [Why is extraction failing with an error?](#why-is-extraction-failing-with-an-error)
  - [Why is my output empty or incomplete?](#why-is-my-output-empty-or-incomplete)
  - [How do I debug extraction issues?](#how-do-i-debug-extraction-issues)
  - [Why does extraction use so much memory?](#why-does-extraction-use-so-much-memory)

---

## General

### What is pdftract?

pdftract is a command-line tool and library for extracting text, structure, and content from PDF files. It combines vector text extraction with OCR fallback to handle both well-formed and problematic PDFs. pdftract is written in Rust and provides Python bindings for programmatic use.

See the [Introduction](introduction.md) for a complete overview.

### What's the difference between extract and extract_text?

- **`extract`**: The primary command that produces structured JSON output with blocks, spans, metadata, and provenance information. Use this when you need the full extraction with layout, reading order, and confidence scores.

- **`extract_text`**: A simplified command that outputs plain text only. Use this for quick text extraction when you don't need the structured JSON output.

Example:
```bash
# Full structured extraction
pdftract extract document.pdf -o output.json

# Plain text only
pdftract extract_text document.pdf -o output.txt
```

### Does pdftract execute JavaScript embedded in PDFs?

**No.** pdftract never executes JavaScript embedded in PDFs. JavaScript is detected during parsing for security analysis, but it is never executed. This design prevents malicious PDFs from exploiting JavaScript vulnerabilities.

If you need to analyze JavaScript in PDFs, pdftract can detect and report its presence, but execution must be done separately with appropriate sandboxing.

### How do I cite an extracted snippet?

The JSON output from `pdftract extract` includes provenance information for each text block:

```json
{
  "blocks": [{
    "spans": [{
      "text": "Example snippet",
      "bbox": [100.0, 200.0, 250.0, 215.0],
      "page": 3,
      "confidence": 0.98
    }]
  }],
  "metadata": {
    "path": "/path/to/document.pdf",
    "fingerprint": "sha256:abc123...",
    "extracted_at": "2026-05-25T12:00:00Z"
  }
}
```

For academic citations, include:
- Document path and fingerprint
- Page number (from the `page` field)
- Extraction timestamp
- The pdftract version used

---

## Installation and Setup

### How do I install pdftract?

See the [Installation](installation.md) guide for complete instructions. Quick summary:

**With cargo (Rust toolchain):**
```bash
cargo install pdftract
```

**With pip (Python bindings):**
```bash
pip install pdftract
```

**Pre-built binaries:** Download from the [releases page](https://github.com/your-org/pdftract/releases).

### How do I run pdftract behind a corporate proxy?

pdftract doesn't have built-in proxy support, but you can use the HTTP serve mode with a reverse proxy:

1. Start pdftract in serve mode:
```bash
pdftract serve --port 8080
```

2. Configure your reverse proxy (nginx, Apache, etc.) to handle authentication and SSL termination.

3. Access pdftract through your proxy endpoint.

See [Advanced Topics: HTTP Serve](../operations/serve-deployment.md) for deployment guidance.

### What are the system requirements?

- **OS**: Linux, macOS, or Windows
- **Rust**: 1.70+ (if building from source)
- **Python**: 3.8+ (for Python bindings)
- **OCR (optional)**: Tesseract 4.0+ for OCR fallback
- **Memory**: 512 MB minimum for typical PDFs; more for large documents

---

## Usage

### Why is my PDF returning broken_vector?

The `broken_vector` classification means the PDF's text layer is unreliable or missing. Common causes:

- **Invisible text overlay**: Text with rendering mode 3 (invisible) overlaid on a raster image
- **Missing ToUnicode CMap**: Font lacks character-to-Unicode mapping
- **Encoding corruption**: Character encodings don't match the actual glyphs

**Solution**: pdftract automatically routes `broken_vector` pages to the OCR pipeline (Phase 5.5). If you see `broken_vector` without OCR output, check that OCR is enabled:

```bash
# Verify OCR is available
pdftract doctor tesseract-langs

# Enable OCR explicitly if needed
pdftract extract document.pdf --enable-ocr
```

See [Troubleshooting: Broken Vector](troubleshooting/common-issues.md) for more details.

### Why is OCR slow?

OCR performance depends on several factors:

- **Image resolution**: Higher DPI images take longer to process
- **Tesseract version**: Version 4.0+ is significantly faster than 3.x
- **Language data**: Additional language packs increase processing time
- **Hardware**: CPU-bound; more cores help with batch processing

**To speed up OCR:**
```bash
# Reduce DPI (trade-off: accuracy)
pdftract extract document.pdf --ocr-dpi 200

# Use fewer languages
pdftract extract document.pdf --ocr-lang eng

# Disable OCR for vector-only PDFs
pdftract extract document.pdf --disable-ocr
```

### How do I extract text from a specific page range?

Use the `--pages` flag:

```bash
# Single page
pdftract extract document.pdf --pages 5

# Range
pdftract extract document.pdf --pages 1-10

# Multiple ranges
pdftract extract document.pdf --pages 1-5,10,15-20

# All pages from page 5 onward
pdftract extract document.pdf --pages 5-
```

### How do I extract images from a PDF?

pdftract automatically detects and records image XObjects during content stream processing. The output JSON includes image metadata:

```json
{
  "images": [{
    "bbox": [100.0, 200.0, 400.0, 500.0],
    "xobject_ref": "5 0 R",
    "name": "Im1"
  }]
}
```

For actual image extraction, use the `serve` mode with the `/images` endpoint or write a custom script using the Python SDK.

### Can I process multiple PDFs at once?

Yes, use shell wildcards or write a batch script:

```bash
# Process all PDFs in a directory
for file in *.pdf; do
    pdftract extract "$file" -o "output/$(basename "$file" .json)"
done

# With parallel processing (GNU parallel)
ls *.pdf | parallel -j 4 pdftract extract {} -o output/{/.}.json
```

---

## Configuration

### How do I add a custom profile?

Create a YAML file defining your profile:

```yaml
# custom-profile.yaml
name: my_custom
description: "Custom extraction profile"

extraction:
  preserve_tables: true
  preserve_columns: true
  ocr_fallback: true

output:
  format: json
  include_provenance: true
  confidence_threshold: 0.7
```

Then use it:
```bash
pdftract extract document.pdf --profile custom-profile.yaml
```

See [Custom Profiles](profiles/custom.md) for complete documentation.

### How do I adjust OCR accuracy?

Adjust Tesseract parameters via environment variables or the OCR configuration:

```bash
# Set OCR engine mode
export TESSERACT_OEM=1  # LSTM only
export TESSERACT_PSM=6  # Assume single column block of text

# Adjust page segmentation mode
pdftract extract document.pdf --tesseract-psm 6
```

Higher accuracy settings may slow down processing. See [OCR Configuration](advanced/ocr.md) for details.

### How do I disable OCR for faster processing?

If you know your PDFs have reliable text layers:

```bash
pdftract extract document.pdf --disable-ocr
```

Or set a confidence threshold to skip low-confidence text:

```bash
pdftract extract document.pdf --min-confidence 0.9
```

### What are confidence scores and how do I use them?

Each text span has a `confidence` score (0.0 to 1.0):

- **1.0**: High confidence (ToUnicode CMap lookup succeeded)
- **0.3**: Medium confidence (encoding + AGL fallback)
- **0.0**: No confidence (PositionHint mode or failed resolution)

Filter by confidence:
```bash
pdftract extract document.pdf --min-confidence 0.5
```

Or filter in post-processing using jq:
```bash
pdftract extract document.pdf | jq '.blocks[].spans[] | select(.confidence > 0.5)'
```

---

## Output and Formats

### How do I get output in Markdown format?

Use the `--format` flag:

```bash
pdftract extract document.pdf --format markdown -o output.md
```

The Markdown output preserves headings, lists, tables, and code blocks where detected.

### How do I preserve table structure?

pdftract includes table detection (Phase 4.2). Ensure table preservation is enabled:

```bash
pdftract extract document.pdf --preserve-tables
```

Tables are output with structured cell information:
```json
{
  "type": "table",
  "rows": 3,
  "columns": 4,
  "cells": [...]
}
```

### Can I extract metadata from PDFs?

Yes, metadata is automatically extracted and included in the output:

```json
{
  "metadata": {
    "title": "Document Title",
    "author": "Author Name",
    "subject": "Subject",
    "keywords": ["keyword1", "keyword2"],
    "creator": "Application",
    "producer": "PDF Producer",
    "creation_date": "2026-01-01T00:00:00Z",
    "modified_date": "2026-05-25T12:00:00Z"
  }
}
```

### How do I handle password-protected PDFs?

Provide the password via the `--password` flag:

```bash
pdftract extract document.pdf --password secret123
```

For security, avoid passing passwords on the command line in production. Use environment variables or a config file:

```bash
export PDFTRACT_PASSWORD=secret123
pdftract extract document.pdf
```

---

## Troubleshooting

### Why is extraction failing with an error?

Check the error message and consult the [Troubleshooting Guide](troubleshooting/README.md). Common issues:

- **Encrypted PDFs**: Use `--password` to decrypt
- **Corrupted PDFs**: pdftract attempts recovery; check diagnostics
- **Missing dependencies**: Verify Tesseract and language packs are installed

Run diagnostics:
```bash
pdftract doctor
```

### Why is my output empty or incomplete?

Possible causes:

1. **No text layer**: PDF may be image-only. Enable OCR.
2. **Encoding issues**: Check diagnostics for `FONT_GLYPH_UNMAPPED` warnings
3. **Page range issue**: Verify your `--pages` argument
4. **Confidence filter**: Lower `--min-confidence` if set too high

Check diagnostics output:
```bash
pdftract extract document.json --verbose
```

### How do I debug extraction issues?

Enable verbose output and diagnostics:

```bash
# Full diagnostic output
pdftract extract document.pdf --verbose --diagnostics

# Save diagnostics for analysis
pdftract extract document.pdf --diagnostics -o diagnostics.json
```

Common diagnostic codes:
- `FONT_GLYPH_UNMAPPED`: Glyph couldn't be mapped to Unicode
- `STREAM_DECODE_ERROR`: Stream decompression failed
- `STRUCT_INVALID_TYPE`: Unexpected object type

See [Diagnostics Reference](troubleshooting/diagnostics.md) for a complete list.

### Why does extraction use so much memory?

Memory usage depends on:

- **PDF size**: Larger PDFs with many images use more memory
- **OCR**: Tesseract loads image data into memory
- **Output buffering**: Large JSON outputs are buffered in memory

**To reduce memory usage:**
```bash
# Process page-by-page
for page in {1..100}; do
    pdftract extract document.pdf --pages $page -o "page-$page.json"
done

# Disable OCR if not needed
pdftract extract document.pdf --disable-ocr

# Stream output (if supported)
pdftract extract document.pdf --stream-output
```

---

## Still have questions?

- Check the [Troubleshooting Guide](troubleshooting/README.md)
- Review the [CLI Reference](cli/README.md)
- Open an issue on [GitHub](https://github.com/your-org/pdftract/issues)
