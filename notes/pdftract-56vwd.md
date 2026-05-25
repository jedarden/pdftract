# pdftract-56vwd: x0 histogram builder

## Summary
Implemented `build_x0_histogram(spans: &[S], page_width: f32) -> Vec<u32>` function for column detection (Phase 4.3).

## Changes Made

### crates/pdftract-core/src/layout/columns.rs
- Added `build_x0_histogram()` function that builds a 1pt-resolution histogram of span x0 coordinates
- Added `HasBBox` trait for generic bbox access (returns `[f32; 4]`)
- Implemented `HasBBox` for `[f32; 4]` and `[f64; 4]` array types
- Function clamps x0 values to valid histogram range and logs diagnostics for out-of-bounds values

### crates/pdftract-core/src/layout/mod.rs
- Exported `build_x0_histogram` function

## Acceptance Criteria Status

| Criterion | Status |
|-----------|--------|
| 1 span at x0=100, page_width=612: hist[100] == 1 | PASS |
| 5 spans at x0=100,100,200,200,300: hist[100]==2, hist[200]==2, hist[300]==1 | PASS |
| Span at x0=-5: clamped to hist[0], diagnostic | PASS |
| Empty spans: returns Vec of zeros | PASS |

## Test Results
All 20 tests in `layout::columns` module pass, including 7 new tests for `build_x0_histogram`:
- `test_build_x0_histogram_single_span` - Single span histogram
- `test_build_x0_histogram_multiple_spans` - Multiple spans at different x0 positions
- `test_build_x0_histogram_clamp_negative_x0` - Negative x0 clamping with diagnostic
- `test_build_x0_histogram_clamp_overflow_x0` - Overflow x0 clamping with diagnostic
- `test_build_x0_histogram_empty_spans` - Empty span handling
- `test_build_x0_histogram_rounding` - Rounding behavior (x0.4 -> x0, x0.6 -> x0+1)
- `test_build_x0_histogram_a4_page` - A4 page width (595pt)

## Notes
- Function signature uses generic `S: HasBBox` trait for flexibility with different span representations
- 1pt resolution per plan: for 612pt letter page, 612 buckets; for 595pt A4, 595 buckets
- Only x0 (LEFT edge) is histogrammed; x1 is not used
- Each span contributes exactly one bucket increment
- Diagnostics use `tracing::warn!` for out-of-bounds x0 values
