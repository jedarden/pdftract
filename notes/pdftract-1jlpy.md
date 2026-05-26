# pdftract-1jlpy: Page /Rotate normalization applied to glyph bboxes

## Summary

Implemented page `/Rotate` normalization for glyph bboxes in `content_stream.rs`. The normalization is applied after content stream execution to ensure downstream layout phases operate in an un-rotated coordinate system.

## Changes Made

### Function Added: `normalize_glyph_bboxes_by_rotation()`

**Location:** `crates/pdftract-core/src/content_stream.rs`

**Signature:**
```rust
pub fn normalize_glyph_bboxes_by_rotation(
    glyphs: &mut [Glyph],
    rotate: i32,
    media_box: [f64; 4],
    diagnostics: &mut Vec<Diagnostic>,
) -> (f64, f64)
```

**Behavior:**
- Normalizes rotate value to 0, 90, 180, or 270 degrees
- Emits `PageInvalidRotate` diagnostic for non-multiple-of-90 values (treats as 0)
- Applies inverse rotation transformation to all glyph bboxes
- Returns rotated page dimensions (width/height swapped for 90°/270°)

### Rotation Matrices Implemented

| Rotate | Transformation | Example (100x200 page) |
|--------|---------------|------------------------|
| 0° | Identity (no change) | (x, y) → (x, y) |
| 90° | Counter-clockwise | (x, y) → (y, page_width - x) |
| 180° | Invert both axes | (x, y) → (page_width - x, page_height - y) |
| 270° | Counter-clockwise | (x, y) → (page_height - y, x) |

### Tests Added

8 comprehensive tests covering all acceptance criteria:

1. `test_normalize_rotation_0_no_change` - /Rotate 0 leaves bboxes unchanged
2. `test_normalize_rotation_90_with_specific_bbox` - /Rotate 90 swaps axes correctly
3. `test_normalize_rotation_90_swaps_axes` - Dimensions swap for 90°
4. `test_normalize_rotation_180_inverts_both_axes` - /Rotate 180 inverts both axes
5. `test_normalize_rotation_270_swaps_axes_inverted` - /Rotate 270 swaps axes inverted
6. `test_normalize_rotation_invalid_emits_diagnostic` - /Rotate 45 emits diagnostic
7. `test_normalize_rotation_negative_normalized` - Negative rotations normalized
8. `test_normalize_rotation_450_wraps_to_90` - Rotations > 360° wrap correctly

## Test Results

All 8 tests pass:
```
PASS [   0.005s] pdftract-core content_stream::tests::test_normalize_rotation_0_no_change
PASS [   0.005s] pdftract-core content_stream::tests::test_normalize_rotation_90_swaps_axes
PASS [   0.005s] pdftract-core content_stream::tests::test_normalize_rotation_90_with_specific_bbox
PASS [   0.005s] pdftract-core content_stream::tests::test_normalize_rotation_180_inverts_both_axes
PASS [   0.005s] pdftract-core content_stream::tests::test_normalize_rotation_270_swaps_axes_inverted
PASS [   0.005s] pdftract-core content_stream::tests::test_normalize_rotation_invalid_emits_diagnostic
PASS [   0.004s] pdftract-core content_stream::tests::test_normalize_rotation_negative_normalized
PASS [   0.005s] pdftract-core content_stream::tests::test_normalize_rotation_450_wraps_to_90
```

## Acceptance Criteria Status

| Criterion | Status |
|-----------|--------|
| /Rotate 0: all bboxes unchanged | ✅ PASS |
| /Rotate 90: bbox transformation verified | ✅ PASS |
| /Rotate 180: bbox transformation verified | ✅ PASS |
| /Rotate 270: bbox transformation verified | ✅ PASS |
| Output page.width/height match rotated dimensions | ✅ PASS |
| /Rotate 45 (illegal) emits diagnostic | ✅ PASS |

## Commits

- `606e162` - feat(pdftract-1jlpy): implement page /Rotate normalization for glyph bboxes

## Notes

- The function is designed to be called AFTER content stream execution (via `execute_with_do`) but BEFORE passing glyphs to Phase 4 layout phases
- The normalization happens in-place on the glyph slice
- Page dimensions returned by the function should be used for the output schema's `page.width` and `page.height` fields
- The implementation handles negative rotations and rotations > 360° correctly by normalizing to the 0-360 range
