# pdftract-5nbp: /Differences overlay handler verification

## Summary

The `/Differences` overlay handler for font encodings was already fully implemented in `crates/pdftract-core/src/font/encoding.rs`. No code changes were required.

## Implementation details

### `DifferencesOverlay` struct (lines 128-234)

- `entries: Vec<(u8, Arc<str>)>` - sparse list of (code, glyph_name) overrides
- `parse()` method walks the array linearly, maintaining a cursor that resets on each integer
- `get()` method performs linear search (fine for <32 entries)

### `FontEncoding` struct (lines 236-364)

- Combines base encoding with differences overlay
- `parse_from_font()` handles all encoding indirection patterns
- `glyph_name_for()` lookup order: differences first, base second

## Acceptance criteria verification

### PASS: `[ 39 /quotesingle 96 /grave ]` parsing
Test: `test_differences_overlay_parse_simple`
- Code 39 → "quotesingle"
- Code 96 → "grave"

### PASS: `[ 39 /a /b /c ]` consecutive assignment
Test: `test_differences_overlay_parse_consecutive`
- Code 39 → "a"
- Code 40 → "b"
- Code 41 → "c"

### PASS: Overlay precedence over base encoding
Tests: `test_font_encoding_glyph_name_with_differences`, `test_font_encoding_lookup_order`
- Overlaid codes return the override
- Non-overlaid codes return the base encoding entry

### PASS: Unknown glyph names returned as-is
Test: `test_font_encoding_unknown_glyph_name`
- Arbitrary glyph names not in AGL are returned (not None)
- Resolver can attempt L3/L4 fallback with the name

## Critical considerations

### PASS: Multiple Differences blocks
Test: `test_differences_overlay_parse_multiple_blocks`
- Handles `[ 39 /a /b 100 /x /y ]` correctly

### PASS: Out-of-range code clamping with diagnostics
Tests: `test_differences_overlay_out_of_range_positive`, `test_differences_overlay_out_of_range_negative`
- Codes < 0 clamped to 0
- Codes > 255 clamped to 255
- Emits `DiagCode::FontEncodingDifferenceOutOfRange` diagnostic

### PASS: Implicit base encoding
- `FontEncoding::parse_from_font()` handles `/Differences` without `/BaseEncoding`
- Falls back to `default_base` parameter (Standard for Type1, built-in for TrueType)

## Test results

```
running 28 tests
test result: ok. 28 passed; 0 failed; 0 ignored
```

## Files

- `crates/pdftract-core/src/font/encoding.rs` - Full implementation
- `crates/pdftract-core/src/font/mod.rs` - Re-exports `FontEncoding`, `DifferencesOverlay`, `NamedEncoding`

## No code changes required

The implementation was already complete and tested. This bead verified existing functionality.
