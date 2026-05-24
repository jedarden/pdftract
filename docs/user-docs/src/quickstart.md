# Quickstart

This five-minute walkthrough covers the core pdftract workflow: extract text from a PDF, inspect the structured JSON output, and try profile-based extraction.

## Five-Minute Walkthrough

### Prerequisites

- pdftract installed (see [Installation](./installation.md))
- A PDF file to extract (any PDF will do)

If you don't have a PDF handy, you can use the sample fixtures from the pdftract repository:

```bash
git clone https://github.com/jedarden/pdftract.git
cd pdftract
```

### Verify Your Environment

Before extracting, verify your environment is properly configured:

```bash
pdftract doctor
```

Expected output:

```
Check                         Status  Detail
─────────────────────────────────────────────
pdftract binary               OK      0.1.0 (git: abc1234)
tesseract install             OK      v5.3.0
...
```

If any check shows FAIL, see the [Operations Runbook](../../operations/manual-platform-smoke.md#troubleshooting) for resolution steps.

### Extract Your First PDF

The simplest extraction outputs plain text to stdout:

```bash
pdftract extract path/to/document.pdf
```

For structured JSON output (default):

```bash
pdftract extract path/to/document.pdf --output result.json
```

Or view JSON directly in your terminal (pipe to `jq` for pretty-printing):

```bash
pdftract extract path/to/document.pdf | jq .
```

### Inspect the Output

The JSON output contains:

- **`pages`** — Array of page objects, each with `blocks` and `spans`
- **`blocks`** — Semantic elements (headings, paragraphs, lists) with reading order
- **`spans`** — Text fragments with bounding boxes, font metadata, and confidence scores
- **`metadata`** — Document title, author, page count, PDF version

Example:

```json
{
  "pages": [
    {
      "page": 1,
      "width": 612,
      "height": 792,
      "blocks": [
        {
          "kind": "heading",
          "text": "Introduction",
          "bbox": [72, 680, 400, 700],
          "level": 1
        },
        {
          "kind": "paragraph",
          "text": "This is the first paragraph...",
          "bbox": [72, 640, 540, 670]
        }
      ],
      "spans": [
        {
          "text": "Introduction",
          "bbox": [72, 680, 400, 700],
          "font": "Times-Bold",
          "size": 14.0,
          "confidence": 0.99
        }
      ]
    }
  ],
  "metadata": {
    "title": "Sample Document",
    "author": "John Doe",
    "page_count": 1,
    "pdf_version": "1.4"
  }
}
```

### Try Auto-Profile Mode

pdftract includes built-in profiles for common document types (invoices, receipts, contracts, etc.). Use `--auto` to automatically detect the profile:

```bash
pdftract extract invoice.pdf --auto
```

The auto-detected profile is logged to stderr:

```
[INFO] Detected profile: invoice
```

Profiles optimize extraction for specific document layouts:
- **invoice** — Extract line items, totals, vendor info
- **receipt** — Extract merchant, date, line items, tax, total
- **contract** — Extract parties, effective date, clauses
- **bank_statement** — Extract account info, statement period, transactions

See [Profiles](./profiles/available.md) for the full list.

### Batch Processing

To extract multiple PDFs in a folder:

```bash
pdftract extract *.pdf --output-dir results/
```

Each PDF produces a corresponding JSON file in `results/`:

```
results/
  invoice1.pdf.json
  invoice2.pdf.json
  receipt.pdf.json
```

For recursive folder processing, use the `grep` command to search across all PDFs:

```bash
pdftract grep "search term" /path/to/folder
```

This outputs matching filenames and page numbers:

```
invoice.pdf:3: "search term" found on page 3
receipt.pdf:1: "search term" found on page 1
```

## Common Options

| Option | Description |
|---|---|
| `--output FILE` | Write output to file instead of stdout |
| `--text` | Output plain text instead of JSON |
| `--output-dir DIR` | Directory for batch output (with `*` glob) |
| `--auto` | Auto-detect and apply document profile |
| `--profile NAME` | Use specific profile (skip auto-detection) |
| `--password PASS` | Password for encrypted PDFs |
| `--pages N-M` | Extract specific page range |
| `--ocr` | Force OCR mode for all pages |

See [CLI Reference](./cli/) for complete command documentation.

## What's Next?

- Explore the [CLI Reference](./cli/) for advanced options
- Read [JSON Schema Reference](./schema/) for output format details
- Check [Profiles](./profiles/) for document-type-specific extraction
- Try the [Python SDK](./sdk/python.md) for programmatic access

## Troubleshooting

**Extraction fails with "unsupported encryption"**

The PDF is encrypted with a password. Use `--password`:

```bash
pdftract extract encrypted.pdf --password yourpassword
```

**Output has wrong reading order**

Some PDFs have malformed internal structure. Try `--auto` to enable profile-based layout recovery, or use `--ocr` to force OCR-based extraction.

**Poor accuracy on scanned documents**

Ensure the OCR features are enabled. The Docker `:ocr` and `:full` images include Tesseract. If building from source, enable the `ocr` feature:

```bash
cargo install pdftract --features ocr
```

For more help, see [Troubleshooting](./troubleshooting/).
