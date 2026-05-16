# Book and Publishing PDF Extraction Patterns

## Overview

Book and publishing PDFs represent one of the most structurally complex document types that a text extraction library must handle. Unlike business documents or forms, books are designed first as physical artifacts — with front matter, body chapters, and back matter serving distinct purposes — and only later digitized into PDF form. Extracting clean, ordered text from these documents requires understanding both the physical layout conventions of printed books and the typographic idioms that publishers use to communicate structure to human readers. pdftract must map these conventions accurately to produce usable output.

---

## 1. Book Structure and Logical Organization

A typeset book follows a predictable physical sequence that does not always map cleanly to PDF page order. Front matter is paginated separately in lowercase Roman numerals (i, ii, iii...) and includes the half title page (title only, no subtitle), the full title page (title, subtitle, author, publisher), the copyright page, any dedication, the table of contents, lists of figures or tables, a foreword (written by a third party), and a preface (written by the author). Body chapters begin at Arabic page 1. Back matter — appendices, bibliography, glossary, and index — continues the Arabic numbering.

pdftract must track two parallel page number streams when present: the typeset page number embedded in the layout (extracted from running footers or explicit text) and the PDF page index (0-based). These diverge at the front matter boundary and must be reconciled to allow consumers to reference extracted text by canonical book page rather than PDF position. A chapter opening on PDF page 14 may carry the typeset page number 1.

## 2. Running Headers and Footers

Professional typeset books place running headers (also called running heads) at the top of body pages. The verso (left/even) running head typically carries the book title or author name; the recto (right/odd) running head carries the current chapter title. Page numbers appear in the footer, often at the outer margin — left on verso pages, right on recto pages — though some designs place them in the header alongside the running head text.

These elements must be extracted and classified as artifact types rather than body text. pdftract should emit `page_header` and `page_footer` artifact records that carry the extracted text and the page position, but exclude them from the primary text stream. The challenge is distinguishing running heads from genuine content: they appear at fixed vertical positions, use a consistent smaller font (often 8–9pt italic), and repeat across pages. Heuristic detection should compare font size against the median body font size, check vertical position against a band threshold (top 8% or bottom 8% of the text area), and verify repetition across consecutive pages before classifying a text block as an artifact.

Chapter opening pages (rectos) and section break pages typically suppress the running header entirely. pdftract must handle missing headers gracefully without treating the absence as an anomaly.

## 3. Verso/Recto Layout and Reading Order

Even/odd page mirroring affects margins, header position, and chapter opening conventions. In traditional book design, chapters open on recto (odd, right-hand) pages. This means that if a chapter ends on a recto, the next verso is intentionally left blank — a "blank verso" — before the new chapter begins. PDF generators from InDesign and QuarkXPress typically include these blank pages in the page stream, contributing a page with no content or only a minimal footer.

pdftract must detect and handle blank versos without misinterpreting them as extraction failures. A page containing zero text runs, or only a page-number footer, should be classified as a structural spacer and noted in metadata. Reading order reconstruction must skip these pages in the content flow while preserving their presence in the page index mapping. Content consumers that are building a chapter-by-chapter text corpus need to know that page 47 was blank and not incorrectly infer a merge between the text on pages 46 and 48.

## 4. Typography: Small Caps, Drop Caps, and Ornamental Dividers

Book typography uses several conventions that challenge naive character extraction.

**Small caps** are used for author names on title pages, for chapter headings in some traditions, and for acronyms in body text. In PDF, small caps may be represented as a dedicated small-caps font variant (e.g., `MinionPro-SmallCaps`) or synthesized by scaling uppercase glyphs. When a dedicated font is used, extraction is straightforward — the characters decode to their Unicode values normally. When small caps are synthesized, the glyphs are uppercase letters rendered at a reduced point size, and extraction may produce a mix of apparent font sizes within a single word. pdftract should detect synthesized small caps by identifying runs of uppercase characters at a size approximately 70–80% of the surrounding text's cap height and normalize them.

**Drop caps** (also called versals or initial caps) span two to three lines of text. In PDF layout, a drop cap is typically a separate text object positioned to the left of a text block, with the text block indented to accommodate it. Extraction that processes objects in top-to-bottom order will often place the drop cap character after the indented paragraph text, garbling the opening of the chapter. pdftract must detect drop caps by identifying oversized single-character text objects whose bounding box intersects the vertical extent of an adjacent paragraph, then prepend the character to that paragraph's text.

**Ornamental dividers** separate sections within a chapter. Common forms include centered asterisks (`* * *`), em-dash sequences (`———`), floral ornaments rendered as glyph characters in symbol fonts, or decorative rule images. Text-based dividers extract cleanly if the font encoding is standard. Ornamental glyphs in symbol fonts require mapping via the font's ToUnicode table or, failing that, classification as a divider artifact based on position (vertically centered in white space, horizontally centered on the text block). pdftract should emit these as `section_break` structural markers rather than body text.

## 5. Chapter and Section Headers

Headers in typeset books are distinguished by font size, weight, alignment, and surrounding white space. Chapter titles are typically 18–24pt, often centered, preceded by several lines of vertical space (the chapter opening sink) and followed by additional space before body text begins. Section headers (A-heads) are 12–14pt bold or small caps; subsection headers (B-heads) may be 11pt bold italic run-in at the start of a paragraph.

pdftract must classify headers by analyzing font size relative to body text, font weight flags from the PDF font descriptor, and the vertical gap before and after the text block. A block whose font size is more than 1.4× the modal body size, centered horizontally, and preceded by more than 2× the normal line spacing is a strong candidate for a chapter or section header. These should be emitted with a structural tag (`h1` for chapter title, `h2` for A-head, `h3` for B-head) so consumers can reconstruct the document outline without post-processing heuristics.

## 6. Index Extraction

The index presents a unique extraction challenge. Index pages are typically set in two columns at a smaller font size (8–9pt) with indentation encoding hierarchy. A top-level entry consists of a term followed by page numbers. Sub-entries are indented 1 em. Page number ranges use an en-dash (23–48), and multiple references are separated by commas (23, 45–48, 112).

Correct extraction requires column detection before line reconstruction. pdftract must identify the gutter between columns and process each column as an independent text stream before merging. Within each column, indentation level (measured in points from the left column margin) distinguishes top-level entries from sub-entries. The page number list at the end of each entry must be parsed separately to support structured output — a consumer building an inverted index needs term, hierarchy level, and page number array, not a flat string.

## 7. Footnotes and Endnotes

Footnotes appear at the bottom of the page, separated from the body text by a short rule (the footnote divider) that spans approximately one-third of the text width. Footnote text is set in a smaller font (typically 8–9pt) and keyed to inline markers in the body — superscript numerals, symbols (`*`, `†`, `‡`), or letters, depending on the publisher's style.

pdftract must associate each footnote block with its inline marker. The inline marker is a superscript character (identifiable by a vertical offset above the baseline and a smaller font size) at a specific position in the body text. The footnote block at the page bottom carries a matching marker at its start. Association is by marker identity within the page scope. For endnotes — collected at chapter end or at the book's back matter — the association must span pages: the inline marker in chapter 3 body text refers to a note in the endnotes section 50 pages later. pdftract should preserve marker identity and emit note references so that downstream consumers can perform cross-page linking.

## 8. Copyright Page and ISBN Extraction

The copyright page (typically the verso of the title page) contains structured bibliographic data: publisher name and address, copyright year, ISBN-10 and ISBN-13, edition number, printing history, Library of Congress Cataloging data, and legal notices. This data is valuable for bibliographic enrichment and must be extracted as structured fields, not flat text.

pdftract should apply pattern matching for ISBN-13 (978- or 979- prefix, 13 digits with optional hyphens), ISBN-10 (10 digits or 9 digits plus X), copyright year (© followed by 4-digit year), and edition markers ("First edition", "Second printing"). The copyright page is identifiable by its position (verso of title page, Roman-numeral page iv or the equivalent) and the density of these patterns. Structured output should separate publisher, year, ISBNs, and edition into discrete fields.

## 9. Publisher PDF Generator Profiles

The `/Producer` entry in the PDF document information dictionary reveals the toolchain used to create the file, and this has predictable implications for extraction quality.

InDesign exports via Direct Export or via Distiller produce well-formed font encoding with complete ToUnicode maps, accurate glyph widths, and predictable text object ordering. These are the cleanest source for extraction. QuarkXPress PDFs are similarly clean but may use older Type 1 fonts with encoding vectors that require manual mapping for ligatures (fi, fl, ff, ffi, ffl). TeX-based PDFs from academic publishers using pdfTeX or XeTeX embed Type 1 or OpenType fonts with full Unicode mapping when compiled with modern packages; older TeX-to-PostScript-to-PDF workflows may produce PDFs with encoding gaps for common ligatures. pdftract should detect the generator string and apply generator-specific heuristics: for TeX sources, apply ligature normalization; for QuarkXPress sources, verify encoding against known encoding vectors.

## 10. EPUB-to-PDF Conversion Artifacts

A significant share of ebook-era books are produced as EPUBs first and converted to PDF via Calibre, Pandoc, or browser print-to-PDF. These conversions produce structurally different PDFs from native InDesign output.

EPUB-sourced PDFs typically have clean Unicode throughout — no encoding gaps, no ligature issues — because the source is HTML. However, they introduce other artifacts: chapter break pages that are blank or contain only a decorative image, inconsistent heading font sizes (because CSS heading styles do not map cleanly to fixed typographic conventions), and ornamental chapter separators as inline images rather than text. Page margins are often uniform (no verso/recto mirroring), and there are no running headers, so the header/footer detection heuristics that work for InDesign output do not apply.

pdftract should detect EPUB-sourced PDFs by checking for common generator strings (Calibre, wkhtmltopdf, headless Chrome, Prince) and adjust extraction strategy accordingly: skip header/footer artifact detection, treat blank pages as chapter breaks rather than structural spacers, and apply a more permissive heading-detection threshold given the flatter typographic hierarchy common in reflowable EPUB layouts.

---

## Summary

Clean text extraction from book and publishing PDFs requires handling two distinct layers: the physical layout conventions of printed books (running heads, verso/recto mirroring, chapter sinks, blank pages) and the typographic idioms that encode structure (drop caps, small caps, ornamental dividers, header hierarchies). pdftract must detect and classify these elements as structural artifacts or semantic markers rather than treating all text objects as body content. The generator-detection approach allows the extraction pipeline to apply source-appropriate heuristics, producing well-ordered, structurally annotated output that faithfully represents the logical organization of the original work.
