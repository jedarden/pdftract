# Multilingual and Mixed-Script PDF Extraction

## Overview

PDF extraction from multilingual documents is one of the most demanding problems in the text extraction domain. A document mixing Latin prose with Arabic footnotes, or a Japanese academic paper that includes Hebrew proper nouns in a citation, demands that the extraction engine handle fundamentally different directionality models, encoding conventions, joining behaviors, and layout assumptions simultaneously. This document defines what pdftract must implement to produce logically ordered, Unicode-correct text from such documents.

## 1. Mixed-Script Pages and Reading Order at the Span Level

When a page contains both Latin and Arabic text — or Latin and CJK, or Latin and Hebrew — the naive approach of concatenating glyphs left-to-right by x-coordinate will produce garbage. Directionality is not a page property; it is a span-level property. A single paragraph can contain a Latin phrase, an embedded Arabic noun, and a Latin continuation, each segment flowing in a different direction before the paragraph proceeds left-to-right.

pdftract must assign a base directionality to each span based on its script content, then compose spans using the Unicode Bidirectional Algorithm rather than geometric order alone. Span boundaries must be drawn where script or directionality changes, not solely where font or size changes. Each code point is tested against Unicode script property tables (UCD `Script` property), and a new span is opened when the dominant script transitions between families with different base directions.

For Latin+CJK mixed text, directionality is less fraught because CJK is nominally left-to-right in horizontal layout, but span assembly still requires care: CJK characters carry different line metrics, and naively joining them with Latin characters using the same inter-glyph spacing assumptions breaks word boundaries.

## 2. Unicode Bidirectional Algorithm (UAX #9) and Paragraph Embedding Levels

The Unicode Bidirectional Algorithm (UBA, defined in UAX #9) is the normative framework for ordering characters within a paragraph of mixed-directionality text. pdftract must implement a conforming UBA resolver for span assembly rather than relying on geometric ordering.

The Paragraph Embedding Level (PEL) is the foundational concept: an integer (0 for LTR, 1 for RTL) determined by the first strong directional character in the paragraph. All relative ordering of runs derives from this level. pdftract should derive the PEL from the dominant script of the first strong-character span on each logical text block.

Explicit embedding uses Unicode control characters: Left-to-Right Embedding (LRE, U+202A), Right-to-Left Embedding (RLE, U+202B), and Pop Directional Formatting (PDF, U+202C). Some generators insert these into ToUnicode output streams. pdftract must preserve them and feed them to the UBA resolver rather than stripping them as noise. When absent, implicit bidi infers run boundaries from strong directional characters. After resolving bidi levels, spans must be reordered to logical order before emission — search engines, NLP pipelines, and screen readers all expect logical order.

## 3. RTL Language Extraction: Visual vs. Logical Order Detection

Arabic and Hebrew are natively right-to-left. The critical divergence is how a given PDF encodes RTL text: in logical order (as a native reader would type) or in visual order (glyph positions left-to-right by x-coordinate, readable only when reversed).

A PDF from XeLaTeX or a modern word processor stores RTL glyphs in logical order, relying on the glyph positioning matrix for visual placement. PDFs from older Acrobat workflows or PostScript distillers may store glyphs in visual order. pdftract must detect which convention applies by comparing the geometric glyph sequence against the UBA-resolved expected sequence. If RTL glyphs are already reversed relative to the UBA prediction, the stream is in visual order and must be reversed before output. Detection must occur per text block, not per page, since a single page may contain content from different software pipelines.

## 4. Arabic Cursive Joining and Encoding Source Detection

Arabic letters have up to four contextual forms — isolated, initial, medial, and final — encoded in the Unicode Arabic Presentation Forms blocks (U+FB50–U+FDFF, U+FE70–U+FEFF) and in the primary Arabic block (U+0620–U+06FF). Presentation forms are legacy encodings; logical-order extraction must normalize them back to base code points.

pdfLaTeX, XeLaTeX, and modern Word PDF export encode Arabic in logical order using base code points. Acrobat distilling from PostScript may encode Arabic using presentation-form code points in visual order. pdftract detects which applies by inspecting the ToUnicode CMap of each font covering the Arabic range. When presentation forms are present, the pipeline must: (a) reverse the glyph sequence from visual to logical order, and (b) normalize presentation-form code points to their base equivalents via Unicode canonical decomposition or an explicit shaping reversal table. This normalization must precede span assembly so that word-boundary detection operates on canonical code points.

## 5. Hebrew: Logical Order, Legacy Visual Order, and Nikud

Modern Hebrew PDFs from Word, LibreOffice, or XeLaTeX store text in logical order, with glyph positioning matrices handling visual placement. Legacy PDFs from WordPerfect for DOS or early Windows word processors may use visual order; the same detection heuristic as Arabic applies.

Hebrew nikud (vowel diacritics) and cantillation marks (trop) are Unicode combining characters in the Hebrew block (U+05B0–U+05C7 for niqqud, U+0591–U+05AF for cantillation). They attach to their base consonant as combining character sequences. pdftract must preserve these sequences in canonical combining class order. Extractors that process glyphs atomically silently drop nikud, producing Hebrew text that loses vowelization — a significant data loss for liturgical, classical, or pedagogically annotated texts.

## 6. Mixed-Column Layout with RTL Reading Order

Arabic-language newspapers and RTL-dominant documents use multi-column layouts where columns themselves flow right-to-left: the rightmost column is read first, not last. Column detection algorithms that assume LTR column order will produce a completely inverted reading sequence.

pdftract's column detection must be script-aware. When the dominant script on a page is RTL, columns are sorted by descending x-coordinate and inter-column reading order proceeds right-to-left; lines within each column remain top-to-bottom. The page's dominant PEL drives this decision. Pages with mixed column layouts — an RTL main body alongside an LTR sidebar — require column-level script classification to assign independent reading orders per region.

## 7. CJK Vertical Text

Japanese and Chinese documents frequently use vertical writing mode, where text flows top-to-bottom within columns that themselves flow right-to-left. PDF represents this with Td and Tm operators: in vertical mode, dominant glyph displacement is along the y-axis, and advance values come from the font's vertical metrics (vmtx table).

pdftract must detect vertical writing mode by examining the Tm matrix for 90-degree rotations and checking whether the font declares vertical metrics. When detected, span assembly uses y-coordinate ordering to sequence glyphs and "column width" replaces "line height" for layout reconstruction. Vertical text also intermixes horizontal Latin numerals and abbreviations via tatechuyoko (horizontal-in-vertical) sub-mode; pdftract must detect these horizontal runs within a vertical column and preserve them inline without rotating their characters.

## 8. Font Fallback and Cross-Script Unicode Contamination

A multilingual PDF may embed ten or more fonts covering different scripts. ToUnicode CMaps must be applied strictly per-font: the same byte sequence in two different font resources may map to entirely different Unicode code points. pdftract must maintain font context for every glyph and never apply a CMap from one font to glyphs rendered in another.

The contamination risk is highest when a PDF uses PUA (Private Use Area) ranges in one font to encode a script that another font encodes in a standard range, or when two fonts share overlapping byte ranges mapped differently. pdftract's span assembly must tag each character with its source font identifier and permit cross-font joining only when Unicode ranges are orthogonal. When joining across font boundaries, Unicode normalization (NFC for most scripts, NFD for scripts with combining characters) must be applied consistently.

## 9. Transliterated Text in Scientific Documents

Scientific and linguistic papers routinely present native-script terms alongside Latin transliterations: a paper on Arabic morphology may write "كتب (kataba)" in a single inline run. pdftract must preserve both the native-script sequence and the transliteration in their presented order, without discarding either as decorative.

Span-merging heuristics that consolidate spans by script will incorrectly split these inline mixed pairs. pdftract's span merger must treat script-mixed spans as atomic when they appear within a single bounding box at the same baseline, joining them as a single logical unit whose internal directionality is resolved by the UBA before output.

## 10. Language Tagging and the PDF Lang Attribute

PDF's Tagged PDF structure supports a `Lang` attribute at the document, page, structure element, and span levels, following BCP 47 syntax (e.g., `ar-SA`, `he-IL`, `ja-JP`). NLP pipelines use these tags to select tokenizers, search engines use them for stemmers, and accessibility tools use them for text-to-speech voice selection.

pdftract must extract `Lang` attributes at every level and propagate them via structure tree inheritance: a span with no explicit `Lang` inherits from its enclosing structure element, which inherits from the page, which inherits from the document. Fine-grained values override coarser defaults. The resolved tag attaches to each output span as metadata, letting downstream consumers validate language detection and correctly process spans whose script alone is ambiguous — Latin text may be English, Turkish, Vietnamese, or dozens of other languages depending on diacritic patterns.

## Summary

Correct multilingual extraction requires pdftract to implement: UBA-conforming bidi resolution with PEL detection; visual-to-logical normalization for Arabic and Hebrew with generator-convention detection; Arabic presentation-form normalization; Hebrew combining character preservation; RTL-aware column ordering; vertical CJK handling with tatechuyoko support; per-font CMap enforcement without cross-contamination; preservation of transliterated inline pairs; and propagation of PDF `Lang` attributes to output spans. Each is a distinct implementation concern. Failure in any one produces silently incorrect output — the bytes are present, but in the wrong order or normalization form.
