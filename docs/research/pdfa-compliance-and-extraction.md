# PDF/A Compliance and Extraction

PDF/A (ISO 19005) is the ISO archival subset of PDF. Its structural guarantees are not merely administrative — they directly eliminate the major failure modes in text extraction. A compliant PDF/A document removes uncertainty about font encoding, reading order, and content accessibility. This document enumerates those guarantees, explains how to detect them from the document catalog, and defines the optimized extraction path for each conformance level.

---

## 1. PDF/A Variants Overview

PDF/A has four published parts, each based on a different PDF specification version:

- **PDF/A-1** (ISO 19005-1, 2005): based on PDF 1.4. Conformance levels **a** (accessible) and **b** (basic). Level A requires full tagging; Level B requires only font embedding and device-independent color.
- **PDF/A-2** (ISO 19005-2, 2011): based on PDF 1.7 (ISO 32000-1). Adds level **u** (Unicode), which mandates ToUnicode mappings for every character. Also permits JPEG2000, transparency, and optional content groups that PDF/A-1 forbids.
- **PDF/A-3** (ISO 19005-3, 2012): based on PDF 1.7. Identical to PDF/A-2 in conformance levels (a, b, u) but lifts the restriction on embedded file formats — arbitrary file attachments are permitted with a declared relationship.
- **PDF/A-4** (ISO 19005-4, 2020): based on PDF 2.0 (ISO 32000-2). Restructures levels into **f** (files — replaces b, requires at least one embedded file or none) and **e** (engineering — for technical drawings). The Unicode requirement from level U is folded into the baseline for PDF/A-4f and PDF/A-4e.

For text extraction, the relevant capability gradient is: B → U → A, where each step adds a stronger structural guarantee that eliminates a class of heuristics.

---

## 2. Detection

### XMP Metadata Declaration

PDF/A conformance is self-declared in an XMP metadata stream attached to the document catalog (the root object). The relevant namespace is:

```
http://www.aiim.org/pdfa/ns/id/
```

Two properties carry the conformance claim:

| XMP Property         | Value Type | Examples       |
|----------------------|------------|----------------|
| `pdfaid:part`        | Integer    | `1`, `2`, `3`, `4` |
| `pdfaid:conformance` | String     | `A`, `B`, `U`, `F`, `E` |

The XMP stream is located via `Catalog -> Metadata` (a stream object with `Subtype /XML`). Parse the raw XML — it is serialized RDF/XML — and extract the two properties from the `pdfaid:` namespace.

A document declaring PDF/A-2u will have:

```xml
<pdfaid:part>2</pdfaid:part>
<pdfaid:conformance>U</pdfaid:conformance>
```

### Corroborating Signals

For level A, the `MarkInfo` dictionary in the catalog provides an independent signal:

```
Catalog -> MarkInfo -> /Marked true
```

If `Marked` is `true` but `pdfaid:conformance` is `B` or absent, the document is tagged but not necessarily PDF/A-compliant — treat the tagging as opportunistic rather than guaranteed correct. The XMP declaration is the authoritative source; `MarkInfo` is confirmatory.

### When to Trust the Declaration

PDF/A validation is an external concern (validators such as veraPDF implement the full rule set). For extraction purposes, treat the XMP declaration as sufficient if:

1. The XMP stream is present and parseable.
2. The `pdfaid:part` and `pdfaid:conformance` values are valid.
3. The document was produced by a known-good authoring tool (check `xmp:CreatorTool` as a heuristic — PostScript distillers and document converters frequently produce non-compliant PDFs that falsely declare PDF/A).

For documents with suspicious provenance, verify independently: confirm that `FontDescriptor` entries contain `FontFile`/`FontFile2`/`FontFile3`, that no `Encrypt` dictionary is present, and that `StructTreeRoot` is present for level A claims.

---

## 3. Level B Guarantees

Level B (basic) is the minimum conformance tier. It establishes the structural preconditions that make reliable extraction possible at all:

- **No encryption**: the `Encrypt` dictionary must be absent. Content is always accessible without a password. The extraction engine can skip the decryption path entirely.
- **All fonts embedded**: every font referenced in a content stream must have its data embedded in `FontDescriptor.FontFile`, `FontDescriptor.FontFile2` (TrueType), or `FontDescriptor.FontFile3` (CFF/OpenType/Type1C). Partial embedding is not permitted if the missing glyphs appear in the document.
- **No external content references**: no `URI` actions that load remote resources, no external graphic imports. The document is self-contained.
- **No JavaScript or launch actions**: `AA` and `OpenAction` entries must not contain `JavaScript` or `Launch` actions.
- **Device-independent color**: all colors are expressed in ICC-profiled or device-independent spaces, or the document declares an `OutputIntent` ICC profile that gives device-dependent operators (`RG`, `rg`, `K`, `k`) a defined meaning.

The practical consequence for extraction: **font data is always present**. The fallback path that handles missing font files (glyph shape fingerprinting, width heuristics, external font databases) is unnecessary for Level B and above.

---

## 4. Level U Guarantees

PDF/A-2u and PDF/A-3u add a single critical requirement on top of Level B:

**Every character code in every content stream must have a ToUnicode mapping.**

The `ToUnicode` CMap stream must be present in every `Font` dictionary, and the mapping must cover every code point that appears in the document's text operators (`Tj`, `TJ`, `'`, `"`). There are no gaps, no unmapped ranges, and no reliance on glyph name heuristics.

This is the most important guarantee for text extraction. The two-stage encoding resolution process — first attempt ToUnicode, fall back to glyph name normalization, fall back to shape fingerprinting — collapses to a single step: read the ToUnicode CMap and apply it directly.

The extraction engine can skip:
- Glyph name to Unicode inference (Adobe Glyph List lookups, `/uni`-prefixed name parsing).
- Shape fingerprinting against reference glyph databases.
- Width-based character disambiguation.
- Encoding difference array fallback for Type1 fonts.

Implement a fast path: if `pdfaid:part` is `2` or `3` and `pdfaid:conformance` is `U` or `A`, assert that every `Font` object has a `ToUnicode` entry and decode all text exclusively through those CMaps. If a `ToUnicode` entry is missing on a Level U document, the document is non-conformant — log a warning and fall back to standard recovery, but do not silently proceed as if it were guaranteed correct.

---

## 5. Level A Guarantees

Level A (accessible) adds full logical structure on top of Level U:

- **Tagged content**: `Catalog.StructTreeRoot` is present. Every page's content stream elements are either tagged (associated with a structure element via marked content sequences `BDC`/`EMC` with an `MCID`) or explicitly marked as artifacts.
- **`MarkInfo /Marked true`**: declared in the catalog.
- **Reading order encoded**: the `StructTreeRoot` tree encodes the logical reading order of all tagged content. The leaf nodes (`Span`, `P`, `Figure`, etc.) appear in the tree in document logical order, not in page painting order.
- **Role mapping**: `Catalog.StructTreeRoot.RoleMap` maps custom element types to standard PDF structure types (defined in ISO 32000 Table 333).

The extraction consequence: reading order is already solved. The heuristic reading order algorithm — column detection, bounding-box sorting, gap analysis — is unnecessary. Walk the structure tree in document order, collect the MCIDs at each leaf, resolve them to marked content sequences on the page, and emit text in that order.

Zone labeling is also resolved: the structure tree distinguishes headings (`H`, `H1`–`H6`), paragraphs (`P`), list items (`LI`), table cells (`TD`, `TH`), figures (`Figure`), and artifacts (headers, footers, page numbers). No heuristic zone classifier is needed.

---

## 6. Font Embedding Requirements

PDF/A's font embedding rule is absolute: if a glyph is painted in the document, its outline must be embedded. This applies to all font types:

| Font Type    | Required Key        |
|--------------|---------------------|
| Type1        | `FontDescriptor.FontFile`  |
| TrueType     | `FontDescriptor.FontFile2` |
| CFF/OpenType | `FontDescriptor.FontFile3` |
| Type0 (CID)  | Descendant font's `FontDescriptor.FontFile2` or `FontFile3` |

Subsetting is allowed but must not remove glyphs that appear in the content stream. The subset tag (a six-uppercase-letter prefix in the `BaseFont` name, e.g., `ABCDEF+TimesNewRoman`) identifies subsetted fonts, but all used glyphs are present by definition.

For extraction, this means: if outline-based fingerprinting is ever needed (e.g., diagnosing a non-conformant Level B document with a broken ToUnicode), the font data is always present to fingerprint against.

---

## 7. Color Space Requirements

PDF/A forbids bare device-dependent color operators without an `OutputIntent`. Specifically:

- Operators `RG`/`rg` (DeviceRGB), `K`/`k` (DeviceCMYK), and `G`/`g` (DeviceGray) are only valid if `Catalog.OutputIntents` contains an ICC-based output intent profile.
- All ICC profiles referenced via `ICCBased` color spaces must be embedded.

For text visibility detection — determining whether text is rendered in a color that contrasts with its background — this simplification means color comparisons always operate in a well-defined space. Converting text and background colors to a common space (via the declared ICC profile) is unambiguous. There are no undefined device-dependent color values that require producer-specific interpretation.

---

## 8. Artifacts and Tagging

In a Level A document, the artifact mechanism makes the distinction between content and decoration explicit at the byte level. Page elements that are not part of the logical document flow are wrapped in artifact marked-content sequences:

```
/Artifact <</Type /Pagination /Subtype /Header>> BDC
  BT ... ET   % page number or running header
EMC
```

Standard artifact subtypes defined by ISO 32000: `Header`, `Footer`, `Watermark`, `PageNum`, `Bates`, `LineNum`, `Redaction`. Custom types are permitted with `RoleMap` entries.

This means: page headers, footers, and decorative rules are identified by the document itself. The extraction engine does not need to infer their status from position or font size. Skip all `Artifact`-tagged content when building the logical text output; include it only if the caller requests full-page text (e.g., for header/footer metadata extraction).

---

## 9. PDF/A-3 Embedded Files

PDF/A-3 lifts the embedded-file prohibition present in PDF/A-1 and PDF/A-2. Every embedded file must declare an `AFRelationship` value in the `EmbeddedFile` stream's dictionary:

| `AFRelationship` | Meaning |
|------------------|---------|
| `Source`         | The embedded file is the source from which the PDF was generated |
| `Data`           | Structured data related to the document |
| `Alternative`    | A machine-readable alternative rendition of the document content |
| `Supplement`     | Supplementary information not contained in the PDF |
| `Unspecified`    | Relationship not declared |

The `Alternative` relationship is significant for extraction: the embedded file may contain the full document text in a structured format (XML, JSON, plain text). The most common real-world case is **ZUGFeRD / Factur-X**: a PDF/A-3 invoice with an embedded XML file (Factur-X XML, `AFRelationship: Alternative`) that contains all invoice fields in machine-readable form. Extracting the embedded XML from a Factur-X document is more reliable than parsing the PDF text layer.

Enumerate embedded files via `Catalog.Names.EmbeddedFiles` (a name tree) or `Catalog.AF` (an array of file specification dictionaries). Check `AFRelationship` and extract the embedded stream when `Alternative` or `Data` is present.

---

## 10. Extraction Strategy by Conformance Level

Detect conformance early — immediately after parsing the catalog, before any content stream processing — and branch into the appropriate extraction path.

### Level B (`pdfaid:part` 1/2/3/4, `pdfaid:conformance` B or F or E)

- Font data always present: skip external font database lookups.
- ToUnicode not guaranteed: run standard two-stage encoding resolution (ToUnicode → glyph name → shape fingerprint).
- Reading order: use heuristic column/block sort.
- Artifacts: not reliably identified; apply heuristic header/footer detection.

### Level U (`pdfaid:part` 2 or 3, `pdfaid:conformance` U)

- Assert ToUnicode present on every font; error-log if absent.
- Decode all text exclusively via ToUnicode CMaps. Skip glyph name resolution and fingerprinting.
- Reading order: still heuristic (no StructTree guarantee).
- Performance gain: eliminates the most expensive fallback path.

### Level A (`pdfaid:part` 1/2/3/4, `pdfaid:conformance` A)

- All Level U guarantees apply.
- Walk `StructTreeRoot` in tree order to determine reading order and zone labels.
- Skip the heuristic reading-order algorithm entirely.
- Skip heuristic header/footer detection: artifacts are explicitly marked.
- Emit text in structure-tree order; annotate output with structure element types.

### Output Metadata

Report `pdfa_level` in the extraction output metadata:

```rust
pub enum PdfaLevel {
    None,
    Part1B, Part1A,
    Part2B, Part2U, Part2A,
    Part3B, Part3U, Part3A,
    Part4F, Part4E,
}
```

This allows callers to know the confidence level of the extracted text and to request the fast path explicitly when processing large batches of known-compliant archival documents.

### Trust Hierarchy

When the declared conformance level implies a guarantee (e.g., ToUnicode always present for Level U), verify the assumption on the first font encountered. If the document is non-conformant, downgrade the active level, emit a diagnostic, and continue with the full fallback pipeline. Never assume compliance is infallible — archival workflows do produce non-conformant files that declare PDF/A.
