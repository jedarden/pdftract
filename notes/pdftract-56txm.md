# Verification Note: pdftract-56txm

## Bead Description
Phase 4.5: Reading Order (coordinator)

## Summary

This coordinator bead oversees the Phase 4.5 reading order determination subsystem. All 4 child beads are closed and verified. The implementation is fully functional with comprehensive test coverage.

## Child Beads Status

| Bead ID | Title | Status | Verification Note |
|---------|-------|--------|-------------------|
| pdftract-5tvv1 | Tagged-PDF fast-path stub | CLOSED | notes/pdftract-5tvv1.md |
| pdftract-4md5z | XY-cut recursive widest-whitespace split | CLOSED | Implementation in reading_order.rs |
| pdftract-4bylb | Docstrum fallback (k=5 NN graph) | CLOSED | notes/pdftract-4bylb.md |
| pdftract-18cb4 | Reading order rank assignment + algorithm tag | CLOSED | notes/pdftract-18cb4.md |

## Implementation Location

All Phase 4.5 reading order code resides in:
- `crates/pdftract-core/src/layout/reading_order.rs` (primary implementation)
- `crates/pdftract-core/src/extract.rs` (integration and tagged-PDF stub)

## Acceptance Criteria Verification

### ✅ All 4 children closed

**Status**: PASS

**Evidence**:
- `bf show` confirms all 4 children have Status: closed
- Each child has verification note or code evidence

### ✅ Two-column academic paper: all left-col blocks before all right-col blocks

**Status**: PASS

**Evidence**:
- Test `test_xy_cut_two_columns_left_then_right` passes
- XY-cut correctly orders blocks by column (left before right, then top-to-bottom within columns)
- Test creates 2 columns with 3 blocks each; verifies order [0,1,2,3,4,5]

### ✅ Magazine with sidebar: main text separated from sidebar

**Status**: PASS

**Evidence**:
- Test `test_docstrum_magazine_main_and_sidebar` passes
- Docstrum correctly identifies 2 connected components (main + sidebar)
- Main column visited before sidebar (roots sorted by column ASC, y DESC)

### ✅ Single-column text: XY-cut produces single region

**Status**: PASS

**Evidence**:
- Test `test_xy_cut_single_column_top_to_bottom` passes
- XY-cut detects single column via overlapping x-ranges (is_single_column)
- Returns order sorted by y descending (top-to-bottom reading)

### ✅ Tagged PDF: TAGGED_PDF_STRUCT_TREE_DEFERRED emitted, falls through to XY-cut

**Status**: PASS

**Evidence**:
- Test `test_tagged_pdf_emits_deferred_diagnostic` passes (extract.rs)
- Diagnostic emitted once per document when `catalog.mark_info.is_tagged`
- Falls through to XY-cut with `reading_order_algorithm = "xy_cut"`

## Test Results

**Compilation**: ✅ PASS
```
cargo check -p pdftract-core
Exit code: 0
```

**Reading order tests**: ✅ PASS (27/27)
```
running 27 tests
test layout::reading_order::tests::test_*
test result: ok. 27 passed; 0 failed; 0 ignored
```

**Key tests**:
- `test_xy_cut_two_columns_left_then_right` - Two-column ordering
- `test_xy_cut_single_column_top_to_bottom` - Single column detection
- `test_docstrum_magazine_main_and_sidebar` - Magazine layout
- `test_assign_reading_order_docstrum_fallback` - Docstrum trigger
- `test_tagged_pdf_emits_deferred_diagnostic` - Tagged PDF diagnostic

## Algorithm Selection Logic

From `assign_reading_order` (reading_order.rs lines 745-778):

1. Run XY-cut to get initial order and region statistics
2. Calculate small_region_ratio = small_region_count / region_count
3. Trigger Docstrum if: small_region_count > 10 AND small_region_ratio > 0.5
4. Assign reading_order_rank = 0, 1, 2, ... to blocks in final order
5. Return algorithm string: "xy_cut" or "docstrum"

Constants:
- REGION_COUNT_THRESHOLD = 10
- MIN_BLOCKS_PER_REGION = 3
- SMALL_REGION_RATIO_THRESHOLD = 0.5

## Integration

The reading order subsystem is integrated into the main extraction pipeline:
- Entry point: `extract.rs` calls `assign_reading_order` after block formation
- Algorithm tag: returned algorithm string set in `PageResult.reading_order_algorithm`
- Rank assignment: each block gets unique `reading_order_rank` in 0..block_count

## Phase 4.5 Completeness

All acceptance criteria met. Reading order determination is fully functional with:
- XY-cut for rectilinear layouts (preferred path)
- Docstrum fallback for irregular layouts
- Tagged-PDF diagnostic and fall-through
- Proper rank assignment and algorithm tagging

## References

- Plan section: Phase 4.5 Reading Order (lines 1734-1759)
- XY-cut: Nagy & Seth 1984
- Docstrum: O'Gorman 1993
