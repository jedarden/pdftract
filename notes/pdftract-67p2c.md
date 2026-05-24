# pdftract-67p2c: Inspector layer renderer - render_confidence_heatmap

## Summary

Implemented the confidence heatmap layer renderer for the inspector debug viewer. This layer displays per-glyph translucent colored cells representing extraction confidence.

## Implementation

### File created
- `crates/pdftract-cli/src/inspect/render/confidence_heatmap.rs`

### Function signature
```rust
pub fn render_confidence_heatmap(spans: &[SpanJson]) -> Vec<String>
```

### Color coding
- Red (#ef4444): confidence < 0.5 (low)
- Yellow (#eab308): 0.5 <= confidence < 0.8 (medium)
- Green (#22c55e): confidence >= 0.8 (high)
- Gray (#94a3b8): no confidence value (direct extraction)

### Data attributes
Each SVG rect includes:
- `data-char`: the character
- `data-confidence`: confidence score or empty string
- `data-span-index`: the parent span's index

### CSS class
- `class="heatmap-cell"` - for frontend CSS toggling (Phase 7.9.3)
- `fill-opacity="0.3"` - translucent cells for visual layering

## Design decisions

### Per-glyph approximation
Since the JSON schema only has span-level confidence (not per-glyph), the implementation approximates per-glyph positions by:
1. Dividing the span bbox width by the number of characters
2. Using font size for glyph height
3. Vertically centering glyphs within the span bbox

This provides a reasonable visual approximation while working with the available data. If true glyph-level confidence becomes available in the future, this function can be updated to use it.

### Helper functions
- `confidence_to_color()`: Maps confidence scores to CSS hex colors
- `escape_xml_attr()`: Escapes special XML characters for attribute values

These match the pattern from the existing `spans.rs` renderer for consistency.

## Tests

All unit tests pass:
- `test_confidence_to_color` - verifies color mapping
- `test_escape_xml_attr` - verifies XML escaping
- `test_render_confidence_heatmap_empty` - handles empty input
- `test_render_confidence_heatmap_single_span` - 3 characters rendered
- `test_render_confidence_heatmap_low_confidence` - red color for low confidence
- `test_render_confidence_heatmap_no_confidence` - gray color for no confidence

## Acceptance criteria

- ✅ Helper compiles and produces valid SVG output
- ✅ Layer is independently toggleable via CSS class (`heatmap-cell`)
- ✅ data-* attrs populated for downstream UI consumption
- ⚠️ Renders correctly in headless browser (pixel-match against fixture) - pending fixture creation
- ✅ Performance: Implementation is O(n) in number of characters; efficient string building

## References

- Plan section: Phase 7.9.5
- Parent coordinator: pdftract-liq5f
- Phase 7.9.3 (frontend CSS-toggling)
- Phase 7.9.6 (tooltip/search/tree consume data-* attrs)
