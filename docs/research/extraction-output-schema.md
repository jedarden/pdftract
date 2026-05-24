# Extraction Output Schema, API Surface, and Structured Output Design

## Overview

The pdftract extraction output schema is designed around a single, governing principle: every downstream use case — RAG ingestion, full-text search indexing, accessibility auditing, forensic analysis, and archival preservation — must be satisfiable from a single extraction pass without requiring re-processing. This demands a schema that is simultaneously comprehensive and layered, exposing fine-grained atomic data at lower levels while assembling semantic structure at higher levels, with clean separation between what belongs at document scope and what belongs at page scope.

**Machine-readable schema:** This document is the human-readable specification. The machine-readable JSON Schema is available at [`docs/schema/v1.0/pdftract.schema.json`](../schema/v1.0/pdftract.schema.json) and should be used for automated validation.

---

## Document-Level Structure

The root JSON object is the document envelope. It carries everything that is not inherently per-page: document `metadata`, the navigation `outline` (bookmark tree derived from the PDF `/Outlines` dictionary), `threads` (article thread chains linking related content across non-contiguous pages), `attachments` (embedded files and portfolio entries), `signatures` (digital signature fields with their coverage and validation state), document-scoped `links` (cross-document or external URI targets that span multiple pages or resolve at document level), `form_fields` (AcroForm and XFA field definitions with their values), an `extraction_quality` aggregate summarizing confidence across the entire document, and an `errors` array holding all diagnostic events from the extraction run.

The `pages` array is also at document level, each entry being a self-contained page object. Placing `pages` at the root allows consumers to address any page by index without traversing nested structures, and makes NDJSON streaming (described below) a natural projection of the same schema rather than a separate format.

Fields that are inherently global — metadata, signatures, the outline tree, embedded attachments — must not be duplicated inside page objects. Conversely, anything that varies per-page (geometry, content blocks, annotations) must not be flattened to the document level. This division is what keeps both the full-document JSON and per-page NDJSON frames self-consistent.

### Root Fields

| Field | Type | Description |
|-------|------|-------------|
| `schema_version` | string | Schema version identifier (e.g., `"1.0"`) |
| `fingerprint` | string | PDF fingerprint for verification (format: `pdftract-v1:<hex>`) |
| `metadata` | object | Document-level metadata (see Metadata Schema below) |
| `pages` | array | Array of page objects (see Page-Level Structure below) |
| `outline` | array | Recursive bookmark tree (empty if no bookmarks) |
| `threads` | array | Article thread chains (empty until Phase 7) |
| `attachments` | array | Embedded files (see Attachments Schema below) |
| `signatures` | array | Digital signature metadata (empty until Phase 7) |
| `form_fields` | array | AcroForm/XFA field definitions (empty until Phase 7) |
| `links` | array | Document-scoped hyperlinks (empty until Phase 7) |
| `extraction_quality` | object | Aggregate quality metrics across all pages |
| `errors` | array | Diagnostic events from extraction run |

---

## Page-Level Structure

Each page object carries both positional identifiers and classification metadata.

### Page Identification Fields

| Field | Type | Description |
|-------|------|-------------|
| `page_index` | integer | Zero-based page index, canonical for programmatic use. Used in all internal references (error diagnostics, NDJSON frame ordering, cache keys). SDK code and downstream tools MUST key on `page_index` for programmatic access. |
| `page_number` | integer | One-based page number, equal to `page_index + 1`. Emitted alongside `page_index` as a convenience for human-facing display. This field is informational only; all programmatic access should use `page_index`. |
| `page_label` | string\|null | Human-readable label from the PDF `/PageLabels` number tree (e.g., `"iv"`, `"A-3"`, `"1"`). Absent (`null`) if the PDF defines no page labels. |

### Page Geometry and Classification

| Field | Type | Description |
|-------|------|-------------|
| `width` | number | Page width in points (1/72 inch) |
| `height` | number | Page height in points (1/72 inch) |
| `rotation` | integer | Page rotation in degrees clockwise (0, 90, 180, or 270) |
| `page_type` | string | Classification hint from the page classifier (see Page Type Enum below) |

### Page Type Enum

The `page_type` field is produced by the classifier and signals to consumers how much confidence to assign to the extracted text. This taxonomy is stable per [INV-9](../plan/plan.md#inv-9) — new values require an ADR.

| Value | Description |
|-------|-------------|
| `"text"` | Pure vector text PDF — all content extracted from font glyphs with high confidence |
| `"scanned"` | Raster image page — text extracted via OCR (or OCR-assisted for broken vector pages) |
| `"mixed"` | Hybrid page containing both vector text regions and scanned image regions |
| `"broken_vector"` | Vector page with corrupted encoding (e.g., bad ToUnicode CMAPs); extraction produced low-confidence text. See Phase 5.5 for the OCR escalation path. If the binary was compiled without the `ocr` feature, `broken_vector` pages are emitted as-is with a `BROKENVECTOR_OCR_UNAVAILABLE` diagnostic. |
| `"blank"` | Page with no text and no images |
| `"figure_only"` | Page with only image XObjects, no text glyphs |

### Content Arrays

Within a page, content is represented at two granularities: `spans` and `blocks`. Spans are the atomic unit — individual sequences of characters sharing identical rendering properties. Blocks are semantic groupings assembled from one or more spans. Both arrays coexist on the page object. This dual representation is deliberate: applications that need character-level font and position data (accessibility auditing, forensic comparison) operate on spans directly; applications that need paragraph flow (RAG chunking, search indexing) operate on blocks. A span carries a reference by index so that block-level consumers can always descend to span-level data when needed.

| Field | Type | Description |
|-------|------|-------------|
| `spans` | array | Atomic text spans (see Span Schema below) |
| `blocks` | array | Semantic block groupings (see Block Schema below) |
| `tables` | array | Parallel table structure objects for `kind: table` blocks (see Table Output below) |
| `annotations` | array | Page-level annotations (highlights, stamps, notes, links; empty until Phase 7) |

Page-level `annotations` are distinct from block content. They include highlights, stamps, sticky notes, and ink annotations, each with their own `bbox`, `subtype`, `author`, `created`, `modified`, and `contents` fields. Links (URI and internal-destination) appear in annotations as `subtype: link` with a `uri` or `dest` field rather than being mixed into the text stream.

---

## Span Schema

A span is the smallest unit of extraction output. Its fields are: `text` (the decoded Unicode string), `bbox` as a four-element array `[x0, y0, x1, y1]` in points with the coordinate origin at the lower-left of the page (PDF default), `font` (the font name as declared in the resource dictionary), `size` (the rendered glyph size in points, combining the font matrix and CTM), `color` (the fill color as a CSS hex string like `"#1a1a1a"`, or `null` if the color is not expressible as RGB, for example a spot color), `rendering_mode` (an integer 0–7 matching the PDF `Tr` operator: 0 = fill, 3 = invisible, etc.), `confidence` (a float 0.0–1.0), `confidence_source` (one of `"native"`, `"ocr"`, `"heuristic"`), `lang` (a BCP-47 language tag if detected, otherwise `null`), and `flags` (a set of strings: `"bold"`, `"italic"`, `"smallcaps"`, `"subscript"`, `"superscript"`).

| Field | Type | Description |
|-------|------|-------------|
| `text` | string | The decoded Unicode string |
| `bbox` | array | Bounding box `[x0, y0, x1, y1]` in PDF user-space points (origin at lower-left) |
| `font` | string | Font name as declared in the resource dictionary |
| `size` | number | Rendered glyph size in points (combines font matrix and CTM) |
| `color` | string\|null | Fill color as CSS hex string (e.g., `"#1a1a1a"`), or `null` if not expressible as RGB |
| `rendering_mode` | integer | PDF `Tr` operator value (0 = fill, 3 = invisible, etc.) |
| `confidence` | number | Confidence score 0.0–1.0 |
| `confidence_source` | string | One of `"native"`, `"ocr"`, `"heuristic"` |
| `lang` | string\|null | BCP-47 language tag if detected, otherwise `null` |
| `flags` | array | Set of style flags: `"bold"`, `"italic"`, `"smallcaps"`, `"subscript"`, `"superscript"` |

The `confidence` and `confidence_source` pair allows consumers to apply their own filtering thresholds. A span with `confidence_source: "native"` and high confidence came from decoded font mapping with no ambiguity. A span with `confidence_source: "ocr"` was produced by the raster OCR pipeline and warrants lower trust. The `rendering_mode` field is critical for invisible-text detection: text placed with `Tr 3` is present in the stream but was never intended to be visible — forensic and accessibility consumers need this distinction.

---

## Block Schema

A block aggregates spans into a semantic unit. The `kind` field is the primary classification signal.

### Block Fields

| Field | Type | Description |
|-------|------|-------------|
| `kind` | string | Block kind/type (see Block Kind Enum below) |
| `text` | string | Concatenated plain text of all member spans, with whitespace normalized |
| `bbox` | array | Union bounding box of all member spans `[x0, y0, x1, y1]` in points |
| `spans` | array | Array of span indices referencing the page-level `spans` array |
| `level` | integer\|null | Heading level 1–6 for `kind: heading` (matches h1–h6 semantics), `null` for other kinds |
| `confidence` | number | Minimum confidence across member spans (weakest link) |

### Block Kind Enum

| Value | Description |
|-------|-------------|
| `"paragraph"` | Default body text block |
| `"heading"` | Heading or subheading (has `level` field 1–6) |
| `"list"` | List item(s) — bullet or numbered |
| `"table"` | Tabular data (see Table Output below) |
| `"figure"` | Image or graphic region with no extractable text |
| `"caption"` | Figure or table caption (small font, follows a figure/table block) |
| `"code"` | Monospace code block (indented, uses monospace font) |
| `"formula"` | Mathematical formula (detected via OpenType Math in Phase 7) |
| `"watermark"` | Watermark or background text (excluded from body text flow) |
| `"header"` | Repeated page-margin content at top (deduplicated across pages) |
| `"footer"` | Repeated page-margin content at bottom (deduplicated across pages) |

Consumers building a table of contents use `kind: heading` with `level`. Consumers extracting body text filter to `kind: paragraph`. The `header` and `footer` kinds identify repeated page-margin content that should typically be excluded from body-text flows. The `figure` kind marks regions where no extractable text is present but a visual element occupies the bbox — useful for flagging gaps in extraction coverage.

---

## Table Output

Tables are represented with two complementary structures. The block with `kind: table` gives the bounding box and concatenated text for downstream consumers that do not need cell structure. For consumers that do, a parallel `table` object at page level (keyed to the block index) provides the full nested structure: `rows` is an array of row objects, each containing a `cells` array. Each cell carries `text`, `bbox`, `rowspan` (default 1), `colspan` (default 1), and `is_header` (boolean, derived from tagged PDF structure or heuristic header-row detection). This separation ensures that table-aware consumers get machine-readable structure while table-unaware consumers still receive coherent concatenated text from the block.

### Table Object Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (e.g., `"table_0"`) |
| `bbox` | array | Bounding box `[x0, y0, x1, y1]` in points |
| `rows` | array | Array of row objects (see Row Schema below) |
| `header_rows` | integer | Number of contiguous header rows at top |
| `detection_method` | string | One of `"line_based"`, `"borderless"` |
| `continued` | boolean | Whether table continues on next page |
| `continued_from_prev` | boolean | Whether table is continuation from previous page |
| `page_index` | integer | Zero-based page index where table appears |

### Row Schema

| Field | Type | Description |
|-------|------|-------------|
| `bbox` | array | Bounding box `[x0, y0, x1, y1]` in points |
| `cells` | array | Array of cell objects (see Cell Schema below) |
| `is_header` | boolean | Whether this row is a header row |

### Cell Schema

| Field | Type | Description |
|-------|------|-------------|
| `bbox` | array | Bounding box `[x0, y0, x1, y1]` in points |
| `text` | string | Concatenated text content of all spans in the cell |
| `spans` | array | References to spans in the page's `spans` array (integer indices) |
| `row` | integer | Zero-based row index within the table |
| `col` | integer | Zero-based column index within the table |
| `rowspan` | integer | Number of rows this cell spans (default 1) |
| `colspan` | integer | Number of columns this cell spans (default 1) |
| `is_header_row` | boolean | Whether this cell is in a header row |

---

## Attachments Schema

Extracted embedded files from PDF portfolios and `/EmbeddedFiles` name trees.

### Attachment Fields

| Field | Type | Description |
|-------|------|-------------|
| `filename` | string | Filename from `/F` or `/UF` in the Filespec dictionary |
| `description` | string\|null | Description from `/Desc`, or `null` if absent |
| `mime_type` | string\|null | MIME type hint from `/Subtype` in the EF stream dictionary |
| `size` | integer\|null | Decoded stream size in bytes, or `null` if unavailable |
| `created` | string\|null | ISO-8601 creation date from `/Params /CreationDate`, or `null` |
| `modified` | string\|null | ISO-8601 modification date from `/Params /ModDate`, or `null` |
| `checksum` | string\|null | Checksum from `/Params /CheckSum`, or `null` |
| `data` | string\|null | Base64-encoded content of the decoded attachment stream, or `null` if truncated (see Size Limit below) |
| `truncated` | boolean | `true` if the attachment exceeded the size limit and `data` is `null` |

### Size Limit and Encoding

If attachment stream decoded size > 50 MB, include metadata only and set `data: null` with `truncated: true`. When non-null, `data` is the base64-encoded content of the decoded attachment stream using the standard Base64 alphabet with no line breaks and padding preserved. The JSON Schema reflects this as `{"type": "string", "contentEncoding": "base64"}` for this field. In the Python API, `data` is returned as a Python `bytes` object (PyO3 converts from base64 automatically). In the CLI `--text` mode, attachments are not included.

---

## Metadata Schema

The document `metadata` object surfaces all standard PDF document information dictionary fields, derived signals, and profile-based classification results.

### Standard PDF Fields

| Field | Type | Description |
|-------|------|-------------|
| `title` | string\|null | PDF `/Title` |
| `author` | string\|null | PDF `/Author` |
| `subject` | string\|null | PDF `/Subject` |
| `keywords` | string\|null | PDF `/Keywords` |
| `creator` | string\|null | PDF `/Creator` |
| `producer` | string\|null | PDF `/Producer` |
| `creation_date` | string\|null | ISO-8601 string from `/CreationDate` |
| `modification_date` | string\|null | ISO-8601 string from `/ModDate` |

### Derived Signals

| Field | Type | Description |
|-------|------|-------------|
| `page_count` | integer | Total number of pages |
| `pdf_version` | string | PDF version (e.g., `"1.7"`) |
| `is_tagged` | boolean | `true` if `/MarkInfo /Marked: true` is present |
| `is_encrypted` | boolean | `true` if document is encrypted |
| `conformance` | string | One of `"none"`, `"PDF-A-1a"`, `"PDF-A-1b"`, `"PDF-A-2a"`, `"PDF-A-2b"`, `"PDF-A-2u"`, `"PDF-A-3a"`, `"PDF-A-3b"`, `"PDF-A-3u"`, `"PDF-UA-1"`, `"PDF-UA-2"`, `"PDF-X-1a"` |
| `contains_javascript` | boolean | `true` if JavaScript actions are present |
| `contains_xfa` | boolean | `true` if XFA forms are present |
| `ocg_present` | boolean | `true` if optional content groups (layers) are present |
| `generator` | string | Heuristic string identifying the producing application |

### Profile-Based Classification (Phase 7.10)

When a document profile matches (via `--auto` or `--profile`), the metadata includes classification fields:

| Field | Type | Description |
|-------|------|-------------|
| `document_type` | string\|null | Matched profile type (e.g., `"invoice"`, `"receipt"`, `"form"`) |
| `document_type_confidence` | number\|null | Classification confidence 0.0–1.0 |
| `document_type_reasons` | array\|null | Array of strings explaining why this type matched (e.g., `"text_contains matched 'Invoice #'"`, `"structural.has_table = true"`) |
| `profile_name` | string\|null | Name of the matched profile (e.g., `"invoice"`) |
| `profile_version` | string\|null | Profile version string (e.g., `"1.0.0"`) |
| `profile_fields` | object\|null | Map from field name to typed value, per the matched profile's schema. Each profile defines its own field set; see `profiles/builtin/<type>/README.md` for profile-specific field documentation. |

XMP metadata is normalized into these same fields where it provides richer values than the document information dictionary.

---

## Plain Text Output Mode

When invoked with `--text`, pdftract emits a single UTF-8 string rather than JSON. Reading order within each page serializes blocks in top-to-bottom, left-to-right order after rotation normalization. Paragraphs are separated by double newlines. Page breaks are represented as a form feed character (`\f`) placed between pages, which is the standard convention recognized by text processing tools. Headers and footers are excluded by default; `--include-headers-footers` re-enables them. The plain text mode is a lossy projection — bbox, font, confidence, and structure are all discarded — intended for indexing pipelines that require only content, not provenance.

---

## NDJSON Streaming Mode

For large documents, the `--stream` flag activates NDJSON output: one JSON object per line, emitted as each page completes extraction. The first line is a document header frame containing the `schema_version`, `metadata`, `outline`, and a `total_pages` count. Each subsequent page frame contains a single page object in the same schema as the `pages` array entries in full-document mode, plus a `frame: "page"` discriminator field. The final line is a document footer frame (`frame: "footer"`) carrying `extraction_quality`, `errors`, `threads`, `attachments`, `signatures`, `form_fields`, and document-scoped `links` — all the fields that can only be finalized after all pages have been processed. This design allows consumers to begin processing page one while pages two through N are still being extracted, which is essential for large documents and server-side streaming APIs.

### Frame Sequence

1. **Header frame:** `{"frame":"header","schema_version":"1.0","metadata":{...},"outline":[...],"total_pages":N}`
2. **Page frames:** `{"frame":"page","page_index":N,...}` — emitted in page_index order with a window of 8 pages maximum for out-of-order buffering
3. **Footer frame:** `{"frame":"footer","extraction_quality":{...},"errors":[...],"threads":[],"attachments":[],"signatures":[],"form_fields":[],"links":[]}`

---

## Error and Diagnostic Schema

Every diagnostic event from the extraction pipeline is recorded in the `errors` array at document level. Each entry has: `code` (a stable string identifier like `"FONT_CMAP_MISSING"`, `"GLYPH_UNMAPPED"`, `"OCR_FALLBACK"`, `"XREF_REPAIRED"`, `"ENCRYPTION_UNSUPPORTED"`), `message` (a human-readable description), `page_index` (integer or `null` for document-level events), `severity` (one of `"error"`, `"warning"`, `"info"`), and `location` (an optional object with `object_number` and `generation_number` identifying the PDF indirect object where the issue originated). Error codes are namespaced by area: `FONT_*` for encoding failures, `OCR_*` for raster fallback events, `STRUCT_*` for structure tree problems, `XREF_*` for cross-reference repairs. Integration developers can key on codes programmatically rather than parsing messages, which remain subject to wording changes between releases.

### Error Entry Fields

| Field | Type | Description |
|-------|------|-------------|
| `code` | string | Stable string identifier (e.g., `"FONT_CMAP_MISSING"`) |
| `message` | string | Human-readable description |
| `page_index` | integer\|null | Page index where error occurred, or `null` for document-level |
| `severity` | string | One of `"error"`, `"warning"`, `"info"` |
| `location` | object\|null | PDF object reference with `object_number` and `generation_number` |

---

## Schema Version Compatibility

The root document object carries `schema_version: "1.0"`. All fields documented here are stable in the 1.x series: their names, types, and semantics will not change in a breaking way. New fields may be added to any object in minor releases; consumers must ignore unknown fields. Fields marked with `"experimental": true` in the specification are exempt from the stability guarantee and may be removed or renamed between minor versions.

The `extensions` object at the root level is reserved for non-breaking additions that have not yet graduated to stable status. Extension fields use a namespaced key format (`"pdftract.ocr.engine_version"`) to avoid collision with future stable fields. Consumers that rely on extension fields must treat them as experimental regardless of the version in which they appear.

### Additive Evolution Rules

This schema follows JSON-Schema-style additive-evolution rules (see [plan.md lines 3659-3685](../plan/plan.md#L3659-L3685)):

- `schema_version: "1.1"` SHALL be a **strict superset** of `"1.0"`: every `"1.0"`-valid document SHALL also be `"1.1"`-valid
- New fields are optional; no field is removed; no field's semantic meaning changes within a major version
- Semantic changes to an existing field require a major-version bump and a corresponding `schema_version` major bump (`"2.0"`)
- Downstream consumers reading `"1.1"` output with a `"1.0"`-aware parser MUST tolerate unknown fields (the schema explicitly sets `additionalProperties: true` for the v1.x line)

This schema is designed so that a consumer written against version 1.0 will continue to function correctly when processing output from any 1.x release, receiving richer data it may ignore rather than encountering structural incompatibilities. Major version increments (2.0, 3.0) signal breaking changes and require explicit consumer migration.
