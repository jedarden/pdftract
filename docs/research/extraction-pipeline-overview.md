# pdftract Extraction Pipeline: End-to-End Architectural Overview

This document synthesizes the 36 specialized research documents in this directory into a coherent architectural blueprint for implementing the pdftract Rust PDF text extraction library. It describes the ordered sequence of stages, decision points, and data transformations that take a PDF file as input and produce readable, structured text as output. Engineers implementing pdftract should treat this as the canonical pipeline reference and consult the named component documents for deeper detail on each subsystem.

---

## Pipeline Inputs and Outputs

**Input.** The pipeline accepts either a file path (opened via memory-mapped I/O for zero-copy reads) or an in-memory byte slice. All subsequent parsing operates on the raw bytes through a shared reference; no additional buffering is introduced at the entry point. Configuration is provided via an `ExtractionOptions` struct with fields including: `ocr_enabled: bool`, `ocr_language: Vec<String>`, `extract_forms: bool`, `extract_annotations: bool`, `extract_attachments: bool`, `extract_images: bool`, `readability_threshold: f32`, `ocr_fallback_threshold: f32`, `include_invisible_text: bool`, and `streaming: bool`.

**Output.** The pipeline produces a structured JSON document (or NDJSON stream in streaming mode) with the following top-level shape:

```
{
  "metadata": { ... },          // document-level metadata and diagnostics
  "outline": [ ... ],           // bookmark tree
  "pages": [ ... ],             // per-page content
  "form_fields": [ ... ],       // AcroForm / XFA fields (if enabled)
  "annotations": [ ... ],       // page annotations (if enabled)
  "attachments": [ ... ],       // embedded files (if enabled)
  "warnings": [ ... ]           // extraction warnings across all stages
}
```

Each page entry carries `blocks` (containing `spans` with per-glyph Unicode and confidence), `extraction_method`, `classification_signals`, `reading_order_algorithm`, `readability_score`, and a page-level `warnings` array. The `--text` flag collapses all block content to plain text separated by `\n\n`. Exit codes follow quality: `0` = clean, `1` = warnings present, `2` = errors or low-confidence pages below threshold.

---

## Stage 1: File Opening and Structure Parsing

See: `pdf-specification.md`, `malformed-pdf-repair-and-recovery.md`, `pdfa-compliance-and-extraction.md`, `pdf-encryption-and-security.md`.

The pipeline opens the input via `mmap` and immediately checks the `%PDF-` header to confirm a valid PDF container, recording `pdf_version` in the output metadata. Parsing then works backward from the end of file to locate the `startxref` offset.

**Encryption detection.** The trailer dictionary is scanned for a `/Encrypt` entry. If present, the encryption handler is identified (standard password, certificate, or custom). `ExtractionOptions` may supply a password; if decryption fails or no password is provided, the pipeline returns an `EncryptionError` immediately. See `pdf-encryption-and-security.md` for the full handler decision tree.

**Cross-reference resolution.** The pipeline first attempts to parse the traditional xref table at the `startxref` offset. If that fails (common in repaired or linearized files), it falls back to xref streams (PDF 1.5+). If both fail, it falls back to a forward object scan — a full-file sequential pass that reconstructs the object map from `obj` / `endobj` markers. This scan is slower but handles severely malformed files. Recovered objects are flagged in `warnings`. The complete strategy is documented in `malformed-pdf-repair-and-recovery.md`.

**Document catalog and page tree.** With a valid object map, the pipeline resolves the `/Root` entry to the document catalog. The page tree (`/Pages` subtree) is traversed once to build a flat index of page dictionaries with their inherited attributes (media box, resources, rotation), enabling O(log n) lookup by page number for parallel access in Stage 4.

**PDF/A and tagging detection.** The catalog's `/Metadata` XMP stream is decoded and inspected for `pdfaid:conformance` and `pdfaid:part` to record the conformance level. The `/MarkInfo` dictionary's `/Marked` flag records whether the document is tagged. Both influence downstream path selection. See `pdfa-compliance-and-extraction.md`.

---

## Stage 2: Document-Level Metadata

See: `xmp-and-document-metadata.md`, `pdf-specification.md`.

Metadata extraction runs once before per-page work. The pipeline first attempts the XMP metadata stream from the catalog `/Metadata` key, parsing it as an RDF/XML document to extract standard Dublin Core and PDF namespace fields: title, author, creator, producer, creation date, modification date, keywords, and subject. If the XMP stream is absent or malformed, it falls back to the `/Info` dictionary, which carries the same fields in PDF string encoding.

When both sources exist, conflicts are resolved in favor of XMP for all fields where XMP provides a value — XMP is the authoritative source in PDF 1.4+ documents. The resolved values are written to `metadata` in the output.

The pipeline also extracts the document outline (bookmarks) by walking the `/Outlines` tree, recording title, destination, and nesting level for each entry. Page labels from the `/PageLabels` number tree are extracted and stored in `metadata.page_labels`, enabling human-readable page numbering in output.

---

## Stage 3: Per-Page Classification

See: `scanned-vs-vector-page-classification.md`, `pdfa-compliance-and-extraction.md`, `raster-ocr-pipeline.md`.

Before any expensive extraction work, each page is classified to select the optimal extraction path. Classification runs a sequence of fast pre-checks on the page content stream and resource dictionary:

1. **No text operators.** If the content stream contains no `Tj`, `TJ`, `'`, `"`, or `TD`/`Tm` operators, the page is initially flagged as `Scanned`.
2. **Full-page Tr=3 + image.** If all text operators set rendering mode 3 (invisible) and a full-page image XObject covers the media box, the page is classified as `BrokenVector` (a PDF/A OCR layer pattern where real text is hidden beneath a scan). See `invisible-and-hidden-text.md`.
3. **Image coverage fraction.** The pipeline computes the fraction of the page media box area covered by raster image XObjects. Coverage above a configurable threshold (default 0.85) is a strong scanned signal.
4. **Character validity rate.** Text operators are parsed and character codes are passed through a quick validity check (ToUnicode CMap lookup + AGL probe). A validity rate below a threshold (default 0.4) indicates a broken or symbolic font encoding, yielding `BrokenVector`.
5. **High-density valid text.** Pages with validity rate above 0.85 and no significant image coverage are classified as `Vector`.

The result is one of four `PageClass` values — `Vector`, `Scanned`, `Hybrid`, `BrokenVector` — each with an associated `confidence` score. Classification signals are recorded in the page output for diagnostics.

---

## Stage 4: Content Extraction (Per-Page, Parallelized)

See: `content-stream-concatenation.md`, `graphics-state-tracking.md`, `raster-ocr-pipeline.md`, `word-boundary-reconstruction.md`, `type3-font-extraction.md`, `optional-content-groups.md`.

Stage 4 is the core extraction stage and is parallelized across pages using `rayon`. Each page runs one of four sub-paths determined by its `PageClass`.

### 4a. Vector Path

Content streams are concatenated (handling `/Length` mismatches, flate-decoding, and multi-stream pages) per `content-stream-concatenation.md`. A PDF graphics state machine processes operators in order, maintaining a stack of `GraphicsState` structs that track the current transformation matrix (CTM), text matrix (Tm), text line matrix (Tlm), font, font size, character spacing, word spacing, horizontal scaling, and text rise. See `graphics-state-tracking.md`.

For each glyph, the text matrix is combined with the CTM to produce a device-space bounding box. Character codes are passed to the font pipeline (Stage 5) for Unicode resolution. Inter-glyph gaps are measured in glyph-space units normalized by the current font size; gaps exceeding the word-boundary threshold produce synthetic space characters. See `word-boundary-reconstruction.md`. Optional content group state (`/OC` entries) is tracked to suppress content from hidden layers. See `optional-content-groups.md`.

### 4b. OCR Path

The page is rendered to a 300 DPI raster using a PDF renderer. The raster undergoes preprocessing: deskew via Hough line detection, binarization via Sauvola local thresholding, and optional denoising. Tesseract is invoked with the language pack(s) specified in `ExtractionOptions.ocr_language`. HOCR output is parsed into glyph-level spans with bounding boxes and confidence scores. See `raster-ocr-pipeline.md` for the full preprocessing and Tesseract integration.

### 4c. Hybrid Path

Vector regions and image regions are identified by comparing text operator bounding boxes and image XObject placements. Regions where vector text is present use sub-path (a); regions covered by raster images with no overlapping vector text use sub-path (b). Spans from both sub-paths are merged by page coordinate order into a unified span list.

### 4d. Assisted OCR (BrokenVector)

Sub-path (a) is run first in position-hint mode: glyph bounding boxes are collected but Unicode values are discarded. These bounding boxes seed Tesseract's segmentation, improving word boundary detection. The OCR output then resolves the actual characters. Conflicts between position hints and OCR word boundaries are resolved in favor of OCR character shapes.

---

## Stage 5: Font Pipeline

See: `pdf-fonts-and-encoding.md`, `cmap-format-and-cid-encoding.md`, `glyph-recognition-and-unicode-recovery.md`, `type3-font-extraction.md`.

For every character code encountered in the Vector path, the font pipeline resolves a Unicode scalar value through a prioritized fallback chain:

1. **ToUnicode CMap.** If the font dictionary carries a `/ToUnicode` stream, the CMap is parsed and the character code is looked up. If the result is a non-sentinel value (not U+FFFD, not empty), it is used and `unicode_source` is set to `"to_unicode"`. See `cmap-format-and-cid-encoding.md`.
2. **Encoding vector + AGL.** If ToUnicode is absent or returns a sentinel, the font's encoding vector maps the character code to a glyph name. The Adobe Glyph List resolves the glyph name to a Unicode code point. `unicode_source` = `"agl"`. See `pdf-fonts-and-encoding.md`.
3. **Font fingerprint cache.** A precomputed database of known font program checksums maps directly to per-glyph Unicode tables. If the font program hash matches a database entry, the precomputed mapping is used. `unicode_source` = `"fingerprint"`.
4. **Glyph shape recognition.** The glyph is rendered to a small bitmap and hashed. If the shape hash matches an entry in the glyph recognition database, the Unicode value is assigned. `unicode_source` = `"shape_match"`. See `glyph-recognition-and-unicode-recovery.md`.
5. **Failure.** If all four steps fail, U+FFFD is emitted and `confidence` is set to `0.0`.

Type 3 fonts, which define glyph shapes as content stream fragments, are handled specially: each glyph's content stream is rasterized and passed to the shape recognition step. See `type3-font-extraction.md`.

Each glyph in the output carries `codepoint`, `unicode_source`, and `confidence`.

---

## Stage 6: Span and Block Assembly

See: `complex-layout-reading-order.md`, `tagged-pdf-structure-and-reading-order.md`, `document-classification-and-zone-labeling.md`, `watermark-and-background-separation.md`, `invisible-and-hidden-text.md`.

Raw glyphs are grouped into **spans** by continuity of font, font size, color (fill and stroke), and rendering mode. A new span begins whenever any of these attributes changes, or when a word boundary gap is detected.

**Reading order.** If the document is tagged (`/MarkInfo /Marked true`) or conforms to PDF/A-a, the StructTree is traversed to derive reading order. `reading_order_algorithm` is set to `"struct_tree"`. For untagged documents, the pipeline applies XY-cut decomposition (for rectilinear layouts) or Docstrum (for documents with irregular column boundaries). See `complex-layout-reading-order.md` and `tagged-pdf-structure-and-reading-order.md`.

**Zone labeling.** After reading order is established, spans are assigned to document zones: `body`, `heading`, `header`, `footer`, `footnote`, `caption`, or `sidebar`. Zone assignment uses margin heuristics (vertical position relative to media box), font size clustering (headings are statistical outliers in the size distribution), and cross-page consistency (running headers/footers appear at similar positions across pages). See `document-classification-and-zone-labeling.md`.

**Watermark and invisible text filtering.** Spans in rendering mode 3 (invisible) are suppressed unless `ExtractionOptions.include_invisible_text` is true. Spans classified as watermarks (low opacity, Z-order beneath body text, or matching common watermark patterns) are filtered per policy. See `watermark-and-background-separation.md` and `invisible-and-hidden-text.md`.

Spans are assembled into **blocks** representing paragraphs or other logical units, and blocks are ordered within each page according to the reading order algorithm's output.

---

## Stage 7: Text Normalization and Quality

See: `post-extraction-normalization.md`, `post-ocr-text-correction.md`, `text-readability-validation.md`, `semantic-text-reconstruction.md`, `language-detection-and-script-handling.md`.

Normalization runs as an ordered pipeline applied to each span's text:

1. **Ligature expansion.** Standard ligatures (fi, fl, ffi, ffl, ſt, st) are expanded to their component characters.
2. **Unicode normalization.** All text is normalized to NFC.
3. **Whitespace collapse.** Runs of whitespace within a span are collapsed to a single space; leading and trailing whitespace is stripped.
4. **Hyphen joining.** Lines ending in a hyphen are joined to the next line's first word, with the hyphen removed, if the joined form appears in a language dictionary.
5. **Paragraph reconstruction.** Short lines that do not end with sentence-terminal punctuation are joined to the following line when their right edge falls significantly short of the text block width. See `semantic-text-reconstruction.md`.
6. **Header/footer deduplication.** Spans in the `header` and `footer` zones that appear with identical or near-identical text across three or more consecutive pages are flagged as `deduplicated` and excluded from the main text flow. They remain in the output under their zone label for reference.

**Readability scoring.** Each span is scored on three signals: Shannon entropy of the character distribution, dictionary hit rate against a word list for the detected language, and character validity rate (fraction of non-U+FFFD codepoints). The composite `readability_score` per block (0.0–1.0) is written to the output. Blocks scoring below `ExtractionOptions.ocr_fallback_threshold` trigger an OCR fallback for that region on vector pages, re-running the block through sub-path (b) of Stage 4. See `text-readability-validation.md`.

**Post-OCR correction.** For spans produced by the OCR path, a correction pass applies: confusable character substitution (0↔O, 1↔l, rn↔m), regex-based pattern correction (dates, identifiers), and bigram/trigram context correction using a language model. See `post-ocr-text-correction.md`.

Language detection runs on the assembled block text to confirm or override the per-page language hint. The detected language is used to select the appropriate dictionary and Tesseract language pack for any OCR fallback runs. See `language-detection-and-script-handling.md`.

---

## Stage 8: Supplementary Content

See: `form-fields-and-annotations.md`, `embedded-files-and-portfolios.md`, `image-and-figure-extraction.md`.

Supplementary extraction runs after all pages complete, guarded by the relevant `ExtractionOptions` flags.

**Forms.** If `extract_forms` is true, the AcroForm dictionary is located in the catalog. Each field in the `/Fields` array is walked recursively. Field type (`Tx`, `Btn`, `Ch`, `Sig`), name, value, and appearance state are extracted. If an `/XFA` stream is present, it is parsed as XFA XML and field values are extracted from the XFA data model. See `form-fields-and-annotations.md`.

**Annotations.** If `extract_annotations` is true, each page's `/Annots` array is iterated. For text and link annotations, `Contents` and `RC` (rich content) fields are extracted. Annotation type, rectangle, and flags are recorded. Redaction annotations (`/Redact`) are noted in warnings.

**Attachments.** If `extract_attachments` is true, the `/EmbeddedFiles` name tree in the catalog is walked. Each `Filespec` dictionary yields a filename, description, MIME type, creation date, and the raw file bytes (or a size-limited excerpt if the attachment is large). See `embedded-files-and-portfolios.md`.

**Images.** If `extract_images` is true, image XObjects referenced from each page's resource dictionary are collected. Metadata (width, height, color space, bits per component, filter chain) is always included. Pixel data is decoded and included as base64 only if `ExtractionOptions.include_image_data` is true. See `image-and-figure-extraction.md`.

---

## Stage 9: Output Serialization

See: `performance-and-streaming-architecture.md`, `chunking-for-llm-consumption.md`.

The final stage assembles all collected data and serializes it.

**Buffered JSON mode** (default). The complete document tree is serialized to a single JSON object. Field ordering follows the schema defined in the Pipeline Inputs and Outputs section above. `serde_json` with `BufWriter` is used; the output is written to stdout or a specified file path.

**Streaming NDJSON mode** (`ExtractionOptions.streaming = true`). Metadata is emitted as the first JSON line. Each page is serialized and emitted as a JSON line immediately after it completes extraction, allowing consumers to begin processing before the full document is done. This mode is documented in `performance-and-streaming-architecture.md` and is designed to support the LLM consumption patterns described in `chunking-for-llm-consumption.md`.

Each page object in both modes carries:

- `page_number` (1-based)
- `extraction_method`: one of `"vector"`, `"ocr"`, `"hybrid"`, `"assisted_ocr"`
- `classification_signals`: the raw signals from Stage 3 (image coverage fraction, character validity rate, operator counts)
- `reading_order_algorithm`: `"struct_tree"`, `"xy_cut"`, or `"docstrum"`
- `readability_score`: composite 0.0–1.0 for the page
- `blocks`: ordered array of text blocks with spans
- `warnings`: page-level warning array

**Exit code semantics.** After all pages are processed, the pipeline computes the worst-case quality across pages. If all pages have readability score above the clean threshold, exit code `0` is returned. If any page emits warnings (OCR fallback triggered, low-confidence spans, unsupported features), exit code `1` is returned. If any page fails extraction entirely or contains errors, exit code `2` is returned. This allows shell pipelines and CI systems to gate on extraction quality without parsing the output JSON.

---

## Summary: Stage Ordering and Data Flow

```
Input (file path / bytes)
  │
  ▼
Stage 1: File opening, xref, decryption, page tree index
  │
  ▼
Stage 2: Document metadata (XMP, /Info, outline, page labels)
  │
  ▼
Stage 3: Per-page classification → PageClass × confidence
  │
  ▼
Stage 4: Content extraction (rayon parallelism across pages)
  ├─ Vector  → graphics state machine → raw glyphs
  ├─ OCR     → raster render → Tesseract → raw spans
  ├─ Hybrid  → Vector regions + OCR regions → merged spans
  └─ BrokenVector → position hints + OCR → spans
       │
       ▼ (from Vector path)
Stage 5: Font pipeline → Unicode + confidence per glyph
  │
  ▼
Stage 6: Span + block assembly → reading order → zone labels
  │
  ▼
Stage 7: Normalization → readability scoring → OCR fallback → correction
  │
  ▼
Stage 8: Forms, annotations, attachments, images (conditional)
  │
  ▼
Stage 9: JSON / NDJSON serialization → exit code
```

Each stage boundary is a well-defined data contract. Stages 1–2 produce document-scoped structures shared across all pages. Stage 3 produces per-page `PageClass` values that gate Stage 4 sub-path selection. Stages 4–7 are the per-page pipeline and are the primary targets for parallelism and optimization. Stages 8–9 are sequential post-processing passes over the fully assembled extraction result.
