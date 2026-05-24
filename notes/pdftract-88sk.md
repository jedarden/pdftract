# Verification Note: pdftract-88sk - Line-based Table Detection

## Summary

Implemented line-based table detection for bordered tables. The implementation was already mostly complete in the existing codebase. Fixed the critical 5x3 table test and added missing unit tests (nested rectangles, disjoint tables) plus a benchmark.

## Changes Made

### Files Modified

1. **crates/pdftract-core/src/table/detector.rs**
   - Fixed `test_detect_5x3_table`: Changed from 3 rows × 5 columns to 5 rows × 3 columns to match acceptance criteria (`row_ys.len() == 6`, `col_xs.len() == 4`)
   - Added `test_detect_nested_rectangles`: Tests handling of nested rectangles (e.g., table within a table)
   - Added `test_detect_disjoint_tables`: Tests detection of multiple disjoint tables on the same page

2. **crates/pdftract-core/Cargo.toml**
   - Added `criterion = "0.5"` to dev-dependencies
   - Added `[[bench]]` section for table_detection benchmark

3. **crates/pdftract-core/benches/table_detection.rs** (new file)
   - Criterion benchmark testing performance with varying segment counts
   - Tests 20, 40, 60, 100, and 1000 segment configurations

## Acceptance Criteria Status

| Criteria | Status | Notes |
|----------|--------|-------|
| Detector emits GridCandidate for every closed grid of >= 4 cells | ✅ PASS | `build_grids()` filters by `min_cells` (default 4) |
| Critical test: 5x3 bordered table returns GridCandidate with row_ys.len()==6, col_xs.len()==4 | ✅ PASS | Fixed test now correctly draws 5 rows × 3 columns (6 horizontal, 4 vertical lines) |
| Unit tests: single rectangle | ✅ PASS | `test_collect_rectangle` |
| Unit tests: nested rectangles | ✅ PASS | `test_detect_nested_rectangles` (new) |
| Unit tests: mixed text+rules | ✅ PASS | `test_filter_text_object_segments` |
| Unit tests: glyph-path noise rejected | ✅ PASS | `test_filter_text_object_segments` |
| Public TableDetector::detect_line_based(&PageContext) -> Vec<GridCandidate> | ✅ PASS | Method exists and is public |
| Benchmark: < 5 ms on 1000-segment page | ✅ PASS | Actual: ~772 µs (0.77 ms) |

## Test Results

```
test result: ok. 35 passed; 0 failed; 0 ignored
```

All 35 table module tests pass, including:
- Segment creation and manipulation tests
- Grid candidate construction tests
- Detector tests (segment collection, clustering, intersection finding, grid building)
- 5x3 bordered table critical test

## Benchmark Results

```
table_detection/dense_table_1000_segments
                        time:   [762.36 µs 772.02 µs 784.69 µs]
```

Performance is well under the 5 ms requirement for 1000-segment pages.

## Implementation Notes

The existing implementation already had:
- Segment extraction from PDF path operators (m, l, re, S, s, f, F, B, B*)
- Text object filtering (BT..ET) to exclude Type 3 font glyph outlines
- Collinear segment clustering with epsilon 1.0 pt tolerance
- Gap tolerance of 2.0 pt for merging overlapping collinear segments
- Intersection finding between horizontal and vertical segments
- Grid construction from intersection points

The main fix was correcting the critical test to match the acceptance criteria (5 rows × 3 columns, not 3 rows × 5 columns).
