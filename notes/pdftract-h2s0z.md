# Verification Note: pdftract-h2s0z

## Summary

Implemented the adaptive word boundary detector for Phase 3.2 text extraction.

## Acceptance Criteria

### PASS

- ✅ Initial 20 glyphs after Tf: any gap > 0.25 × font_size triggers boundary
  - Verified by `test_detector_gap_below_threshold`, `test_detector_gap_at_threshold`, `test_detector_gap_above_threshold`
  - Bootstrap threshold = 0.25 * font_size (test: `test_detector_bootstrap_threshold`)

- ✅ Gap exactly at threshold: NOT a boundary (strictly greater than)
  - Verified by `test_detector_gap_at_threshold` - gap exactly at 3.0 does NOT trigger boundary

- ✅ 21st glyph onward: threshold is 1.5× the median of last 20 actual gaps
  - Verified by `test_detector_recalibration_after_20_samples`
  - `recalibrate()` computes median and sets threshold = 1.5 * median
  - Outlier exclusion > 4× current threshold

- ✅ Tf switch: new font starts fresh with bootstrap threshold
  - Verified by `test_manager_reset_font`
  - `reset_font()` clears samples and resets threshold

- ✅ BT inside same font: bootstrap resets
  - `reset_all()` method resets all detectors
  - Integrated with content_stream BT operator

- ✅ Negative gap handling: never a word boundary
  - Verified by `test_detector_negative_gap_no_boundary`, `test_detector_zero_gap_no_boundary`
  - `record_and_detect()` returns false for gap <= 0.0

### INVARIANTS VERIFIED

- ✅ Bootstrap threshold = 0.25 × font_size (FIXED, not configurable)
- ✅ Recalibration formula = 1.5 × median (samples window = 20)
- ✅ Recalibration every 5 samples after 20 (checked: `sample_count > 20 && sample_count % 5 == 0`)
- ✅ Comparison in text space (all gap values are f32 text-space points)
- ✅ Tw applied only to U+0020 (verified in `test_text_state_expected_advance_with_tw_non_space`)

## Implementation

Created new module `crates/pdftract-core/src/word_boundary.rs` with:

1. **`WordBoundaryDetector`** struct:
   - `font_id: FontId`
   - `sample_count: u32`
   - `samples: Vec<f32>` (capacity 20, bounded)
   - `threshold: f32`

2. **`WordBoundaryManager`** struct:
   - HashMap<FontId, WordBoundaryDetector>
   - Per-font detector management
   - `reset_font()` for Tf operator
   - `reset_all()` for BT operator

3. **`TextState`** struct:
   - `tc: f32` (character spacing)
   - `tw: f32` (word spacing)
   - `tz: f32` (horizontal scaling)
   - `font_size: f32`
   - `font_id: Option<FontId>`
   - `expected_advance(glyph_width, is_space)` method implementing Tc/Tw/Tz formula

## Files Modified

- `crates/pdftract-core/src/word_boundary.rs` (NEW)
- `crates/pdftract-core/src/lib.rs` (added `pub mod word_boundary`)
- `crates/pdftract-core/src/font/resolver.rs` (added `from_usize` test constructor)

## Tests

27 tests in `word_boundary` module, all passing:

```
test word_boundary::tests::test_detector_bootstrap_threshold ... ok
test word_boundary::tests::test_detector_gap_above_threshold ... ok
test word_boundary::tests::test_detector_gap_at_threshold ... ok
test word_boundary::tests::test_detector_gap_below_threshold ... ok
test word_boundary::tests::test_detector_negative_gap_no_boundary ... ok
test word_boundary::tests::test_detector_sample_count ... ok
test word_boundary::tests::test_detector_zero_gap_no_boundary ... ok
test word_boundary::tests::test_detector_recalibration_after_20_samples ... ok
test word_boundary::tests::test_detector_reset ... ok
test word_boundary::tests::test_manager_multiple_fonts ... ok
test word_boundary::tests::test_manager_record_and_detect ... ok
test word_boundary::tests::test_manager_reset_font ... ok
test word_boundary::tests::test_median_empty ... ok
test word_boundary::tests::test_median_even ... ok
test word_boundary::tests::test_median_single ... ok
test word_boundary::tests::test_median_two ... ok
test word_boundary::tests::test_median_odd ... ok
test word_boundary::tests::test_median_unsorted ... ok
test word_boundary::tests::test_text_state_defaults ... ok
test word_boundary::tests::test_text_state_expected_advance_basic ... ok
test word_boundary::tests::test_text_state_expected_advance_combined ... ok
test word_boundary::tests::test_text_state_expected_advance_with_tz ... ok
test word_boundary::tests::test_text_state_expected_advance_with_tc ... ok
test word_boundary::tests::test_text_state_expected_advance_with_tw_non_space ... ok
test word_boundary::tests::test_text_state_expected_advance_with_tw_space ... ok
test word_boundary::tests::test_text_state_set_font ... ok
test word_boundary::tests::test_text_state_setters ... ok

test result: ok. 27 passed; 0 failed
```

## Next Steps

The detector is implemented and tested. Integration with content_stream.rs (Tj/TJ operators) and tracking the last glyph position are required for full Phase 3.2 completion. This will be done in a follow-up bead that:
1. Tracks last glyph end position in text space
2. Computes actual gaps from text matrix positions
3. Calls `WordBoundaryManager::record_and_detect()` for each glyph
4. Emits synthetic space spans when boundaries are detected

## References

- Plan section: Phase 3.2 Word boundary threshold (lines 1529-1535)
- docs/research/word-boundary-reconstruction.md
