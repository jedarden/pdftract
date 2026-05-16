# Embedded Files and PDF Portfolios

**Project:** pdftract — Rust PDF text extraction library  
**Scope:** Handling PDFs that carry embedded files (attachments) and PDF Portfolio/Collection containers

---

## 1. Embedded File Streams

An embedded file is a PDF stream object with `/Subtype /EmbeddedFile`. It carries the raw bytes of the attached file in its compressed stream body, exactly like any other stream object.

**Stream dictionary keys of interest:**

| Key | Type | Description |
|-----|------|-------------|
| `/Subtype` | name | MIME type string, e.g. `application/xml`, `text/plain`. Optional but common. |
| `/Params` | dictionary | File metadata: `/Size` (integer, uncompressed byte count), `/CreationDate` and `/ModDate` (PDF date strings), `/CheckSum` (MD5 digest as a 16-byte string). |
| `/Filter` | name or array | Decompression chain applied to the stream. Typically `/FlateDecode`; may be a multi-stage chain like `[/ASCII85Decode /FlateDecode]`. |
| `/Length` | integer | Compressed byte count in the file. Always present. |

To read the raw bytes of an embedded file: locate the stream object, apply the `/Filter` chain in sequence (outermost filter first in array order), and the result is the uncompressed file payload. The `/Params` `/Size` entry should match the decompressed length — use it as a sanity check.

---

## 2. File Specification Dictionaries

A **Filespec dictionary** wraps an embedded file stream and carries filename and relationship metadata. It is the object that the rest of the document refers to, not the raw stream directly.

```
<<
  /Type /Filespec
  /F    (invoice.xml)          % PDFDocEncoding filename
  /UF   <FEFF0069006E...>      % UTF-16BE Unicode filename (preferred)
  /Desc (Factur-X invoice XML) % Human-readable description
  /AFRelationship /Data        % Semantic relationship to the document (PDF 2.0)
  /EF   <<
    /F  12 0 R                 % EmbeddedFile stream for /F name
    /UF 12 0 R                 % May point to the same or different stream
  >>
>>
```

**`/EF` sub-dictionary** maps the platform filename keys (`/F`, `/UF`, `/DOS`, `/Mac`, `/Unix`) to indirect references to EmbeddedFile stream objects. In modern PDFs, `/F` and `/UF` point to the same stream; the platform-specific keys are legacy.

**`/AFRelationship`** (PDF 2.0, ISO 32000-2 §7.11.3) declares how the embedded file relates to the containing PDF:

| Value | Meaning |
|-------|---------|
| `Source` | The embedded file is the source for generating this PDF. |
| `Data` | Structured data that this PDF was generated from (e.g. XML invoice). |
| `Alternative` | Alternative representation of document content. |
| `Supplement` | Supplemental information. |
| `EncryptedPayload` | Encrypted payload, used in PDF encryption workflows. |
| `FormData` | Data submitted from a form. |
| `Schema` | An XSD or schema file describing another attachment. |
| `Unspecified` | No declared relationship. |

---

## 3. Document-Level Attachments — The `EmbeddedFiles` Name Tree

Document-level attachments are registered in the document catalog under `/Names` → `EmbeddedFiles`. This is a PDF **name tree** — a B-tree structure mapping UTF-8 (or PDFDocEncoding) string keys to Filespec dictionary references.

**Name tree structure:**

- **Leaf node:** `<< /Names [ (key1) ref1 (key2) ref2 ... ] >>`  
- **Intermediate node:** `<< /Limits [ (first-key) (last-key) ] /Kids [ ref ... ] >>`

To iterate all entries, walk the tree depth-first: if a node has `/Kids`, recurse into each child; if a node has `/Names`, extract the key/value pairs in sequence. Keys are the filename strings used for display; values are indirect references to Filespec dictionaries.

Navigation from catalog:

```
Catalog → /Names → /EmbeddedFiles → (name tree root node)
```

A PDF can have document-level attachments without being a Portfolio. Check for the `/Collection` key (§6 below) to distinguish the two cases.

---

## 4. Annotation-Based Attachments

A `FileAttachment` annotation attaches a file to a specific page location. It appears in the page's `/Annots` array with `/Subtype /FileAttachment`.

**Relevant annotation keys:**

| Key | Description |
|-----|-------------|
| `/FS` | Indirect reference to a Filespec dictionary. |
| `/Name` | Icon style hint: `PushPin`, `Graph`, `Paperclip`, `Tag`. |
| `/Contents` | Tooltip/description string shown on hover. |

The Filespec under `/FS` is identical in structure to document-level Filespecs. To extract the file bytes, follow `/FS` → Filespec → `/EF` → EmbeddedFile stream → decompress.

When iterating pages for text extraction, collect all `/Annots` entries and filter for `/Subtype /FileAttachment`. Record the page number as the `page` field in the attachment output (see §9).

---

## 5. Associated Files (PDF 2.0)

PDF 2.0 (ISO 32000-2) introduces the `/AF` (associated files) array. Unlike `EmbeddedFiles`, which is a document-wide name tree, `/AF` arrays appear directly on specific objects and express a tighter structural association.

**Where `/AF` can appear:**

- Document catalog (document-wide association)
- Page dictionaries (page-specific file)
- Content stream dictionaries
- XObject dictionaries
- Structure element dictionaries (tagged PDF)

Each entry in an `/AF` array is an indirect reference to a Filespec dictionary carrying an `AFRelationship`.

**Key use cases:**

- **PDF/A-3** (ISO 19005-3): requires that embedded source data be declared via `/AF` with `AFRelationship /Data` or `/Source`. PDF/A-3 is the conformance level that permits arbitrary embedded files; lower levels forbid them.
- **ZUGFeRD / Factur-X**: a PDF/A-3 invoice with an embedded XML file referenced via `/AF` on the catalog and also present in `EmbeddedFiles`. Both access paths should be checked.
- **PDF/UA-2**: MathML representations of mathematical content can be attached via `/AF` on structure elements with `AFRelationship /Alternative`.

When processing a document, collect `/AF` arrays from all of these object types. Deduplicate by object number — the same Filespec may be referenced from multiple `/AF` arrays.

---

## 6. PDF Portfolio / Collection

A PDF Portfolio (PDF 1.7+, ISO 32000-1 §12.3.5) is a PDF whose catalog contains a `/Collection` dictionary. The Portfolio container acts as a navigator shell; the actual attached documents are Filespecs in the `EmbeddedFiles` name tree.

**Detection:** `Catalog → /Collection` present → this is a Portfolio.

**`/Collection` dictionary keys:**

| Key | Description |
|-----|-------------|
| `/Schema` | Dictionary of column field definitions (name, type, visibility order) for the Portfolio UI. Each entry maps a field name to `<< /E (label) /T /S|D|N|F /O integer /V bool >>`. |
| `/D` | The default document to display — a string key into the `EmbeddedFiles` name tree, or a string naming the cover sheet. |
| `/View` | Preferred initial view: `D` (details), `T` (tile), `H` (hidden). |
| `/Navigator` | Indirect reference to a Filespec pointing to a Navigator PDF (a separate PDF providing the portfolio UI shell). |
| `/Sort` | Specifies the default sort column and order. |

**Portfolio items** are regular Filespecs in the `EmbeddedFiles` name tree. The `/Collection` dictionary's `/Schema` assigns metadata field names; individual Filespecs carry those field values in their `/CI` (collection item) dictionary.

**Navigator PDF:** Some Portfolios embed a separate PDF as the visual container/shell. This Navigator PDF is itself an embedded file referenced from `/Collection /Navigator`. It should be treated as infrastructure rather than a content document — do not recurse into it for text extraction unless explicitly requested.

**Distinguishing Portfolio from a PDF with attachments:** if `/Collection` is present in the catalog, treat all `EmbeddedFiles` entries as Portfolio items (each is a first-class document). Without `/Collection`, `EmbeddedFiles` entries are attachments supplementary to the parent document's content.

---

## 7. Recursive Extraction

When an embedded file's MIME type is `application/pdf` (declared in `/Subtype`) or detected as PDF by magic bytes (`%PDF-`), the file can be extracted and parsed as a standalone PDF.

**Depth limiting:** maintain a recursion depth counter; enforce a configurable maximum (default: 3). Beyond the limit, record the file in the attachment list with a `recursion_limit_reached` flag but do not parse it. This prevents pathological inputs with circular or deeply nested PDFs from exhausting memory.

**Binary file handling:** identify the MIME type from `/Subtype` and confirm with magic byte inspection. Files whose MIME type is not `application/pdf` and whose content does not begin with `%PDF-` are binary attachments — extract bytes and metadata only, do not attempt PDF parsing.

**Object streams and cross-reference:** each recursively parsed PDF is an independent document with its own cross-reference table and object numbering. Do not share object caches across recursion levels.

---

## 8. ZUGFeRD / Factur-X

ZUGFeRD (DE) and Factur-X (FR) are electronic invoicing standards that encode a human-readable PDF invoice (conforming to PDF/A-3) with an embedded XML invoice payload conforming to EN 16931 / UN/CEFACT CII Cross Industry Invoice.

**Detection pattern:**

1. Catalog has `/AF` array (PDF/A-3 conformance).
2. `EmbeddedFiles` name tree contains an entry with filename `factur-x.xml` (Factur-X) or `zugferd-invoice.xml` / `ZUGFeRD-invoice.xml` (ZUGFeRD).
3. The Filespec's `AFRelationship` is `/Data`.
4. MIME type is `application/xml` or `text/xml`.

**Extraction value:** for financial document processing pipelines, the embedded XML carries structured data (line items, amounts, VAT, party identifiers) that is far more reliable than text extracted from the PDF rendering. pdftract should expose this XML in the `attachments` array output alongside the extracted text, enabling callers to consume both without reparsing the PDF.

A secondary indicator is the PDF/A conformance metadata in the XMP stream (`pdfaid:part = 3`, `pdfaid:conformance = B` or `U`).

---

## 9. Extraction Policy and Output

**JSON output schema for attachments:**

```json
{
  "attachments": [
    {
      "filename": "factur-x.xml",
      "mime_type": "application/xml",
      "size_bytes": 14230,
      "description": "Factur-X structured invoice",
      "af_relationship": "Data",
      "page": null,
      "data_b64": "<base64-encoded bytes, if extract_attachments=true>"
    }
  ]
}
```

| Field | Notes |
|-------|-------|
| `filename` | From `/UF` if present, otherwise `/F`. |
| `mime_type` | From EmbeddedFile `/Subtype`; `null` if absent. |
| `size_bytes` | From `Params /Size`; `null` if absent. |
| `description` | From Filespec `/Desc`; `null` if absent. |
| `af_relationship` | String value of `AFRelationship`; `null` if not PDF 2.0. |
| `page` | 0-based page index for annotation-sourced attachments; `null` for document-level. |
| `data_b64` | Present only when `extract_attachments: true` in caller options. |

**Caller options:**

```rust
pub struct ExtractionOptions {
    /// Include attachment byte payloads (base64) in output.
    pub extract_attachments: bool,
    /// Recursively extract text from PDF attachments.
    pub recursive: bool,
    /// Maximum recursion depth for recursive PDF extraction.
    pub max_recursion_depth: u8,
}
```

Regardless of `extract_attachments`, always populate the metadata fields (`filename`, `mime_type`, `size_bytes`, etc.) so callers can make informed decisions about which files to retrieve.

---

## 10. Security Considerations

Embedded files are opaque byte payloads from potentially untrusted sources. pdftract must extract bytes without executing or interpreting them.

**Execution surface:** never pass embedded file bytes to a shell, interpreter, or OS loader. The extraction pipeline is: locate stream → apply `/Filter` decompression chain → write raw bytes. No further processing.

**MIME type verification:** the declared `/Subtype` in the EmbeddedFile stream is author-controlled and untrustworthy. Perform independent MIME detection from the first 512 bytes of the decompressed payload using magic byte patterns. If the detected type differs materially from the declared type, record both and emit a warning.

**High-risk MIME types to flag:**

- `application/javascript`, `text/javascript`
- `application/x-executable`, `application/x-elf`, `application/x-mach-binary`, `application/x-msdownload`
- `application/x-sh`, `application/x-bat`, `application/x-powershell`
- `application/x-java-archive` (JAR files can contain auto-run manifests)

**Output field:** add `"security_flags": ["mime_mismatch", "high_risk_type"]` to the attachment JSON object when either condition is detected.

**Decompression safety:** the `/Filter` chain may cause decompression bombs. Enforce a maximum decompressed size (configurable, default 256 MB per file). Abort decompression and record a `decompression_limit_exceeded` flag if the limit is reached. Streaming decompression (rather than full buffer allocation) is preferred — decompress incrementally and track bytes written.

**Name traversal:** Filespec `/F` and `/UF` filenames may contain path separators or `..` sequences. When writing extracted files to disk (if pdftract exposes a save-to-disk API), sanitize filenames by stripping directory components and rejecting names that resolve outside the target directory.
