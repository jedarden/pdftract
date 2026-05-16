# Extraction Output Schema, API Surface, and Structured Output Design

## Overview

The pdftract extraction output schema is designed around a single, governing principle: every downstream use case — RAG ingestion, full-text search indexing, accessibility auditing, forensic analysis, and archival preservation — must be satisfiable from a single extraction pass without requiring re-processing. This demands a schema that is simultaneously comprehensive and layered, exposing fine-grained atomic data at lower levels while assembling semantic structure at higher levels, with clean separation between what belongs at document scope and what belongs at page scope.

---

## Document-Level Structure

The root JSON object is the document envelope. It carries everything that is not inherently per-page: document `metadata`, the navigation `outline` (bookmark tree derived from the PDF `/Outlines` dictionary), `threads` (article thread chains linking related content across non-contiguous pages), `attachments` (embedded files and portfolio entries), `signatures` (digital signature fields with their coverage and validation state), document-scoped `links` (cross-document or external URI targets that span multiple pages or resolve at document level), `form_fields` (AcroForm and XFA field definitions with their values), an `extraction_quality` aggregate summarizing confidence across the entire document, and an `errors` array holding all diagnostic events from the extraction run.

The `pages` array is also at document level, each entry being a self-contained page object. Placing `pages` at the root allows consumers to address any page by index without traversing nested structures, and makes NDJSON streaming (described below) a natural projection of the same schema rather than a separate format.

Fields that are inherently global — metadata, signatures, the outline tree, embedded attachments — must not be duplicated inside page objects. Conversely, anything that varies per-page (geometry, content blocks, annotations) must not be flattened to the document level. This division is what keeps both the full-document JSON and per-page NDJSON frames self-consistent.

---

## Page-Level Structure

Each page object carries `page_index` (zero-based integer, matching array position), `page_label` (the human-readable label from the PDF `/PageLabels` number tree, e.g. "iv", "A-3", "1"), `width` and `height` in points (1/72 inch), and `rotation` in degrees clockwise (0, 90, 180, or 270). The `page_type` field is a hint produced by the classifier: `text`, `scanned`, `mixed`, `blank`, or `figure_only`. This hint is informational; it does not gate access to content but signals to consumers how much confidence to assign to the extracted text.

Within a page, content is represented at two granularities: `spans` and `blocks`. Spans are the atomic unit — individual sequences of characters sharing identical rendering properties. Blocks are semantic groupings assembled from one or more spans. Both arrays coexist on the page object. This dual representation is deliberate: applications that need character-level font and position data (accessibility auditing, forensic comparison) operate on spans directly; applications that need paragraph flow (RAG chunking, search indexing) operate on blocks. A span carries a reference by index so that block-level consumers can always descend to span-level data when needed.

Page-level `annotations` are distinct from block content. They include highlights, stamps, sticky notes, and ink annotations, each with their own `bbox`, `subtype`, `author`, `created`, `modified`, and `contents` fields. Links (URI and internal-destination) appear in annotations as `subtype: link` with a `uri` or `dest` field rather than being mixed into the text stream.

---

## Span Schema

A span is the smallest unit of extraction output. Its fields are: `text` (the decoded Unicode string), `bbox` as a four-element array `[x0, y0, x1, y1]` in points with the coordinate origin at the lower-left of the page (PDF default), `font` (the font name as declared in the resource dictionary), `size` (the rendered glyph size in points, combining the font matrix and CTM), `color` (the fill color as a CSS hex string like `"#1a1a1a"`, or `null` if the color is not expressible as RGB, for example a spot color), `rendering_mode` (an integer 0–7 matching the PDF `Tr` operator: 0 = fill, 3 = invisible, etc.), `confidence` (a float 0.0–1.0), `confidence_source` (one of `"native"`, `"ocr"`, `"heuristic"`), `lang` (a BCP-47 language tag if detected, otherwise `null`), and `flags` (a set of strings: `"bold"`, `"italic"`, `"smallcaps"`, `"subscript"`, `"superscript"`).

The `confidence` and `confidence_source` pair allows consumers to apply their own filtering thresholds. A span with `confidence_source: "native"` and high confidence came from decoded font mapping with no ambiguity. A span with `confidence_source: "ocr"` was produced by the raster OCR pipeline and warrants lower trust. The `rendering_mode` field is critical for invisible-text detection: text placed with `Tr 3` is present in the stream but was never intended to be visible — forensic and accessibility consumers need this distinction.

---

## Block Schema

A block aggregates spans into a semantic unit. Its fields are: `kind` (one of `paragraph`, `heading`, `table`, `list`, `figure`, `header`, `footer`, `caption`, `code`, `formula`), `text` (the concatenated plain text of all member spans, with whitespace normalized), `bbox` (the union bounding box of all member spans), `spans` (an array of span indices referencing the page-level `spans` array), `level` (an integer 1–6 for `kind: heading`, matching h1–h6 semantics, omitted for all other kinds), and `confidence` (the minimum confidence across member spans, representing the weakest link).

The `kind` field is the primary classification signal. Consumers building a table of contents use `kind: heading` with `level`. Consumers extracting body text filter to `kind: paragraph`. The `header` and `footer` kinds identify repeated page-margin content that should typically be excluded from body-text flows. The `figure` kind marks regions where no extractable text is present but a visual element occupies the bbox — useful for flagging gaps in extraction coverage.

---

## Table Output

Tables are represented with two complementary structures. The block with `kind: table` gives the bounding box and concatenated text for downstream consumers that do not need cell structure. For consumers that do, a parallel `table` object at page level (keyed to the block index) provides the full nested structure: `rows` is an array of row objects, each containing a `cells` array. Each cell carries `text`, `bbox`, `rowspan` (default 1), `colspan` (default 1), and `is_header` (boolean, derived from tagged PDF structure or heuristic header-row detection). This separation ensures that table-aware consumers get machine-readable structure while table-unaware consumers still receive coherent concatenated text from the block.

---

## Metadata Schema

The document `metadata` object surfaces all standard PDF document information dictionary fields: `/Title`, `/Author`, `/Subject`, `/Keywords`, `/Creator`, `/Producer`, `/CreationDate`, and `/ModDate` (ISO-8601 strings). It also carries derived signals: `page_count`, `pdf_version` (e.g. `"1.7"`), `is_tagged` (boolean, true if a `/MarkInfo` dictionary with `Marked: true` is present), `is_encrypted` (boolean), `conformance` (one of `"none"`, `"PDF-A-1a"`, `"PDF-A-1b"`, `"PDF-A-2a"`, `"PDF-A-2b"`, `"PDF-A-2u"`, `"PDF-A-3a"`, `"PDF-A-3b"`, `"PDF-A-3u"`, `"PDF-UA-1"`, `"PDF-UA-2"`, `"PDF-X-1a"` — validated, not merely declared), `contains_javascript` (boolean), `contains_xfa` (boolean), and `generator` (a heuristic string identifying the producing application inferred from `/Creator` and `/Producer` patterns). XMP metadata is normalized into these same fields where it provides richer values than the document information dictionary.

---

## Plain Text Output Mode

When invoked with `--text`, pdftract emits a single UTF-8 string rather than JSON. Reading order within each page serializes blocks in top-to-bottom, left-to-right order after rotation normalization. Paragraphs are separated by double newlines. Page breaks are represented as a form feed character (`\f`) placed between pages, which is the standard convention recognized by text processing tools. Headers and footers are excluded by default; `--include-headers-footers` re-enables them. The plain text mode is a lossy projection — bbox, font, confidence, and structure are all discarded — intended for indexing pipelines that require only content, not provenance.

---

## NDJSON Streaming Mode

For large documents, the `--stream` flag activates NDJSON output: one JSON object per line, emitted as each page completes extraction. The first line is a document header frame containing the `schema_version`, `metadata`, `outline`, and a `total_pages` count. Each subsequent page frame contains a single page object in the same schema as the `pages` array entries in full-document mode, plus a `frame: "page"` discriminator field. The final line is a document footer frame (`frame: "footer"`) carrying `extraction_quality`, `errors`, `threads`, `attachments`, `signatures`, `form_fields`, and document-scoped `links` — all the fields that can only be finalized after all pages have been processed. This design allows consumers to begin processing page one while pages two through N are still being extracted, which is essential for large documents and server-side streaming APIs.

---

## Error and Diagnostic Schema

Every diagnostic event from the extraction pipeline is recorded in the `errors` array at document level. Each entry has: `code` (a stable string identifier like `"FONT_CMAP_MISSING"`, `"GLYPH_UNMAPPED"`, `"OCR_FALLBACK"`, `"XREF_REPAIRED"`, `"ENCRYPTION_UNSUPPORTED"`), `message` (a human-readable description), `page_index` (integer or `null` for document-level events), `severity` (one of `"error"`, `"warning"`, `"info"`), and `location` (an optional object with `object_number` and `generation_number` identifying the PDF indirect object where the issue originated). Error codes are namespaced by area: `FONT_*` for encoding failures, `OCR_*` for raster fallback events, `STRUCT_*` for structure tree problems, `XREF_*` for cross-reference repairs. Integration developers can key on codes programmatically rather than parsing messages, which remain subject to wording changes between releases.

---

## Versioning and Stability

The root document object carries `schema_version: "1.0"`. All fields documented here are stable in the 1.x series: their names, types, and semantics will not change in a breaking way. New fields may be added to any object in minor releases; consumers must ignore unknown fields. Fields marked with `"experimental": true` in the specification are exempt from the stability guarantee and may be removed or renamed between minor versions.

The `extensions` object at the root level is reserved for non-breaking additions that have not yet graduated to stable status. Extension fields use a namespaced key format (`"pdftract.ocr.engine_version"`) to avoid collision with future stable fields. Consumers that rely on extension fields must treat them as experimental regardless of the version in which they appear.

This schema is designed so that a consumer written against version 1.0 will continue to function correctly when processing output from any 1.x release, receiving richer data it may ignore rather than encountering structural incompatibilities. Major version increments (2.0, 3.0) signal breaking changes and require explicit consumer migration.
