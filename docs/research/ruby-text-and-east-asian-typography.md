# Ruby Text and East Asian Typography

## Overview

Japanese and broader East Asian PDFs present a distinct set of extraction challenges that go beyond the concerns of Latin-script documents. Ruby annotations (furigana), vertical writing modes, full-width character normalization, CJK punctuation conventions, and mixed-script line composition all require dedicated handling. This document specifies what pdftract must implement to extract readable, semantically accurate text from these documents.

---

## 1. Ruby Text in Tagged PDFs

The PDF specification defines a Ruby structure type for encoding phonetic annotations alongside base text. A Ruby element contains three child types: `Rb` (ruby base), `Rt` (ruby text, the phonetic gloss), and optionally `Rp` (ruby parenthesis, used as fallback delimiters in non-ruby-aware renderers).

When pdftract processes a tagged PDF, the structure tree is traversed before any geometric analysis. On encountering a `Ruby` structure element, the extractor must:

1. Collect all `Rb` children and concatenate their marked-content spans to form the base text string.
2. Collect all `Rt` children and concatenate their spans to form the phonetic annotation string.
3. Discard `Rp` children from the output text — they are presentational fallbacks and should not appear in the extracted result.

The output span for a tagged Ruby element carries two distinct fields: `text` holds the base characters (e.g., the kanji), and `ruby_text` holds the phonetic annotation (e.g., the hiragana reading). These are never merged. Merging them would produce a string that interleaves kanji and kana in an order that misrepresents the document's content and breaks downstream NLP pipelines.

---

## 2. Untagged Ruby: Geometric Detection

Most Japanese PDFs in the wild are not tagged. Furigana in untagged documents appears as a cluster of small-font glyphs positioned above — or occasionally beside — the corresponding kanji. Detecting and associating these annotations requires geometric reasoning.

The primary signal is font size ratio. Furigana glyphs are conventionally half the size of the base text, and the PDF specification's informal guidance places the typical ruby-to-base size ratio below 0.6. pdftract computes this ratio per span by comparing the font size of candidate small glyphs against the dominant font size on the line. Any span with a size ratio below 0.6 and a vertical offset placing its baseline above the base line (in default top-to-bottom coordinates, a lower y-origin in PDF space) is flagged as a ruby candidate.

Association proceeds by horizontal overlap. For each ruby candidate span, pdftract finds the base text span or spans whose x-extent overlaps the candidate's x-extent. Where a single ruby candidate overlaps multiple base characters, the annotation is associated with the full overlapping base segment as a unit, not with individual glyphs. This mirrors the tagged structure: `Rt` annotates `Rb` as a whole, not character by character.

Once association is confirmed, the small-font spans are removed from the main text stream and placed into `ruby_text` fields on the corresponding base spans. This prevents furigana from being emitted inline, which would corrupt word boundaries and reading order.

---

## 3. Japanese Justification (Kinsoku Shori)

Japanese typesetting enforces line-boundary constraints called kinsoku shori. Certain characters — closing brackets, closing parentheses, the ideographic period (。), the ideographic comma (、) — must not appear at the start of a line. Others — opening brackets, opening parentheses — must not appear at the end. PDF generators enforce these constraints by distributing extra inter-character spacing across the line, rather than by inserting visible gaps between words.

pdftract's word-boundary detection must not misread kinsoku-adjusted spacing as word gaps. The correct approach is to apply a CJK-aware gap threshold: for lines where the dominant script is CJK, word boundaries are detected only when inter-glyph spacing exceeds a substantially higher fraction of the em width than would apply to Latin text. In practice, adjacent CJK characters with spacing up to roughly 0.5 em should be treated as part of the same word. Spacing introduced by kinsoku shori is generally far smaller than this threshold.

Additionally, pdftract should detect lines where the last character is a kinsoku-prohibited opening bracket or the first character is a kinsoku-prohibited closing mark, and flag these as potential justification artifacts rather than paragraph breaks.

---

## 4. Full-Width and Half-Width Character Normalization

East Asian PDFs frequently mix full-width forms of Latin characters and digits (U+FF01–U+FF5E range) with their ASCII equivalents. Full-width Latin letters and digits are semantically identical to their half-width counterparts and should be normalized to ASCII for interoperability with downstream text processing.

pdftract exposes this normalization as a configurable option, enabled by default. When active, full-width Latin letters (Ａ–Ｚ, ａ–ｚ) and full-width digits (０–９) are mapped to their ASCII equivalents (A–Z, a–z, 0–9). Full-width punctuation — including the ideographic space (U+3000), fullwidth comma (U+FF0C), and fullwidth period (U+FF0E) — is preserved by default, because full-width punctuation carries typographic meaning in CJK contexts and its conversion would alter the visual and semantic representation of the source document.

Half-width katakana (U+FF65–U+FF9F) is left unconverted: normalizing it to full-width katakana changes glyph identity in ways that are not always desirable, and the correct mapping is context-dependent. Applications requiring half-to-full katakana normalization should apply Unicode NFKC decomposition independently.

---

## 5. Tate-Chu-Yoko and Vertical Mode Punctuation Rotation

Vertical writing mode (tategumi) is covered in the multilingual document extraction research. Two Japanese-specific complications are addressed here.

Tate-chu-yoko (縦中横) is the convention of typesetting a short horizontal sequence — typically a two- or three-digit number, a Latin abbreviation, or a year — horizontally within a vertical text column. In PDFs this appears as a text run whose individual glyphs are rotated 90 degrees relative to the surrounding vertical text, positioned so that the horizontal sequence reads left-to-right as a unit within the downward flow.

pdftract detects tate-chu-yoko by identifying short horizontal runs (two to four glyphs) embedded in a vertical writing-mode column whose glyph transform matrices indicate a 90-degree rotation relative to the enclosing text direction. These runs are extracted as a single token and inserted into the vertical reading order at the correct position.

Punctuation rotation is a related issue. In vertical mode, certain punctuation glyphs (commas, periods, brackets) are rotated or replaced with alternate glyph forms whose center point sits at the glyph center rather than the baseline. pdftract must account for this when computing bounding boxes and reading order for vertical text runs, using the glyph's advance vector in the text matrix rather than assuming a fixed baseline.

---

## 6. CJK Punctuation Preservation

The ideographic period (。 U+3002), corner bracket open (「 U+300C), corner bracket close (」 U+300D), ideographic comma (、 U+3001), and katakana middle dot (・ U+30FB) each carry specific semantic and typographic roles in CJK text. pdftract preserves these characters exactly as encoded. They must not be stripped, replaced with their Latin near-equivalents, or normalized by Unicode composition.

Spacing around CJK punctuation in extraction output follows the source glyphs: if the PDF places no space before an ideographic period, none is emitted. This differs from Latin punctuation normalization, where pdftract may insert or collapse spaces around sentence-terminal marks.

---

## 7. Proportional vs. Monospaced CJK Glyphs

Traditional CJK typesetting assigns every character an advance width of exactly one em, producing a monospaced grid. Modern OpenType CJK fonts — particularly those used in web-origin or office-suite PDFs — may assign proportional advance widths, so a narrow character like ー (katakana long vowel mark) may have an advance width less than 1 em.

This matters for word-boundary detection. pdftract's gap threshold for CJK is expressed as a fraction of the observed advance width, not a fixed em fraction. When advance widths are uniform (monospaced), the threshold is a fraction of that width. When advance widths vary, pdftract computes the gap as actual inter-glyph white space (the distance between one glyph's right edge and the next glyph's left edge) and applies the threshold to that value. This prevents proportionally-spaced CJK from generating false word breaks at every narrow character.

---

## 8. Mixed CJK/Latin Line Composition

A single line in a Japanese document may contain Latin words, numerals, or identifiers interspersed with CJK characters. The gap-detection logic must treat CJK-to-Latin and Latin-to-CJK transitions differently from Latin-to-Latin transitions.

Between adjacent CJK characters, no gap is expected and none is inserted in the output. Between a CJK character and an adjacent Latin character (or vice versa), a thin space is conventionally added by the PDF generator. pdftract detects this thin space — typically around 0.25 em — and does not treat it as a word boundary. A full word boundary between a CJK and Latin token requires a gap substantially larger than this conventional thin space, typically exceeding 0.5 em. Latin-to-Latin word boundaries use the standard Latin gap threshold independently of the surrounding CJK context.

---

## 9. Chinese Traditional vs. Simplified Detection

The most authoritative source for distinguishing Traditional Chinese (zh-TW, zh-HK) from Simplified Chinese (zh-CN) is the `/Lang` attribute on the document catalog, the page dictionary, or individual structure elements. pdftract reads this attribute at the lowest available level — structure element first, then page, then catalog — and tags extracted spans with the resolved language code.

When no `/Lang` attribute is present, pdftract falls back to character-set heuristics. A set of characters exists that appear only in Simplified Chinese orthography (simplified-only characters with no traditional equivalent in common use) and a complementary set for Traditional Chinese. pdftract maintains a compact lookup table of high-frequency discriminating characters. If a page's character content contains more than a configurable threshold of simplified-only characters and zero traditional-only characters, the page is labeled `zh-CN`. The reverse yields `zh-TW`. Mixed or ambiguous pages receive no language label from this heuristic; the caller is expected to treat unlabeled pages as undetermined.

---

## 10. Output Representation for Ruby

Whether ruby is detected via tagged structure or geometric inference, the output schema is uniform. Each span that carries a phonetic annotation emits:

- `text`: the base text (kanji or base characters), used as the primary readable content.
- `ruby_text`: the phonetic annotation (hiragana or katakana reading), stored as a sibling field on the span.

Applications that need plain text concatenate only `text` fields, producing natural-reading prose without interleaved annotations. Applications that need both readings — dictionary tools, accessibility pipelines, OCR training data generators — read `ruby_text` from the same span without requiring a separate parse pass.

Base and ruby text are never concatenated, interleaved by character, or emitted as parenthetical inline annotations by default. An optional rendering mode for accessibility output may emit the ruby parenthesis form `base(ruby)`, but this must be an explicit opt-in and must not be the default behavior.

---

## Implementation Priority

Ruby detection (both tagged and geometric) and kinsoku-aware gap thresholds are the highest-priority items, as they directly determine whether Japanese PDF text is readable after extraction. Full-width normalization and CJK punctuation preservation are low-risk, high-correctness improvements that can be implemented early. Tate-chu-yoko detection and proportional CJK handling are lower frequency but must be addressed before pdftract is considered production-ready for Japanese documents.
