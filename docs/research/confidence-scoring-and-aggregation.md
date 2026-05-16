# Character-Level Confidence Scoring and Span Aggregation

## Why Per-Character Confidence Matters

PDF text extraction is not a uniform process. Within a single span — a contiguous run of characters sharing the same font, size, and rendering mode — individual characters may originate from completely different recovery paths. One glyph resolves cleanly through a ToUnicode entry. The adjacent glyph has no ToUnicode mapping and is recovered by AGL lookup from the glyph name. A third glyph in the same word has no name and is matched by shape fingerprint with a similarity of 0.71. A fourth falls through entirely to OCR.

If confidence is tracked only at the word or span level, this heterogeneity is invisible to consumers. A span reported as "medium confidence" may contain a mix of high-confidence characters and completely guessed ones. A word-level score papers over exactly the glyphs most likely to contain extraction errors. Per-character confidence preserves the information needed to reconstruct which specific positions in the output are reliable and which are not, enabling downstream consumers — search indexers, entity recognizers, document QA systems — to weight their processing accordingly.

## Confidence Sources and Their Native Granularity

Each extraction path exposes confidence at a different granularity, and the aggregation strategy must account for the mismatch between the native signal and the character level at which pdftract operates.

**ToUnicode** is binary: a code point either has a ToUnicode entry or it does not. When a mapping is present and valid, the character is assigned a confidence of `1.0`. When absent, the character falls to the next recovery path. The confidence is per code point, which maps cleanly to per character.

**AGL (Adobe Glyph List) recovery** uses the glyph name embedded in the font. A successful AGL lookup — for example, `fi` resolving to U+FB01 or `Agrave` resolving to U+00C0 — is assigned `0.95`. The name lookup is deterministic and the AGL is exhaustively specified, so near-full confidence is warranted. The small discount from 1.0 accounts for fonts that reuse AGL names for custom glyphs (a known pathology in older PDF producers).

**Shape fingerprint matching** produces a continuous similarity score in `[0.0, 1.0]`. The score is derived from the cosine similarity of normalized contour feature vectors, optionally combined with aspect ratio and stroke width penalties. This score is the most informative raw signal in the pipeline and maps directly to per-character confidence without transformation.

**Tesseract HOCR** reports `x_conf` at the word level, as an integer in `[0, 100]`, normalized to `[0.0, 1.0]` by dividing by 100. Character-level confidence is not natively available from HOCR output. Per-character confidence within an OCR-recovered word is therefore uniform: every character in the word receives the same score as the word. This is a known limitation and is flagged in the `confidence_source` field so consumers can interpret the score accordingly.

**Synthetic characters** (spaces inserted at gap thresholds, hyphens inferred from line geometry, soft hyphens suppressed during normalization) are assigned `1.0` if the structural inference is deterministic, or a configurable value (default `0.85`) when heuristic.

## Aggregating Character Confidence to Word Confidence

Given per-character confidences `c_1, c_2, ..., c_n` for the `n` characters in a word, three aggregation strategies are worth considering: minimum, arithmetic mean, and harmonic mean.

The **minimum** is maximally conservative: the word is only as reliable as its least reliable character. This is appropriate for applications where a single wrong character invalidates a token (entity recognition, numeric extraction).

The **arithmetic mean** is the conventional choice and gives equal weight to each character position. For a five-character word with four ToUnicode characters (`c = 1.0`) and one shape match (`c = 0.71`), the mean is `0.943`.

The **harmonic mean** is defined as:

```
H(c_1..n) = n / Σ(1/c_i)
```

For the same example: `5 / (4×(1/1.0) + 1/0.71) = 5 / (4 + 1.408) = 5 / 5.408 ≈ 0.924`.

The harmonic mean penalizes outliers more aggressively than the arithmetic mean. One very low confidence character has an outsized downward pull because `1/c_i` grows rapidly as `c_i` approaches zero. This property is desirable: a word where one glyph is a shape-match guess with similarity `0.40` should not receive a word confidence close to `1.0` merely because the other characters are clean. pdftract uses the harmonic mean as its default word-level aggregation, with minimum and arithmetic mean available as configuration options.

## Aggregating Word Confidence to Span and Block Confidence

Word confidence scores are aggregated to span confidence using a character-count-weighted mean, not a word-count-weighted mean. Words vary in length, and a two-character word and a twelve-character word should not contribute equally to the span score. The formula is:

```
span_confidence = Σ(word_confidence_i × char_count_i) / Σ(char_count_i)
```

Block confidence applies the same formula across all spans in the block, weighted by the character count of each span.

## The `confidence` Field on a Span

The `confidence` field on a span is a `f32` in `[0.0, 1.0]`. It represents the character-count-weighted harmonic-mean aggregation of per-character confidence scores across all words in the span. A value of `1.0` means every character in the span was recovered via ToUnicode. A value near `0.0` means the span is effectively unextractable by non-OCR paths and OCR itself returned low confidence.

Downstream consumers should treat this as an estimate of the probability that the extracted text accurately represents the source glyphs, not as a strict probability of correctness. The field is always present; it is never `null` or omitted.

```rust
pub struct Span {
    pub text: String,
    pub confidence: f32,
    pub confidence_source: ConfidenceSource,
    pub bbox: Rect,
    pub font_name: Option<String>,
    pub font_size: f32,
}
```

## Confidence Tiers

Four tiers are defined for reporting and CLI output:

| Tier | Range | Typical extraction path |
|---|---|---|
| High | ≥ 0.95 | ToUnicode or full AGL coverage |
| Medium | 0.70 – 0.94 | Partial AGL, shape fingerprint matches ≥ 0.80 |
| Low | 0.40 – 0.69 | Shape fingerprint matches 0.40–0.79, OCR on clean scans |
| Unextractable | < 0.40 | OCR on degraded scans, no viable shape match |

These boundaries are not arbitrary. The `0.95` high-confidence floor excludes spans with any AGL-recovered glyph at `0.95` scaled down by word-level harmonic mean, ensuring that only spans where every character is either ToUnicode or high-quality AGL qualify as high-confidence. The `0.40` unextractable floor corresponds to shape match similarity below which empirical error rates exceed 30% in validation against ground-truth corpora.

## The `confidence_source` Enum

The `confidence` scalar alone is insufficient for downstream interpretation. A score of `0.85` from ToUnicode (which is binary, so this would indicate a word with some non-mapped characters that fell to AGL) means something different from `0.85` from Tesseract HOCR, where the score is word-level and characters within may vary unpredictably. The `confidence_source` field identifies the dominant source:

```rust
pub enum ConfidenceSource {
    ToUnicode,
    AGL,
    ShapeMatch,
    OCR,
    Synthetic,
    Mixed,  // multiple sources within the span
}
```

`Mixed` is reported when a span contains characters from more than one source. Consumers that require uniform provenance can split or filter on `Mixed` spans. The field is a string enum in JSON output: `"to_unicode"`, `"agl"`, `"shape_match"`, `"ocr"`, `"synthetic"`, `"mixed"`.

## Propagating Confidence Through Normalization

Text normalization — ligature expansion, hyphenation rejoining, diacritic composition — transforms the extracted character sequence after per-character confidence is established. The confidence propagation rule is conservative: the output character inherits the minimum confidence of all input characters that contributed to it.

When `fi` (U+FB01) is expanded to `f` + `i`, both output characters inherit the ligature's confidence. When two lines joined by a soft hyphen are rejoined into a single token (`con-\nfidence` → `confidence`), the rejoined word's confidence is `min(line1_word_conf, line2_word_conf)`. When precomposed diacritics are synthesized from base character plus combining mark, the composed character's confidence is the minimum of the two components.

This conservative rule ensures normalization never inflates confidence. A consumer relying on the confidence score receives a lower bound that accounts for all transformations applied to produce the final text.

## Confidence in JSON Output

The JSON output schema includes confidence fields at three levels:

```json
{
  "pages": [
    {
      "page_number": 1,
      "confidence_summary": {
        "mean": 0.91,
        "min": 0.43,
        "high_pct": 0.72,
        "medium_pct": 0.18,
        "low_pct": 0.08,
        "unextractable_pct": 0.02
      },
      "blocks": [
        {
          "confidence": 0.94,
          "spans": [
            {
              "text": "example",
              "confidence": 0.94,
              "confidence_source": "to_unicode"
            }
          ]
        }
      ]
    }
  ],
  "document_confidence": {
    "mean": 0.89,
    "estimated_cer": 0.03
  }
}
```

`confidence` at the span level is the primary field. Block-level `confidence` is the character-count-weighted mean of its spans. Page-level `confidence_summary` contains `mean`, `min`, and tier percentage breakdowns (`high_pct`, `medium_pct`, `low_pct`, `unextractable_pct`), each representing the fraction of characters (by count) falling into that tier. Document-level `document_confidence` includes an `estimated_cer` (Character Error Rate estimate) derived from the inverse of mean confidence with an empirical calibration factor.

## Using Confidence for Extraction Quality Reporting

The CLI `--report` flag emits a structured quality summary after extraction. At the page level, a histogram of confidence bins (10 bins from 0.0 to 1.0) provides a visual distribution of extraction quality. Pages dominated by the 0.0–0.40 bin signal heavy OCR reliance on degraded content and should trigger a warning.

The document-level CER estimate is computed as:

```
estimated_cer = 1.0 - document_mean_confidence × calibration_factor
```

where `calibration_factor` is `0.92` by default, derived from validation against documents with ground-truth transcriptions. This estimate is informational and carries a disclaimer in CLI output.

Threshold-based warnings are emitted when:
- Any page has `unextractable_pct > 0.10` (more than 10% of characters unextractable)
- Document mean confidence falls below `0.70` (Medium tier boundary)
- Any span has `confidence_source = "ocr"` and `confidence < 0.50`

These warnings are machine-readable in JSON report mode and human-readable in plain text mode, giving integrators a clear signal to route documents through enhanced OCR pipelines or flag them for manual review.
