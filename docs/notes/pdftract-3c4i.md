# Verification Note: pdftract-3c4i (Merged Cell Detection)

## Summary

Task: 7.2.5 Merged cell detection via missing interior edges (rowspan/colspan)

Status: **COMPLETE** - Implementation was already present in codebase

## What Was Done

### Bug Fix (compilation error)

The merged cell detection code was already fully implemented in `crates/pdftract-core/src/table/cell.rs`, but there was a compilation error in the FFI layer (`crates/pdftract-libpdftract/src/api.rs`):

**Error:** The trait bound `SpanData: serde::Serialize` is not satisfied

**Root Cause:** `PageExtraction` (in `document.rs`) and `SpanData` (in `receipts/verifier.rs`) did not derive `Serialize`/`Deserialize`, but the FFI layer attempted to serialize `PageExtraction` to JSON.

**Fix:**
1. Added `#[derive(Serialize, Deserialize)]` to `SpanData` in `receipts/verifier.rs`
2. Added `#[derive(Serialize, Deserialize)]` to `PageExtraction` in `document.rs`
3. Added `use serde::{Deserialize, Serialize};` import to `receipts/verifier.rs`

### Export Fix (module visibility)

**Issue:** The `detect_merged_cells` function was not exported from the `table` module, making it inaccessible to library users.

**Fix:** Added `detect_merged_cells` to the public exports in `crates/pdftract-core/src/table/mod.rs`:
```rust
pub use cell::{Cell, TableSpan, detect_merged_cells};
```

This allows users to call `pdftract_core::table::detect_merged_cells()` after assigning spans to cells.

## Implementation Verification

### Existing Implementation in `table/cell.rs`

The merged cell detection is fully implemented with the following components:

#### Core Function: `detect_merged_cells()`
- Lines 247-319 in `table/cell.rs`
- Detects merged cells by checking for missing interior edges
- Returns tuple of (merged_cells, diagnostics)
- Handles borderless tables (NO-OP with diagnostic)
- Iteratively applies merges until no more can be applied (transitive merges)

#### Helper Functions:
- `is_vertical_edge_present()`: Checks if vertical edge has >=80% segment coverage
- `is_horizontal_edge_present()`: Checks if horizontal edge has >=80% segment coverage
- `merge_cells_right()`: Merges cell with neighbor to the right (colspan)
- `merge_cells_down()`: Merges cell with neighbor below (rowspan)

#### Cell Structure:
- `Cell` struct has `rowspan: u32` and `colspan: u32` fields (default 1)
- Absorbed cells are flagged and excluded from final output
- Surviving cell's bbox is expanded to cover absorbed cells

### Test Results

All 6 merged cell detection tests pass:
- `test_detect_merged_cells_borderless_table_noop` - Borderless tables skip merge detection
- `debug_test_colspan_3` - Debug test for colspan=3
- `test_detect_merged_cells_colspan_3_critical_test` - Critical test: merged header spanning 3 columns
- `test_detect_merged_cells_pure_rowspan` - Pure rowspan (vertical merge)
- `test_detect_merged_cells_diagonal_merge` - Diagonal merge (rowspan=2, colspan=2)
- `test_detect_merged_cells_no_merges_complete_grid` - Complete grid with all edges present
- `test_is_vertical_edge_present_full_coverage` - Edge presence detection
- `test_is_vertical_edge_present_partial_coverage_below_threshold` - Below threshold detection
- `test_is_horizontal_edge_present_full_coverage` - Horizontal edge detection
- `test_is_horizontal_edge_present_partial_coverage_above_threshold` - Above threshold detection
- `test_detect_merged_cells_transitive_merge` - Transitive merges (multiple absorptions)

All 56 table cell tests pass overall.

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Merged header cell spanning 3 columns - colspan=3 in output | ✅ PASS | `test_detect_merged_cells_colspan_3_critical_test` |
| Unit test: pure rowspan | ✅ PASS | `test_detect_merged_cells_pure_rowspan` |
| Unit test: pure colspan | ✅ PASS | `test_detect_merged_cells_colspan_3_critical_test` |
| Unit test: diagonal merge (rowspan=2, colspan=2) | ✅ PASS | `test_detect_merged_cells_diagonal_merge` |
| Unit test: borderless table NO-OP with diagnostic | ✅ PASS | `test_detect_merged_cells_borderless_table_noop` |
| Public Cell.rowspan: u32, defaulted to 1 | ✅ PASS | Field exists in struct definition |
| Public Cell.colspan: u32, defaulted to 1 | ✅ PASS | Field exists in struct definition |
| Absorbed cells excluded from final Vec<Cell> | ✅ PASS | Filter applied in detect_merged_cells |
| Bbox of merged cell covers all absorbed cells | ✅ PASS | Bbox expansion in merge functions |

## References

- Plan section: 7.2 line 2593 (merged cell detection)
- Implementation: `crates/pdftract-core/src/table/cell.rs` lines 214-505
- Tests: `crates/pdftract-core/src/table/cell.rs` lines 1627-2050

## Conclusion

The merged cell detection (7.2.5) was already fully implemented in the codebase. The task involved fixing a compilation error that prevented the code from being used by the FFI layer. The implementation correctly handles all specified requirements including colspan, rowspan, diagonal merges, transitive merges, and borderless table handling.
