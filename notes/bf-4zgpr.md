# Verification Note: bf-4zgpr

## Task
Add unmapped_glyph_names config field to CMAP generator

## Summary
Added `unmapped_glyph_names` configuration field to the `DifferencesOverlay` struct, which serves as the CMAP generator for Type1 font encoding differences.

## Changes Made

### File: `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs`

1. **Added `unmapped_glyph_names` field to `DifferencesOverlay` struct**:
   - Type: `std::collections::HashSet<String>`
   - Stores glyph names that should be skipped during CMAP entry creation
   - Defaults to global `UNMAPPED_GLYPH_NAMES` set if not explicitly configured

2. **Added `with_unmapped_glyph_names()` constructor**:
   - Allows creating an overlay with a custom unmapped glyph names set
   - Enables the field to be populated from configuration

3. **Added `is_unmapped_glyph_name()` helper method**:
   - Instance method that checks if a glyph name is in the unmapped set
   - Handles leading `/` stripping for consistency

4. **Added getter and setter methods**:
   - `unmapped_glyph_names()` - Get reference to the set
   - `set_unmapped_glyph_names()` - Set a new set

5. **Updated `parse()` method**:
   - Changed from global `crate::font::is_unmapped_glyph_name()` to instance method `overlay.is_unmapped_glyph_name()`
   - Now uses the configurable field instead of hardcoded global set

## Acceptance Criteria

- ✅ CMAP generator struct has `unmapped_glyph_names` field
- ✅ Field is of appropriate type (`HashSet<String>` for efficient lookup)
- ✅ Field can be set from config/passed in constructor via `with_unmapped_glyph_names()`
- ✅ Code compiles without errors (verified with `cargo build --release`)
- ✅ All tests pass (24 encoding tests, including 2 specific unmapped glyph name tests)

## Test Results
```
running 24 tests
test font::encoding::tests::test_differences_overlay_skips_notdef ... ok
test font::encoding::tests::test_differences_overlay_skips_notdef_with_slash ... ok
test font::encoding::tests::test_font_encoding_lookup_order ... ok
...
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured
```

## Related Files
- `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs` - Main implementation
- `/home/coding/pdftract/crates/pdftract-core/src/font/unmapped.rs` - Global UNMAPPED_GLYPH_NAMES set (used as default)
