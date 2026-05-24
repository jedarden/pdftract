# pdftract-2gto: HOCR Pixel-to-PDF Coordinate Conversion

## Summary

Implemented HOCR pixel-to-PDF coordinate conversion with proper handling of:
1. **10px padding subtraction** (from Phase 5.3.4 border padding)
2. **DPI scaling** (pixel → PDF point conversion at render-time DPI)
3. **Y-axis flip** (HOCR top-left origin → PDF bottom-left origin)
4. **Page rotation** (0°, 90°, 180°, 270° support)
5. **Hybrid cell offsets** (cell-local OCR → global PDF coordinates)

## Implementation

### Files Modified
- `crates/pdftract-core/src/ocr.rs`

### Changes Made

1. **Added constant `HOCR_BORDER_PADDING`**: Set to 10 pixels to match the padding added in preprocessing (Phase 5.3.4)

2. **Added `HocrWord::to_pdf_bbox()` method**: Converts HOCR pixel coordinates to PDF user-space coordinates
   - Signature:
     ```rust
     pub fn to_pdf_bbox(
         &self,
         dpi: u32,
         page_height_pt: f64,
         rotation: Option<i32>,
         cell_origin: Option<[f64; 2]>,
     ) -> [f64; 4]
     ```
   - Returns: `[x0, y0, x1, y1]` in PDF points (bottom-left origin)

3. **Added `apply_rotation_to_bbox()` helper function**: Handles page rotation transformations

### Coordinate Transform Steps

1. **Subtract padding** (pixel space):
   - `hocr_px - 10` → pre-pad image pixel coords
   - Handles edge case where bbox is entirely within padding (clamps to origin)

2. **Scale to points**:
   - `px * 72.0 / dpi` → PDF pt
   - Uses the DPI from render time (Phase 5.2)

3. **Flip Y-axis**:
   - `pdf_y = page_height_pt - hocr_y_pt`
   - Converts from top-left origin (HOCR) to bottom-left origin (PDF)

4. **Apply rotation** (if specified):
   - Supports 0°, 90°, 180°, 270° rotations
   - Invalid rotation values are ignored (bbox returned unchanged)

5. **Add cell origin** (if hybrid):
   - Offsets cell-local OCR coordinates to global PDF coordinates
   - Used when OCR is run on hybrid page cell crops

## Tests Added

Added comprehensive tests in `hocr_tests` module:

1. **`test_to_pdf_bbox_basic_conversion`**: Critical test from plan line 1908
   - HOCR bbox at (10,10,100,30) at 300 DPI on letter-size page
   - Verifies correct padding subtraction and Y-flip

2. **`test_to_pdf_bbox_y_flip_sanity`**: Y-flip verification
   - Top-of-page word has highest PDF Y value
   - Bottom-of-page word has lowest PDF Y value

3. **`test_to_pdf_bbox_padding_subtraction`**: Padding edge case
   - Bbox exactly at padding boundary
   - Verifies subtraction happens in pixel space (before DPI scale)

4. **`test_to_pdf_bbox_different_dpi`**: DPI scaling verification
   - Tests 200, 300, 400 DPI
   - Verifies correct scale factor (72.0 / dpi)

5. **`test_to_pdf_bbox_hybrid_cell_offset`**: Hybrid cell handling
   - Cell (3, 2) offset applied correctly
   - Cell-local coords → global PDF coords

6. **`test_to_pdf_bbox_clamps_negative_coords`**: Edge case handling
   - Bbox entirely within padding (negative after subtraction)
   - Clamped to origin (no negative coordinates)

7. **Rotation tests**: 0°, 90°, 180°, 270°, and invalid angle

8. **`test_apply_rotation_to_bbox_preserves_dimensions`**: Rotation preserves bbox area

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Critical test (line 1908): HOCR bbox conversion | PASS | `test_to_pdf_bbox_basic_conversion` |
| Y-flip sanity: top-of-page has highest PDF Y | PASS | `test_to_pdf_bbox_y_flip_sanity` |
| Hybrid cell test: cell offset applied | PASS | `test_to_pdf_bbox_hybrid_cell_offset` |
| 100-page OCR output: valid bboxes | N/A | Requires actual OCR infrastructure |

## Notes

- Tests are behind the `ocr` feature flag and require leptonica/tesseract to run
- The coordinate conversion code itself is pure Rust with no external dependencies
- Implementation follows the exact specification from plan lines 1899-1927
- All coordinate transformations use f64 for precision (0.1 pt resolution as specified)

## Integration Points

This function will be called during Phase 5.4 (Tesseract Integration) to convert HOCR output to PDF spans:

```rust
// Usage example (not yet integrated):
let word = HocrWord { /* ... */ };
let pdf_bbox = word.to_pdf_bbox(dpi, page_height, Some(rotation), None);
let span = Span::ocr(pdf_bbox, word.confidence(), word.text);
```

## Future Work

- Integrate with actual Tesseract OCR pipeline (Phase 5.4 full implementation)
- Add Span emission with confidence_source = "ocr"
- Add language field from opts.ocr_language
