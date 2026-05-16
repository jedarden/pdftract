# Post-Extraction Normalization Pipeline

Raw text extracted from a PDF is not presentation-ready text. Glyphs are decoded individually, positioned by absolute coordinates, and carry no semantic information about word boundaries, paragraph structure, or typographic intent. This document describes the normalization pipeline that transforms raw extracted text into clean, semantically coherent output.

---

## 1. Hyphenation Handling

PDF typesetters insert hyphens at line boundaries for optical justification. Three distinct codepoints appear in practice:

- **U+002D HYPHEN-MINUS** — the workhorse character. Used both as a hard (intentional) hyphen and as an end-of-line break marker inserted by the typesetter.
- **U+00AD SOFT HYPHEN** — a Unicode line-break hint. When present mid-word, it signals that the word *may* be broken here, but is not itself visible. Remove it unconditionally during normalization.
- **U+2010 HYPHEN** — unambiguous hard hyphen, always intentional.

The difficult case is U+002D at end-of-line. Detecting it requires combining positional evidence with lexical evidence:

1. **Positional test**: the glyph is the last character on the line, and its right edge is within a configurable threshold (typically 5–10% of column width) of the right text margin for that column.
2. **Lexical test**: concatenate the word prefix (characters before the hyphen on the current line) with the word suffix (first token on the next line). Query a language-appropriate dictionary or spell-checker. If the concatenated form is a known word and the hyphenated form is not, the hyphen is a break artifact and should be removed when joining lines.
3. **Compound-word fallback**: if neither form resolves cleanly, preserve the hyphen. Compound words in German, Dutch, and Norwegian are frequently hyphenated intentionally even mid-line.

Language-specific rules add complexity. German has mandatory spelling hyphens (e.g., *Dampf-schiff* as a stylistic compound variant) that must not be joined. Arabic and Hebrew text flow right-to-left; end-of-line positions are mirrored. Thai and CJK scripts do not use hyphens at all.

The implementation strategy: build a `HyphenResolver` trait with a method `fn should_join(prefix: &str, suffix: &str, lang: Language) -> bool`, backed by a word-frequency dictionary for the target language. For `lang = Unknown`, default to preserving the hyphen.

---

## 2. Ligature Expansion

OpenType fonts frequently encode multi-character sequences as single glyphs in the Private Use Area or as Unicode Compatibility Area codepoints. The standard Latin ligatures with assigned Unicode codepoints are:

| Codepoint | Form | Expansion |
|-----------|------|-----------|
| U+FB00    | ff   | f + f     |
| U+FB01    | fi   | f + i     |
| U+FB02    | fl   | f + l     |
| U+FB03    | ffi  | f + f + i |
| U+FB04    | ffl  | f + f + l |
| U+FB05    | ſt   | ſ + t     |
| U+FB06    | st   | s + t     |

Expand all of these unconditionally for search-oriented output. A full-text search index that receives `U+FB01` will not match the query `fi`; expansion to component letters is required.

For display-fidelity output where the caller wants to preserve typographic forms, expansion should be optional. Expose a `LigatureMode` enum: `Expand` (default for search), `Preserve` (for display). Note that NFKC normalization (see §5) collapses these ligatures automatically, so if NFKC is applied, ligature mode has no additional effect.

**Arabic ligatures** are more complex. The mandatory ligature *lam-alef* (U+FEFB/U+FEFC) must be expanded to lam (U+0644) + alef (U+0627) for correct text processing. Other Arabic presentation forms in the FBxx–FExx range should similarly be decomposed. Arabic shaping is the font renderer's responsibility, not the extraction layer's; after expansion, a bidirectional algorithm (Unicode Bidirectional Algorithm, UBA) determines display order.

**Devanagari** uses conjunct consonants that are orthographically distinct from their component sequences. These are *not* ligatures in the presentation sense; they represent distinct orthographic units. Do not attempt to decompose them.

---

## 3. Line and Paragraph Break Reconstruction

PDF text streams contain positioned runs, not lines. Reconstruction requires:

**Soft wrap detection (same paragraph)**: a line break is a soft wrap when:
- The vertical gap between the bottom of line *n* and the top of line *n+1* is within 1.2× the line height (the typeset leading).
- The last character of line *n* is not sentence-ending punctuation (`.`, `?`, `!`, `:`) or when it is but the next line begins with a lowercase letter (indicating mid-sentence break).
- The right edge of the last glyph on line *n* is within the right-margin proximity threshold (the line was wrapped, not short).

When all conditions hold, join with a single U+0020 SPACE.

**Hard paragraph break detection**: a vertical gap exceeding 1.5× the line height, or a first-line indent on the following line (detected as a left-edge offset exceeding a threshold), signals a paragraph boundary. Emit a double newline or a paragraph separator (U+2029 PARAGRAPH SEPARATOR) depending on output format.

**Short lines**: a line whose right edge falls well inside the right margin that is followed by a line with a reset left margin signals a paragraph break even without a large vertical gap (common in ragged-right body text and poetry).

Store each text segment with its bounding box `(x0, y0, x1, y1)` in page coordinates. Sort by `(y0, x0)` for left-to-right scripts; use the dominant reading direction for bidi content.

---

## 4. Whitespace Normalization

PDF character positioning uses absolute coordinates. Adjacent glyphs separated by a small positive advance (less than one-third of the space glyph width for the current font) are concatenated without a space. Larger gaps produce either an explicit space glyph or an implicit word boundary.

After joining glyph runs:

- **Collapse runs of U+0020**: replace any sequence of two or more SPACE characters with a single SPACE.
- **Remove invisible Unicode spaces**: strip U+200B ZERO WIDTH SPACE, U+200C ZERO WIDTH NON-JOINER, U+200D ZERO WIDTH JOINER, and U+FEFF BOM/ZWNBSP where they appear mid-text.
- **NO-BREAK SPACE (U+00A0)**: normalize to U+0020 in body text. Preserve in contexts where breaking is semantically wrong (between a number and its unit, e.g., *42 kg*) if the caller opts in.
- **Trim per block**: strip leading and trailing whitespace from each reconstructed paragraph block before emitting.

---

## 5. Unicode Normalization

Unicode defines four normalization forms:

- **NFD**: Canonical Decomposition. Precomposed characters are decomposed into base + combining sequences. Useful for accent stripping downstream.
- **NFC**: Canonical Decomposition followed by Canonical Composition. The standard interchange form; round-trips with NFD.
- **NFKD**: Compatibility Decomposition. Collapses compatibility variants: fullwidth ASCII, circled letters, fraction characters, ligatures, superscripts.
- **NFKC**: Compatibility Decomposition followed by Canonical Composition. The most aggressive normalization.

For PDF extraction:

- **Apply NFC** to the output by default. It normalizes precomposed characters extracted via different code paths into a consistent form without destroying content.
- **Do not apply NFKC by default.** NFKC collapses `ﬁ` (U+FB01) to `fi` (collapsing the ligature, which is usually correct), but also collapses `①` to `1`, `½` to `1⁄2`, and fullwidth `Ａ` to `A`. This alters content that may be semantically significant (fractions in mathematical texts, circled numbers in diagrams). Expose NFKC as a caller-controlled option.
- **Surrogates and noncharacters**: codepoints U+D800–U+DFFF (lone surrogates) and U+FDD0–U+FDEF plus U+FFFE/U+FFFF (noncharacters) must be removed. They appear when a font's CMap maps a glyph to a malformed Unicode value. Replace with U+FFFD or drop, depending on caller preference.
- **Private Use Area codepoints**: U+E000–U+F8FF are frequently used as glyph placeholders in symbolic fonts. Strip them unless the caller's glyph recovery layer has already mapped them to real codepoints (see the glyph recognition research document).

The `unicode-normalization` crate provides `nfc()`, `nfd()`, `nfkc()`, `nfkd()` iterators over `char` streams and is the canonical implementation for Rust.

---

## 6. Quote and Dash Normalization

PDFs from professional typesetters use typographic punctuation. Two normalization strategies are useful:

**Preserve typographic forms** (default, for display fidelity):
- Left single quotation mark: U+2018 `'`
- Right single quotation mark / apostrophe: U+2019 `'`
- Left double quotation mark: U+201C `"`
- Right double quotation mark: U+201D `"`
- Em dash: U+2014 `—`
- En dash: U+2013 `–`

**Normalize to ASCII equivalents** (for search and downstream NLP):
- U+2018, U+2019 → U+0027 `'`
- U+201C, U+201D → U+0022 `"`
- U+2014, U+2013, U+2012 → U+002D `-` (or preserve dashes as-is; NLP pipelines vary)

The figure dash (U+2012) is rare but appears in some European typesetting. The horizontal bar (U+2015) appears in Greek text for dialogue attribution.

Expose this as a `QuoteMode` enum (`Preserve`, `AsciiEquivalents`) and a `DashMode` enum (`Preserve`, `HyphenMinus`, `Retain`). Neither should default to normalization; lossy transformations require explicit opt-in.

---

## 7. Running Header and Footer Deduplication

After zone classification (header zone, footer zone, body zone), headers and footers must be removed from the primary text stream. The extraction strategy:

1. Classify text blocks by vertical position: blocks in the top 10% or bottom 10% of the page area are candidates.
2. Across a document, compare candidate blocks across pages. A block whose text (ignoring page numbers) appears on ≥ 80% of pages is a running header or footer.
3. Strip these blocks from the text stream. Optionally emit them into a parallel `headers: Vec<String>` / `footers: Vec<String>` field on the page output.
4. Page numbers embedded in headers/footers are identified by the pattern of incrementing integers. Normalize them out when stripping. If a header reads `Chapter 3 — Methodology   42`, the page number `42` varies per page while `Chapter 3 — Methodology` is the repeated fragment.

For deduplication, a normalized comparison (lowercased, whitespace-collapsed, digits wildcarded) across pages is sufficient. Store a fingerprint `(text_without_digits_normalized, frequency_count)` per candidate block.

---

## 8. Control Character and Artifact Removal

Strip the following unconditionally:

- **U+000C FORM FEED** — page separators inserted by some PDF export tools.
- **U+000D CARRIAGE RETURN** not followed by U+000A — normalize CR+LF to LF; standalone CR to LF.
- **U+0000 NULL** — produced by malformed CMap entries.
- **U+FFFD REPLACEMENT CHARACTER** — indicates a failed codepoint decode upstream; remove or log and drop.
- **Private Use Area codepoints** (U+E000–U+F8FF, U+F0000–U+FFFFF, U+100000–U+10FFFF) that were not resolved by the glyph recovery layer.

These characters are artifacts of the extraction process, not content. A downstream consumer encountering a NULL byte or PUA codepoint in extracted text has no correct interpretation for it.

---

## 9. Number and Digit Form Normalization

Unicode encodes digit sequences for multiple scripts:

- **Arabic-Indic**: U+0660–U+0669 (`٠١٢٣٤٥٦٧٨٩`)
- **Extended Arabic-Indic**: U+06F0–U+06F9
- **Devanagari**: U+0966–U+096F (`०१२३४५६७८९`)
- **Thai**: U+0E50–U+0E59 (`๐๑๒๓๔๕๖๗๘`)

Normalizing these to ASCII digits (U+0030–U+0039) aids downstream numeric parsing but destroys information in multilingual documents. This normalization must be opt-in. The default should preserve the original digit forms.

Date normalization (parsing and re-emitting dates in a canonical format) is out of scope for the extraction layer and belongs in a higher-level application.

---

## 10. Pipeline Ordering

The normalization steps must execute in the following order to avoid interactions:

1. **Ligature expansion** — before Unicode normalization, so that NFKC (if applied) does not need to handle ligatures separately; expansion maps are simpler than NF decompositions.
2. **Unicode normalization (NFC)** — after ligature expansion but before any string comparison operations; ensures that precomposed characters from different code paths produce identical byte sequences.
3. **Control character and artifact removal** — after NFC so that NFC does not accidentally compose an artifact codepoint with a preceding base character.
4. **Whitespace collapse** — after artifact removal, which may produce adjacent spaces when stripped codepoints had surrounding whitespace.
5. **Hyphen joining / line reconstruction** — requires clean whitespace and consistent codepoints to correctly detect end-of-line positions and perform dictionary lookups.
6. **Paragraph reconstruction** — after line joining; requires final line boundaries to be determined.
7. **Header and footer removal** — after paragraph reconstruction, so that block boundaries are stable before cross-page comparison.
8. **Quote and dash normalization (optional)** — last, so it operates on coherent paragraph text rather than on fragments that might contain split quotation contexts.

Order matters concretely: applying whitespace collapse before hyphen joining can destroy the space that should separate words after an erroneous join. Applying Unicode normalization after quote normalization can alter the bytes used for smart quotes if the normalization form affects the Letterlike Symbols block.

The pipeline should be implemented as a sequence of `fn normalize(input: &str, config: &NormalizationConfig) -> String` transforms chained via iterator adapters, with `NormalizationConfig` carrying all opt-in flags (`ligature_mode`, `nfkc`, `quote_mode`, `dash_mode`, `digit_normalization`, `no_break_space_handling`). Each step is independently testable and the chain is short-circuit capable if a step is disabled.
