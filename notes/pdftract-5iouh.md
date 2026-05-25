# pdftract-5iouh: Block layer renderer implementation

## Summary

Implemented `render_blocks` helper that builds translucent rectangles per Block, colored by BlockKind. This is one of the 8 inspector debugging overlay layers specified in Phase 7.9.

## Changes made

### New file: `crates/pdftract-cli/src/inspect/render/blocks.rs`

- Implemented `render_blocks(blocks: &[BlockJson]) -> Vec<String>` function
- Color encoding per plan §7.9:
  - heading: blue (#3b82f6)
  - paragraph: gray (#9ca3af)
  - table: teal (#14b8a6)
  - list: purple (#a855f7)
  - code: orange (#f97316)
  - header/footer: light gray (#d1d5db)
  - figure: brown (#a52a2a)
  - caption: pink (#ec4899)
  - unknown kinds: default gray (#9ca3af)
- Each rect includes data-* attributes:
  - `data-kind`: block kind string
  - `data-text`: block text content (truncated to ~100 chars with "..." suffix)
  - `data-level`: heading level for heading blocks
  - `data-table-index`: table index for table blocks
  - `data-block-index`: block index in the page (for JSON-tree navigation)
- Translucent fill (0.3 opacity) with matching stroke (0.5 opacity)
- CSS class: `block-rect`

### Updated: `crates/pdftract-cli/src/inspect/render/mod.rs`

- Added `pub mod blocks;` to export the new renderer

### Fixed: Pre-existing test issues

- Fixed missing `column` field in SpanJson test fixtures across:
  - `crates/pdftract-cli/src/inspect/render/spans.rs`
  - `crates/pdftract-cli/src/inspect/render/confidence_heatmap.rs`

## Test results

All 14 tests in the blocks module pass:
- `test_render_blocks_empty`: Empty blocks list produces empty output
- `test_render_blocks_single`: Single block renders correctly with all attributes
- `test_render_blocks_heading`: Heading blocks render with blue color and level attribute
- `test_render_blocks_table`: Table blocks render with teal color and table_index attribute
- `test_render_blocks_all_kinds`: All 8 block kinds render with correct colors
- `test_render_blocks_unknown_kind`: Unknown kinds default to gray
- `test_render_blocks_text_truncation`: Long text is truncated to ~100 chars with "..." suffix
- `test_render_blocks_xml_escaping`: Special XML characters are properly escaped
- `test_render_blocks_css_class`: All rects have the correct CSS class
- `test_render_blocks_multiple`: Multiple blocks render with correct indices
- `test_kind_to_color`: Color mapping function works correctly
- `test_render_blocks_float_bbox`: Floating point coordinates are rounded to 2 decimals
- `test_render_blocks_output_is_valid_svg`: Output is valid SVG XML
- `test_render_blocks_empty_level_and_table_index`: Empty strings for non-heading/non-table blocks

All 41 tests in `inspect::render` module pass.

## Acceptance criteria status

- [x] Helper compiles and produces valid SVG output
- [x] Layer is independently toggleable via CSS class (`layer-blocks` - to be added by 7.9.3 frontend)
- [x] data-* attrs populated for downstream UI consumption
- [ ] Renders correctly in headless browser (pixel-match against fixture) - depends on 7.9.3 frontend
- [ ] Performance: 1000-element page renders in < 200ms - depends on integration testing

## References

- Plan section: Phase 7.9.5 (line ~2852: blocks layer in overlay table)
- Coordinator bead: pdftract-liq5f (8 layer renderers bundle)
- Phase 7.9.3 (frontend CSS-toggling) - not yet implemented
- Phase 7.9.6 (tooltip/search/tree consume data-* attrs) - not yet implemented
