# PDF/A Archival Format Extraction Guarantees and Fast-Path Optimization

## Overview

PDF/A is a family of ISO standards that constrain the PDF specification to ensure long-term document preservation and reproducibility. For a text extraction library, PDF/A conformance is not merely a metadata curiosity — it is a contractual statement about what the document contains and how it is encoded. Each PDF/A level carries specific structural guarantees that pdftract can exploit to choose faster, more confident extraction paths, skipping heuristics and fallbacks that are only necessary for unconstrained PDFs.

Understanding what each conformance level actually guarantees — and what it does not — is the foundation for building a reliable fast-path dispatcher.

---

## PDF/A-1: The Baseline Contract

PDF/A-1 (ISO 19005-1), based on PDF 1.4, defines two conformance levels with meaningfully different extraction implications.

**Level B (Basic)** establishes the minimum floor: all fonts must be embedded in their entirety, including the font program and font descriptor, so that rendering is never dependent on a system-installed substitute. PDF encryption is prohibited. Transparency is disallowed, and all color must be device-independent (CMYK or ICC-tagged). XMP metadata is required in the document catalog. Level B does not require a logical structure tree, does not mandate ToUnicode CMaps beyond what is needed for rendering, and does not require language tagging.

For extraction, Level B means that font substitution is impossible — pdftract can trust that whatever ToUnicode CMap is present is the actual encoding used during rendering. However, because structure trees are optional at Level B, reading order must still be inferred from the glyph position stream. Layout heuristics remain necessary, but the encoding layer is reliable.

**Level A (Accessible)** adds a full set of structural requirements on top of Level B. All fonts must include complete ToUnicode CMaps covering every glyph present in the document. A logical structure tree must be present and must reflect the natural reading order of the content. Every text run in the content stream must be associated with a structure element. Non-standard glyphs — ligatures, decorative characters, or any glyph whose Unicode mapping is ambiguous — must carry ActualText attributes in the structure tree. Language tags at both the document and element level are required.

Level A is the gold standard for extraction. The structure tree provides reading order directly, the ToUnicode CMaps provide authoritative character mapping, and ActualText resolves all encoding ambiguities without glyph shape analysis. At Level A, pdftract has everything it needs to perform extraction via pure structure traversal, with zero reliance on layout geometry.

---

## PDF/A-2: Extended Capabilities, Familiar Guarantees

PDF/A-2 (ISO 19005-2, PDF 1.7) extends the Level A/B distinction and introduces several PDF 1.7 features that were not available in PDF/A-1.

JPEG2000 (JPX) image compression is now permitted, alongside all existing PDF 1.7 compression types. Optional Content Groups (OCGs) are allowed, provided that all content is visible in the default view — a conforming reader must not need to toggle any layer to see the document as intended. Transparency and blending modes are permitted, as is the use of PDF 1.7 digital signatures. Embedded file attachments are not supported at this level. PDF/A-2 also allows PDF/A-1 compliant documents to be embedded as attachments, enabling composite archival bundles.

For extraction, PDF/A-2 adds one important consideration: OCGs. Even though the standard requires all content to be visible by default, pdftract must be aware that content stream objects may be wrapped in optional content markers (`/OC` dictionary references). When traversing structure elements, pdftract should resolve OCG visibility using the default configuration (`/D` entry in the `/OCProperties` dictionary) and skip any content marked as off by default. Ignoring this layer means extracting text that users would not see in a standard rendering.

The Level A and Level B guarantees carry forward unchanged from PDF/A-1. A PDF/A-2a document still guarantees a complete structure tree, ToUnicode coverage, and ActualText where needed. A PDF/A-2b document still guarantees font embedding without requiring structure.

---

## PDF/A-3: Arbitrary Attachments and the ZUGFeRD Pattern

PDF/A-3 (ISO 19005-3, PDF 1.7) is structurally identical to PDF/A-2 with one significant addition: it permits arbitrary file attachments via the embedded file mechanism, with any MIME type, provided that each attachment carries an `AFRelationship` key in its embedded file stream dictionary.

The primary use case driving PDF/A-3 adoption is hybrid invoice formats such as ZUGFeRD and Factur-X, where a human-readable PDF invoice is paired with a machine-readable XML attachment (typically `factur-x.xml` or `ZUGFeRD-invoice.xml`) carrying the same financial data in a structured electronic form. The `AFRelationship` value in these documents is typically `/Alternative`, indicating that the attachment is a full-fidelity alternative representation of the visual content.

For pdftract, PDF/A-3 introduces an extraction opportunity beyond plain text: when an embedded file with `AFRelationship /Alternative` or `/Source` is detected, the structured data in the attachment may be more semantically rich than what can be extracted from the visual layer. pdftract should surface embedded file metadata — including file name, MIME type, and `AFRelationship` value — alongside the text extraction result so that callers can decide whether to consume the attachment directly.

The Level A and Level B extraction guarantees for the visual layer are identical to PDF/A-2.

---

## PDF/A-4: A Restructured Conformance Hierarchy

PDF/A-4 (ISO 19005-4, PDF 2.0) abandons the Level A/B distinction in favor of three new levels aligned with PDF 2.0 capabilities.

**Level F (Full)** permits attached files with `AFRelationship` labels, similar to PDF/A-3, and requires PDF 2.0 conformance throughout. It does not mandate a logical structure tree. This is the base level, analogous to Level B in earlier versions but without the carryover of the A/B vocabulary.

**Level E (Engineering)** is an extension of Level F intended for engineering and technical documents. It adds requirements specific to technical drawing workflows but does not fundamentally change the extraction guarantee set compared to Level F.

**Level U (Unencrypted)** explicitly prohibits encryption and is intended for environments where unobstructed long-term access is a hard requirement. It does not add structure tree requirements beyond what Level F establishes.

Notably, PDF/A-4 does not have a dedicated accessibility level equivalent to the old Level A. Accessibility requirements in PDF 2.0 are addressed by the PDF/UA-2 standard (ISO 14289-2) rather than being embedded in the archival standard. A PDF/A-4 document that also satisfies PDF/UA-2 carries both conformance claims in its XMP metadata, and that combination is the PDF/A-4 equivalent of the old Level A for extraction purposes.

For pdftract, this means that detecting full extraction confidence for a PDF/A-4 document requires checking for both the PDF/A-4 conformance claim and a PDF/UA-2 conformance claim in the XMP metadata. A PDF/A-4 document without the PDF/UA-2 pairing should be treated like Level B: font embedding is reliable, but structure-tree extraction cannot be assumed.

---

## Conformance Detection in pdftract

PDF/A conformance is declared in two complementary locations that pdftract must both inspect.

The `/OutputIntents` array in the document catalog contains one or more output intent dictionaries. A PDF/A conforming document includes an output intent with `/S /GTS_PDFA1` (for PDF/A-1), `/S /GTS_PDFA2` (for PDF/A-2 and PDF/A-3), or `/S /GTS_PDFA4` (for PDF/A-4). The presence of this key provides a fast structural signal of conformance intent, though it is not the authoritative source.

The authoritative source is the XMP metadata stream embedded in the document catalog's `/Metadata` stream. A conforming PDF/A document must include `pdfaid:part` (the integer version number: 1, 2, 3, or 4) and `pdfaid:conformance` (a single uppercase letter: A, B, F, E, or U) in the XMP namespace `http://www.aiim.org/pdfa/ns/id/`. pdftract should parse these two fields directly from the raw XMP XML rather than relying on an intermediate metadata abstraction, since encoding errors in higher-level parsers can silently misreport conformance.

When both sources are present, they should agree. Divergence between the `/OutputIntents` signal and the XMP claim is itself an indicator of a non-conformant document.

---

## The Level A Fast Path

For a document confirmed to be PDF/A-1a, PDF/A-2a, or PDF/A-3a (or PDF/A-4 with PDF/UA-2), pdftract can activate the structure-tree fast path:

1. Traverse the structure tree using the `/StructTreeRoot` entry in the document catalog.
2. Walk the tree in document order, collecting `Span`, `P`, `H`, `L`, `Table`, and other leaf elements.
3. For each marked content reference (`/MCID`), resolve the corresponding content stream segment and decode characters using the ToUnicode CMap of the active font.
4. For any element carrying an `ActualText` attribute, use the ActualText value directly rather than decoding from glyphs.
5. Use the language tags at each element to annotate the extracted text spans.

This path bypasses glyph shape matching entirely, bypasses OCR (since all text is already encoded), and bypasses all layout heuristics for reading order. In practice, the structure-tree path is dramatically faster — typically an order of magnitude or more — compared to a geometry-based extraction pipeline, because it operates on the logical tree rather than the dense coordinate space of the content stream.

---

## Level B Extraction and Confidence Calibration

For Level B documents, pdftract takes a partially accelerated path. Font embedding guarantees mean that character decoding via ToUnicode CMaps is reliable — there is no risk that a missing font causes systematic encoding failure. However, the absence of a structure tree means reading order must be reconstructed from the glyph position stream using standard layout analysis: line grouping by vertical proximity, column detection, and reading-order sorting.

The extraction confidence score for a Level B document should be set lower than for Level A, reflecting the fact that reading order is inferred rather than specified. The character-level accuracy can still be very high, but structural accuracy (paragraph boundaries, column order, footnote placement) is heuristic.

---

## Validating Conformance Claims

A significant minority of production PDFs claim PDF/A conformance through the `/OutputIntents` or XMP mechanism but would fail validation by a conformance checker. These documents may have been produced by tools that stamp a conformance claim without verifying the underlying document structure.

pdftract should treat the conformance claim as a hypothesis to be partially verified rather than a fact to be accepted. The key checks are: (1) at least one font embedded in the `/Resources` dictionary of each page that renders text; (2) no `/Encrypt` dictionary present in the document catalog; (3) a `/Metadata` stream present with parseable XMP; and (4) for Level A, a `/StructTreeRoot` entry present in the document catalog.

If any of these checks fails on a document claiming PDF/A Level A, pdftract should downgrade the extraction path — falling back to Level B treatment if the structural tree is absent, or to unconstrained PDF treatment if fonts appear unembedded. The downgrade should be recorded in the extraction result's metadata so that callers can understand why the fast path was not taken and investigate the source document if needed.

---

## Summary

PDF/A conformance levels form a spectrum of structural guarantees that pdftract can translate directly into extraction strategy. Level A across all version families (PDF/A-1a through PDF/A-3a, and PDF/A-4 paired with PDF/UA-2) provides the complete extraction contract: structure tree, ToUnicode CMaps, ActualText, and language tags. This enables a pure structure-traversal fast path that is significantly faster and more accurate than geometry-based extraction. Level B and its PDF/A-4 equivalents guarantee font embedding and encoding reliability but require layout heuristics for reading order. Non-conformant documents claiming PDF/A status must be detected through structural cross-checks and routed to the appropriate fallback path rather than silently receiving a fast path they have not earned.
