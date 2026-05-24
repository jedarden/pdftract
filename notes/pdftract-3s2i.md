# pdftract-3s2i: Phase 5.5.2 Validation Filter Implementation

## Summary

Implemented the per-word validation filter for the assisted-OCR BrokenVector path (Phase 5.5.2). The filter validates each Tesseract word result against the nearest vector glyph bbox center and adjusts confidence accordingly.

## Changes Made

### 1. Added `SpanSource::OcrAssisted` variant (crates/pdftract-core/src/hybrid.rs)
- Extended the `SpanSource` enum to include `OcrAssisted` for position-validated OCR spans
- Added `Span::ocr_assisted()` helper method

### 2. Implemented validation filter (crates/pdftract-core/src/ocr.rs)
- Added `validate_ocr_with_position_hints()` function
- Constants:
  - `ASSISTED_OCR_DISTANCE_PT = 5.0` (distance threshold in PDF points)
  - `ASSISTED_OCR_CONFIDENCE_CAP = 0.4` (confidence cap for rejected words)
  - `ASSISTED_OCR_KDTREE_THRESHOLD = 100` (glyph count for KD-tree optimization)
- Algorithm:
  1. Extract vector glyph bbox centers from position hints
  2. For each OCR word: compute word center and find nearest glyph center
  3. If distance < 5pt: accept with full OCR confidence
  4. If distance >= 5pt: cap confidence at 0.4
  5. Return `Vec<Span>` with `SpanSource::OcrAssisted`

### 3. Unit tests (assisted_ocr_tests module)
- `test_validation_filter_near_glyph`: Words near glyphs get full confidence
- `test_validation_filter_far_from_glyph`: Words far from glyphs are capped at 0.4
- `test_validation_filter_confidence_already_below_cap`: Low-confidence words stay as-is
- `test_validation_filter_no_glyphs`: No position hints → all words capped
- `test_validation_filter_multiple_words_preserves_order`: HOCR document order preserved
- `test_validation_filter_distance_threshold`: 5pt boundary behavior
- `test_assisted_ocr_constants`: Verify constants match spec

## Acceptance Criteria

### PASS
- ✅ Unit test: vector glyph at (100, 200); Tesseract word at (102, 201) → accepted full conf
- ✅ Unit test: word at (110, 210) (distance > 5 pt) → cap at 0.4
- ✅ Reproducibility: same inputs → identical Span outputs
- ✅ Code compiles: `cargo check --all-targets` passes
- ✅ Code formatted: `cargo fmt` applied

### WARN (environmental issues, out of scope)
- ⚠️ Critical-fixture test (PDF/A with invisible text layer) requires OCR feature + Tesseract installation
- ⚠️ WER comparison tests require full integration pipeline

### FAIL (true blockers)
- None

## Technical Notes

- Performance: Linear scan O(N*M) is used for now; KD-tree optimization (O(N*log(M))) is deferred until N > 100 glyphs
- The 5pt threshold is approximately one space-character width at 12pt font
- The 0.4 confidence cap is below the 0.5 threshold used in bbox-merge (Phase 5.2.4), ensuring unassisted OCR won't override legitimate vector spans
- HOCR document order is preserved in the output

## References

- Plan section: Phase 5.5 pipeline step 3 (line 1935)
- Bead ID: pdftract-3s2i
