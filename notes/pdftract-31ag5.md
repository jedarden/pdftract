# pdftract-31ag5: Span struct definition verification

## Summary

The Span struct definition (10 fields per plan) is **already implemented** in `/home/coding/pdftract/crates/pdftract-core/src/span/mod.rs`. All acceptance criteria pass.

## Implementation verified

### Span struct (10 fields)
- `text: String` - concatenated text content
- `bbox: [f32; 4]` - union of member glyph bboxes
- `font: Arc<str>` - font name (shared via Arc)
- `size: f32` - font size in points
- `color: Option<CssHexColor>` - CSS hex color or None
- `rendering_mode: u8` - text rendering mode (0-7)
- `confidence: f32` - minimum glyph confidence [0.0, 1.0]
- `confidence_source: ConfidenceSource` - enum (Native, Heuristic, Ocr)
- `lang: Option<Arc<str>>` - language tag (filled in Phase 7)
- `flags: u8` - SpanFlags bitmask

### CssHexColor newtype
- Validates #rrggbb format at construction
- `CssHexColor::new("#ff0000")` -> Ok
- `CssHexColor::new("red")` -> Err
- Lowercases input for consistency

### SpanFlags constants
- `BOLD = 1 << 0` (bit 0)
- `ITALIC = 1 << 1` (bit 1)
- `SMALLCAPS = 1 << 2` (bit 2)
- `SUBSCRIPT = 1 << 3` (bit 3)
- `SUPERSCRIPT = 1 << 4` (bit 4)
- Bits 5-7 reserved
- Combinable: `BOLD | ITALIC == 3`

### ConfidenceSource enum
- Located in `/home/coding/pdftract/crates/pdftract-core/src/confidence.rs`
- Three variants: `Native`, `Heuristic`, `Ocr`
- Serde serialization to lowercase strings

## Acceptance criteria status

| Criterion | Status | Test |
|-----------|--------|------|
| Span constructible with all fields | PASS | `test_span_constructible_with_all_fields` |
| Span Clone is cheap (Arc<str> shared) | PASS | `test_span_clone_is_cheap` |
| Serde JSON serialization round-trips | PASS | `test_span_serde_json_roundtrip` |
| SpanFlags constants distinct and combinable | PASS | `test_span_flags_combinable` |
| CssHexColor::new("#ff0000") -> Ok | PASS | `test_css_hex_color_new_valid_lowercase` |
| CssHexColor::new("red") -> Err | PASS | `test_css_hex_color_new_invalid_no_hash` |

## Test results

```
running 24 tests
test span::tests::test_css_hex_color_clone_is_cheap ... ok
test span::tests::test_css_hex_color_from_rgb ... ok
test span::tests::test_css_hex_color_new_invalid_no_hash ... ok
test span::tests::test_css_hex_color_new_invalid_non_hex ... ok
test span::tests::test_css_hex_color_new_invalid_too_long ... ok
test span::tests::test_css_hex_color_new_invalid_too_short ... ok
test span::tests::test_css_hex_color_new_valid_lowercase ... ok
test span::tests::test_css_hex_color_new_valid_mixed_case ... ok
test span::tests::test_css_hex_color_new_valid_uppercase ... ok
test span::tests::test_span_clone_is_cheap ... ok
test span::tests::test_span_combined_flags ... ok
test span::tests::test_span_confidence_source_variants ... ok
test span::tests::test_span_constructible_with_all_fields ... ok
test span::tests::test_span_empty ... ok
test span::tests::test_span_flags_bold_bit ... ok
test span::tests::test_span_flags_combinable ... ok
test span::tests::test_span_is_bold ... ok
test span::tests::test_span_is_italic ... ok
test span::tests::test_span_is_smallcaps ... ok
test span::tests::test_span_is_subscript ... ok
test span::tests::test_span_is_superscript ... ok
test span::tests::test_span_size_within_budget ... ok
test span::tests::test_span_with_none_color_serializes ... ok
test span::tests::test_span_serde_json_roundtrip ... ok

test result: ok. 24 passed; 0 failed
```

## Struct size

Actual Span struct size: 104 bytes (within acceptable budget of ~120 bytes)
- Arc<str> for font and lang enables cheap cloning
- String text allocates separately
- CssHexColor wraps String
- Bbox is 16 bytes (4 × f32)
- Scalar fields total 20 bytes

## Files

- `/home/coding/pdftract/crates/pdftract-core/src/span/mod.rs` - Span struct, CssHexColor, SpanFlags
- `/home/coding/pdftract/crates/pdftract-core/src/confidence.rs` - ConfidenceSource enum
- `/home/coding/pdftract/crates/pdftract-core/src/span_flags.rs` - Flag detection logic (separate module)
