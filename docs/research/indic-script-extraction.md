# Indic Script PDF Extraction: Devanagari, Tamil, Telugu, Bengali, and Related Scripts

## 1. Indic Script Complexity

Indic scripts — Devanagari, Tamil, Telugu, Bengali, Kannada, Malayalam, Gujarati, and Gurmukhi — are abugidas, a writing system family distinct from alphabets and syllabaries. In an abugida, each base character represents a consonant with an implicit inherent vowel (typically "a" in most Indic scripts). Explicit vowel sounds are written as combining marks called vowel signs, or matras, that attach to the base consonant.

This structure creates the first layer of extraction difficulty: a single visual unit on the page may correspond to multiple Unicode code points, and the visual arrangement of those code points does not follow a simple left-to-right spatial order. Matras can appear to the left, right, above, or below the base consonant, and some vowels are split, appearing on both sides simultaneously (e.g., certain Malayalam and Tamil vowels). A naïve extraction that maps glyph positions directly to reading order will produce character sequences in the wrong order.

Conjunct consonants compound the problem significantly. When two or more consonants appear in sequence without an intervening vowel, the shaping engine merges them into a ligated or stacked conjunct form. A conjunct may be rendered as a single glyph by the font or as a tightly composed cluster of sub-glyphs. The Unicode logical encoding of a conjunct is always the sequence of base consonant code points separated by the virama (halant), but the visual glyph or glyph cluster has no visible halant. Extracting such sequences correctly requires knowledge of which glyph or glyph combination corresponds to which phoneme sequence.

Compared to Latin scripts, where glyph-to-character correspondence is nearly one-to-one, Indic extraction is fundamentally a shaping reversal problem. Compared to CJK, where large glyph inventories are used but each glyph maps to a single Unicode scalar, Indic presents many-to-one and one-to-many mappings at the glyph level.

## 2. Unicode Encoding of Indic Scripts

The Unicode Standard assigns dedicated blocks to each major Indic script: Devanagari (U+0900–U+097F), Bengali (U+0980–U+09FF), Gurmukhi (U+0A00–U+0A7F), Gujarati (U+0A80–U+0AFF), Tamil (U+0B80–U+0BFF), Telugu (U+0C00–U+0C7F), Kannada (U+0C80–U+0CFF), and Malayalam (U+0D00–U+0D7F).

A critical property of Unicode Indic encoding is that the logical order is always phonemic order, not visual order. The Unicode code point sequence for a syllable is: base consonant(s) joined by viramas, followed by any vowel sign, followed by any anusvara or visarga. This ordering is independent of where the matra visually appears on the page. The "i" vowel sign in Devanagari (U+093F) appears to the left of its base consonant visually, but in Unicode it follows the consonant. The Tamil "i" vowel (U+0BBF) behaves identically.

pdftract must produce output that conforms to this phonemic ordering contract. Any extraction path that assembles text by reading glyph positions from left to right on the page will violate this contract for pre-base matras and split vowels.

## 3. PDF Generator Behavior for Indic Text

When a PDF is generated from an authoring application with proper Indic language support — such as InDesign with an Indic language pack, Word with Devanagari or Tamil fonts, or a properly configured web-to-PDF renderer — the application invokes an OpenType shaping engine (typically HarfBuzz or the platform's native shaper) before embedding text into the PDF.

The shaping engine applies GSUB (glyph substitution) and GPOS (glyph positioning) rules from the font's OpenType tables to transform the logical Unicode sequence into a visual glyph sequence suitable for rendering. The output of shaping is a sequence of glyph IDs in the font's internal numbering space. These glyph IDs, not Unicode code points, are what the PDF content stream encodes in its text-showing operators (Tj, TJ, etc.).

To allow text extraction, the PDF should include a ToUnicode CMap for each embedded font. This CMap maps glyph IDs back to Unicode character sequences. The failure modes are numerous and common:

- The ToUnicode CMap may be absent entirely. Applications that treat Indic text as purely graphical, or that use embedded bitmaps, produce PDFs with no extraction path.
- The CMap maps each glyph to a single Unicode scalar, but a conjunct glyph represents a multi-codepoint sequence. The extractor then produces a single codepoint where a sequence is required.
- The CMap maps shaped glyphs to the pre-shaping Unicode sequence but records only the first consonant of a conjunct, silently discarding the virama and subsequent consonants.
- Multiple glyphs collectively represent one phoneme sequence (e.g., a split vowel rendered as two separate glyphs for the left and right halves), but each glyph's CMap entry maps to a partial or empty sequence, producing duplicated or missing vowels.
- The CMap entries are in visual order rather than phonemic order, reflecting the order glyphs appear on the rendered line rather than the logical Unicode order.

## 4. ToUnicode CMap Problems with Indic Scripts

The PDF specification allows ToUnicode CMap entries to map a single glyph to a sequence of Unicode code points using the `bfrange` and `bfchar` syntax with bracket notation. This supports the one-to-many case needed for conjuncts. However, the many-to-one case — where a cluster of glyphs collectively encodes one phoneme sequence — is not directly representable; the CMap format is per-glyph.

In practice, many PDF generators produce ToUnicode CMaps using tools that were designed for Latin or CJK scripts. These tools enumerate shaped glyph sequences from the font's cmap table, which maps Unicode code points to glyph IDs, not the reverse. The reversal is often naively implemented: for each Unicode code point in the font's cmap, record a CMap entry from the default glyph for that code point to the code point. Shaped glyphs (conjuncts, half-forms, vowel sign glyphs in alternate positions) are not reachable from the Unicode cmap and receive no ToUnicode entry.

pdftract must detect the incomplete-CMap condition for Indic fonts. Indicators include: a font with a large glyph count but ToUnicode entries covering only the base consonant range, ToUnicode entries that produce only base consonants with no vowel sign coverage, or extracted text that contains no virama characters despite the document clearly containing conjunct-heavy text (as detectable from glyph count per word-spacing unit).

## 5. ActualText as the Reliable Extraction Path

For PDFs produced with proper accessibility tagging — typically InDesign with tagged PDF export, or XSL-FO renderers with accessibility output — the correct extraction path is ActualText. The PDF specification allows content to carry an ActualText attribute on marked content sequences (via marked content points with `Span` tags) or on structure elements in the document's logical structure tree.

ActualText provides the Unicode string that the marked content visually represents, encoded by the document author or by the application's accessibility layer, independent of glyph encoding. For Indic text, a properly set ActualText value gives the phonemically ordered Unicode sequence for the entire sequence of shaped glyphs — including conjuncts, half-forms, and reordered matras — as a single string.

pdftract's extraction pipeline should check for ActualText on every span of content before attempting glyph-level CMap lookup. If ActualText is present and non-empty, it should be used directly as the canonical text for that content unit. The glyph-level extraction pipeline is then a fallback for untagged PDFs.

## 6. OpenType GSUB Lookup for Shaping Reversal

When neither ActualText nor a complete ToUnicode CMap is available, pdftract can attempt to reverse-engineer the glyph-to-Unicode mapping using the font's embedded OpenType GSUB tables.

The Indic shaping specification defines a mandatory sequence of GSUB feature lookups applied in order. For most Indic scripts (using the USE — Universal Shaping Engine — or the older Indic v2 specification), the ordered feature sequence includes: `akhn` (akhand, pre-shaping conjuncts), `rphf` (reph form), `blwf` (below-base form), `half` (half form), `pstf` (post-base form), `vatu` (vattu variant), `cjct` (conjunct form), followed by presentational features `pres`, `abvs`, `blws`, `psts`, and `haln`. Each GSUB lookup is a substitution table mapping input glyph sequences to output glyph sequences or single glyphs.

To reverse a GSUB substitution, pdftract can build an inverted lookup index: for each single-substitution or ligature-substitution table, record the mapping from output glyph ID to input glyph ID sequence. Given a shaped glyph from the PDF content stream, the inverted index can recover the pre-shaping glyph sequence, which can then be mapped to Unicode via the font's standard cmap table. Multi-step shaping chains require following the inversion through multiple lookup levels.

This approach is computationally feasible for single-substitution and ligature-substitution (LookupType 1 and 4) tables. Contextual substitutions (LookupType 5, 6) are harder to invert and may require heuristics or exhaustive candidate search.

## 7. Font-Specific Glyph Naming for Indic Fonts

Older Indic fonts — particularly those designed for legacy encoding systems before Unicode became prevalent — may use custom PostScript glyph naming conventions. Common patterns include glyph names like `ka`, `kha`, `ga`, encoding phoneme names directly; `uni0915`, `uni0916`, encoding Unicode code points in the standard `uni` prefix format; or purely numeric names like `glyph0032`.

The Adobe Glyph List (AGL) covers Latin, Greek, Cyrillic, and some other scripts but does not include Indic phoneme names. The `uni` prefix convention is defined in the AGL specification and maps directly to Unicode code points, making it reliable when present. Numeric glyph names convey no semantic information.

When pdftract encounters unrecognized glyph names in an Indic font with no ToUnicode CMap, it applies the following fallback order: parse `uni`-prefixed names to extract Unicode scalars; attempt lookup in a supplementary Indic glyph name table derived from common font naming conventions; flag glyphs with purely numeric or unrecognized names as unresolvable, emitting placeholder markers in the extraction output rather than silently dropping content or substituting incorrect characters.

## 8. Vowel Sign Reordering in Extraction

Pre-base vowel signs present a specific reordering requirement that must be handled explicitly. In Tamil, the short "i" vowel (U+0BBF), "ii" vowel (U+0BC0), and several others visually appear to the left of their base consonant. In Devanagari, the "i" vowel (U+093F) appears similarly. The shaping engine places these glyphs to the left during layout, but Unicode requires them to follow the base consonant in the code point sequence.

A PDF content stream that records glyphs in visual left-to-right order will therefore encode a pre-base matra before its consonant, violating Unicode phonemic order. pdftract must detect this condition and reorder: after assembling a candidate character sequence from CMap lookup, identify any vowel sign code points that fall in known pre-base ranges for their respective script, and relocate them to their correct position after the base consonant.

Split vowels — those rendered as two separate glyphs on either side of the consonant — require reassembly: the left glyph and the right glyph must be recognized as halves of a single vowel sign and merged into the correct single Unicode code point (or two-code-point canonical sequence where applicable).

## 9. Halant (Virama) Handling

The virama (halant) is the diacritic that suppresses the inherent vowel of a consonant. In Devanagari it is U+094D; each other Indic script has its own corresponding code point. In properly encoded Indic text, the virama appears between consonants of a conjunct cluster, and also appears at the end of a word-final consonant when that consonant has no inherent vowel.

Several failure modes are common in PDF extraction of viramas. When a conjunct is represented as a ligature glyph, the ToUnicode entry should include the virama between the constituent consonants (e.g., `<0915 094D 0916>` for the ka-halant-kha conjunct), but many generators omit the virama, producing `<0915 0916>` instead. This sequence is phonemically distinct — it implies a vowel between the consonants — and will render differently in word processors, cause incorrect spell-checking, and produce wrong search results.

Conversely, a virama that is encoded in the ToUnicode but placed at the wrong position in the output sequence (e.g., before the first consonant rather than between them) breaks downstream Indic text processing entirely. pdftract's GSUB reversal logic must validate virama placement against the logical structure of each conjunct it decodes.

## 10. OCR Fallback for Indic Scripts

When ToUnicode is absent, ActualText is not present, and GSUB reversal fails to produce a coherent Unicode sequence (as indicated by high rates of unresolvable glyph names, invalid code point sequences, or detection of a purely graphical rendering pipeline), pdftract falls back to OCR using Tesseract with script-specific language models.

Tesseract provides dedicated trained models for major Indic scripts: `deva` for Devanagari, `tam` for Tamil, `tel` for Telugu, `ben` for Bengali, `kan` for Kannada, `mal` for Malayalam, `guj` for Gujarati, and `pan` for Gurmukhi. These models produce Unicode output in logical (phonemic) order directly, as Tesseract's Indic pipeline includes its own shaping-aware recognition layer.

pdftract must render the affected PDF page region to a raster image at sufficient resolution (minimum 300 DPI, preferably 400 DPI for small body text) before passing it to Tesseract. The script of the content must be identified to select the correct model; this can be done heuristically from any glyph name information or from the font name embedded in the PDF. All text extracted via the OCR path is annotated in pdftract's output with a `source: ocr` field and a per-word or per-line confidence score as reported by Tesseract's HOCR output, so consumers can distinguish OCR-derived text from CMap-derived or ActualText-derived text and apply appropriate downstream validation.
