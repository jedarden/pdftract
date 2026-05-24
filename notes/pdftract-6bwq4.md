# pdftract-6bwq4: Baseline clustering algorithm implementation

## Summary

Implemented `cluster_spans_into_lines` function for Phase 4.2 line formation. The function groups spans into lines by baseline proximity using a threshold of `0.5 * median_font_size`.

## Changes Made

### crates/pdftract-core/src/layout/line.rs
- Added `HasFontSize` trait for types that have font_size
- Implemented `cluster_spans_into_lines<S>(spans: Vec<S>, median_font_size: f32) -> Vec<Line<S>>`
  - Computes baseline for each span using existing `compute_baseline` function
  - Sorts spans by baseline ASC
  - Sweeps through spans, clustering those within threshold (0.5 * median_font_size)
  - Emits one `Line` per cluster
  - Sorts spans by x0 within each line (left-to-right)
  - Computes line metadata: union bbox, average baseline, median font size
- Added `finalize_line_cluster` helper function

### crates/pdftract-core/src/layout/mod.rs
- Exported `HasFontSize` trait and `cluster_spans_into_lines` function

## Tests Added

All acceptance criteria tests pass:
1. `test_cluster_spans_baselines_100_100_5_105_median_12_one_line` - Spans baselines 100, 100.5, 105 with median 12 (threshold 6): all one line. PASS
2. `test_cluster_spans_baselines_100_110_median_12_two_lines` - Same with 100, 110: 2 lines (delta 10 > 6). PASS
3. `test_cluster_spans_superscript_stays_on_same_line` - Superscript at 105, line baseline 100, font 12: SAME line. PASS
4. `test_cluster_spans_empty_input_empty_output` - Empty input: empty output. PASS
5. `test_cluster_spans_threshold_is_0_5_times_median_font_size` - INV: threshold = 0.5 * median_font_size; do NOT hardcode. PASS
6. `test_cluster_spans_sorted_by_x0_within_line` - Spans within a line sorted by x0. PASS
7. `test_cluster_spans_two_column_at_same_y_one_line` - Two-column at same y: cluster into one Line. PASS
8. `test_cluster_spans_union_bbox_computed_correctly` - Union bbox computed correctly. PASS
9. `test_cluster_spans_baseline_computed_as_average` - Baseline is average of member span baselines. PASS
10. `test_cluster_spans_median_font_size_computed` - Median font size computed from line spans. PASS
11. `test_cluster_spans_single_span_single_line` - Single span produces single line. PASS

## Verification

- `cargo test -p pdftract-core --lib layout::line`: 32 tests passed
- `cargo check -p pdftract-core --lib`: Compiles successfully
- `cargo fmt -p pdftract-core`: Code formatted

## References

- Plan: Phase 4.2 Algorithm step 2 (line 1667)
- Bead: pdftract-6bwq4
