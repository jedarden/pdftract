# Text Readability Validation

## Overview

Extracting bytes from a PDF font stream and producing a sequence of Unicode codepoints is necessary but not sufficient. A PDF can encode every character correctly at the byte level while still emitting text that is semantically unreadable — because the font has no ToUnicode map, because a custom encoding overlaps with a standard encoding at the wrong offset, or because the renderer selected the wrong code path. This document defines the algorithms and data structures that `pdftract` uses to detect and remediate unreadable output before it reaches a caller.

---

## 1. Failure Modes: What "Unreadable" Looks Like

Unreadable extraction output falls into several distinct categories, each with a different root cause and remediation path.

**Mojibake** occurs when bytes are decoded with the wrong code page. The classic form is Latin-1 interpreted as UTF-8 (or vice versa), producing sequences like `Ã©` for `é` or `â€™` for `'`. These are valid Unicode codepoints, but they are wrong ones.

**Replacement characters** (U+FFFD) appear when a decoder encounters byte sequences that are invalid in the target encoding. A high density of U+FFFD is an unambiguous signal of encoding mismatch.

**Private Use Area codepoints** (U+E000–U+F8FF) are legitimately used in some PDF fonts to encode glyphs that have no standard Unicode assignment, but a prose span where more than a small fraction of codepoints are PUA almost certainly reflects a missing or incorrect ToUnicode map.

**Control characters** in the range U+0000–U+001F (excluding U+0009 TAB and U+000A LF) should never appear in prose extracted from a document. Their presence indicates that glyph IDs are being emitted directly without Unicode mapping.

**Symbol font bleed-through** happens when a font that uses Zapf Dingbats, Symbol, or a custom pi font is decoded as if it were a text font. The result is runs of symbols — ♦ ♣ ♥ ♠ — where letters should be.

**Impossible character sequences** for the detected language include strings like `xzqbvw` in English or `aeiouaeiou` in Czech. Natural languages have strong constraints on consonant/vowel alternation and on which n-grams can appear adjacently.

**Mixed-directionality fragments** without a Unicode Bidirectional Algorithm context marker produce visually disordered text when a span mixes Arabic or Hebrew runs with Latin runs and the bidi embedding levels are absent.

**Zero-width characters** — U+200B ZERO WIDTH SPACE, U+200C/D ZWNJ/ZWJ, U+FEFF BOM used mid-stream — should be rare in extracted prose; dense runs of them indicate malformed CMap output.

---

## 2. Character-Level Validity Checks

Character-level checks are the first filter in the validation pipeline. They operate per-span in O(n) time with no external data dependencies.

**U+FFFD density:** compute `replacement_ratio = fffd_count / total_codepoints`. Flag the span as `"garbled"` if `replacement_ratio > 0.10` and `"low"` quality if `replacement_ratio > 0.02`.

**PUA density:** compute `pua_ratio` over U+E000–U+F8FF and Supplementary PUA (U+F0000–U+FFFFF). Flag as `"garbled"` if `pua_ratio > 0.40`. A small PUA ratio (< 0.05) may be acceptable for documents using custom ligature glyphs.

**Control character scan:** iterate codepoints; any U+0000–U+0008, U+000B–U+001F (excluding 0x09, 0x0A) in a prose span is an immediate `"low"` flag and adds `"control_chars"` to `quality_signals`.

**Combining character orphans:** a sequence of combining characters (Unicode category M) not preceded by a base character (category L, N, or P) indicates CMap corruption. Detect runs of three or more consecutive combining characters.

**Anomalous Unicode block concentration:** compute the fraction of codepoints falling in Mathematical Alphanumeric Symbols (U+1D400–U+1D7FF) or Enclosed Alphanumerics (U+2460–U+24FF). Values above 0.15 in a prose context indicate symbol font confusion.

**Unicode category distribution:** for a valid English paragraph, the dominant categories are `Ll` (lowercase letter), `Lu` (uppercase letter), `Nd` (decimal digit), `Po` (other punctuation), and `Zs` (space separator). Compute category histograms and compare against expected priors. A span where `Lo` (other letter — CJK, Arabic, etc.) exceeds 0.60 but the document language is detected as Latin-script warrants a `"medium"` flag.

---

## 3. Word-Level Validity

Word-level checks require tokenizing the span on whitespace and punctuation boundaries, then evaluating each token.

**Bloom filter word list:** maintain a Bloom filter over a ~500,000-word corpus (one per supported language) stored in approximately 3 MB per language at a 0.1% false positive rate using 10 hash functions. The filter supports O(1) probabilistic membership queries. In Rust, the `bloomfilter` crate or a hand-rolled implementation over `xxhash` works well. Load the filter lazily per detected language.

**Real-word ratio:** `real_word_ratio = (dictionary_hits + numeric_tokens) / total_tokens`. Require `real_word_ratio >= 0.60` for `"high"` quality. Values in [0.35, 0.60) map to `"medium"`. Below 0.35, flag `"low"`.

**Consonant/vowel ratio:** for Latin-script languages, compute the ratio of consonant letters to vowel letters in the span. English prose clusters around 1.4–1.8. A ratio above 5.0 or below 0.3 is anomalous. This check catches both garbled encoding and accidental extraction of phoneme tables.

**Character n-gram plausibility:** build a bigram or trigram presence set from a reference corpus (compactly encoded as a sorted array of 16-bit hashes). For each character trigram in the extracted text, check membership. If more than 20% of trigrams are absent from the reference set, add `"ngram_anomaly"` to `quality_signals`. Trigrams like `fsqz`, `bxwk`, or `qzjv` have near-zero frequency in English and their presence is diagnostic.

---

## 4. Entropy-Based Detection

Shannon entropy provides a language-agnostic, O(n) garble detector. Compute character-level entropy over a span as:

```
H = -Σ p(c) * log2(p(c))
```

where `p(c)` is the empirical frequency of codepoint `c` in the span.

Expected entropy ranges:
- English prose: 4.0–5.0 bits/char
- Random Unicode glyphs: 7.0–8.0 bits/char
- Repeated patterns or single-glyph runs: < 1.5 bits/char
- Base64 or hex strings: 5.0–6.0 bits/char

Spans with `H > 6.5` are likely garbled; spans with `H < 1.5` are likely repeated/template noise. Both conditions add `"entropy_anomaly"` to `quality_signals` and reduce quality to at most `"medium"`.

Per-block entropy scoring: divide each page block into 128-codepoint windows and compute entropy per window. A bimodal distribution within a single block (some windows normal, some high entropy) indicates interleaved readable and garbled content, which may call for span-level rather than block-level remediation.

Entropy alone cannot distinguish high-entropy valid content (technical identifiers, URLs, code snippets) from garble. It is a necessary but not sufficient signal; always pair with word-level and n-gram checks.

---

## 5. Language Model Perplexity Scoring

A character-level n-gram language model assigns a probability to each character given its context, enabling perplexity scoring without a word boundary assumption.

**Model choice:** a 4-gram character model trained on language-specific Common Crawl shards. Store log-probabilities in a compact trie or a flat sorted array of (n-gram hash → log-prob) pairs. A 4-gram model for English requires approximately 8–20 MB in this encoding. The `whichlang` crate provides language identification but not perplexity; build or embed a separate compact model.

**Perplexity computation:** for a span of length N, perplexity is `PP = exp(-1/N * Σ log P(c_i | c_{i-3}..c_{i-1}))`. Valid English text has perplexity roughly in [5, 30] under a well-trained model. Garbled text commonly exceeds 200.

**Threshold:** flag spans with perplexity > 100 as `"low"` quality; above 300 as `"garbled"`. Spans below 10 may indicate repeated boilerplate and are worth a separate low-entropy check.

**Runtime tradeoff:** perplexity scoring is more expensive than entropy. Apply it only to spans that pass character-level checks but fail word-level checks — treating it as a second-pass arbiter rather than a first-line filter.

---

## 6. Cross-Validation Between Extraction Paths

When both vector text extraction and OCR output are available for the same page region (e.g., a page with embedded text on which OCR was also run as a confidence check), compare the two using normalized edit distance (Levenshtein distance divided by the length of the longer string).

**Agreement criterion:** if `normalized_edit_distance < 0.15`, both paths agree and confidence is high regardless of individual quality signals. If `0.15 ≤ distance < 0.40`, flag for review but prefer the vector path. If `distance ≥ 0.40`, the paths disagree significantly; use OCR output as a spell-check oracle — compute per-word overlap between OCR and vector output, and prefer whichever achieves higher real-word ratio.

This cross-validation is also useful for detecting symbol font bleed-through: OCR on a symbol font region will produce incoherent results too, which confirms the region is non-textual, whereas OCR on correctly encoded text that the vector path garbled will produce coherent text that diverges significantly from the vector output.

---

## 7. Symbol Font Detection and Recovery

Symbol fonts are the most common source of coherent-looking but semantically wrong text. Detection combines font metadata with codepoint analysis.

**Font-level signal:** inspect the font's `FontDescriptor.Flags` bit field. Bit 3 (`Symbolic`) set and bit 6 (`Nonsymbolic`) clear indicates the font self-declares as symbolic. Additionally, check the font name against known symbol font names: `Symbol`, `ZapfDingbats`, `Wingdings`, `Webdings`, and variants.

**Codepoint-level signal:** compute the fraction of output codepoints in Unicode Dingbats (U+2700–U+27BF), Miscellaneous Symbols (U+2600–U+26FF), Mathematical Operators (U+2200–U+22FF), and Box Drawing (U+2500–U+257F). A combined fraction above 0.30 in a body-text span is strongly indicative.

**Remediation:** do not emit these spans as prose. Annotate them with `readable: false`, `quality: "garbled"`, and add `"symbol_font"` to `quality_signals`. If the caller has requested exhaustive extraction, emit the raw codepoints under a `raw_glyphs` field. Do not attempt character correction on symbol font output — the mapping is fundamentally wrong at the encoding level, not the decoding level.

---

## 8. Post-Detection Remediation

When a span fails validation, the remediation decision tree is:

1. **Try font encoding recovery** (see `glyph-recognition-and-unicode-recovery.md`). If the font has a usable glyph outline and the issue is a missing ToUnicode map, heuristic name-based mapping or shape similarity to a reference font may recover the correct codepoints. Re-run validation on the recovered span.

2. **Re-run OCR on the page region** if encoding recovery fails or if the span is flagged `"garbled"` and the page has raster content at sufficient DPI. OCR is slow but authoritative on the visual content. Store the OCR result under `ocr_text` alongside the vector extraction.

3. **Emit with degraded quality metadata** if neither recovery path succeeds or is available. Set `quality: "low"` or `quality: "garbled"` and `readable: false`. Populate `quality_signals` with the list of triggered checks. This allows callers to filter, log, or surface the spans without crashing on unexpected content.

4. **Character-level correction** using edit distance to the nearest dictionary word is a last resort, applicable only to short tokens (≤ 12 characters) that fail the real-word check by a small margin. Compute Levenshtein distance to candidates within distance 2 using a BK-tree over the word list. Apply correction only if a unique nearest neighbor exists at distance 1 and the corrected span passes n-gram validation.

---

## 9. Span-Level Quality Metadata

Each extracted `TextSpan` carries the following readability fields:

```rust
pub struct TextSpan {
    pub text: String,
    pub quality: SpanQuality,       // High, Medium, Low, Garbled
    pub readable: bool,             // true iff quality is High or Medium
    pub quality_signals: Vec<QualitySignal>, // which checks triggered
    pub confidence: f32,            // 0.0–1.0 composite score
}

pub enum SpanQuality { High, Medium, Low, Garbled }

pub enum QualitySignal {
    ReplacementChars,
    PuaCodepoints,
    ControlChars,
    EntropyAnomaly,
    NgramAnomaly,
    LowRealWordRatio,
    SymbolFont,
    CvRatioAnomaly,
    CombiningOrphan,
}
```

`quality: "high"` requires: `real_word_ratio ≥ 0.60`, `replacement_ratio < 0.02`, `pua_ratio < 0.05`, entropy in [3.5, 6.5], no `quality_signals` triggered.

`quality: "medium"` requires: at most two non-critical signals triggered, `real_word_ratio ≥ 0.35`, no garble-level entropy.

`quality: "low"` means the span may contain recoverable text but significant anomalies are present.

`quality: "garbled"` means the span almost certainly does not contain readable prose in its current form.

---

## 10. Block-Level Readability Score

Aggregate span quality into a block-level score using a weighted mean:

```
block_score = Σ (span_confidence * span_char_count) / Σ span_char_count
```

Map `SpanQuality` to a base confidence: `High → 1.0`, `Medium → 0.65`, `Low → 0.30`, `Garbled → 0.0`. Adjust by the `confidence` field if finer-grained scoring is available from perplexity.

**Page-level readability score:** compute the character-weighted mean of block scores across the page. A score below 0.50 on a nominally vector page should trigger automatic OCR fallback for that page. Expose both block and page scores in the output:

```rust
pub struct PageReadability {
    pub score: f32,           // 0.0–1.0
    pub ocr_recommended: bool, // score < threshold
    pub block_scores: Vec<(BlockId, f32)>,
}
```

The threshold for `ocr_recommended` is configurable, defaulting to 0.50. Callers building pipelines that prioritize accuracy over speed can lower this to 0.70; callers that trust vector extraction for well-formed documents can raise it to 0.35 or disable the check entirely.

The page-level score also serves as a signal for the block-level zone labeling pipeline (see `document-classification-and-zone-labeling.md`): a page with a score below 0.30 is a candidate for whole-page OCR rather than incremental span recovery.
