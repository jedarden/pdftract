# pdftract-1q4ku: Span readability composite scoring

## Summary

The `score_span_readability` function is fully implemented in `crates/pdftract-core/src/layout/readability.rs` (lines 208-235).

## Implementation

**Function signature:**
```rust
pub fn score_span_readability(text: &str, confidence: f32, document_lang: Option<&str>) -> f32
```

**Composite score formula:**
- 0.35 * printable_fraction (non-U+FFFD, non-control chars / total)
- 0.30 * dict_coverage (words in 20k English wordlist / total; disabled for non-English)
- 0.15 * whitespace_score (1.0 if ratio in [0.05, 0.40], else 0.0)
- 0.10 * ligature_integrity (1.0 if no split ligatures, else 0.0)
- 0.10 * confidence_floor (min(1.0, confidence / 0.6))

**Constants:**
- `READABILITY_WEIGHTS: [f32; 5] = [0.35, 0.30, 0.15, 0.10, 0.10]` (sums to 1.0)
- `CONFIDENCE_THRESHOLD: f32 = 0.6`
- `WHITESPACE_MIN: f32 = 0.05`, `WHITESPACE_MAX: f32 = 0.40`

## Acceptance Criteria Status

| AC | Description | Status | Test |
|----|-------------|--------|------|
| AC1 | All-printable English high coverage: > 0.9 | PASS | `test_all_printable_english_high_coverage` |
| AC2 | All-U+FFFD: < 0.1 | PASS | `test_all_replacement_chars` |
| AC3 | All-whitespace: whitespace_score = 0 (binary fail) | PASS | `test_all_whitespace` |
| AC4 | Single short low-confidence word: scaled by confidence_floor | PASS | `test_low_confidence_scaling` |
| AC5 | Non-English doc: dict forced to 1.0; score from other signals | PASS | `test_non_english_dict_disabled` |
| AC6 | Ligature-split span: integrity 0 lowers score | PASS | `test_ligature_split_penalty` |

## Additional Tests

- `test_empty_span_returns_zero`: Empty span returns 0.0
- `test_confidence_threshold`: 0.6 confidence -> 1.0 confidence_floor
- `test_whitespace_bounds`: Whitespace ratio [0.05, 0.40] -> 1.0
- `test_printable_fraction_perfect`: All printable -> 1.0
- `test_dict_coverage_disabled_non_english`: Non-English returns 1.0
- `test_non_english_enables_dict_only_for_en`: Only "en" prefix enables dict

## Implementation Details

1. **printable_fraction**: Counts chars that are not U+FFFD and not control chars
2. **dict_coverage**: Uses unicode-segmentation UAX #29 word boundary split; checks 20k wordlist via `is_english_word`; forced to 1.0 when disabled
3. **whitespace_score**: Binary - ratio in [0.05, 0.40] yields 1.0, else 0.0
4. **ligature_integrity**: Checks for patterns like "f<U+FFFD>i" indicating split ligatures
5. **confidence_floor**: Scaled by min(1.0, confidence / 0.6)

## Git History

- `9970935` (pdftract-oh30a): Implemented `score_span_readability` function
- `8a5d9e9` (pdftract-1q4ku): Added acceptance criteria tests

## Verification

The lib compiles successfully. All acceptance criteria are implemented and tested.
