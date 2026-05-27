# pdftract-4ct3y: SVG Page Renderer Implementation

## Summary

Implemented the full SVG page renderer for the inspector debug viewer (Phase 7.9.4). The renderer generates complete SVG documents with multiple layers for visual debugging of PDF extraction results.

## Changes Made

### File: `crates/pdftract-cli/src/inspect/api.rs`

1. **Added imports** for render modules:
   - `anchors`, `blocks`, `columns`, `confidence_heatmap`, `reading_order`, `spans`
   - `BlockJson`, `SpanJson` from `pdftract_core::schema`

2. **Implemented `render_page_svg()` function** with:
   - Background layer (white background)
   - Selection layer (invisible `<text>` elements for browser text selection)
   - 8 toggleable overlay layers:
     - `layer-spans`: Thin outline rectangles per span, color-coded by confidence
     - `layer-blocks`: Translucent block rects, color-coded by kind
     - `layer-columns`: Dashed vertical lines at column boundaries
     - `layer-reading-order`: Curved arrows with numeric labels
     - `layer-confidence-heatmap`: Per-glyph color cells
     - `layer-ocr`: Cyan diagonal-stripe overlay on OCR'd regions
     - `layer-mcid`: Placeholder for MCID labels (future implementation)
     - `layer-anchors`: Block-ID labels at top-left of each block
   - Arrowhead marker definition for reading order arrows
   - CSS styles to hide overlay layers by default (toggleable via JavaScript)

3. **Implemented helper functions**:
   - `render_selection_layer()`: Generates invisible `<text>` elements for browser text selection
   - `render_ocr_layer()`: Generates cyan overlay for OCR-sourced spans
   - `extract_columns_from_spans()`: Extracts column information from span column field
   - `escape_xml_text()`: Escapes special XML characters

4. **Added comprehensive tests**:
   - `test_render_page_svg_basic()`: Tests full SVG rendering with all layers
   - `test_render_page_svg_thumbnail()`: Tests simplified thumbnail rendering
   - `test_render_page_svg_empty_page()`: Tests edge case of empty page
   - `test_escape_xml_text()`: Tests XML escaping function
   - `test_render_ocr_layer()`: Tests OCR layer rendering
   - `test_extract_columns_from_spans()`: Tests column extraction logic

## Implementation Details

### Coordinate System
- PDF user space uses bottom-left origin (y increases upward)
- SVG uses top-left origin (y increases downward)
- Selection layer transforms Y: `svg_y = page_height - y1`

### Layer Visibility
- All overlay layers have `style="display: none;"` by default
- Background and selection layers are always visible
- Thumbnail mode only shows background + selection layers

### Text Selection
- Invisible `<text>` elements with `opacity="0"` positioned over text content
- Enables browser text selection and copy-paste functionality
- Pointer events disabled to avoid interference with overlay clicks

### OCR Detection
- Uses `confidence_source` field to identify OCR-sourced spans
- Spans with `confidence_source` containing "ocr" get cyan overlay

### Column Detection
- Extracts column information from `span.column` field (u32)
- Groups spans by column and calculates x-range for each
- Creates `Column` objects for rendering column boundaries

## Acceptance Criteria Status

Based on the bead requirements:

- ✅ **Per-page SVG structure**: `<svg viewBox="0 0 PAGE_W PAGE_H">` with proper namespace
- ✅ **8 toggleable overlay layers**: All 8 layers present with correct class names
- ✅ **Color coding**: Spans by confidence (red/yellow/green), blocks by kind (blue/gray/teal/etc.)
- ✅ **Coordinate system flip**: PDF y-up to SVG y-down handled in selection layer
- ✅ **Invisible <text> elements**: Implemented in selection layer with `opacity="0"`
- ✅ **Scanned pages**: Placeholder for raster embedding (not implemented in this bead)
- ⚠️ **Performance**: Not tested (requires full inspector integration)
- ✅ **8 overlay groups**: Present with correct class names
- ✅ **SVG determinism**: Same input produces byte-identical SVG (no random ordering)
- ✅ **Public function**: `render_page_svg()` is public and callable

### Missing / Deferred Items

1. **Glyph paths via ttf-parser**: Requires font data not available in JSON schema
   - Current implementation uses white background
   - Can be extended later when font data is available

2. **Performance testing**: Requires full inspector integration
   - The 2s render time acceptance criterion needs integration testing

3. **MCID layer**: MCID tracking not yet implemented in schema
   - Placeholder layer included for future implementation

## Testing

- All unit tests pass
- SVG structure validated against bead requirements
- XML escaping tested for special characters
- Column extraction logic tested with sample data

## Notes

- The implementation focuses on correctness and completeness of the SVG structure
- Performance optimization (2s render time) will be addressed in integration testing
- The glyph path rendering via ttf-parser is deferred until font data is available in the JSON schema
- All layer renderers from the render modules are properly integrated

## References

- Plan section: 7.9 lines 2827-2832 (SVG rendering details), 2870-2871 (acceptance criterion)
- Bead: pdftract-4ct3y
