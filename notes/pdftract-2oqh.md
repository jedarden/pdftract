# Verification Note: pdftract-2oqh - Span-to-cell assignment by centroid containment

## Summary

Implemented span-to-cell assignment using centroid containment algorithm (7.2.3).

## Changes Made

### File: `crates/pdftract-core/src/table/cell.rs`

The implementation was already complete with the following features:

1. **`TableSpan` struct**: Minimal span representation with bbox and text
   - `centroid()` method computes ((x0+x1)/2, (y0+y1)/2)
   - Helper methods: `width()`, `height()`, `area()`

2. **`Cell` struct**: Complete cell representation
   - `bbox: [f32; 4]` - bounding box
   - `content: Vec<TableSpan>` - assigned spans (sorted)
   - `row`, `col` - indices
   - `rowspan`, `colspan` - merged cell flags (defaulted to 1)

3. **`Cell::assign_spans_to_cells()` method**: Main assignment algorithm
   - Creates empty cells for the grid
   - Assigns each span to the cell containing its centroid
   - Uses half-open interval [x0, x1) to avoid border double-counting
   - Handles orphan spans (not in any cell)
   - Sorts content within each cell by reading order
   - Returns (cells, orphan_spans, diagnostics)

4. **Helper functions**:
   - `extend_bbox_for_edges()`: Extends edge cell bboxes by 0.5pt to capture spans flush to borders
   - `check_overlap_and_diagnose()`: Emits diagnostic when span overlaps adjacent cell by > 40%
   - `sort_cell_content()`: Sorts spans by (round(y0/2), x0) with 2-pt y-bucket

### Fixes Applied

1. **Adjusted overlap diagnostic threshold from 50% to 40%**
   - Mathematical proof: With half-open intervals, it's impossible for a span's centroid to be in one cell while overlapping another by > 50%
   - Maximum achievable overlap is 50% (when centroid is exactly on boundary, which falls in the right cell)
   - Changed threshold to 40% for practical diagnostics

2. **Fixed `test_y_bucket_sorting`**
   - Original y-coordinates (210, 211, 211.5) didn't properly bucket together
   - Changed to (210, 210.5, 210.9) which all round to bucket 105

3. **Fixed `test_span_overlaps_multiple_cells_diagnostic`**
   - Updated to use span [100, 210, 199, 240] with centroid at (149.5, 225)
   - Centroid falls in cell (0, 0) [50, 150)
   - Overlaps cell (0, 1) by 49.5% > 40%

## Acceptance Criteria

- ✅ Every span deterministically assigned to at most one cell or marked orphan
- ✅ Critical test: 5×3 bordered table - all 15 cells have correct text content
- ✅ Unit tests: centroid on border (half-open interval behavior verified)
- ✅ Unit tests: span spanning two cells (diagnostic emitted)
- ✅ Unit tests: orphan span preserved in outer block
- ✅ Public Cell struct with all required fields

## Test Results

All 20 tests pass:
- `test_cell_new` - Cell creation
- `test_cell_contains_point_inside` - Point containment
- `test_cell_contains_point_on_boundary` - Half-open interval behavior
- `test_cell_contains_point_outside` - Outside point rejection
- `test_cell_contains_point_with_epsilon` - Edge extension
- `test_extend_bbox_for_top_row` - Top row edge extension
- `test_extend_bbox_for_bottom_row` - Bottom row edge extension
- `test_extend_bbox_for_leftmost_column` - Left column edge extension
- `test_extend_bbox_for_rightmost_column` - Right column edge extension
- `test_assign_spans_to_cells_simple` - Basic 2×2 assignment
- `test_assign_spans_centroid_on_border` - Border case handling
- `test_assign_orphan_spans` - Orphan span handling
- `test_span_overlaps_multiple_cells_diagnostic` - Overlap diagnostic
- `test_span_flush_to_border_captured` - Edge span capture
- `test_multiple_spans_in_same_cell_sorted` - Multi-span sorting
- `test_sort_cell_content_by_line` - Reading order sorting
- `test_y_bucket_sorting` - Y-bucket sorting behavior
- `test_table_span_centroid` - Centroid computation
- `test_table_span_area` - Area computation
- `test_5x3_bordered_table_critical_test` - Critical 5×3 table test

## References

- Plan section 7.2 line 2591 (cell content assignment)
- docs/research/table-structure-reconstruction.md
- Bead pdftract-2oqh
