# Rust SDK

The Rust SDK is the `pdftract-core` crate. It provides native PDF text extraction with zero-copy memory mapping and streaming support.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
pdftract-core = "1.0"
```

For OCR support, enable the `ocr` feature:

```toml
[dependencies]
pdftract-core = { version = "1.0", features = ["ocr"] }
```

## Basic Extraction

```rust
use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};

fn main() -> anyhow::Result<()> {
    let opts = ExtractionOptions::default();
    let output = OutputOptions::default();

    let result = extract_pdf("document.pdf", &opts, &output)?;

    for (i, page) in result.pages.iter().enumerate() {
        println!("Page {}: {} chars", i + 1, page.text.len());
        for span in &page.spans {
            println!("  {}", span.text);
        }
    }
    Ok(())
}
```

## Streaming Extraction

For large PDFs, stream pages one at a time to keep memory usage bounded:

```rust
use pdftract_core::{extract_pdf_streaming, ExtractionOptions, OutputOptions};
use std::fs::File;

fn main() -> anyhow::Result<()> {
    let mut output = File::create("output.ndjson")?;
    extract_pdf_streaming(
        "large_document.pdf",
        &ExtractionOptions::default(),
        &OutputOptions::default(),
        &mut output,
    )?;
    Ok(())
}
```

## Options

### ExtractionOptions

| Field | Type | Default | Use Case |
|-------|------|---------|----------|
| `receipts` | `ReceiptsMode` | `Off` | Generate cryptographic receipts |
| `max_parallel_pages` | `usize` | `4` | Control memory for concurrent page processing |
| `memory_budget_mb` | `usize` | `512` | Target peak RSS in MB |
| `full_render` | `bool` | `false` | Enable PDFium rendering (requires `full-render` feature) |
| `ocr_dpi_override` | `Option<u32>` | `None` | Override automatic DPI selection |
| `ocr_language` | `Vec<String>` | `vec!["eng"]` | Tesseract language codes |
| `markdown_anchors` | `bool` | `false` | Emit HTML comment anchors in Markdown |
| `max_decompress_bytes` | `u64` | `512 MiB` | Bomb limit for decompressed streams |
| `output` | `OutputOptions` | `default()` | Output filtering options |
| `pages` | `Option<String>` | `None` | Page range (e.g., `"1-5,7,12-"`) |
| `password` | `Option<SecretString>` | `None` | PDF password for encrypted documents |

### OutputOptions

| Field | Type | Default | Use Case |
|-------|------|---------|----------|
| `include_invisible` | `bool` | `false` | Include invisible text in output |
| `extract_forms` | `bool` | `true` | Extract AcroForm fields |
| `extract_attachments` | `bool` | `true` | Extract embedded attachments |

## Receipts

Generate cryptographic receipts for verification:

```rust
use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};
use pdftract_core::options::ReceiptsMode;

fn main() -> anyhow::Result<()> {
    let opts = ExtractionOptions {
        receipts: ReceiptsMode::Lite,
        ..Default::default()
    };
    let output = OutputOptions::default();
    let result = extract_pdf("document.pdf", &opts, &output)?;

    // Receipts are embedded in page metadata
    if let Some(receipt) = &result.pages[0].receipt {
        println!("Receipt: {}", receipt);
    }
    Ok(())
}
```

## Remote PDFs

With the `remote` feature, fetch PDFs via HTTP:

```rust
use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};

fn main() -> anyhow::Result<()> {
    let opts = ExtractionOptions::default();
    let output = OutputOptions::default();
    let result = extract_pdf("https://example.com/document.pdf", &opts, &output)?;
    Ok(())
}
```

## Error Handling

Most functions return `anyhow::Result<T>` which wraps various error types:

```rust
use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};

fn main() {
    let opts = ExtractionOptions::default();
    let output = OutputOptions::default();

    match extract_pdf("document.pdf", &opts, &output) {
        Ok(result) => {
            println!("Extracted {} pages", result.pages.len());
        }
        Err(e) => {
            eprintln!("Extraction failed: {}", e);
            // Inspect error chain
            for cause in e.chain() {
                eprintln!("  caused by: {}", cause);
            }
        }
    }
}
```

## Feature Flags

| Feature | Adds | Default |
|---------|------|---------|
| `serde` | JSON serialization support | ✓ |
| `decrypt` | Decryption of encrypted PDFs | ✓ |
| `quick-xml` | Conformance detection via XML metadata | ✓ |
| `ocr` | Tesseract OCR for scanned documents | - |
| `full-render` | PDFium-based rendering (requires `ocr`) | - |
| `remote` | HTTP range fetching for remote PDFs | - |
| `profiles` | Extraction profiles | - |
| `receipts` | Cryptographic receipt generation | - |
| `cjk` | CJK text extraction via predefined CMap registry | - |
| `schemars` | JSON Schema generation | - |

## Source Types

The SDK supports multiple source types via the `PdfSource` trait:

```rust
use pdftract_core::source::{FileSource, MmapSource, MemorySource};

// Memory-mapped source (zero-copy for large files)
let source = MmapSource::open("document.pdf")?;

// In-memory source (for byte buffers)
let data = std::fs::read("document.pdf")?;
let source = MemorySource::new(data);

// Standard file source
let source = FileSource::open("document.pdf")?;
```

## See Also

- [JSON Schema Reference](../json-schema-reference.md)
- [CLI Reference](../cli/README.md)
- [Advanced: OCR Configuration](../advanced/ocr.md)
