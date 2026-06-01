# Python SDK

The Python SDK (`pdftract`) provides native Python bindings with idiomatic ergonomics including an exception hierarchy, dataclass types, and optional asyncio wrappers.

## Installation

```bash
pip install pdftract
```

The package includes a precompiled native module for your platform. If the native module fails to import, a subprocess fallback is automatically used (with significantly degraded performance).

## Basic Extraction

```python
import pdftract

doc = pdftract.extract("document.pdf")
print(f"Extracted {len(doc.pages)} pages")

for page in doc.pages:
    for span in page.spans:
        print(span.text)
```

## Text-Only Extraction

For RAG pipelines that just need the text body:

```python
import pdftract

text = pdftract.extract_text("document.pdf")
print(text)
```

## Streaming

For large PDFs, stream pages one at a time to keep memory usage bounded:

```python
import pdftract

for page in pdftract.extract_stream("large_document.pdf"):
    print(f"Page {page.page_index}: {len(page.spans)} spans")
    # Process page while only one page is resident in memory
```

## Markdown Extraction

Extract Markdown with optional anchor links for mapping back to PDF locations:

```python
import pdftract

# Basic Markdown
markdown = pdftract.extract_markdown("document.pdf")

# With anchor links (HTML comments)
markdown = pdftract.extract_markdown("document.pdf", anchors=True)
```

## Options

Pass extraction options as keyword arguments:

```python
import pdftract

doc = pdftract.extract(
    "document.pdf",
    pages="1-5,7",           # Page range
    password="secret123",    # PDF password
    receipts="lite"          # Receipt generation mode
)
```

### Available Options

| Option | Type | Default | Use Case |
|--------|------|---------|----------|
| `pages` | `str \| None` | `None` | Page range (e.g., `"1-5,7,12-"`) |
| `password` | `str \| None` | `None` | PDF password for encrypted documents |
| `receipts` | `str \| None` | `None` | Receipt mode: `"off"`, `"lite"`, or `"full"` |
| `ocr` | `bool` | `False` | Enable OCR for scanned documents |
| `ocr_language` | `list[str]` | `["eng"]` | OCR language codes |
| `include_invisible` | `bool` | `False` | Include invisible text in output |
| `extract_forms` | `bool` | `True` | Extract AcroForm fields |
| `extract_attachments` | `bool` | `True` | Extract embedded attachments |
| `readability_threshold` | `float` | `0.0` | Minimum readability score |
| `max_decompress_gb` | `int` | `512` | Max decompressed GB per stream |
| `full_render` | `bool` | `False` | Enable full rendering |

## Error Handling

The SDK provides a structured exception hierarchy:

```python
import pdftract

try:
    doc = pdftract.extract("encrypted.pdf", password="wrong")
except pdftract.EncryptionError as e:
    print(f"Encryption error: {e.code} - {e.hint}")
except pdftract.CorruptPdfError as e:
    print(f"Corrupt PDF: {e}")
except pdftract.SourceUnreachableError as e:
    print(f"File not found: {e}")
except pdftract.PdftractError as e:
    print(f"Extraction failed: {e}")
```

### Exception Hierarchy

All exceptions inherit from `PdftractError`:

- `PdftractError` — Base exception for all extraction errors
- `EncryptionError` — PDF encryption/password errors
- `CorruptPdfError` — Malformed or corrupted PDF
- `SourceUnreachableError` — File or URL unreachable
- `RemoteFetchInterruptedError` — Network interruption during fetch
- `TlsError` — TLS/certificate errors
- `ReceiptVerifyError` — Receipt verification failed
- `UnsupportedOperationError` — Requested operation not available

### Exception Attributes

All exceptions have the following attributes:

- `code` — Diagnostic code (e.g., `"ENCRYPTION_WRONG_PASSWORD"`)
- `page_index` — Page number where error occurred (if applicable)
- `hint` — Suggested action for resolution

## Metadata

Get document metadata without full extraction:

```python
import pdftract

metadata = pdftract.get_metadata("document.pdf")
print(f"Pages: {metadata.page_count}")
print(f"Title: {metadata.title}")
print(f"Author: {metadata.author}")
print(f"Fingerprint: {metadata.fingerprint}")
```

## Search

Search for a regex pattern in the PDF:

```python
import pdftract

for match in pdftract.search("document.pdf", r"\b\d{3}-\d{2}-\d{4}\b"):
    print(f"Found SSN at page {match.page_index}: {match.text}")
```

## Fingerprint

Compute the structural fingerprint of a PDF:

```python
import pdftract

fingerprint = pdftract.hash("document.pdf")
print(f"Fingerprint: {fingerprint.value}")
```

## Classify

Classify a PDF page type:

```python
import pdftract

classification = pdftract.classify("document.pdf")
print(f"Type: {classification.class_name}")
print(f"Confidence: {classification.confidence}")
```

## Verify Receipt

Verify a cryptographic receipt:

```python
import pdftract

# Extract with receipts enabled
doc = pdftract.extract("document.pdf", receipts="lite")
receipt = doc.pages[0].receipt

# Verify later
verified = pdftract.verify_receipt("document.pdf", receipt)
print(f"Verified: {verified}")
```

## Remote PDFs

Extract from HTTP/HTTPS URLs:

```python
import pdftract

doc = pdftract.extract("https://example.com/document.pdf")
```

## MCP Integration

For AI-assisted PDF extraction, pdftract provides an [MCP (Model Context Protocol) server](../integrations/mcp-clients.md). The Python SDK can be used alongside MCP clients like Claude Desktop:

```bash
pdftract mcp --stdio
```

See [MCP Client Configuration Guide](../integrations/mcp-clients.md) for setup instructions.

## Types

The SDK provides typed wrappers for all output structures:

```python
from pdftract.types import Document, Page, Span, Block, Metadata

# All extraction functions return typed objects
doc: Document = pdftract.extract("document.pdf")
page: Page = doc.pages[0]
span: Span = page.spans[0]
block: Block = page.blocks[0]
metadata: Metadata = pdftract.get_metadata("document.pdf")
```

## Async API

For asyncio-based applications, use the async API:

```python
import pdftract.asyncio as pdftract_async

async def extract_async():
    doc = await pdftract_async.extract("document.pdf")
    print(f"Extracted {len(doc.pages)} pages")
```

## See Also

- [MCP Client Configuration Guide](../integrations/mcp-clients.md)
- [JSON Schema Reference](../json-schema-reference.md)
- [CLI Reference](../cli/README.md)
- [Advanced: OCR Configuration](../advanced/ocr.md)
