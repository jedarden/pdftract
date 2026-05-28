# pdftract-3zz9n: 5-trigger break detector + glyph-to-span merger

## Summary

Verified that `merge_glyphs_to_spans(glyphs: &[Glyph]) -> Vec<Span>` is correctly implemented in `/home/coding/pdftract/crates/pdftract-core/src/span/mod.rs` (lines 388-487). All 54 span module tests pass.

## Implementation Details

The function implements the 5-trigger break detector:

1. **Trigger 1: font_name != prev font_name** (line 412)
   - Compares `&glyph.font_name != &span.font`

2. **Trigger 2: font_size delta > 0.5pt** (line 415)
   - Uses `(glyph.font_size - span.size).abs() > 0.5`

3. **Trigger 3: rendering_mode != prev rendering_mode** (line 418)
   - Direct comparison `glyph.rendering_mode != span.rendering_mode`

4. **Trigger 4: RGB-normalized fill_color != prev color** (lines 421-425)
   - Uses `colors_equal()` helper which normalizes:
     - DeviceGray(v) → (v,v,v) RGB tuple
     - DeviceCMYK → RGB via formula R=(1-C)*(1-K)
     - DeviceRGB → as-is
     - Spot/Other → compared by variant (Spot by name+tint exactly)

5. **Trigger 5: is_word_boundary == true** (lines 399-407)
   - Implements option (a): appends space to previous span
   - Then finalizes that span and starts fresh

## Word Boundary Handling

The implementation uses option (a) from the plan:
- When `is_word_boundary == true`, appends " " to the PREVIOUS span text
- Finalizes the span with trailing space
- Next glyph starts a new span WITHOUT leading space
- Produces cleaner JSON output: 2 spans "Hello " and "World" instead of 3 spans "Hello", " ", "World"

## Confidence Tracking

- Confidence is MINIMUM of all member glyphs: `span.confidence.min(glyph.confidence)` (line 474)
- Confidence_source mapped from WORST glyph (lowest confidence) source (lines 469-472)
- Mapping: ToUnicode/Agl/Fingerprint → Native, ShapeMatch/Unknown → Heuristic

## Bbox Union

Bbox is extended to union of member glyphs (lines 461-465):
- `span.bbox[0] = span.bbox[0].min(glyph.bbox[0])` (x0)
- `span.bbox[1] = span.bbox[1].min(glyph.bbox[1])` (y0)
- `span.bbox[2] = span.bbox[2].max(glyph.bbox[2])` (x1)
- `span.bbox[3] = span.bbox[3].max(glyph.bbox[3])` (y1)

## Acceptance Criteria - PASS

All acceptance criteria tests pass:

| AC | Test | Result |
|---|------|--------|
| "Hello World" → 2 spans | `test_merge_glyphs_to_spans_hello_world_with_word_boundary` | PASS |
| Font name change triggers break | `test_merge_glyphs_to_spans_font_name_change_triggers_break` | PASS |
| Font size 12pt vs 12.2pt → 1 span | `test_merge_glyphs_to_spans_font_size_within_threshold_no_break` | PASS |
| DeviceGray vs RGB normalized | `test_merge_glyphs_to_spans_device_gray_and_rgb_normalized_same_color` | PASS |
| Spot vs DeviceRGB different | `test_merge_glyphs_to_spans_spot_vs_device_rgb_different_colors` | PASS |
| Empty glyph list | `test_merge_glyphs_to_spans_empty_glyph_list` | PASS |
| Confidence is minimum | `test_merge_glyphs_to_spans_confidence_minimum` | PASS |
| Confidence source from worst glyph | `test_merge_glyphs_to_spans_confidence_source_worst_glyph` | PASS |
| Bbox is union | `test_merge_glyphs_to_spans_bbox_union` | PASS |

## Test Run

```bash
cargo test -p pdftract-core --lib 'span::'
```

Result: **54 passed; 0 failed; 0 ignored**

## Invariants Verified

- INV: text round-trips - joining all span texts produces original document text
- INV: confidence is MINIMUM of all member glyphs
- INV: confidence_source is from WORST glyph source
- INV: bbox is union of member glyph bboxes
- INV: font_size delta uses 0.5pt threshold
- INV: fill_color compared via RGB-normalized equality
- INV: Spot colors compared by name AND tint exactly

## Files Verified

- `/home/coding/pdftract/crates/pdftract-core/src/span/mod.rs` - Implementation (lines 388-487)
- `/home/coding/pdftract/crates/pdftract-core/src/span/mod.rs` - Tests (lines 768-1355)

## Status

**COMPLETE** - Implementation exists, all tests pass, all acceptance criteria met.
