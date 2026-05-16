# PDF/UA-2, WCAG Alignment, and Next-Generation Accessibility Standards

## Overview

PDF/UA-2 (ISO 14289-2) represents a significant architectural departure from its predecessor, anchored to PDF 2.0 (ISO 32000-2) rather than PDF 1.7. For a text extraction library like pdftract, this matters because the structural foundations that govern how content is tagged, ordered, and described have been systematically improved. A pdftract extraction pipeline that already handles PDF/UA-1 correctly is well-positioned to support PDF/UA-2 — the incremental work centers on namespace resolution, MathML extraction, Unicode normalization, and a more precise handling of artifact classification.

---

## PDF/UA-2: Key Changes from PDF/UA-1

The most consequential structural change in PDF/UA-2 is the adoption of the PDF 2.0 namespace mechanism for structure element tag names. In PDF/UA-1, structure types like `P`, `H1`, `Table`, and `Figure` were drawn from a flat global namespace defined by the PDF specification. PDF/UA-2 requires that every structure element be namespace-qualified, binding tag names to a specific namespace URI. The standard namespace for PDF 2.0 structure types is `http://iso.org/pdf2/ssn`. Processors that assume a flat namespace will misidentify or drop elements in conforming PDF/UA-2 documents, so pdftract must resolve the `/NS` dictionary on each structure element and apply namespace-aware tag matching rather than bare string comparison.

Artifact classification in PDF/UA-2 is substantially more granular. PDF/UA-1 recognized artifact subtypes of `Layout`, `Page`, and `Pagination`, but the classification criteria were loosely specified. PDF/UA-2 formalizes these subtypes and adds `Background` as an explicit artifact subtype for purely decorative content — content that conveys no information and should be excluded from any logical reading order. The `/BBox` attribute on artifact dictionaries now carries a specific meaning: it defines the bounding box of the artifact in page coordinates, which enables pdftract to spatially exclude artifactual content during extraction without relying solely on the type label. The `/AttachedTop` attribute indicates whether a page artifact (such as a header or footer region) is anchored to the top of the page, providing layout semantics that pdftract can use when reconstructing reading order. For extraction purposes, pdftract should filter Background artifacts entirely from text output and should handle Page and Pagination artifacts as configurable — either excluded by default or surfaced in a separate metadata channel.

Unicode normalization requirements are made explicit in PDF/UA-2: all text content must be in NFC (Canonical Decomposition followed by Canonical Composition). This is a hard requirement, not a recommendation. In practice, many legacy PDFs — particularly those produced by Arabic or Hebrew typesetting systems — emit text in NFD or NFKD form, where combining characters appear as separate code points following their base character. pdftract must apply NFC normalization to all extracted text strings as a post-processing step regardless of the document's claimed conformance level, since the consistency guarantee matters for downstream consumers even when the source PDF predates UA-2.

Language tagging requirements are also tightened in PDF/UA-2. The `/Lang` entry must be a valid BCP 47 language tag, and inheritance rules are more strictly defined: a structure element without a `/Lang` entry inherits the language of its nearest ancestor that carries one, ultimately falling back to the document-level `/Lang` in the document catalog. pdftract should validate inherited language tags when processing PDF/UA-2 documents and surface the resolved language for each extracted content run, rather than only the document-level default. Invalid or absent language tags should be flagged in the extraction metadata, since they constitute an accessibility violation that affects how downstream TTS engines and screen readers interpret the content.

---

## PDF 2.0 Structure Improvements

PDF 2.0 removed a number of structure types that had become ambiguous or were poorly supported in practice. Deprecated types from PDF 1.7 — including `BlockQuote`, `Caption` used outside its defined context, and several others — are no longer valid in the standard structure namespace. In their place, PDF 2.0 introduced several new types: `DocumentFragment` for embedded sub-documents, `Aside` for supplementary or tangentially related content, `Title` as a dedicated type distinct from heading levels, `FENote` for footnotes and endnotes, and `Sub` for inline subexpression content. These additions give pdftract more precise semantic signals during extraction. An `Aside` element, for instance, should be extractable but may warrant a different confidence weight in reading-order heuristics, since asides are by definition non-linear content. `FENote` provides a clean hook for extracting footnote content with its source anchor, rather than having to infer footnote structure from spatial positioning.

pdftract's handling of PDF 2.0 structure types should begin with a namespace-aware type resolution step. When the `/NS` dictionary on a structure element references a known namespace URI, the tag name is interpreted in that namespace's vocabulary. If the namespace is unrecognized, pdftract should treat the element as an application-defined extension and fall back to extracting its text content without semantic classification. This ensures forward compatibility: future namespaces will not cause extraction failures, only a loss of type-specific enrichment.

---

## MathML in PDF 2.0

PDF 2.0 introduces first-class support for mathematical content via the MathML namespace (`http://www.w3.org/1998/Math/MathML`). When a structure element's `/NS` entry references the MathML namespace, the element subtree represents a MathML expression rather than a PDF structure type. The glyph content rendered to the page is still present in the content stream — PDF must remain renderable without MathML support — but the MathML subtree carries the full semantic meaning of the expression: operator precedence, variable binding, and mathematical relationships that are entirely absent from the rendered glyph sequence.

For pdftract, the extraction strategy for mathematical content should prefer MathML when present. The MathML subtree can be serialized as a self-contained MathML fragment and included in the extraction output as a dedicated content block, with the associated page glyphs available as a fallback representation. Attempting to reconstruct mathematical meaning from glyph sequences alone is fragile: ligatures, spacing glyphs, and operator symbols used in typeset mathematics do not map reliably to semantic mathematical intent. MathML extraction sidesteps this problem entirely by reading the semantic annotation that the PDF author has already encoded. pdftract's extraction pipeline should identify structure elements carrying a MathML namespace, serialize the full MathML subtree, and emit it as a typed content block alongside positional metadata.

---

## WCAG 2.1 and PDF Techniques

The PDF-specific techniques in WCAG 2.1 — PDF1 through PDF23 — map directly onto features that PDF/UA-2 either requires or formalizes. PDF1 (applying text alternatives to images) corresponds to the `/Alt` attribute on `Figure` elements; PDF2 (bookmark navigation) corresponds to the document outline; PDF11 and PDF12 address form field accessibility; PDF17 covers consistent heading structure. PDF/UA-2 does not merely align with these techniques — for conforming documents, it mandates the underlying structural features that make those techniques achievable.

pdftract's confidence scoring system can surface WCAG-relevant signals as part of its extraction output. Structure elements carrying `/Alt` text, correctly ordered heading hierarchies, explicit language tags, and proper artifact classification all contribute to an accessible document. When these signals are present and well-formed, pdftract can report high confidence in the semantic accuracy of extracted content. When they are absent or malformed — a `Figure` without `/Alt`, a heading sequence that skips levels, a document with no language tag — pdftract can report reduced confidence and flag specific accessibility gaps. This is not a full WCAG audit, but it gives downstream consumers actionable metadata about the reliability of the extraction and the accessibility posture of the source document.

---

## Associated Files in PDF 2.0

PDF 2.0 extends the `/AF` (Associated Files) key beyond page and XObject dictionaries to structure elements themselves. An `AF` array on a structure element can reference embedded files that are semantically associated with that element's content — for example, a source spreadsheet linked to a `Table` structure element, or a data file associated with a `Figure`. pdftract should traverse the `/AF` arrays on structure elements during extraction and surface associated file metadata — including the file relationship type specified in the `/AFRelationship` key — as part of the element's extracted output. The actual file content can be optionally extracted and written to a sidecar path or included as base64 in structured output formats. This is particularly valuable for data-rich documents where the associated files contain the machine-readable source underlying rendered content.

---

## Phoneme Metadata

PDF/UA-2 allows `/Phoneme` attributes on structure elements to provide pronunciation hints for text-to-speech engines. These attributes carry phonemic transcriptions in a format specified by the document's `/PhoneticAlphabet` entry. pdftract can surface phoneme attributes as supplementary metadata on extracted content spans without requiring any TTS capability itself. Downstream consumers that feed extracted text into speech synthesis pipelines benefit from having these hints available in the extraction output, since they encode the document author's explicit pronunciation intent for ambiguous terms, abbreviations, and proper nouns.

---

## Backwards Compatibility and the pdftract Upgrade Path

A pdftract pipeline that correctly handles PDF/UA-1 already covers the structural fundamentals: logical structure tree traversal, reading order reconstruction from structure order rather than content stream order, artifact filtering, and `/Alt` text extraction for non-text content. What PDF/UA-2 adds is a defined set of extensions to that foundation.

The concrete additions required are: (1) namespace-aware structure type resolution using the `/NS` dictionary, replacing bare string tag matching; (2) MathML subtree serialization when the MathML namespace is detected on a structure element; (3) NFC normalization applied to all extracted text, regardless of document conformance level; (4) BCP 47 validation and inheritance resolution for `/Lang` entries; (5) Background artifact filtering using the formally defined subtype; (6) `/BBox` and `/AttachedTop` consumption on artifact dictionaries for spatial exclusion; (7) associated file extraction via `/AF` arrays on structure elements; and (8) phoneme attribute surfacing as extraction metadata. None of these changes are in conflict with the PDF/UA-1 handling path — they are additive. A pdftract binary that implements all eight extensions will correctly extract content from PDF/UA-1, PDF/UA-2, and non-conforming PDF 2.0 documents, degrading gracefully where conformance features are absent.
