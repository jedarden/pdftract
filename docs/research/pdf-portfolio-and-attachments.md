# PDF Portfolios, Collections, and Embedded File Extraction

**Project:** pdftract — Rust PDF text extraction library  
**Scope:** Portfolio detection, component enumeration, recursive extraction, ZUGFeRD/Factur-X, PDF/A-3 constraints, associated files, and output schema

---

## 1. Portfolio Detection and the `/Collection` Dictionary

A PDF Portfolio is a container document whose catalog carries a `/Collection` dictionary. This key is the definitive distinguisher between a plain PDF with attachments and a Portfolio: the presence of `Catalog → /Collection` signals that the embedded files are first-class component documents organized into a navigable collection, not supplementary attachments to a single document.

The `/Collection` dictionary contains several keys that describe the Portfolio's structure and presentation. `/Schema` defines the metadata columns displayed in the portfolio navigator UI. `/D` names the default component to open on launch — either a string key into the `EmbeddedFiles` name tree or the string `"__COVER_SHEET__"` indicating the cover page. `/View` specifies the preferred initial layout (`D` for details list, `T` for tile, `H` for hidden). `/Navigator` holds an indirect reference to a Filespec wrapping a separate Navigator PDF that provides the shell UI. `/Sort` carries default sort column and order.

The cover page — also called the navigator page — is a fully rendered PDF page that viewers display when no component is active. It is rendered from the Portfolio PDF's own page tree, not from any embedded file. For text extraction purposes, this page must be processed identically to any other PDF page: parse the content streams, resolve fonts, and extract glyph sequences. Its text contributes to the top-level document output, distinct from the extracted text of component files.

A PDF that lacks `/Collection` but contains an `EmbeddedFiles` name tree is a regular PDF with attachments. The extraction logic is similar, but the semantic framing differs: without `/Collection`, embedded files are supplementary to the parent document; within a Portfolio, they are the primary content.

---

## 2. Component File Enumeration via the `EmbeddedFiles` Name Tree

Regardless of whether `/Collection` is present, all document-level attachments are registered in the `EmbeddedFiles` name tree, reached via `Catalog → /Names → /EmbeddedFiles`. This is a PDF name tree — a balanced B-tree whose leaf nodes contain `(key, value)` pairs mapping string keys to indirect references to Filespec dictionaries.

Walking the tree requires handling two node types. An intermediate node carries `/Limits` (a two-element array with the first and last key in the subtree) and `/Kids` (an array of indirect references to child nodes). A leaf node carries `/Names` (a flat array alternating string keys and indirect references). The traversal is depth-first; collect all key/value pairs from every leaf.

Each value resolved from the tree is a Filespec dictionary. The fields relevant to enumeration are:

- `/F` — filename in PDFDocEncoding (legacy; always present)
- `/UF` — Unicode filename in UTF-16BE (preferred when present; use over `/F`)
- `/Desc` — human-readable description string
- `/Type` (value `/Filespec`) — confirms the object type
- `/CI` — collection item dictionary carrying per-column metadata values for Portfolio display
- `/EF` — the embedded file stream sub-dictionary

The `/CI` dictionary maps column field names (as defined in `/Collection/Schema`) to their values for this component. For example, a Portfolio with a "Size" column and a "Date Modified" column will have corresponding entries in each component's `/CI`. These values are structured metadata that pdftract should capture as part of the attachment record, since they carry author-supplied organizational context.

MIME type is not stored in the Filespec but in the EmbeddedFile stream dictionary itself, described in §3 below. Date fields — creation and modification — appear in the EmbeddedFile stream's `/Params` sub-dictionary.

---

## 3. Component File Access via the `/EF` Stream Dictionary

The `/EF` (embedded file) key within a Filespec maps platform filename variants to indirect references to EmbeddedFile stream objects. Modern PDFs use `/F` and `/UF` pointing to the same stream object; the legacy platform-specific keys (`/DOS`, `/Mac`, `/Unix`) should be handled for compatibility but are rarely present in contemporary portfolios.

The EmbeddedFile stream dictionary carries:

- `/Subtype` — a MIME type string (e.g., `application/pdf`, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`, `application/xml`, `text/csv`). This field is optional but common; absent values require MIME detection from content.
- `/Filter` — the decompression filter or array of filters applied to the stream body. `FlateDecode` is nearly universal; multi-stage chains like `[/ASCII85Decode /FlateDecode]` appear in older files.
- `/Length` — the compressed byte count within the file.
- `/Params` — a sub-dictionary carrying `/Size` (decompressed byte count, usable as a sanity check), `/CreationDate`, `/ModDate` (PDF date strings in the format `D:YYYYMMDDHHmmSSOHH'mm'`), and `/CheckSum` (16-byte MD5 digest of the uncompressed content).

To extract raw bytes: locate the stream object, apply the `/Filter` chain in sequence (each filter in array order operates on the output of the preceding one), and the resulting byte sequence is the uncompressed file payload. The decompressed length should equal `/Params/Size`; a mismatch indicates corruption or a miscalculated filter chain.

File types typically embedded in Portfolios include PDF documents (nested portfolios or standalone reports), Office Open XML formats (Word `.docx`, Excel `.xlsx`, PowerPoint `.pptx`), legacy Office formats (`.doc`, `.xls`), XML data files, CSV spreadsheets, and plain text. All non-PDF types should be surfaced in the output with their bytes available for caller retrieval; PDF types trigger recursive processing (§5).

---

## 4. Portfolio Schema: Extractable Structured Metadata

The `/Collection/Schema` dictionary defines the columns that the portfolio viewer displays. Each entry maps a field name (a PDF name object) to a field descriptor dictionary with these keys:

- `/E` — the display label string (e.g., `"File Name"`, `"Description"`, `"Date Created"`)
- `/T` — the field type: `/S` (string), `/D` (date), `/N` (number), `/F` (filename — a special case of string)
- `/O` — display order (integer; lower values appear first in the UI column list)
- `/V` — visibility flag (boolean; `false` means the field exists but is hidden in the default view)

This schema is machine-readable structured metadata that pdftract can surface as part of the portfolio-level output. A caller processing a portfolio of financial reports can use the schema to understand what metadata columns exist, then read each component's `/CI` dictionary values against those column definitions to construct a structured table of all component metadata without opening any embedded files.

The schema extraction path is: `Catalog → /Collection → /Schema → iterate each key/value pair → record field name, label, type, order, and visibility`.

---

## 5. Recursive Portfolio Extraction: Depth Limiting and Cycle Detection

When an embedded component's MIME type is `application/pdf` or its first four bytes are `%PDF`, the component is itself a PDF and must be parsed as a standalone document. This recursion is essential for portfolios that bundle other portfolios as components, a pattern found in document packages where a top-level portfolio indexes sub-portfolios grouped by topic or date.

pdftract must enforce a configurable recursion depth limit, with a default of three levels. At the limit, the component is recorded in the attachment list with `extraction_status: "skipped"` and a `recursion_limit_reached` flag, but its bytes are not parsed. This prevents stack exhaustion and memory overconsumption from adversarially nested PDFs.

Cycle detection requires tracking the MD5 or SHA-256 digest of each PDF payload encountered during a single extraction job. Before recursing into a component, compute the digest of its decompressed bytes and check against the seen-digests set for the current traversal. If the digest is already present, record the component as `extraction_status: "skipped"` with a `cycle_detected` flag. The digest set must be passed down through recursive calls, not maintained as global state, so that independent top-level extraction jobs do not share state.

Each recursively parsed PDF is a fully independent document: it has its own cross-reference table, object numbering, and name trees. Do not share any object cache or font cache across recursion levels.

---

## 6. ZUGFeRD and Factur-X Invoice PDFs

ZUGFeRD (Germany) and Factur-X (France/EU) are electronic invoicing profiles built on PDF/A-3 (ISO 19005-3). The document is simultaneously a human-readable PDF invoice and a machine-readable structured data package. The XML payload embedded within conforms to EN 16931 (the European e-invoicing standard) using the UN/CEFACT Cross Industry Invoice (CII) data model.

Detection requires checking multiple indicators in combination:

1. `Catalog → /AF` array is present (mandatory in PDF/A-3).
2. The `EmbeddedFiles` name tree contains a Filespec whose `/UF` or `/F` value matches `factur-x.xml` (Factur-X) or `zugferd-invoice.xml` / `ZUGFeRD-invoice.xml` (ZUGFeRD 1.x). ZUGFeRD 2.x aligns with Factur-X and uses `factur-x.xml`.
3. The matching Filespec has `AFRelationship /Data`.
4. The EmbeddedFile stream's `/Subtype` is `application/xml` or `text/xml`.
5. The XMP metadata stream on the catalog contains `pdfaid:part = 3` confirming PDF/A-3 conformance.

For these documents, pdftract has two distinct extraction targets: the visual text of the PDF pages (the human-readable invoice rendition) and the raw XML bytes of the embedded file (the machine-readable invoice data). Both targets should appear in the output. The XML bytes should be exposed in the `attachments` array entry for the embedded file. Callers processing invoices in bulk will often prefer the XML path, but the page text remains valuable for validation and fallback.

---

## 7. PDF/A-3 Attachment Constraints and `AFRelationship` Prioritization

PDF/A-3 (ISO 19005-3) is the only PDF/A conformance level that permits embedding arbitrary file formats. Lower levels (PDF/A-1, PDF/A-2) prohibit embedded files entirely. When a document declares PDF/A-3 conformance in its XMP metadata (`pdfaid:part = 3`), all attachments must carry an `AFRelationship` value — `Unspecified` is the fallback for attachments without a declared semantic role.

The `AFRelationship` value directly informs extraction priority:

- `Data` and `Source` indicate the attachment is structured data either generated from or used to generate this PDF. These are the highest-priority extraction targets because they carry non-redundant information unavailable from the page text.
- `Alternative` indicates a different representation of the document content — useful when the PDF page text is degraded or encoded with poor font mapping.
- `Supplement` indicates ancillary information that augments the document.
- `Unspecified` is the lowest priority; the attachment's value must be inferred from MIME type and filename.

pdftract should sort the `attachments` array by this priority order when presenting results, and should tag each attachment record with its `af_relationship` string for caller-side filtering.

---

## 8. ISO 32000-2 Associated Files on Pages, Fields, and XObjects

PDF 2.0 (ISO 32000-2) generalizes the association between files and document objects via the `/AF` (associated files) array. This array can appear on the document catalog, on individual page dictionaries, on form field objects, on XObject dictionaries, and on structure elements in tagged PDFs.

Each entry in an `/AF` array is an indirect reference to a Filespec dictionary. When `/AF` appears on a page, the associated file relates specifically to that page's content — for example, a transcript of audio described on that page, or a data table whose values are visualized in a chart on that page. When `/AF` appears on an XObject, the association is with a specific figure or image element. When `/AF` appears on a form field, it carries data submitted with or relevant to that field.

During page iteration for text extraction, pdftract must collect `/AF` entries from each page dictionary and merge them with any document-level `/AF` entries. During XObject resolution, if the XObject dictionary carries `/AF`, those Filespecs should be recorded with the containing page number and XObject name as context. Deduplication by PDF object number is required since the same Filespec can be referenced from multiple `/AF` arrays across the document.

The practical impact on text extraction: a page with an associated file carrying `AFRelationship /Alternative` may contain image-only content where the associated file is the text alternative. Surfacing this relationship allows callers to fall back to the associated text when OCR is unavailable or unreliable.

---

## 9. Cover Page Text Extraction

The cover or navigator page of a PDF Portfolio is a regular PDF page rendered by the containing PDF's page tree. It is not an embedded file. Viewers display it as the initial landing page of the portfolio — it typically contains the portfolio title, a description, and branding elements.

From pdftract's perspective, the cover page is structurally identical to any other PDF page. Its content streams must be parsed, its fonts resolved, and glyph sequences mapped to Unicode following the standard extraction pipeline. The resulting text contributes to the top-level document's page output, tagged with its page index.

The only Portfolio-specific consideration is that when `/Collection/D` equals `"__COVER_SHEET__"` or a similar sentinel, the intent is that the cover page is the default view — this is a presentation hint only and does not affect extraction. Extract all pages in the parent PDF's page tree regardless of `/Collection/D`.

---

## 10. Output Schema for Portfolios

The pdftract JSON output for a portfolio document must surface both the parent document's text and the structured attachment list. For embedded PDFs processed recursively, the nested extraction result appears inline.

```json
{
  "pages": [ { "page": 0, "text": "Portfolio cover page text..." } ],
  "portfolio": true,
  "attachments": [
    {
      "filename": "Q1-Report.pdf",
      "mime_type": "application/pdf",
      "size_bytes": 204800,
      "description": "Q1 Financial Report",
      "af_relationship": "Data",
      "extraction_status": "extracted",
      "nested_result": {
        "pages": [ { "page": 0, "text": "..." } ],
        "portfolio": false,
        "attachments": []
      }
    },
    {
      "filename": "factur-x.xml",
      "mime_type": "application/xml",
      "size_bytes": 14230,
      "description": "Factur-X structured invoice",
      "af_relationship": "Data",
      "extraction_status": "extracted",
      "nested_result": null
    },
    {
      "filename": "archive.pdf",
      "mime_type": "application/pdf",
      "size_bytes": 10485760,
      "description": null,
      "af_relationship": "Unspecified",
      "extraction_status": "skipped",
      "skip_reason": "recursion_limit_reached",
      "nested_result": null
    }
  ]
}
```

Field definitions:

| Field | Type | Notes |
|-------|------|-------|
| `portfolio` | boolean | `true` if `Catalog → /Collection` was present. |
| `filename` | string | From `/UF`; falls back to `/F`. |
| `mime_type` | string or null | From EmbeddedFile `/Subtype`; null if absent. |
| `size_bytes` | integer or null | From `EmbeddedFile/Params/Size`; null if absent. |
| `description` | string or null | From Filespec `/Desc`. |
| `af_relationship` | string or null | String value of `AFRelationship`; null if not declared. |
| `extraction_status` | string | `"extracted"`, `"skipped"`, or `"error"`. |
| `skip_reason` | string or null | Present when `extraction_status` is `"skipped"`; values: `"recursion_limit_reached"`, `"cycle_detected"`, `"size_limit_exceeded"`. |
| `nested_result` | object or null | Full extraction result for embedded PDFs when `recursive: true`; null for non-PDF attachments or skipped entries. |

The `portfolio` boolean at the top level allows callers to distinguish a portfolio response from a regular document response without inspecting the `attachments` array. When `portfolio` is `true`, callers should treat the top-level `pages` text as the cover/navigator content and the `attachments` entries as the primary documents.
