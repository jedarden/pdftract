# Verification Note: pdftract-fy89c

## Bead
Line-to-block heuristic detector (5 break triggers in order)

## Implementation

### Files Modified
- `crates/pdftract-core/src/layout/line.rs`
- `crates/pdftract-core/src/layout/mod.rs`

### Changes Made

1. **Extended `Line<S>` struct** with new fields:
   - `median_font_size: f32` - median font size of spans in the line
   - `rendering_mode: Option<u32>` - PDF text rendering mode (Tr operator)
   - `column: Option<usize>` - column index assigned by Phase 4.3

2. **Added `LineMetadata` trait** - abstracts over different line representations for block formation

3. **Added `Block<S>` struct** - represents a block of text composed of one or more lines

4. **Added `BlockInput<L>` struct** - internal block representation used during formation

5. **Implemented `group_lines_into_blocks()` function** with 5 ordered heuristics:
   - **Trigger 1:** Vertical gap > 1.5 * line_height → new block
   - **Trigger 2:** Indent change > 0.03 * column_width → new block
   - **Trigger 3:** Font size change > 1pt → new block
   - **Trigger 4:** Rendering mode change → new block
   - **Trigger 5:** Column boundary → MANDATORY block break

### Key Implementation Details

- Lines are sorted by (column ASC, baseline DESC) before processing
- Column changes are MANDATORY block breaks (per INV in bead description)
- Line height is computed as baseline-to-baseline distance
- Vertical gap is computed as previous baseline minus current baseline
- Block state (avg_x0, median_font_size, rendering_mode, column) is tracked per block

### Tests Added

All acceptance criteria tests pass:

1. `test_five_lines_equal_spacing_one_block` - 5 lines with equal spacing/font → 1 block ✓
2. `test_thirty_pt_gap_creates_two_blocks` - 30pt gap → 2 blocks ✓
3. `test_heading_18pt_above_12pt_body_two_blocks` - Font size change (18pt vs 12pt) → 2 blocks ✓
4. `test_two_column_separate_blocks` - Column boundary → 2 blocks ✓
5. `test_indented_first_line_new_block` - Indent change (>9pt offset, 300pt column_width) → 2 blocks ✓
6. `test_rendering_mode_change_creates_new_block` - Rendering mode change → 2 blocks ✓
7. `test_empty_lines_returns_empty_blocks` - Empty input → empty blocks ✓
8. `test_single_line_returns_single_block` - Single line → single block ✓
9. `test_lines_sorted_by_column_then_baseline` - Sorting verification ✓

## Acceptance Criteria

- [PASS] 5 lines equal spacing/font: 1 block
- [PASS] 5 lines, 30pt gap, 5 more: 2 blocks
- [PASS] Heading 18pt above 12pt body: 2 blocks
- [PASS] Two-column: lines in col 0 separate from col 1
- [PASS] Indented first line (>9pt offset, 300pt column_width): NEW BLOCK starts

## Gates Passed

- [PASS] `cargo check --all-targets`
- [PASS] `cargo fmt`
- [PASS] `cargo test --package pdftract-core --lib layout::line` (21/21 tests passed)

## References

- Plan section: Phase 4.4 Heuristics (lines 1694-1699)
- Bead ID: pdftract-fy89c
