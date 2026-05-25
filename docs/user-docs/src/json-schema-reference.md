# JSON Schema Reference

> **Schema version:** 1.0  
> **Schema URL:** https://pdftract.com/schema/v1.0/pdftract.schema.json  
> **Source of truth:** `docs/schema/v1.0/pdftract.schema.json`

This page provides a human-readable rendering of the pdftract output schema. The JSON Schema is the authoritative definition (per [INV-11](../plan/plan.md)), validated in CI for all test fixtures.

## Top-Level Structure

```json
{
  "fingerprint": "pdftract-v1:a7f3c8d9...",
  "pages": [...],
  "metadata": {...},
  "signatures": [...],
  "form_fields": [...]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `fingerprint` | string | Yes | Phase 1.7 fingerprint of the source PDF. Format: `"pdftract-v1:" + hex(SHA-256)`. Used for receipt verification. |
| `pages` | array | Yes | Extracted pages, each containing spans and blocks. |
| `metadata` | object | Yes | ExtractionMetadata object with page count, diagnostics, receipts mode, etc. |
| `signatures` | array | Yes | Digital signatures extracted from the document. Empty when no signature fields exist. |
| `form_fields` | array | Yes | Interactive form fields from AcroForm/XFA. Empty when no form fields exist. |

## Document Metadata

The `metadata` object contains extraction-level information:

```json
{
  "page_count": 10,
  "span_count": 842,
  "block_count": 156,
  "error_count": 0,
  "receipts_mode": "off",
  "diagnostics": ["WARN: page 3: low coverage (54%) - possible scanned content"],
  "cache_status": "hit",
  "cache_age_seconds": 1240,
  "reading_order_algorithm": "robust-topo"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `page_count` | integer | Total number of pages in the document. |
| `span_count` | integer | Number of spans extracted across all pages. |
| `block_count` | integer | Number of blocks extracted across all pages. |
| `error_count` | integer | Number of pages that failed to extract. |
| `receipts_mode` | string | Receipts mode used: `"off"`, `"lite"`, or `"svg"`. |
| `diagnostics` | array | Diagnostic messages emitted during extraction (coverage warnings, etc.). |
| `cache_status` | string/null | Cache status: `"hit"`, `"miss"`, or `"skipped"`. |
| `cache_age_seconds` | integer/null | Cache entry age in seconds (only present when `cache_status == "hit"`). |
| `reading_order_algorithm` | string/null | Reading order algorithm used for this extraction. |

## Page Result

Each page in the `pages` array contains:

```json
{
  "index": 0,
  "spans": [...],
  "blocks": [...],
  "tables": [...],
  "error": null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `index` | integer | Yes | Zero-based page index. This is the canonical identifier for programmatic use. |
| `spans` | array | Yes | Extracted spans (text fragments with consistent styling). |
| `blocks` | array | Yes | Extracted blocks (semantic units like paragraphs, headings). |
| `tables` | array | Yes | Extracted tables with cell-level structure. Empty when no tables detected. |
| `error` | string/null | Yes | Error message if extraction failed for this page. |

### Span

A span is the smallest unit of extracted text, representing a contiguous run of text with consistent font and styling.

```json
{
  "text": "The quick brown fox",
  "bbox": [72.0, 612.0, 245.5, 624.3],
  "font": "Helvetica-Bold",
  "size": 12.0,
  "column": 0,
  "confidence": 0.98,
  "receipt": null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `text` | string | Yes | The extracted text content. |
| `bbox` | array | Yes | Bounding box in PDF user-space points. Format: `[x0, y0, x1, y1]` where (x0, y0) is the bottom-left corner and (x1, y1) is the top-right corner. Units are 1/72 inch. |
| `font` | string | Yes | Font name or identifier. |
| `size` | number | Yes | Font size in points. |
| `column` | integer/null | No | Column index (0-based) assigned by Phase 4.3 column detection. Null for spans outside any detected column. |
| `confidence` | number/null | No | Confidence score (0.0 to 1.0). Present when OCR is used or extraction has uncertainty. |
| `receipt` | object/null | No | Cryptographic receipt for verification. Present when `--receipts=lite` or `--receipts=svg` is enabled. |

### Block

A block is a higher-level semantic unit composed of one or more spans.

```json
{
  "kind": "paragraph",
  "text": "The quick brown fox jumps over the lazy dog.",
  "bbox": [72.0, 600.0, 540.0, 650.0],
  "level": null,
  "table_index": null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `kind` | string | Yes | The block kind/type. Common values: `"paragraph"`, `"heading"`, `"list"`, `"table"`, `"figure"`. |
| `text` | string | Yes | The concatenated text content of all spans in the block. |
| `bbox` | array | Yes | Bounding box in PDF user-space points. Same format as spans. |
| `level` | integer/null | No | Heading level (1-6) for `"heading"` kind blocks. Null for other block types. |
| `table_index` | integer/null | No | Table index for `"table"` kind blocks. Points to the corresponding entry in the page's `tables` array. |
| `receipt` | object/null | No | Cryptographic receipt for verification. Present when receipts are enabled. |

#### Block Kind Enum

| Value | Description |
|-------|-------------|
| `paragraph` | A paragraph block. |
| `heading` | A heading block (with `level` field 1-6). |
| `list` | A list item block. |
| `table` | A table block (references `tables` array via `table_index`). |
| `figure` | A figure or image block. |
| `code` | A code block or monospace text. |
| `formula` | A mathematical formula. |
| `header` | A page header block. |
| `footer` | A page footer block. |
| `watermark` | A watermark block. |
| `caption` | A caption for a figure or table. |
| `quote` | A blockquote. |

### Table

Tables provide detailed cell-level structure for table blocks.

```json
{
  "id": "table_0",
  "page_index": 2,
  "bbox": [72.0, 400.0, 540.0, 550.0],
  "detection_method": "line_based",
  "header_rows": 1,
  "continued": false,
  "continued_from_prev": false,
  "rows": [...]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique identifier for this table (e.g., `"table_0"`). |
| `page_index` | integer | Yes | Zero-based page index where this table appears. |
| `bbox` | array | Yes | Bounding box in PDF user-space points. |
| `detection_method` | string | Yes | Detection method: `"line_based"` (ruling lines) or `"borderless"` (x0 alignment heuristics). |
| `header_rows` | integer | Yes | Number of contiguous header rows at the top of the table. |
| `continued` | boolean | Yes | Whether this table continues on the next page. |
| `continued_from_prev` | boolean | Yes | Whether this table is a continuation from the previous page. |
| `rows` | array | Yes | Rows in this table, ordered top-to-bottom. |

#### Row

Each row contains cells ordered left-to-right:

```json
{
  "bbox": [72.0, 520.0, 540.0, 540.0],
  "is_header": true,
  "cells": [...]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `bbox` | array | Yes | Bounding box in PDF user-space points. |
| `is_header` | boolean | Yes | Whether this row is a header row. |
| `cells` | array | Yes | Cells in this row, ordered left-to-right. |

#### Cell

```json
{
  "text": "Revenue",
  "bbox": [72.0, 520.0, 180.0, 540.0],
  "row": 0,
  "col": 0,
  "rowspan": 1,
  "colspan": 1,
  "is_header_row": true,
  "spans": [0, 1]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `text` | string | Yes | The concatenated text content of all spans in the cell. |
| `bbox` | array | Yes | Bounding box in PDF user-space points. |
| `row` | integer | Yes | Zero-based row index within the table. |
| `col` | integer | Yes | Zero-based column index within the table. |
| `rowspan` | integer | Yes | Number of rows this cell spans (default 1). |
| `colspan` | integer | Yes | Number of columns this cell spans (default 1). |
| `is_header_row` | boolean | Yes | Whether this cell is in a header row. |
| `spans` | array | Yes | References to spans in the page's `spans` array (indices). |

## Form Fields (Phase 7.4)

Form fields represent interactive form fields from the PDF's AcroForm or XFA data.

> **Note:** Phase 7 placeholders are documented here for forward-compatibility. Fields are present in the schema but return empty arrays until Phase 7 implementation.

```json
{
  "name": "employer_signature",
  "type": "text",
  "value": "John Doe",
  "default": null,
  "read_only": false,
  "required": true,
  "page_index": 2,
  "rect": [72.0, 400.0, 288.0, 420.0],
  "multiline": true,
  "max_length": 100
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | The absolute (dot-joined) field name from the AcroForm. |
| `type` | string | Yes | Field type: `"text"`, `"button"`, `"choice"`, or `"signature"`. |
| `value` | varies | Yes | The current value (structure varies by `type`). |
| `default` | varies | No | The default value (`/DV` entry). |
| `read_only` | boolean | Yes | Whether this field is read-only (bit 1 of `/Ff` flags). |
| `required` | boolean | Yes | Whether this field is required (bit 2 of `/Ff` flags). |
| `page_index` | integer/null | No | Zero-based page index where this field's widget appears. |
| `rect` | array/null | No | Bounding box in PDF user-space points. |
| `multiline` | boolean/null | No | Whether this text field supports multiple lines (text fields only). |
| `max_length` | integer/null | No | Maximum length for text fields (`/MaxLen` entry). |
| `multi_select` | boolean/null | No | Whether this choice field supports multiple selections. |
| `options` | array/null | No | Available options for choice fields (`[export_value, display_name]` pairs). |
| `radio` | boolean/null | No | Whether this button is a radio button (button fields only). |
| `pushbutton` | boolean/null | No | Whether this button is a pushbutton (button fields only). |
| `selected` | boolean/null | No | Selected state for button fields. |
| `state_name` | string/null | No | Appearance state name for button fields (e.g., `"Yes"`, `"Off"`). |

## Signatures (Phase 7.3)

Digital signatures extracted from signature fields.

```json
{
  "field_name": "employer_signature",
  "signer_name": "Jane Corporation",
  "signing_date": "2024-03-15T14:23:51Z",
  "location": "New York, NY",
  "reason": "Contract approval",
  "sub_filter": "adbe.pkcs7.detached",
  "byte_range": [0, 12345, 67890, 456],
  "coverage_fraction": 0.95,
  "validation_status": "not_checked"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `field_name` | string | Yes | The absolute (dot-joined) field name from the AcroForm. |
| `signer_name` | string | Yes | The signer's name from the `/Name` entry. Empty string if absent. |
| `validation_status` | string | Yes | Validation status — always `"not_checked"` in v1. Future versions may add `"valid"`, `"invalid"`, `"indeterminate"`. |
| `signing_date` | string/null | No | The signing date as an ISO 8601 string (RFC 3339 format). |
| `location` | string/null | No | The location of signing from the `/Location` entry. |
| `reason` | string/null | No | The reason for signing from the `/Reason` entry. |
| `sub_filter` | string/null | No | The signature format/filter from the `/SubFilter` entry. |
| `byte_range` | array/null | No | The `/ByteRange` array defining which bytes of the file are signed. |
| `coverage_fraction` | number/null | No | Fraction of the file covered by the signature (0.0 to 1.0). |

## Receipts (Phase 6.8)

Visual citation receipts provide cryptographic proof that extracted text originated from a specific region in a specific PDF.

```json
{
  "pdf_fingerprint": "pdftract-v1:a7f3c8d9...",
  "page_index": 14,
  "bbox": [220.0, 412.0, 412.0, 432.0],
  "content_hash": "sha256:9b21c4e5...",
  "extraction_version": "1.0.0",
  "svg_clip": null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pdf_fingerprint` | string | Yes | Phase 1.7 fingerprint of the source PDF. |
| `page_index` | integer | Yes | Zero-based page index in the source PDF. |
| `bbox` | array | Yes | Bounding box in PDF user-space points. |
| `content_hash` | string | Yes | SHA-256 hash of the NFC-normalized text content. Format: `"sha256:" + hex(SHA-256)`. |
| `extraction_version` | string | Yes | The pdftract version that produced this receipt (semver string). |
| `svg_clip` | string/null | No | SVG clip rendering the glyphs (present only in SVG mode). |

### Receipts Mode

| Mode | Description |
|------|-------------|
| `off` | No receipts generated (default). |
| `lite` | Minimal receipts (~120 bytes each) with fingerprint, page index, bbox, and content hash. |
| `svg` | Extended receipts that include an SVG clip rendering the glyphs. |

## Phase 7 Placeholders

The following fields are included in the schema for forward compatibility but are not yet populated in Phase 6. They will be populated in Phase 7:

- **`pages[].annotations`** - Highlights, stamps, notes, links from `/Annots` (Phase 7)
- **`attachments`** - From `/EmbeddedFiles` name tree (Phase 7.5)
- **`links`** - Document-scoped URI and internal destination links (Phase 7.6)
- **`threads`** - Article thread chains (Phase 7.7)

These fields are present in the schema as empty arrays or null values, allowing consumers to pre-allocate space for future data without breaking when Phase 7 features are added.

## Diagnostics

Diagnostic messages provide visibility into extraction quality and issues:

| Severity | Description |
|----------|-------------|
| `WARN` | Warning - extraction succeeded but with potential quality issues (e.g., low coverage suggesting scanned content). |
| `ERROR` | Error - extraction failed for a specific page or region. |

Example diagnostics:
```json
[
  "WARN: page 3: low coverage (54%) - possible scanned content",
  "ERROR: page 7: failed to extract - corrupt content stream"
]
```

## Coordinate System

All `bbox` values use PDF user-space coordinates:

- **Units:** PDF points (1/72 inch, approximately 0.353 mm)
- **Origin:** Lower-left corner of the page (x=0, y=0)
- **Format:** `[x0, y0, x1, y1]` where (x0, y0) is bottom-left and (x1, y1) is top-right

Example: For a US Letter page (8.5 × 11 inches):
- Width: 612 points (8.5 × 72)
- Height: 792 points (11 × 72)
- Full page bbox: `[0, 0, 612, 792]`

## Schema Validation

Per [INV-11](../plan/plan.md), all JSON output must validate against the schema. CI runs a schema validation step on every fixture:

```bash
# Python validation example
pip install jsonschema
jsonschema -i output.json docs/schema/v1.0/pdftract.schema.json
```

## Plan References

- **Phase 6.1** (lines 2018-2051): JSON output full schema implementation
- **Phase 6.8** (lines 2400+): Visual citation receipts
- **Phase 7.3** (lines 2750+): Digital signatures
- **Phase 7.4** (lines 2800+): Form fields
- **INV-11** (line 841): Schema validation invariant

For the complete field-by-field rationale, see the [extraction output schema research doc](../research/extraction-output-schema.md).
