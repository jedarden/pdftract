# XMP and Document Metadata in PDF

**Project:** pdftract — Rust PDF text extraction library  
**Scope:** Structured metadata extraction from PDF files, covering the legacy `/Info` dictionary and XMP metadata streams

---

## 1. The /Info Dictionary

The document information dictionary is an optional indirect object referenced by the `Info` key in the PDF file's cross-reference trailer (`trailer << /Root ... /Info N G R >>`). It predates XMP and was the sole metadata mechanism through PDF 1.6.

### Standard Keys

| Key | Type | Description |
|-----|------|-------------|
| `Title` | text string | Human-readable document title |
| `Author` | text string | Name of the person who created the document |
| `Subject` | text string | Subject matter summary |
| `Keywords` | text string | Space- or comma-delimited keyword list |
| `Creator` | text string | The authoring application (e.g., "Microsoft Word 2019") |
| `Producer` | text string | The PDF-writing library that generated the file (e.g., "Acrobat Distiller 23.0") |
| `CreationDate` | date string | When the document was first created |
| `ModDate` | date string | When the document was last modified |
| `Trapped` | name | `/True`, `/False`, or `/Unknown` — trapping status for print production |

### Date Format

PDF date strings use the format `D:YYYYMMDDHHmmSSOHH'mm'` where:

- `D:` is a required literal prefix
- `YYYY` through `SS` are the year, month, day, hour, minute, and second (all numeric, left-zero-padded)
- `O` is the timezone offset sign: `+`, `-`, or `Z` (for UTC)
- `HH'mm'` are the timezone hour and minute offsets, separated by a literal apostrophe, with a trailing apostrophe

All components after `YYYY` are optional but must be omitted from the right. For example, `D:20230415143022+05'30'` is valid; `D:202304` is also valid. When parsing, treat missing components as their minimum values (month 01, day 01, time 00:00:00, timezone UTC).

### String Encoding

`/Info` text string values follow two encoding paths depending on a leading BOM:

- **PDFDocEncoding**: The default single-byte encoding when no BOM is present. It is a superset of Latin-1 with custom assignments in the `0x18`–`0x1F` and `0x80`–`0x9F` ranges. Rust extraction must implement the PDFDocEncoding-to-Unicode mapping table (PDF spec Annex D).
- **UTF-16BE with BOM**: If the first two bytes of the string object are `0xFE 0xFF`, the entire string (including BOM) is UTF-16BE. Rust's `std::str::from_utf8` cannot handle this; use `encoding_rs` or a manual UTF-16BE decoder.

### Deprecation in PDF 2.0

PDF 2.0 (ISO 32000-2:2020) formally deprecates the `/Info` dictionary. Processors conforming to PDF 2.0 must treat XMP as the authoritative source. `/Info` may still appear in PDF 2.0 files for backward compatibility but shall not be used as the definitive source when XMP is present.

---

## 2. XMP Overview

XMP (Extensible Metadata Platform) is defined by ISO 16684-1 and ISO 16684-2. It encodes metadata as an RDF/XML document embedded in a PDF metadata stream.

### Embedding in PDF

The document-level XMP stream is attached to the document catalog (`/Type /Catalog`) via the `/Metadata` key:

```
1 0 obj
<< /Type /Catalog /Pages 2 0 R /Metadata 10 0 R >>
endobj

10 0 obj
<< /Type /Metadata /Subtype /XML /Length ... >>
stream
<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    ...
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>
endstream
endobj
```

The `<?xpacket begin="..." ...?>` processing instruction marks the start of an XMP packet. The `begin` attribute value is the Unicode BOM character (`U+FEFF`, encoded as UTF-8: `0xEF 0xBB 0xBF`), serving as an encoding hint. The `id` value `W5M0MpCehiHzreSzNTczkc9d` is a fixed magic string defined by the XMP specification.

The closing `<?xpacket end="...">` uses `end="r"` (read-only, fixed-size packet) or `end="w"` (writable, in-place update permitted).

---

## 3. XMP Namespaces Relevant to PDF

XMP organizes properties into namespaces identified by URI, conventionally bound to prefixes:

### Dublin Core (`dc:` — `http://purl.org/dc/elements/1.1/`)
- `dc:title` — document title (typically an `rdf:Alt` with language tags)
- `dc:creator` — author(s) as `rdf:Seq` of strings
- `dc:description` — abstract or summary (often `rdf:Alt`)
- `dc:subject` — topics as `rdf:Bag` of strings
- `dc:rights` — copyright statement (often `rdf:Alt`)
- `dc:date` — `rdf:Seq` of ISO 8601 date strings (last modification usually most relevant)
- `dc:format` — MIME type, typically `application/pdf`
- `dc:language` — `rdf:Bag` of BCP 47 language tags
- `dc:identifier` — document identifier

### XMP Basic (`xmp:` — `http://ns.adobe.com/xap/1.0/`)
- `xmp:CreateDate` — ISO 8601 creation timestamp
- `xmp:ModifyDate` — ISO 8601 last modification timestamp
- `xmp:MetadataDate` — when the XMP metadata itself was last written
- `xmp:CreatorTool` — authoring application string
- `xmp:Label` — user-assigned label string
- `xmp:Rating` — numeric rating (integer)

### XMP Media Management (`xmpMM:` — `http://ns.adobe.com/xap/1.0/mm/`)
- `xmpMM:DocumentID` — persistent unique identifier assigned at document creation; does not change across saves
- `xmpMM:InstanceID` — unique identifier for this specific rendition; changes on every save
- `xmpMM:History` — `rdf:Seq` of `stEvt:` resource events recording revision history

### PDF-Specific (`pdf:` — `http://ns.adobe.com/pdf/1.3/`)
- `pdf:Keywords` — keyword string (mirrors `/Info` Keywords)
- `pdf:PDFVersion` — string such as `"1.7"` or `"2.0"`
- `pdf:Producer` — PDF-writing library string

### Photoshop (`photoshop:` — `http://ns.adobe.com/photoshop/1.0/`)
- `photoshop:Instructions` — special handling instructions
- `photoshop:Source` — originating organization
- `photoshop:City`, `photoshop:Country` — geographic metadata (common in editorial/press workflows)

---

## 4. RDF/XML Parsing

XMP uses a constrained subset of RDF/XML (W3C RDF 1.1 XML Syntax).

### Core Structure

```xml
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:xmp="http://ns.adobe.com/xap/1.0/">
    <xmp:CreatorTool>Adobe InDesign</xmp:CreatorTool>
    <dc:title>
      <rdf:Alt>
        <rdf:li xml:lang="x-default">My Document</rdf:li>
      </rdf:Alt>
    </dc:title>
  </rdf:Description>
</rdf:RDF>
```

The `rdf:about` attribute on `rdf:Description` is typically the empty string for document-level metadata, identifying the document itself.

### Collection Types

| RDF Type | Semantics | Use in XMP |
|----------|-----------|------------|
| `rdf:Seq` | Ordered list | Author list, date list, history |
| `rdf:Bag` | Unordered set | Subject keywords, language list |
| `rdf:Alt` | Alternatives | Localized strings (one value per language) |

Items within these collections are `rdf:li` elements.

### Localized Strings

`rdf:Alt` containers carry `xml:lang` attributes on each `rdf:li`. The special tag `x-default` marks the preferred default. When extracting a title or description for a non-localized use case, select the `x-default` item; if absent, use the first item.

### Attribute vs. Element Form

Simple string properties may appear as XML attributes on `rdf:Description` rather than child elements:

```xml
<rdf:Description rdf:about="" xmp:CreatorTool="Adobe InDesign 2024" />
```

Both forms are semantically equivalent. A conforming parser must handle both.

### Inline Objects (`rdf:parseType="Resource"`)

Structured sub-properties can be inlined without a wrapper element:

```xml
<xmpMM:History>
  <rdf:Seq>
    <rdf:li rdf:parseType="Resource">
      <stEvt:action>saved</stEvt:action>
      <stEvt:instanceID>xmp.iid:abc123</stEvt:instanceID>
    </rdf:li>
  </rdf:Seq>
</xmpMM:History>
```

---

## 5. Conflict Resolution Between /Info and XMP

When `/Info` and XMP both exist and disagree, the resolution rules are:

1. **PDF 2.0 mandates XMP as authoritative.** When the PDF version is 2.0, discard `/Info` values in favor of XMP with no ambiguity.
2. **For pre-2.0 PDFs, prefer XMP when present.** Tools that update metadata often write only to XMP (Adobe Acrobat, LibreOffice), leaving `/Info` stale. XMP is more likely to be current.
3. **Fall back to `/Info` when an XMP field is absent.** Not all producers write all XMP namespaces.
4. **Log discrepancies as structured warnings.** Expose a list of conflict records (field name, `/Info` value, XMP value) in the extraction result so callers can decide how to handle them.

**Common divergence pattern:** Microsoft Word exports PDFs with synchronized `/Info` and XMP initially. If Acrobat subsequently edits the XMP (adding subject keywords, changing title), `/Info` remains at the Word-exported values while XMP reflects the Acrobat edits. The `xmp:MetadataDate` field can help determine which was written later, but it is not always present.

---

## 6. Page-Level and Object-Level Metadata

XMP streams are not limited to the document catalog. Any stream object may carry a `/Metadata` key:

- **Page objects** (`/Type /Page`): carry provenance for that specific page, important when pages from different source documents are merged. The page-level `xmpMM:DocumentID` will differ from the document-level one.
- **Image XObjects** (`/Subtype /Image`): carry rights, author, and capture metadata for individual embedded images.
- **Form XObjects** and other content streams: less common but permitted.

When extracting, enumerate all page objects and XObjects for `/Metadata` keys. Collect page-level XMP into a per-page metadata array in the output, recording the page index and any differing `DocumentID` or `InstanceID` values. This supports provenance tracking in document assembly workflows.

---

## 7. Encrypted Metadata

The `/Encrypt` dictionary controls whether the `/Metadata` stream participates in encryption:

- **`EncryptMetadata false`**: The metadata stream is stored in plaintext regardless of the file password. This is explicitly permitted so that document management systems can index XMP without possessing the decryption key. Detect this by checking the `EncryptMetadata` boolean in the `/Encrypt` dictionary (default is `true` if the key is absent).
- **`EncryptMetadata true` (default)**: The metadata stream is encrypted with the same key derivation as all other streams. XMP is inaccessible without the user or owner password.

In pdftract, check `EncryptMetadata` before attempting to parse the `/Metadata` stream on encrypted files. If `false`, parse it unconditionally. If `true` and no decryption key is available, record the metadata as unavailable rather than emitting a parse error.

---

## 8. XMP Packets and In-Place Update

The xpacket wrapper enables XMP to be updated in a PDF file without rewriting the entire file. The packet is padded with ASCII spaces between the closing `</x:xmpmeta>` tag and the `<?xpacket end="...">` instruction:

```
</x:xmpmeta>
                                                         [hundreds of spaces]
<?xpacket end="w"?>
```

Implications for extraction:

- **Padded XMP is not malformed.** Strip trailing whitespace before passing to an XML parser if the parser does not tolerate trailing content after the document element.
- **`end="r"`** signals a read-only packet (fixed-size, not intended for in-place rewrite). **`end="w"`** signals a writable packet. pdftract is read-only, so this distinction matters only for completeness.
- When the metadata stream length differs significantly from the XML content length, the excess is padding. Do not treat this as a parse error.

---

## 9. Practical Extraction Output

The normalized metadata structure pdftract should expose:

```rust
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
    pub creator_tool: Option<String>,   // authoring application
    pub producer: Option<String>,       // PDF-writing library
    pub creation_date: Option<DateTime<Utc>>,
    pub modification_date: Option<DateTime<Utc>>,
    pub metadata_date: Option<DateTime<Utc>>,
    pub language: Option<String>,       // BCP 47
    pub document_id: Option<String>,    // xmpMM:DocumentID
    pub instance_id: Option<String>,    // xmpMM:InstanceID
    pub page_count: u32,
    pub pdf_version: Option<String>,    // e.g. "1.7"
    pub trapped: Option<Trapped>,
    pub raw_xmp: Option<String>,        // full XMP XML for callers needing fidelity
    pub metadata_conflicts: Vec<MetadataConflict>,
}

pub struct MetadataConflict {
    pub field: &'static str,
    pub info_value: String,
    pub xmp_value: String,
}
```

**Sourcing each field:**

| Field | Primary | Fallback |
|-------|---------|----------|
| `title` | `dc:title` (x-default) | `/Info` Title |
| `authors` | `dc:creator` (Seq) | `/Info` Author (split on `;`) |
| `subject` | `dc:description` (x-default) | `/Info` Subject |
| `keywords` | `pdf:Keywords` or `dc:subject` (Bag) | `/Info` Keywords (split on `,`/`;`/space) |
| `creator_tool` | `xmp:CreatorTool` | `/Info` Creator |
| `producer` | `pdf:Producer` | `/Info` Producer |
| `creation_date` | `xmp:CreateDate` | `/Info` CreationDate |
| `modification_date` | `xmp:ModifyDate` or last `dc:date` | `/Info` ModDate |
| `language` | `dc:language` (first item) | — |
| `document_id` | `xmpMM:DocumentID` | — |
| `instance_id` | `xmpMM:InstanceID` | — |
| `pdf_version` | `pdf:PDFVersion` | Header `%PDF-x.y` |
| `trapped` | `/Info` Trapped (name) | — |

Always expose `raw_xmp` so callers with domain-specific namespaces (e.g., IPTC, PRISM, MusicXML-adjacent) can parse the full packet themselves without re-reading the file.

---

## 10. Thumbnail and Preview Images

### XMP Thumbnails

The `xmp:Thumbnails` property (namespace `http://ns.adobe.com/xap/1.0/`) is an `rdf:Alt` of thumbnail structures. Each item uses the `xmpGImg:` namespace (`http://ns.adobe.com/xap/1.0/g/img/`):

- `xmpGImg:width`, `xmpGImg:height` — pixel dimensions
- `xmpGImg:format` — typically `"JPEG"`
- `xmpGImg:image` — base64-encoded image data

Decoding these requires base64 decoding followed by interpretation as the declared format (usually JPEG).

### /Thumb on Page Dictionaries

Page objects may carry a `/Thumb` entry pointing to an image XObject (a stream with `/Subtype /Image`) that represents a low-resolution preview of that page. This is independent of XMP.

### Extraction Recommendation

Thumbnail data is large (base64 JPEG) and rarely needed by text-extraction callers. The recommended approach is to exclude thumbnail bytes from the default `DocumentMetadata` output and expose thumbnail extraction as an opt-in API:

```rust
pub fn extract_thumbnail(doc: &PdfDocument, page_index: u32) -> Option<Vec<u8>>;
pub fn extract_document_thumbnail(doc: &PdfDocument) -> Option<Vec<u8>>;
```

This prevents unnecessary allocations when the caller only needs text or structured metadata. The presence of a thumbnail can still be signaled via a boolean flag in `DocumentMetadata` without materializing the bytes.

---

## References

- ISO 32000-2:2020 — PDF 2.0 specification (§ 14.3 Metadata, § 7.9.2 String Object Encoding)
- ISO 16684-1:2019 — XMP specification, Part 1: Data model, serialization and core properties
- ISO 16684-2:2014 — XMP specification, Part 2: Description of core schemas
- W3C RDF 1.1 XML Syntax — `https://www.w3.org/TR/rdf-syntax-grammar/`
- PDF spec Annex D — PDFDocEncoding character set table
