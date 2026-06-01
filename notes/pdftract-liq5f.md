# Verification Note: pdftract-liq5f (7.9.5 - 8 Toggleable Overlay Layers)

## Summary

All 8 overlay layers are implemented and integrated into the inspector SVG renderer. Each layer is independently toggleable via CSS classes.

## Implementation Status

### 1. Spans Layer (`layer-spans`)
- **Location**: `crates/pdftract-cli/src/inspect/render/spans.rs`
- **Function**: `render_spans(spans, blocks) -> Vec<String>`
- **Elements**: SVG `<rect>` outline rectangles per span
- **Color coding**: Red (< 0.5), Yellow (0.5-0.8), Green (> 0.8)
- **Data attributes**: `data-text`, `data-confidence`, `data-font`, `data-size`, `data-span-index`, `data-bbox`
- **Status**: ✓ PASS - Fully implemented with tests

### 2. Blocks Layer (`layer-blocks`)
- **Location**: `crates/pdftract-cli/src/inspect/render/blocks.rs`
- **Function**: `render_blocks(blocks) -> Vec<String>`
- **Elements**: SVG `<rect>` translucent rectangles per block
- **Color coding**: Blue (heading), Gray (paragraph), Teal (table), Purple (list), Orange (code), Light gray (header/footer), Brown (figure), Pink (caption)
- **Data attributes**: `data-kind`, `data-text`, `data-level`, `data-table-index`, `data-block-index`
- **Status**: ✓ PASS - Fully implemented with tests

### 3. Columns Layer (`layer-columns`)
- **Location**: `crates/pdftract-cli/src/inspect/render/columns.rs`
- **Function**: `render_columns(columns, page_height) -> Vec<String>`
- **Elements**: SVG `<line>` dashed vertical lines at column boundaries
- **Color coding**: 8-color palette cycling through cyan, magenta, yellow, green, orange, blue, purple, red
- **Data attributes**: `data-column-index`, `data-boundary`, `data-x0`, `data-x1`
- **Status**: ✓ PASS - Fully implemented with tests

### 4. Reading Order Layer (`layer-reading-order`)
- **Location**: `crates/pdftract-cli/src/inspect/render/reading_order.rs`
- **Function**: `render_reading_order(blocks, order) -> Vec<String>`
- **Elements**: SVG `<path>` curved arrows + `<text>` numeric labels
- **Limit**: First 50 blocks only (to prevent clutter)
- **Color coding**: Blue arrows (#3b82f6)
- **Data attributes**: `data-from-block`, `data-to-block`, `data-reading-index`
- **Status**: ✓ PASS - Fully implemented with tests

### 5. Confidence Heatmap Layer (`layer-confidence-heatmap`)
- **Location**: `crates/pdftract-cli/src/inspect/render/confidence_heatmap.rs`
- **Function**: `render_confidence_heatmap(spans) -> Vec<String>`
- **Elements**: SVG `<rect>` per-glyph colored cells
- **Color coding**: Red (< 0.5), Yellow (0.5-0.8), Green (> 0.8), Gray (no confidence)
- **Data attributes**: `data-char`, `data-confidence`, `data-span-index`
- **Status**: ✓ PASS - Fully implemented with tests

### 6. OCR Regions Layer (`layer-ocr`)
- **Location**: `crates/pdftract-cli/src/inspect/render/ocr_regions.rs`
- **Function**: `render_ocr_regions(spans) -> Vec<String>`
- **Elements**: SVG `<defs>` pattern + `<rect>` overlays
- **Visual**: Cyan diagonal stripes (#00d9ff)
- **Data attributes**: `data-ocr-source`, `data-confidence`, `data-text`, `data-span-index`
- **Status**: ✓ PASS - Fully implemented with tests

### 7. MCID Labels Layer (`layer-mcid`)
- **Location**: `crates/pdftract-cli/src/inspect/render/mcid.rs`
- **Function**: `render_mcid_labels(mcid_map, blocks) -> Vec<String>`
- **Elements**: SVG `<text>` numeric MCID labels at block corners
- **Color**: Amber/orange (#f59e0b)
- **Data attributes**: `data-mcid`, `data-block-index`, `data-block-kind`
- **Status**: ⚠️ WARN - Renderer implemented but data not available in JSON (Phase 3.4 incomplete)
- **Note**: The API renders an empty `<g class="layer-mcid"></g>` placeholder

### 8. Anchor Labels Layer (`layer-anchors`)
- **Location**: `crates/pdftract-cli/src/inspect/render/anchors.rs`
- **Function**: `render_anchors(page_index, page_number, blocks) -> Vec<String>`
- **Elements**: SVG `<text>` block ID labels at top-left
- **Format**: `p{page_number}-b{block_index}`
- **Data attributes**: `data-page-index`, `data-page-number`, `data-block-index`, `data-bbox`, `data-kind`
- **Status**: ✓ PASS - Fully implemented with tests

## Integration in API

**Location**: `crates/pdftract-cli/src/inspect/api.rs`

The `render_page_svg` function renders all 8 layers:
```rust
// Layers are added to svg_layers vector
// Each layer wrapped in: <g class="layer-{name}" style="display: none;">...</g>
```

All layers are present in SVG output with correct class names for CSS toggling.

## Core Library

**Location**: `crates/pdftract-core/src/output/inspector/`

- `mod.rs` - Module exports
- `colors.rs` - Color encoding constants
- `layers.rs` - `LayerGroup` struct and `render_all` orchestrator

## Color Encodings

All color constants defined in `crates/pdftract-cli/src/inspect/render/colors.rs`:
- Confidence: RED_LOW (#ef4444), YELLOW_MEDIUM (#eab308), GREEN_HIGH (#22c55e), GRAY_NEUTRAL (#94a3b8)
- Block kinds: BLUE_HEADING (#3b82f6), GRAY_PARAGRAPH (#9ca3af), TEAL_TABLE (#14b8a6), etc.
- Special layers: BLUE_READING_ORDER (#3b82f6), PURPLE_MCID (#9333ea), BLACK_ANCHOR (#000000), CYAN_OCR (#00d9ff)

## Test Results

All render-related tests pass:
```
cargo test --lib -p pdftract-cli render
```

## Files Modified/Verified

1. `crates/pdftract-cli/src/inspect/render/spans.rs` - ✓ Existing implementation
2. `crates/pdftract-cli/src/inspect/render/blocks.rs` - ✓ Existing implementation
3. `crates/pdftract-cli/src/inspect/render/columns.rs` - ✓ Existing implementation
4. `crates/pdftract-cli/src/inspect/render/reading_order.rs` - ✓ Existing implementation
5. `crates/pdftract-cli/src/inspect/render/confidence_heatmap.rs` - ✓ Existing implementation
6. `crates/pdftract-cli/src/inspect/render/ocr_regions.rs` - ✓ Existing implementation
7. `crates/pdftract-cli/src/inspect/render/mcid.rs` - ✓ Existing implementation (awaiting Phase 3.4 data)
8. `crates/pdftract-cli/src/inspect/render/anchors.rs` - ✓ Existing implementation
9. `crates/pdftract-cli/src/inspect/render/colors.rs` - ✓ Existing implementation
10. `crates/pdftract-cli/src/inspect/render/mod.rs` - ✓ Existing orchestrator
11. `crates/pdftract-cli/src/inspect/api.rs` - ✓ Existing integration
12. `crates/pdftract-core/src/output/inspector/mod.rs` - ✓ Existing exports
13. `crates/pdftract-core/src/output/inspector/colors.rs` - ✓ Existing implementation
14. `crates/pdftract-core/src/output/inspector/layers.rs` - ✓ Existing orchestrator

## Acceptance Criteria

- ✅ 8 layer functions implemented, each returning Vec<SvgNode> (as Vec<String>)
- ✅ All 8 layer groups present in SVG output with correct class names
- ✅ Color encodings match plan (section 2837-2845)
- ✅ data-* attrs on span rects feed tooltip (data-text, data-confidence, data-font, data-size, data-span-index, data-bbox)
- ⏸️ Critical test (all eight layer toggles produce DOM changes) - Pending frontend test (7.9.3)
- ✅ Public `render_all` function exists in `crates/pdftract-cli/src/inspect/render/mod.rs`

## WARN Items

1. **MCID layer data not available**: The MCID renderer exists and works correctly when given MCID data, but the JSON schema (`PageJson`) doesn't include an `mcid_map` field. This is expected as Phase 3.4 (marked content tracking) is not complete. The layer is rendered as an empty placeholder with correct class name.

## Notes

- All layers use CSS-only toggling (no JavaScript re-render needed)
- SVG payload is managed by sampling for dense layers (confidence heatmap)
- Reading order arrows limited to 50 blocks to prevent visual clutter
- All coordinate values rounded to 2 decimal places for SVG precision
