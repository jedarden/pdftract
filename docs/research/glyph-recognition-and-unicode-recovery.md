# Glyph Recognition and Unicode Recovery in PDF Text Extraction

## Overview

PDF text extraction depends on the font's encoding machinery to map raw glyph identifiers — character codes in a content stream — to Unicode codepoints. When that machinery is absent, broken, or intentionally obscured, a robust extractor must fall back through a layered series of heuristics. This document surveys the failure modes and recovery strategies a Rust engineer needs to understand when building `pdftract`.

---

## 1. Why CMaps Fail

A ToUnicode CMap is an optional but critical PDF object that maps each glyph's character code to one or more Unicode codepoints. Its absence or incorrectness is a frequent source of garbled extraction output.

**Custom encoding without ToUnicode.** Type 1 and TrueType fonts embedded in PDF can use a custom Encoding dictionary that remaps character codes arbitrarily. If no ToUnicode CMap is present, the only remaining signal is the glyph name — and only if the author did not rename glyphs. Many print-production workflows strip ToUnicode entries during PDF/X conversion to reduce file size.

**Type 3 fonts with arbitrary glyph procedures.** A Type 3 font defines each glyph as a sequence of PDF content stream operators. There is no standardized shape; the glyph procedure could draw anything, including decorative symbols, logos, or redacted characters. The font's Encoding maps codes to glyph names, but those names are arbitrary strings chosen by the document author.

**Scanned PDFs with fake text layers.** OCR pipelines sometimes embed a hidden Type 3 or Type 1 font whose glyphs are designed to be invisible at normal rendering, purely to carry searchable text. The ToUnicode CMap may be correct but carry OCR errors, or may be present only for a subset of characters. In pathological cases the text layer and visual content are deliberately misaligned (common in forms with print-and-sign workflows).

**Symbol fonts repurposed for body text.** ZapfDingbats, Symbol, and similar fonts have standard glyph shapes that encode mathematical or decorative characters. Documents that route body-text characters through these fonts — especially via PDF/A compliance workarounds or legacy WordPerfect exports — will produce garbled output when a consumer naively interprets character codes as Latin.

**Intentionally obfuscated PDFs (DRM).** Some DRM schemes replace ToUnicode CMaps with shuffled or encrypted equivalents. The content stream references glyph codes whose ToUnicode entries map to decoy codepoints, while the real text requires a key or rendering to recover. Detecting this is an open problem; the best practical heuristic is low-confidence scoring on known-word frequency after extraction.

**Authoring tool bugs.** Adobe InDesign, Microsoft Word, and LibreOffice all have historically shipped versions that generated incorrect ToUnicode CMaps — most commonly for ligatures (fi, fl, ff), for characters outside Basic Latin, and for fonts using expert-set or OldStyle figure variants. The ToUnicode entry may be structurally valid (parseable) but semantically wrong, mapping the fi ligature to U+0066 U+0069 in one range definition and to U+FB01 in another, with the wrong range selected at runtime.

---

## 2. Glyph Name Heuristics

When ToUnicode is absent, the font's Encoding dictionary may still provide glyph names — strings like `A`, `comma`, `fi`, `uni0041`, `u1D400`. The Adobe Glyph List (AGL) 2.0 and its companion specification define an algorithm to extract Unicode codepoints from these names.

**The AGL algorithm (abbreviated):**

1. If the name is in the AGL table (a ~4000-entry mapping from name to codepoint), return the mapped codepoint.
2. If the name is of the form `uniXXXX` (exactly four uppercase hex digits), return U+XXXX. Multiple consecutive `uniXXXX` segments encode a sequence (ligatures or decomposed characters).
3. If the name is of the form `uXXXXXX` (four to six uppercase hex digits), return U+XXXXXX, provided the codepoint is in a valid Unicode range (not a surrogate, not above U+10FFFF).
4. If the name contains a period (`.`), strip the suffix and reapply the algorithm to the base name. The suffix is a variant tag and carries no Unicode meaning.
5. Otherwise, the name is unrecognized; return REPLACEMENT CHARACTER or signal failure.

The full AGL table is published by Adobe at `https://github.com/adobe-type-tools/agl-aglfn`. The `aglfn` variant (Adobe Glyph List for New Fonts) is the normative source for production use — it includes only names that unambiguously map to a single codepoint. The broader AGL includes legacy names with complex decompositions.

**ZapfDingbats and Symbol.** These fonts are explicitly carved out of the AGL algorithm. The PDF specification (ISO 32000-2, section 9.10.2) mandates a separate glyph-name-to-Unicode mapping for each. Symbol uses an encoding close to ISO Latin-1 for printable ASCII, then maps higher bytes to Greek letters and mathematical operators via a font-specific table. ZapfDingbats maps character codes 33–254 to a defined set of Unicode dingbat and geometric shape codepoints. Both tables are small (< 300 entries) and should be hardcoded; attempting to apply AGL to them produces wrong results.

---

## 3. Font Fingerprinting Approaches

When glyph names are absent or unhelpful, characteristics of the font itself may identify it.

**FontDescriptor metrics.** Every embedded font should include a FontDescriptor dictionary with numeric metrics: `Ascent`, `Descent`, `CapHeight`, `XHeight`, `StemV`, `StemH`, `ItalicAngle`, and a `FontBBox` rectangle. These values are not unique enough alone, but they prune the candidate space significantly. A font with CapHeight 716 and XHeight 523 in a 1000-unit em square is almost certainly Times New Roman Regular or a metric-equivalent clone. Combining four or five metrics gives a coarse but useful fingerprint.

**Checksum and hash matching.** Embedded TrueType and OpenType fonts contain a `checkSumAdjustment` field in the `head` table. More reliably, the raw bytes of the `cmap`, `glyf`, or `CFF ` table can be hashed (SHA-256) and looked up in a pre-built database of known fonts. This is the most precise fingerprinting strategy; the challenge is building and maintaining the database. Google Fonts, Adobe Fonts, and the web safe fonts cover the majority of PDFs encountered in practice.

**PostScript name matching.** The `FontName` in the FontDescriptor and `BaseFont` in the font dictionary are PostScript names (e.g., `TimesNewRomanPSMT`, `ArialMT`, `HelveticaNeue-Bold`). These frequently identify the font family and style without metric lookup. Normalize by stripping common suffixes (`-Regular`, `-MT`, `PS`, `LT`), folding to lowercase, and removing whitespace before matching against a known-font table. False positives are common (many fonts claim to be "Helvetica"), so use name matching only to select a candidate, then confirm with metrics.

---

## 4. Glyph Outline Analysis

If a font is embedded with full outline data, glyph shapes can serve as fingerprints against Unicode character databases, without full raster OCR.

**Type 1 charstrings.** A Type 1 charstring encodes a glyph's Bezier outline as a compact stack-based bytecode. Parsing charstrings yields a sequence of moveto/lineto/curveto operations. Normalize the resulting path: translate to origin, scale to unit square, and compute a fixed-size feature vector (e.g., a grid of orientation histograms, or moment invariants). Compare against pre-computed vectors for every Unicode character in candidate fonts.

**TrueType glyph programs.** TrueType stores outlines in the `glyf` table as contour sequences with on-curve and off-curve control points. The same normalization-and-comparison approach applies. One practical simplification: rasterize the normalized outline to a small bitmap (e.g., 32×32 grayscale) and compute a perceptual hash (pHash or dHash). This loses some precision but is fast and storage-efficient for the reference database.

**Approximate shape matching tradeoffs.** Vector-based outline matching is accurate for clean outlines but degrades with variation in design weight, optical size, or deliberate distortion. It cannot handle Type 3 fonts where the glyph procedure uses fill rules or clip paths that the Bezier extraction misses. Full raster OCR (e.g., Tesseract on a rasterized glyph image) is more robust but orders of magnitude slower and introduces an external binary dependency. The recommended middle ground is outline matching as a fast first pass, falling back to OCR only for glyphs where outline matching confidence is below a threshold.

---

## 5. Context-Based Recovery

When a document is mostly well-decoded, poorly decoded characters can be inferred from context.

**Statistical character prediction.** Character n-gram models trained on text corpora assign probabilities to candidate codepoints given surrounding decoded characters. For a position where extraction fails, score each candidate against the n-gram model. This is most useful for single-glyph substitutions in otherwise Latin text (e.g., a missing `e` in English).

**Dictionary-based gap filling.** If a word contains one or two unknown characters and the surrounding characters form a near-match to a dictionary entry, the dictionary entry is a candidate. Restrict to the same script as the surrounding characters. Edit distance (Levenshtein with wildcards for unknown positions) is the standard metric. This works well for ligatures: an unknown glyph between `o` and `e` in an English word is almost certainly `ff` or `fi`.

**Language model scoring.** A word-level or subword language model can rescore candidates from the above methods. For `pdftract`, integrating a full LM is heavy; a practical approximation is a ranked word-list with bigram statistics. The Norvig frequency list or Zipf-weighted lists from Wikipedia work well for English; CLDR/BabelNet equivalents exist for other scripts.

---

## 6. Practical Recovery Pipeline

The recommended priority order for `pdftract` is:

### Step 1: ToUnicode CMap
Parse the ToUnicode stream, validate that it is a well-formed CMap (check `begincmap`/`endcmap`, `beginbfchar`/`endbfchar`, `beginbfrange`/`endbfrange` blocks). Apply the mapping. Flag any character codes that fall outside the mapped ranges as unresolved. If the CMap maps a code to U+FFFD or U+0000, treat those mappings as missing rather than authoritative.

### Step 2: Glyph Name via AGL
For each unresolved code, retrieve its glyph name from the font's Encoding dictionary. Apply the AGL algorithm in order: direct AGL table lookup, `uniXXXX` expansion, `uXXXXXX` expansion, period-stripped base name retry. Apply the ZapfDingbats or Symbol override table if the font is identified by name as one of those two. Assign the resulting codepoint with high confidence.

### Step 3: Font Name Fingerprinting
For glyphs still unresolved, normalize the `BaseFont` / `FontName` strings and look up in a known-font database. If matched, use the font's standard encoding for the matched font (e.g., look up the character code in the font's standard cmap). Validate against FontDescriptor metrics if present. If the font is a known metric-equivalent, retrieve its standard glyph-to-Unicode mapping. Assign the result with medium confidence and tag for downstream review.

### Step 4: Outline Shape Matching
For glyphs where steps 1–3 failed or produced low-confidence results, extract the glyph outline from the font program (Type 1 charstring parser or TrueType `glyf` reader). Normalize and compute the shape fingerprint. Query a pre-built reference database of Unicode character outlines. Return the top-k candidates with similarity scores. Select the highest-scoring candidate above a threshold (empirically ~0.85 cosine similarity on moment-invariant vectors). Below the threshold, mark as unresolved and defer to step 5.

### Step 5: OCR Fallback
As a last resort, rasterize the unresolved glyph at a sufficient resolution (>= 150 DPI equivalent on the normalized em square, typically 32–64px) and pass it to a character-level OCR recognizer. Tesseract's single-character mode or a custom CNN trained on Unicode character images are both viable. OCR introduces latency and an external dependency, so it should be gated on a configuration flag and applied only when no other step has produced a confident result.

**Cross-step confidence aggregation.** Assign each step a base confidence tier (Step 1: 0.95, Step 2: 0.90, Step 3: 0.70, Step 4: 0.60–0.90, Step 5: 0.50–0.85). After the pipeline, apply context-based rescoring (Section 5) to candidates below 0.80 confidence, using the surrounding high-confidence characters as context. Expose the final confidence score and the recovery step taken as metadata on each extracted character, so callers can choose to suppress or highlight uncertain output.

---

## References

- ISO 32000-2:2020 (PDF 2.0), Section 9 (Text) and Annex D (Character Sets and Encodings)
- Adobe Glyph List Specification, version 1.7 — `adobe-type-tools/agl-specification`
- Adobe Glyph List for New Fonts (aglfn) — `adobe-type-tools/agl-aglfn`
- Adobe Type 1 Font Format specification (Black Book), Chapter 6 (Charstrings)
- Apple TrueType Reference Manual — `glyf` table specification
- OpenType Specification 1.9, Microsoft Typography (CFF / CFF2 charstring formats)
- Unicode Standard Annex #29 (Unicode Text Segmentation) — relevant for ligature decomposition
