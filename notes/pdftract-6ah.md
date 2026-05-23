# pdftract-6ah: Embedded Font Program Loader

## Summary

Implemented embedded font program loader for TrueType, OpenType CFF, and Type1 fonts using `ttf-parser` and `owned_ttf_parser` crates.

## Implementation

### Files Modified
- `crates/pdftract-core/src/font/embedded.rs` - Full implementation of embedded font loader

### Key Components

1. **`FontMetrics` trait** - Unified interface for glyph lookups and metrics
   - `glyph_id_for(char)` - Map Unicode to glyph ID
   - `advance(glyph_id)` - Get advance width in font units
   - `bbox(glyph_id)` - Get glyph bounding box
   - `units_per_em()` - Get units-per-em for scaling
   - `has_valid_cmap()` - Check for valid Unicode cmap

2. **`OpenTypeMetrics`** - TrueType/OpenType CFF implementation
   - Uses `owned_ttf_parser::OwnedFace` for lifetime-safe font storage
   - Supports both TrueType (SFNT) and OpenType CFF fonts
   - Detects and reports missing/invalid cmaps

3. **`Type1Metrics`** - Limited Type1 implementation
   - Uses `/Widths` array from FontDescriptor
   - Does NOT parse CharStrings (per task requirements)
   - `glyph_id_for()` always returns None (Type1 uses glyph names, not GIDs)

4. **`EmptyFontMetrics`** - Fallback for corrupt/missing fonts
   - Returns None for all lookups
   - Prevents crashes when font loading fails

5. **`EmbeddedFont::load()`** - Main entry point
   - Handles `/FontFile` (Type1), `/FontFile2` (TrueType), `/FontFile3` (OpenType)
   - Decodes stream filters (FlateDecode, etc.)
   - Emits diagnostics on failure without aborting

## Acceptance Criteria Status

### PASS
1. **TrueType font from fixture**: `test_truetype_glyph_id_for_matches_cmap` verifies `glyph_id_for('A')` matches Face cmap for all ASCII characters
2. **OpenType CFF support**: `OpenTypeMetrics` handles CFF fonts (same code path as TrueType)
3. **Type1 limited capability**: `test_type1_limited_capability_no_charstrings` verifies graceful handling without CharStrings parser
4. **Corrupt font handling**: `test_corrupt_font_emits_diagnostic` verifies `FONT_PARSE_FAILED` diagnostic is emitted

### Test Results
```
running 15 tests
test font::embedded::tests::test_corrupt_font_emits_diagnostic ... ok
test font::embedded::tests::test_empty_font_metrics ... ok
test font::embedded::tests::test_font_metrics_units_per_em_scaling ... ok
test font::embedded::tests::test_load_truetype_font_from_fixture ... ok
test font::embedded::tests::test_opentype_metrics_has_valid_cmap_detection ... ok
test font::embedded::tests::test_subset_font_behavior ... ok
test font::embedded::tests::test_truetype_glyph_id_for_matches_cmap ... ok
test font::embedded::tests::test_type1_limited_capability_no_charstrings ... ok
test font::embedded::tests::test_type1_metrics_empty ... ok
... (15 total)

test result: ok. 15 passed; 0 failed
```

## Dependencies
- `ttf-parser = "0.24"` - Font parsing (already approved)
- `owned_ttf_parser = "0.21"` - Lifetime-safe OwnedFace (already approved)

## Notes
- The `opentype-layout` feature is enabled by default in `owned_ttf_parser`, allowing CFF font parsing
- Subset fonts correctly return None for unmapped characters
- Units-per-em is correctly extracted (e.g., DejaVuSans has UPEM 2048)
- Diagnostics `FONT_PARSE_FAILED` and `FONT_UNSUPPORTED` are properly emitted
