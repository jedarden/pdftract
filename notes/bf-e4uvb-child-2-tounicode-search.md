# ToUnicode Entry Creation Code Search

**Task**: Search and identify ToUnicode entry creation code
**Bead**: bf-29ioc
**Date**: 2026-07-05

## Primary Location

**File**: `/home/coding/pdftract/crates/pdftract-core/src/font/cmap.rs`

### Core Function: `ToUnicodeMap::add_mapping()`

**Lines**: 86-88

```rust
/// Add a single mapping from source bytes to destination chars.
///
/// MARKER: CMAP entry creation point - this is where individual ToUnicode
/// mappings are stored. Called by parse_beginbfchar() and parse_beginbfrange().
/// See notes/bf-e4uvb-child-1.md for documentation.
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
    self.mappings.insert(src, dst);
}
```

This is the **primary entry creation point** where individual ToUnicode mappings are stored in the internal HashMap.

### Functions that Call `add_mapping()`

#### 1. `parse_beginbfchar()`
**Lines**: 190-216
- Called at line 209: `map.add_mapping(src, dst);`
- Handles single-character mappings from `beginbfchar...endbfchar` blocks
- Format: `beginbfchar <count> <src1> <dst1> <src2> <dst2> ... endbfchar`

#### 2. `parse_beginbfrange()` - Two Forms
**Lines**: 223-309

**a) Explicit Array Form** (line 279):
```rust
map.add_mapping(current.clone(), dst);
```
- Format: `beginbfrange <count> <lo> <hi> [<d0> <d1> ...] endbfrange`
- Each element in the array is mapped individually

**b) Contiguous Range Form** (line 292):
```rust
map.add_mapping(current.clone(), dst.clone());
```
- Format: `beginbfrange <count> <lo> <hi> <dst> endbfrange`
- Range is expanded with incrementing destination values

### Public Parser Functions

- `parse_to_unicode()` (line 493) - Convenience function, returns map only
- `parse_to_unicode_with_diags()` (line 502) - Returns map + diagnostics

### Related Test Functions

Lines 508-732 contain comprehensive tests:
- `test_parse_single_bfchar` (line 520)
- `test_parse_bfchar_ligature` (line 532)
- `test_parse_bfrange_contiguous` (line 570)
- `test_parse_bfrange_explicit_array` (line 586)
- And 14 more tests covering edge cases

## Usage Location (Query Side)

**File**: `/home/coding/pdftract/crates/pdftract-core/src/font/resolver.rs`

### `resolve_level1()` Function
**Lines**: 383-399

```rust
fn resolve_level1(char_code: &[u8], to_unicode: Option<&ToUnicodeMap>) -> ResolvedGlyph {
    let Some(cmap) = to_unicode else {
        return ResolvedGlyph::failure();
    };

    let Some(chars) = cmap.lookup(char_code) else {
        return ResolvedGlyph::failure();
    };
    // ... returns resolved glyph with UnicodeSource::ToUnicode
}
```

This function **queries** the ToUnicode entries created by `add_mapping()`, using `ToUnicodeMap::lookup()` (line 93-95 in cmap.rs).

## Summary

| Aspect | Location | Lines |
|--------|----------|-------|
| **Entry creation** | `cmap.rs::ToUnicodeMap::add_mapping()` | 86-88 |
| **Single mapping parser** | `cmap.rs::parse_beginbfchar()` | 190-216 (calls at 209) |
| **Range mapping parser** | `cmap.rs::parse_beginbfrange()` | 223-309 (calls at 279, 292) |
| **Entry query/lookup** | `cmap.rs::ToUnicodeMap::lookup()` | 93-95 |
| **Usage in resolver** | `resolver.rs::resolve_level1()` | 383-399 |

## Data Structure

The `ToUnicodeMap` struct (lines 67-71 in cmap.rs) stores:
```rust
pub struct ToUnicodeMap {
    /// Mapping from source byte sequence to destination Unicode codepoints.
    /// Uses `Vec<u8>` as key (source bytes) and `Vec<char>` as value (destination chars).
    mappings: HashMap<Vec<u8>, Vec<char>>,
}
```

This allows:
- Variable-length source byte sequences (1-4 bytes)
- Multi-codepoint destinations (for ligature expansion like `fi` → U+0066 U+0069)
