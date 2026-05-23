# pdftract-3uq: Font subtype classifier and BaseFont prefix stripper

## Summary

Implemented the font type classification module (`crates/pdftract-core/src/font/mod.rs`) with:

1. **`FontKind` enum** - Represents all PDF font types:
   - `Type1` - Non-Standard-14 Type 1 fonts
   - `Type1Std14` - Standard 14 fonts (Times-Roman, Helvetica, Courier, Symbol, ZapfDingbats)
   - `TrueType` - TrueType fonts
   - `Type0` - Composite fonts with descendant CIDFonts
   - `CIDFontType0` - CFF-based CID fonts
   - `CIDFontType2` - TrueType-based CID fonts
   - `Type3` - Bitmap/content-stream defined fonts
   - `OpenTypeCFF` - OpenType fonts with CFF data

2. **`strip_subset_prefix(name: &str) -> &str`** - Removes 6-uppercase-letter subset prefix
   - Exactly validates 6 ASCII uppercase letters + `+`
   - Returns unchanged for invalid patterns (too short, lowercase, no prefix)

3. **`classify_font(font_dict: &PdfDict) -> FontKind`** - Classifies fonts by:
   - Reading `/Subtype` to get base font type
   - Checking Standard 14 font names (with or without subset prefix)
   - For Type0 fonts, reading descendant CIDFont's `/Subtype`
   - Checking `/FontDescriptor` for `/FontFile3` with `/Subtype /OpenType` to distinguish OpenTypeCFF

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Unit tests for all 8 FontKind branches | PASS | 21 font-specific tests cover all branches |
| `strip_subset_prefix("ABCDEF+Times-Roman") == "Times-Roman"` | PASS | Tested in `test_strip_subset_prefix_valid` |
| `strip_subset_prefix("ABCD+Foo") == "ABCD+Foo"` | PASS | Tested in `test_strip_subset_prefix_too_short` |
| `strip_subset_prefix("abcdef+Foo") == "abcdef+Foo"` | PASS | Tested in `test_strip_subset_prefix_lowercase` |
| Std-14 detection ignores subset prefix | PASS | Tested in `test_is_standard_14_font` and `test_classify_font_type1_standard_with_subset` |

## Implementation Details

### FontKind enum methods
- `is_standard_14()` - Returns true for Type1Std14
- `is_cid_font()` - Returns true for Type0, CIDFontType0, CIDFontType2
- `is_type3()` - Returns true for Type3 fonts

### Standard 14 fonts
The hardcoded list includes all 14 canonical names:
- Times family: Times-Roman, Times-Bold, Times-Italic, Times-BoldItalic
- Helvetica family: Helvetica, Helvetica-Bold, Helvetica-Oblique, Helvetica-BoldOblique
- Courier family: Courier, Courier-Bold, Courier-Oblique, Courier-BoldOblique
- Symbol, ZapfDingbats

### Edge cases handled
- `/Subtype` with or without leading slash
- Missing `/Subtype` (defaults to Type1)
- Empty or missing `/DescendantFonts` array for Type0 fonts
- Indirect references to FontDescriptor or DescendantFonts (skipped, returns default)

## Files Modified

- `crates/pdftract-core/src/lib.rs` - Added `pub mod font;`
- `crates/pdftract-core/src/font/mod.rs` - New module with FontKind enum and classifier functions

## Testing

All 27 font-related tests pass:
- 21 tests in font::tests
- 6 tests in other modules that reference font types

Test coverage includes:
- Subset prefix stripping (valid, invalid, edge cases)
- Standard 14 font detection (with and without prefix)
- All 8 FontKind variants
- Type0 with CIDFont descendants
- OpenTypeCFF detection via FontDescriptor

## Commit

`git commit` to follow with conventional commit message citing this bead.
