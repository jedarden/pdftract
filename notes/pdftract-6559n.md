# pdftract-6559n: render_reading_order Implementation

## Bead
**ID:** pdftract-6559n
**Title:** Inspector layer renderer: render_reading_order (curved numbered arrows)

## Implementation Summary

Implemented the `render_reading_order` function that renders curved arrows between consecutive blocks in reading order for the inspector debug viewer.

### Files Created
- `crates/pdftract-cli/src/inspect/render/reading_order.rs` (377 lines)

### Files Modified
- `crates/pdftract-cli/src/inspect/render/mod.rs` - Added `reading_order` module export

## Acceptance Criteria Status

### PASS
- ✅ Helper compiles and produces valid SVG output
  - Function signature: `pub fn render_reading_order(blocks: &[BlockJson], order: &[usize]) -> Vec<String>`
  - Returns `<path>` elements for curved arrows and `<text>` elements for numeric labels
- ✅ Layer is independently toggleable via CSS class
  - Arrows have `class="reading-order-arrow"`
  - Labels have `class="reading-order-label"`
- ✅ data-* attrs populated for downstream UI consumption
  - `data-from-block`: index of source block
  - `data-to-block`: index of target block
  - `data-reading-index`: sequence number (1, 2, 3, ...)
- ✅ Unit tests pass (10/10)
  - Empty input handling
  - Single block (no arrows)
  - Two blocks (one arrow)
  - Three blocks (two arrows)
  - Non-sequential reading order (columnar layouts)
  - Max arrows limit (50 arrows to prevent clutter)
  - Block center calculation
  - CSS class presence
  - Out-of-bounds index handling

### Technical Details

**Arrow rendering:**
- Stroke: blue (#3b82f6) with 1.5px width
- Marker-end: url(#arrowhead) - expects arrowhead definition in parent SVG `<defs>`
- Quadratic bezier curves (`Q` command) with control point at midpoint + 10pt downward
- SVG path format: `M{x1},{y1} Q{cx},{cy} {x2},{y2}`

**Labels:**
- Numeric labels (1, 2, 3, ...) at arrow midpoints
- Positioned 5pt above the midpoint
- Blue (#3b82f6), bold, 10pt font

**Performance:**
- Limits to first 50 blocks (49 arrows max) to prevent visual clutter
- O(n) complexity where n = number of arrows

## Testing

```bash
cargo test -p pdftract-cli --lib reading_order
```

All 10 tests pass:
- test_block_center
- test_block_center_fractional
- test_render_reading_order_empty
- test_render_reading_order_css_class
- test_render_reading_order_out_of_bounds_indices
- test_render_reading_order_non_sequential
- test_render_reading_order_single_block
- test_render_reading_order_three_blocks
- test_render_reading_order_two_blocks
- test_render_reading_order_max_arrows_limit

## Integration Notes

This renderer will be called by the inspector layer rendering pipeline (Phase 7.9.4) to generate the reading-order overlay layer. The SVG elements returned by this function are placed inside a `<g class="layer-reading-order">` group in the final output.

The parent SVG must define the arrowhead marker in `<defs>`:
```svg
<defs>
  <marker id="arrowhead" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
    <path d="M0,0 L0,6 L9,3 z" fill="#3b82f6" />
  </marker>
</defs>
```

## References
- Plan section: Phase 7.9 lines 2836-2845 (layer table)
- Coordinator: pdftract-liq5f (parent — 8 layer renderers bundle)
- Phase 7.9.3 (frontend CSS-toggling)
- Phase 7.9.6 (tooltip/search/tree consume data-* attrs)
