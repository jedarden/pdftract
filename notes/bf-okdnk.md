# bf-okdnk: CMAP Output Parsing

## Summary
Extended the CMAP unmapped glyph test from bf-4y66q to access and parse the generated CMAP output structure.

## Implementation

### 1. Added `iter()` method to `ToUnicodeMap`
**File:** `crates/pdftract-core/src/font/cmap.rs`

Added a public iterator method to `ToUnicodeMap` that allows tests (and other code) to iterate over all CMAP mappings for inspection:

```rust
pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Vec<char>)> {
    self.mappings.iter()
}
```

This returns an iterator over `(source_bytes, target_chars)` pairs.

### 2. Extended tests to show CMAP contents
**File:** `crates/pdftract-core/tests/cmap_unmapped_glyphs.rs`

Extended two tests to demonstrate CMAP output structure access:

- `test_cmap_unmapped_glyph_skip`: Shows basic single mapping with detailed debug output
- `test_cmap_multiple_mappings_with_unmapped_check`: Shows multiple mappings with compact debug output

Both tests now include inspection sections that display:
- Source bytes in hex format
- Target characters as strings
- Unicode code points
- Total mapping count

## Verification

### Test Results
Both extended tests pass successfully:

```
=== CMAP Output Structure Inspection ===
  Source bytes: [00]
  Target chars: "A" (Unicode: [0041])
  Total mappings: 1
=== End CMAP Inspection ===

=== CMAP Multiple Mappings Inspection ===
  [[02]] → C
  [[01]] → B
  [[00]] → A
  Total mappings: 3
=== End Inspection ===
```

### Acceptance Criteria
- ✅ Test can access CMAP output structure - `iter()` method provides access
- ✅ Test compiles without errors - All code compiles
- ✅ Test runs to completion - Tests pass without hanging or failing
- ✅ CMAP contents are visible for inspection - Debug output shows all mappings

## Artifacts
- Modified: `crates/pdftract-core/src/font/cmap.rs` (added `iter()` method)
- Modified: `crates/pdftract-core/tests/cmap_unmapped_glyphs.rs` (extended two tests)
