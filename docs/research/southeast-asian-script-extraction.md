# Southeast Asian Script PDF Extraction

## Thai, Khmer, Myanmar, Lao, Tibetan, and Ethiopic

---

## Overview

Extracting text from PDFs that contain Southeast Asian scripts requires a fundamentally different approach than Latin or CJK extraction. These scripts share common traits: complex cluster structures, combining characters that modify base consonants, and encoding histories that predate Unicode standardization. pdftract must handle each script's cluster mechanics correctly at the glyph-to-codepoint mapping stage, and it must resist the temptation to impose word boundaries that are meaningless at the extraction layer.

---

## Thai Script

Thai occupies Unicode block U+0E00–U+0EFF. Its character model is an abugida where each syllable cluster is built from a base consonant, optional dependent vowel signs (which may appear before, above, or below the consonant in visual order), and optional tone marks. All of these combining elements are encoded in logical order in Unicode, but they do not appear in left-to-right visual sequence — a vowel sign written to the left of its consonant is still encoded after it.

The defining extraction challenge for Thai is that written Thai has no spaces between words. Word boundaries are semantic, determined by the mental lexicon of the reader, not by any orthographic marker in the character stream. This means that a PDF containing Thai text will have glyph sequences with no inter-word spacing signals visible at the PDF content stream level, and the PDF's text positioning commands will only reflect intra-cluster spacing or inter-sentence spacing at best.

pdftract must not attempt to inject word boundaries during extraction. Word-boundary injection at the glyph level requires a full lexical analyzer operating over the Thai Unicode range, and such an analyzer operating on raw PDF glyph sequences will produce systematic errors — particularly around multi-syllable words, compound words, and proper nouns. The correct approach is to extract the full cluster sequence in Unicode logical order and emit it as a contiguous string. Downstream consumers — ICU's BreakIterator with the `th` locale, the libthai library, or the Thai Character Cluster (TCC) rules implemented in various NLP toolkits — are the appropriate place to perform word segmentation. pdftract's responsibility is to produce a correct, ordered Unicode sequence; segmentation is outside its scope.

Within each cluster, the extraction order must follow Unicode logical order: base consonant first, then any vowel signs (regardless of their visual position), then tone mark. If the PDF encodes these elements through a ToUnicode CMap, the CMap should already express them in this order. When the ToUnicode CMap is absent or incomplete, glyph name lookup must fall back to a Thai-aware heuristic or OCR rather than positional inference.

Tonal marks (U+0E48 through U+0E4B) are combining characters and must be preserved. They carry phonemic meaning — omitting them produces a different word or an unrecognizable sequence.

---

## Khmer Script

Khmer occupies U+1780–U+17FF. It is an abugida with independent vowels, dependent vowels, and a subscript consonant mechanism that significantly complicates extraction. Subscript consonants (coeng forms) are encoded using the coeng character U+17D2 followed by the subscript consonant codepoint. This sequence signals that the second consonant should be rendered below the base, forming a stacked cluster. The encoding is explicit: U+17D2 acts as a joiner, and its presence in the extracted stream is required for correct downstream text processing.

Khmer PDF extraction must preserve the full coeng+consonant sequences. A naive implementation that strips or misidentifies U+17D2 will produce malformed Khmer text that is neither visually nor semantically correct. As with Thai, Khmer has no inter-word spaces, so pdftract should extract contiguous cluster sequences without injecting boundaries.

The Khmer Unicode block includes independent vowels (U+17A3–U+17B3), which stand alone as syllable nuclei and must not be confused with dependent vowels. The extraction logic must distinguish these by codepoint range when building cluster sequences from glyph runs.

---

## Myanmar/Burmese Script

Myanmar script occupies U+1000–U+109F for the core block, with extended ranges at U+A9E0–U+A9FF (Myanmar Extended-B) covering minority scripts written in Myanmar-derived letterforms. Stacked consonants use the asat character (U+103A, the killer mark) in combination, and vowel signs appear above, below, and to the sides of the base consonant. The encoding mechanics are similar in complexity to Khmer.

Myanmar presents an additional extraction hazard unique among the scripts covered here: legacy Zawgyi encoding. Zawgyi is a proprietary 8-bit font encoding developed before Myanmar was well-supported in Unicode. Zawgyi maps Myanmar glyphs to the Unicode Myanmar block codepoints but with a completely different assignment — a character that is U+1060 in proper Unicode encoding is a different glyph under Zawgyi. PDFs produced with Zawgyi-encoded fonts will have ToUnicode CMaps that yield Zawgyi codepoints in the Myanmar range, which are semantically incorrect as Unicode.

pdftract needs a Zawgyi detection heuristic. A reliable signal is the presence of codepoints in ranges that are legally defined in Unicode Myanmar but are used differently by Zawgyi — specifically, Zawgyi overloads U+1060–U+1099 with glyph forms that duplicate the lower Myanmar range. A frequency analysis of the extracted Myanmar codepoints, combined with a check for characteristic Zawgyi-only sequences, can identify Zawgyi-encoded output. When Zawgyi is detected, the extracted string should be passed through a Zawgyi-to-Unicode converter (such as the algorithm documented in the Rabbit or Parabaik libraries) before being emitted. This conversion must happen at the string level, not the glyph level.

For Myanmar PDFs that use neither Unicode nor Zawgyi but instead use arbitrary 8-bit glyph mappings without ToUnicode CMaps, OCR via Tesseract with the `mya` language model is the reliable fallback.

---

## Lao Script

Lao occupies U+0E80–U+0EFF and is closely related to Thai in both its visual structure and its encoding model. Cluster structure follows the same consonant-plus-vowel-plus-tone pattern, vowel signs appear in all four positions relative to the consonant, and there are no spaces between words. The extraction strategy for Lao mirrors Thai exactly: extract complete clusters in Unicode logical order and emit them without injecting word boundaries. Downstream tools handle Lao word segmentation.

Lao is less commonly encountered in PDFs than Thai, and legacy font coverage is thinner. The probability of encountering Lao PDFs with missing or incorrect ToUnicode CMaps is higher than for Thai, making the OCR fallback path via Tesseract's `lao` model more frequently necessary in practice.

---

## Tibetan Script

Tibetan occupies U+0F00–U+0FFF. Its segmentation unit is the syllable, not the word in the Latin sense, and syllables are separated by the tsek mark (U+0F0B), a small dot-like character that appears after each syllable. This makes Tibetan extractable with clear segment boundaries, which is a significant advantage over Thai and Lao.

The internal structure of a Tibetan syllable involves stacked consonants encoded with subjoined consonant forms (U+0F90–U+0FAD). These subjoiners follow their base consonant in Unicode logical order. The stacking is a shaping instruction to the renderer, not a semantic reordering, so the extraction must preserve the subjoiner codepoints to maintain correct Unicode text.

pdftract should treat the tsek as a legitimate segment separator. When emitting Tibetan text, tsek characters must be preserved in the output stream — they are not punctuation to be stripped but the primary unit delimiter in the script. Applications that process Tibetan text will use the tsek to segment syllables in the same way that applications processing Thai use ICU word segmentation.

Tibetan PDFs vary significantly in quality. Modern Tibetan PDFs produced with Unicode fonts and correct ToUnicode CMaps extract cleanly. Older PDFs, particularly those produced with Tibetan fonts developed for pre-Unicode systems, may require OCR.

---

## Ethiopic (Amharic and Related Languages)

Ethiopic occupies U+1200–U+137F, with extensions at U+1380–U+139F and U+2D80–U+2DDF. It is a syllabic script where each character encodes a consonant-vowel pair. There are no combining characters for core Ethiopic — each fidel (syllable glyph) is a single codepoint, making extraction substantially simpler than the abugida scripts above when a correct ToUnicode CMap is present.

Word separation in Ethiopic uses the Ethiopic word separator (U+1361, the Ethiopian full stop is U+1362). Many Ethiopic texts also use the standard ASCII space for inter-word separation, particularly in documents produced with modern word processors. pdftract should handle both.

When a ToUnicode CMap correctly maps Ethiopic glyphs, extraction reduces to straightforward codepoint emission. The primary failure mode is legacy Ethiopic fonts that predate Unicode adoption in Ethiopia — these use 8-bit encodings that map to Latin codepoints or to private-use Ethiopic encodings, and they produce garbage on standard Unicode extraction paths. Tesseract's `amh` model handles Amharic OCR, and `tir` covers Tigrinya, providing fallback coverage for the major Ethiopic-script languages.

---

## Font Encoding Failures and OCR Fallback

A common thread across all Southeast Asian and Ethiopic scripts is the prevalence of legacy PDFs that predate Unicode standardization for these scripts. Such PDFs use custom 8-bit encodings where glyph names are local identifiers (e.g., `uni_a1`, `glyph0042`) with no semantic mapping to Unicode codepoints. ToUnicode CMaps are absent, and the standard glyph name lookup tables used for Latin and Greek extraction yield nothing useful for these ranges.

For these PDFs, pdftract must detect the encoding failure — signaled by an empty or incomplete ToUnicode map combined with glyph names outside the AGL (Adobe Glyph List) — and escalate to OCR. Tesseract provides language models for the primary Southeast Asian scripts: `tha` for Thai, `khm` for Khmer, `mya` for Myanmar, `lao` for Lao. These models are part of the standard `tessdata` distribution, with Thai being the most mature and Khmer and Lao being functional but less extensively trained. Tibetan and Ethiopic have Tesseract models (`bod` and `amh` respectively) with varying coverage.

When OCR is invoked, pdftract should render the affected page region at sufficient DPI (300 minimum, 400 preferred for small text) before passing it to Tesseract with the appropriate language tag. Tesseract outputs Unicode in logical order for all of these scripts, which means the resulting text is correct for downstream processing without additional reordering.

---

## OpenType Shaping Considerations

Modern PDFs that use OpenType fonts with Universal Shaping Engine (USE) support present a different challenge. USE handles Myanmar, Khmer, and several other Southeast Asian scripts by applying GSUB lookup chains that transform Unicode codepoint sequences into glyph sequences for rendering. The glyph order in the PDF content stream may differ from the Unicode logical order because USE reorders input sequences during shaping.

A correctly constructed PDF will include a ToUnicode CMap that maps each output glyph back to its original Unicode codepoint sequence, reversing the GSUB transformations. pdftract should rely on this CMap rather than attempting to reverse the GSUB substitutions algorithmically — GSUB reversal is script-specific and highly sensitive to font-specific lookup ordering.

When the ToUnicode CMap is present and complete, USE-shaped PDFs extract cleanly through the standard CMap lookup path. The shaping complexity is opaque to the extraction layer. When the CMap is absent for a USE-shaped font — which occurs in some programmatically generated PDFs — the glyph sequence cannot be reliably reversed to Unicode without script-specific GSUB analysis, and OCR is the correct fallback.

---

## Summary of pdftract Requirements

For each script, the extraction pipeline must: apply ToUnicode CMap lookup as the primary path; detect encoding failures from absent or semantically empty CMaps; invoke Tesseract with the appropriate language model as the fallback; preserve all combining characters and cluster joiners in the output (coeng for Khmer, subjoined consonants for Tibetan); detect Zawgyi encoding for Myanmar and convert to Unicode before output; and emit cluster sequences for Thai and Lao without injecting word boundaries. Word segmentation for Thai and Lao is explicitly out of scope for pdftract and belongs to the consumer application layer.
