# pdftract-xzfkt: Caption block classifier - Verification

## Summary
Implemented the caption block classifier for Phase 4 layout analysis. The module identifies blocks as captions based on font size, proximity to figures, and column alignment.

## Implementation
- **Module**: `crates/pdftract-core/src/layout/caption.rs`
- **Public API**:
  - `Block` - Block struct with layout properties (kind, text, median_font_size, bbox, column)
  - `PageContext` - Page metrics (page_body_median, line_height, num_columns)
  - `classify_caption(block, prev_block, ctx) -> bool` - Single block classifier
  - `classify_page_captions(blocks, ctx)` - Batch classifier for all blocks on a page

## Classification Criteria
A block is classified as a caption when ALL of the following are true:
1. `block.median_font_size < ctx.page_body_median` (smaller font)
2. `vertical_distance(block.top, prev_figure.bottom) < 2 * ctx.line_height` (within 2 lines)
3. `block.column == figure.column` (same column, only checked if num_columns > 1)

## Test Results
All 9 unit tests passed:
- `test_caption_immediately_below_figure` - Caption 1 line below figure → PASS
- `test_caption_too_far_below_figure` - Caption 3+ lines below → NOT caption
- `test_caption_font_not_smaller` - Same font size as body → NOT caption
- `test_caption_different_column` - Two-column layout, different columns → NOT caption
- `test_no_previous_figure` - No previous block → NOT caption
- `test_caption_above_figure` - Caption positioned above figure → NOT caption (v0.1.0 limitation)
- `test_page_classification` - Multi-block page classification → PASS
- `test_block_accessors` - Block geometry methods → PASS

## Acceptance Criteria Status
| Criterion | Status |
|-----------|--------|
| Block immediately below Figure, small font, same column → kind: Caption | PASS |
| Block 5 lines below Figure → NOT Caption | PASS |
| Block with body-size font below Figure → NOT Caption | PASS |
| Block in different column from Figure → NOT Caption | PASS |
| Markdown emission of Caption block (Phase 6.5) | N/A - Future phase |

## Compilation & Linting
- `cargo check --all-targets` - PASS
- `cargo clippy --lib` - PASS (no warnings in layout module)
- `cargo test --lib caption` - 9/9 tests PASS

## Files Modified
- `crates/pdftract-core/src/layout/caption.rs` - New module (277 lines)
- `crates/pdftract-core/src/layout/mod.rs` - New module file
- `crates/pdftract-core/src/lib.rs` - Added `pub mod layout;`
- `clippy.toml` - Fixed invalid configuration option

## Git Commit
- Commit: `597f536` (feat(pdftract-xzfkt): implement caption block classifier)
- Pushed to: `main` branch

## Notes
- The classifier works with the assumption that Figure blocks are already detected (sibling bead: figure detection)
- Caption-above-figure detection is NOT implemented in v0.1.0 per the critical considerations
- Column membership is assumed to be computed by Phase 4.3 (not yet implemented)
- Line height is assumed to be computed by Phase 4.2 (not yet implemented)
- The implementation is self-contained and ready for integration once the Phase 4 pipeline is complete
