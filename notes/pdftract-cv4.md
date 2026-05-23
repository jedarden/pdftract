# Verification Note: pdftract-cv4

**Task:** Type 0 composite font + descendant CIDFont loader

**Date:** 2026-05-23

## Summary

Implemented `Type0Font::load()` following /DescendantFonts to the CIDFont dictionary, classifying the descendant as CIDFontType0 or CIDFontType2, reading /DW (default width), parsing /W array (two formats), and producing Type0Font containing both parent and descendant.

## Acceptance Criteria Results

### PASS

1. **Type0 font with CIDFontType2 descendant loads; widths from `[10 [500 600]]` resolve as CID 10 -> 500, CID 11 -> 600**
   - Test: `test_acceptance_type0_with_cidfonttype2` passes
   - Implementation: `parse_w_array()` handles per-CID form `[c [w1 w2 ...]]`

2. **Range form `[100 200 800]` resolves: CIDs 100..=200 all -> 800**
   - Test: `test_acceptance_range_form` passes
   - Implementation: `parse_w_array()` handles range form `[cfirst clast w]`

3. **Missing CID falls back to DW (default 1000)**
   - Test: `test_acceptance_missing_cid_fallback` passes
   - Implementation: `get_width()` returns `widths.get(&cid).copied().unwrap_or(default_width)`

4. **CIDFontType0 (CFF) descendant: ttf-parser CFF entrypoint used**
   - Test: `test_load_type0_font_cidfonttype0` passes
   - Implementation: `load_font_program()` delegates to `EmbeddedFont::load()` which uses `OpenTypeMetrics::from_data()` - ttf-parser handles both TrueType and CFF

### WARN

None

### FAIL

None

## Files Modified

- `crates/pdftract-core/src/font/mod.rs`: Added `pub mod type0;` and re-exports
- `crates/pdftract-core/src/font/type0.rs`: New file (1035 lines) implementing:
  - `Type0Font::load()` - main entry point
  - `parse_w_array()` - parses /W array in both formats
  - `load_cid_to_gid_map()` - loads CIDToGIDMap for CIDFontType2
  - `load_font_program()` - loads embedded font from FontDescriptor
  - `CIDToGIDMap` enum with Identity and Custom variants
  - `DescendantCIDFont` struct with metrics and font program
  - 23 unit tests (all passing)

## Test Results

```
cargo test -p pdftract-core --lib font::type0
running 23 tests
test result: ok. 23 passed; 0 failed
```

All 75 font module tests pass.

## Implementation Notes

1. **/W Array Parsing**: Token-by-token scan that switches between per-CID and range formats based on whether the second element is an array or integer
2. **Sparse Storage**: Uses `BTreeMap<u32, u16>` for widths to handle arbitrary CID values (e.g., 50000+)
3. **CIDToGIDMap**: Supports Identity (GID == CID) and Custom (2-byte big-endian stream) variants
4. **Font Program**: Loaded via `EmbeddedFont::load()` which handles both CFF and TrueType via ttf-parser
5. **Graceful Degradation**: Missing FontDescriptor or font program emits diagnostic but doesn't fail the load

## Git Commit

```
commit c12148a
feat(pdftract-cv4): Type 0 composite font + descendant CIDFont loader
```
