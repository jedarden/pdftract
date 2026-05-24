# pdftract-44f6: GraphicsState struct + Color enum + matrix initialization

## Summary

Implemented the complete `GraphicsState` struct with all 13 fields per PDF spec section 8.4, plus the `Color` enum with CSS hex serialization and matrix operations.

## Changes Made

### File: `crates/pdftract-core/src/graphics_state.rs`

1. **Added `Color` enum** with 5 variants:
   - `DeviceGray(f32)` - single grayscale component
   - `DeviceRGB([f32; 3])` - RGB color
   - `DeviceCMYK([f32; 4])` - CMYK color  
   - `Spot(Arc<str>, f32)` - spot color with tint
   - `Other` - other color spaces

2. **Added `Color::to_css_hex()`** method:
   - Converts DeviceGray/RGB/CMYK to "#rrggbb" format
   - Returns None for Spot/Other
   - Includes clamp for safety
   - CMYK→RGB uses naive formula: R=(1-C)*(1-K)

3. **Extended `GraphicsState` struct** with all 13 fields:
   - `ctm: Matrix3x3` - current transformation matrix
   - `text_matrix: Matrix3x3` - Tm
   - `text_line_matrix: Matrix3x3` - Tlm
   - `font: Option<Arc<Font>>` - current font
   - `font_size: f64` - Tf
   - `char_spacing: f64` - Tc
   - `word_spacing: f64` - Tw
   - `horiz_scaling: f64` - Tz (default 100)
   - `leading: f64` - TL
   - `text_rise: f64` - Ts
   - `text_rendering_mode: u8` - Tr (0-7)
   - `fill_color: Color` - fill color
   - `stroke_color: Color` - stroke color

4. **Added `GraphicsState::initial()`** returning default state:
   - Identity CTM, text_matrix, text_line_matrix
   - font: None
   - font_size: 0.0
   - All spacings: 0.0
   - horiz_scaling: 100.0
   - text_rendering_mode: 0
   - fill/stroke_color: DeviceGray(0.0) (black)

5. **Added matrix operations** to `Matrix3x3`:
   - `scale(sx, sy)` - create scale matrix
   - `translate(tx, ty)` - create translation matrix
   - `rotate(angle)` - create rotation matrix (angle in radians)
   - `invert()` - invert matrix, returns None if singular

6. **Implemented `Debug` manually** for `GraphicsState`:
   - Font doesn't implement Debug, so we show `<Arc<Font>>` placeholder

7. **Added comprehensive tests**:
   - `test_gstate_initial_ctm_is_identity`
   - `test_gstate_initial_font_size_is_zero`
   - `test_gstate_initial_fill_color_is_black`
   - `test_gstate_initial_horiz_scaling_is_100`
   - `test_gstate_initial_text_matrices_are_identity`
   - `test_gstate_initial_font_is_none`
   - `test_gstate_initial_text_rendering_mode_is_0`
   - `test_gstate_clone_deep_equal`
   - `test_color_device_rgb_to_css_hex` - verifies `#ff0000` for red
   - `test_color_device_gray_to_css_hex`
   - `test_color_device_cmyk_to_css_hex`
   - `test_color_spot_to_css_hex_none`
   - `test_color_other_to_css_hex_none`
   - `test_color_device_rgb_clamped`
   - `test_matrix_scale`
   - `test_matrix_translate`
   - `test_matrix_rotate`
   - `test_matrix_invert_identity`
   - `test_matrix_invert_translation`
   - `test_matrix_invert_singular`
   - `test_identity_multiply_identity`
   - `test_multiply_within_tolerance`

## Acceptance Criteria Status

- ✅ `GraphicsState::initial()` returns state with ctm == identity
- ✅ `GraphicsState::initial()` returns state with font_size == 0.0
- ✅ `GraphicsState::initial()` returns state with fill_color == DeviceGray(0.0)
- ✅ Clone produces a deep-equal value (Arc cloned cheaply)
- ✅ `Color::DeviceRGB([1.0, 0.0, 0.0]).to_css_hex() == Some("#ff0000".into())`
- ✅ `Color::Spot("PANTONE".into(), 0.5).to_css_hex() == None`
- ✅ Matrix multiply test: identity * identity == identity within 1e-10

## Notes

- Font doesn't implement Debug, so GraphicsState has a manual Debug impl that shows `<Arc<Font>>` placeholder for the font field
- All matrix operations use f64 for precision (INV-30)
- Color uses f32 for components per PDF spec convention
- GraphicsState is Clone thanks to Arc<Font> (cheap clone for q/Q operators)

## References

- Plan section: Phase 3.1 State struct fields (lines 1444-1460), Color type (lines 1463-1471), CSS hex rules (line 1473)
