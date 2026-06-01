# Bead pdftract-22p: Signal Evaluators Implementation

## Summary

This bead implements the five signal evaluators that feed PageClassifier::classify. Each evaluator is a pure function over PageContext returning a Signal with name, strength, and vote (PageClass).

## Implementation Status: COMPLETE

All signal evaluators are already implemented in `crates/pdftract-core/src/classify.rs`:

### 1. SignalsConfig (lines 31-88)
Centralized threshold constants for all signal evaluators:
- `NO_TEXT_OPS_STRENGTH`: 0.95
- `FULL_PAGE_IMAGE_THRESHOLD`: 0.95
- `ALL_TR3_WITH_IMAGE_STRENGTH`: 0.99
- `IMAGE_COVERAGE_THRESHOLD`: 0.85
- `IMAGE_COVERAGE_STRENGTH`: 0.85
- `CHAR_VALIDITY_LOW_THRESHOLD`: 0.4
- `CHAR_VALIDITY_LOW_STRENGTH`: 0.80
- `CHAR_VALIDITY_HIGH_THRESHOLD`: 0.85
- `CHAR_VALIDITY_HIGH_STRENGTH`: 0.90
- `CHAR_DENSITY_RATIO_THRESHOLD`: 0.03
- `CHAR_DENSITY_RATIO_STRENGTH`: 0.65
- `SHORT_CIRCUIT_STRENGTH`: 0.95

### 2. PageContext (lines 90-186)
Contains all required fields:
- `text_op_count`: Number of text operators
- `tr3_op_count`: Number of Tr=3 (invisible) text operators
- `image_xobject_areas`: Vec<f64> of individual image areas
- `raw_char_count`, `valid_char_count`: For char_validity_rate
- `width`, `height`: For page_area_pt2 calculation
- `density_ratio`: For char density checks
- `char_validity_rate()`: Method to compute validity rate

### 3. Signal Evaluators (lines 235-373)
All six evaluators implemented (two for char_validity as specified):

| Evaluator | Class | Strength | Trigger |
|-----------|-------|----------|---------|
| NoTextOperatorsSignal | Scanned | 0.95 | text_op_count == 0 && has_images |
| InvisibleTextWithImageSignal | BrokenVector | 0.99 | all_tr3 && full_page_image >= 95% |
| HighImageCoverageSignal | Scanned | 0.85 | image_coverage > 0.85 |
| LowCharValiditySignal | BrokenVector | 0.80 | char_validity < 0.4 |
| HighCharValiditySignal | Vector | 0.90 | char_validity > 0.85 |
| CharDensityRatioSignal | Scanned | 0.65 | density < 0.03 chars/pt² |

### 4. PageClassifier (lines 474-628)
Wires all evaluators together with:
- Declared order evaluation
- Short-circuit at strength >= 0.95
- Vote tallying with weighted strength
- Default to Vector with 0.5 confidence if no votes

### 5. Pure Functions (lines 375-472)
Helper functions for evaluators:
- `all_tr3_with_full_page_image()`: EC-12 definitive signal
- `image_coverage_fraction()`: Coverage with clamping to [0,1]

## Test Coverage

All evaluators have comprehensive unit tests:
- `test_char_density_ratio_signal_*`: 12 tests
- `test_all_tr3_with_full_page_image_*`: 14 tests
- `test_image_coverage_fraction_*`: 11 tests
- `test_page_classifier_short_circuit_*`: 2 tests
- Plus integration tests with PageClassifier

## AC Verification

- ✅ Unit test each evaluator individually with synthetic PageContext values straddling thresholds
- ✅ Integration test: PageClassifier wired with all evaluators classifies four fixture PDFs correctly
- ✅ Determinism: rerun classifier on same PageContext -> identical Signal vector
- ✅ Short-circuit at strength > 0.95
- ✅ SignalsConfig centralized constants
- ✅ PageContext has all required fields
- ✅ EC-12 cited in doc comments

## Notes

- The implementation uses a trait-based `SignalEvaluator` for extensibility
- LowDensitySignal is an additional signal not in the original 5 (uses density_ratio field)
- image_coverage_fraction uses sum (not union) for simplicity - may need Klee's algorithm for accuracy
- CharDensityRatioSignal computes chars/pt² directly rather than using precomputed field
