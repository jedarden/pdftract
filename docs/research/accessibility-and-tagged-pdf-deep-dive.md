# Accessibility, Tagged PDF, and PDF/UA Compliance — Deep Technical Dive

## Overview

PDF/UA-1 (ISO 14289-1) defines the minimum technical requirements for universally accessible PDF documents. Where PDF/A concerns itself primarily with archival fidelity and self-containment, PDF/UA imposes a much stricter contract on the logical structure of the document: every piece of content must be accounted for in the structure tree, reading order must be derivable from that tree without ambiguity, and non-text elements must carry textual descriptions. For pdftract, a PDF/UA-conformant document represents the ideal input — a document that provides a complete, authoritative map from structure to content stream bytes, eliminating the need for heuristic reconstruction of reading order or semantic grouping.

---

## 1. PDF/UA Requirements and What They Give pdftract

Under PDF/UA-1, every content item in a page's content stream must either be tagged — meaning it appears as the marked content of a structure element reachable from the structure tree — or explicitly marked as an artifact. Nothing is allowed to be anonymous. This requirement transforms the structure tree from an optional annotation layer into a complete index of all page content.

For extraction, this has three direct consequences. First, the structure tree can be traversed depth-first to yield a guaranteed logical reading order that is independent of content stream ordering. In a non-tagged PDF, the content stream order reflects the painter's model (back-to-front rendering order), which rarely matches human reading order, particularly in multi-column layouts, footnoted pages, or documents with sidebars. PDF/UA eliminates this ambiguity. Second, artifacts can be identified and excluded categorically rather than heuristically. Third, the combination of ActualText, Alt, and E attributes on structure elements provides machine-readable text alternatives for content that would otherwise require glyph-to-Unicode mapping or OCR. pdftract should treat PDF/UA conformance as a capability flag that, when set, unlocks a higher-confidence extraction path.

---

## 2. Standard Structure Types and Semantic Extraction

The PDF specification defines a fixed set of standard structure types. pdftract must recognize all of them and map each to an appropriate extraction result. Grouping elements — Document, Part, Art, Sect, Div — establish the document hierarchy and produce no direct text output but define scope for attribute inheritance and section boundary detection. BlockQuote signals an indented quotation; extraction should preserve it as a distinct block with a semantic role annotation. Caption, when it is a child of Table or Figure, binds a text string to a non-text element. TOC and TOCI represent the table of contents and its individual entries; pdftract can reconstruct a structured outline from these without parsing page numbers from the visual layout.

Index and NonStruct are notable edge cases. Index groups index entries but does not itself constitute body text. NonStruct is a structure element with no semantic role — it exists purely as a grouping convenience and should be treated transparently, passing its children's content through without adding semantic meaning. Private is similar but signals proprietary structure; extraction should recurse into it without assuming any meaning.

Among the inline and block content types: P is a paragraph; H and H1–H6 are headings at a specific outline level; L, LI, Lbl, and LBody form the list model where Lbl holds the bullet or number and LBody holds the list item's paragraph content; Table, TR, TH, and TD implement the table model with optional THead, TBody, and TFoot groupings for header/body/footer row groups. Span groups inline content; Quote marks an inline quotation; Note is a footnote or endnote and should be extracted separately from the paragraph that references it; Reference is a citation; BibEntry is a bibliographic entry. Code marks programmatic text. Figure, Formula, and Form are non-text elements for which Alt text is the primary extraction target.

For every element type, pdftract's structure tree walker must map the tag name to one of these categories, apply the appropriate block or inline model, and produce a typed output node rather than a flat text string.

---

## 3. MCID: The Link from Structure Tree to Content Stream

Marked Content Identifiers (MCIDs) are the mechanism that connects structure tree leaf nodes to the actual bytes in a content stream. A structure element's content array may contain MCID references (integer dictionaries with /MCID and /Pg keys). On the content stream side, the operators BDC (Begin Marked Content with a property dictionary) and BMC (Begin Marked Content without properties) open marked content sequences, and EMC closes them. A BDC operator with a /MCID entry in its property dictionary creates a named marked content sequence; the MCID value must match a reference in the structure tree.

pdftract's extraction pipeline must build a two-way index: a forward map from (page, MCID) to the content stream byte range, and a reverse map from that byte range back to the structure element. The actual text bytes — glyphs, glyph widths, font encoding — are extracted from the content stream in the usual manner, but the order in which they are assembled is determined by the structure tree traversal, not by content stream position. This is the core inversion that distinguishes tagged PDF extraction from untagged PDF extraction. For each leaf structure element, pdftract collects all MCID references, resolves each to a content stream segment, extracts the text from each segment using the active font's encoding and ToUnicode CMap, and concatenates the results in MCID order within the element.

---

## 4. ActualText: Overriding Character Codes

The ActualText attribute, when present on a structure element or on a marked content sequence property dictionary, provides a verbatim Unicode string that replaces the decoded character sequence from the content stream. pdftract must check for ActualText before performing any glyph-to-Unicode mapping on a segment. If present, the stream bytes are treated as opaque rendering instructions, and the ActualText value is the extracted text.

ActualText is critical for ligatures (the glyph U+FB01 "fi" may be encoded as a single code point or as two code points with a single glyph; ActualText ensures "fi" appears in extraction), for accessible mathematics (equation renderers often encode symbols in private-use areas and provide ActualText with the correct Unicode representation), and for stylized text (decorative fonts with non-standard encodings). pdftract's MCID resolver should apply an ActualText check as the first step before falling through to encoding-based extraction.

---

## 5. Alt Text: Extraction from Non-Text Elements

The Alt attribute on Figure, Formula, Form, and other non-text structure elements provides a text alternative intended for screen reader users. For pdftract, Alt text is a first-class extraction target. When a Figure element is encountered, the extraction result should include the Alt value as a text node annotated with a role of "alt-text" rather than silently dropping it or treating the element as empty. This enables downstream consumers — search indexers, summarizers, accessibility auditors — to include figure descriptions in their text model.

---

## 6. E (Expansion) Attribute: Abbreviation Resolution

The E attribute on Span elements provides an expansion for an abbreviation or acronym. If a Span element containing the text "WHO" carries /E "World Health Organization", the expansion is the semantically correct text for extraction contexts that prioritize meaning over surface form. pdftract should expose both: the surface text (from the content stream or ActualText) and the expansion (from E), allowing callers to choose which to use. A technical extraction mode might return "WHO" for exact-match indexing; a semantic mode would substitute or append "World Health Organization".

---

## 7. RoleMap: Resolving Custom Structure Types

PDF allows documents to define custom structure element names in the document catalog's /RoleMap dictionary, mapping each custom name to a standard type. A document might use /Section mapped to /Sect or /Callout mapped to /Note. pdftract must resolve all structure element names through the RoleMap before applying extraction semantics. The resolution algorithm is: if the element name appears in RoleMap, substitute the mapped name and repeat until a standard type is reached or a cycle is detected. Cycle detection is required because malformed documents can create circular RoleMap entries. Unresolvable names should be treated as NonStruct — transparent grouping with no semantic role.

---

## 8. Artifact Marking: Excluding Non-Body Content

Content marked with the /Artifact tag type (using BMC or BDC with /Artifact as the tag name) falls outside the structure tree by definition. PDF/UA defines four artifact subtypes: Header, Footer, Background, and Page. pdftract must detect artifact-marked content sequences and route them to a separate extraction bucket, not the body text stream. For most extraction use cases, headers and footers are noise; providing them as optional annotated output rather than suppressing them entirely gives callers the flexibility to include or exclude them. Background and Page artifacts should be excluded by default since they represent decorative or layout elements with no textual value.

---

## 9. Attribute Inheritance in the Structure Tree

PDF structure attributes propagate from parent elements to descendants unless overridden. The /Lang attribute is the most consequential for extraction: a document in English with a single /Lang "en-US" on the Document root propagates that language to every element. A /Sect element in French within an English document carries /Lang "fr-FR", which propagates to all /P and /Span descendants within that section. pdftract must implement attribute inheritance as a stack-based operation during structure tree traversal, pushing the active attribute set when entering an element and popping it on exit. Inherited attributes that matter for extraction include Lang (for language-aware text processing and hyphenation), WritingMode (for right-to-left or vertical text assembly), and any custom attributes conveyed via class maps.

---

## 10. Fallback for Partially-Tagged PDFs

Many PDFs claim PDF/UA conformance but deliver incomplete structure trees. Common failure modes include: structure elements with no MCID references (orphaned nodes with no content), content stream segments with MCIDs that have no corresponding structure tree entry (orphaned content), and artifacts that are not marked in the content stream but are not tagged in the structure tree either.

pdftract's fallback strategy must be layered. The first pass attempts full structure-tree-driven extraction: resolve all MCIDs, collect all content, and verify that all content stream text operators are accounted for. If unaccounted content remains — text-drawing operators not associated with any MCID — the fallback activates. Untagged text segments are extracted using the content stream ordering heuristic: sort by vertical position (descending) and then horizontal position (ascending), grouping by proximity into synthetic paragraph blocks. These blocks are emitted with a provenance annotation indicating they originated from the fallback path, allowing callers to treat them with reduced confidence. If the structure tree is so incomplete that fewer than a threshold percentage of content stream text is accounted for, pdftract should demote the document to untagged-extraction mode entirely rather than producing a mixed output where structured and unstructured content is interleaved without clear boundaries.

The practical implication is that pdftract maintains two extraction pipelines sharing a common content stream reader: a structure-tree-driven pipeline for tagged documents and a heuristic pipeline for untagged documents, with a validation pass that determines which pipeline to engage and whether a hybrid fallback is appropriate.
