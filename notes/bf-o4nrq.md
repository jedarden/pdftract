# CMAP Entry Creation Point - Documentation

## Task: Locate CMAP entry creation point

## Summary

Successfully identified the exact location in the codebase where CMAP entries are created for glyphs.

## Location Details

### Primary Creation Point

**File**: `crates/pdftract-core/src/font/cmap.rs`

**Function**: `ToUnicodeMap::add_mapping()`

**Line numbers**: 86-88

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

### Data Structure

**Struct**: `ToUnicodeMap` (lines 67-71)

```rust
pub struct ToUnicodeMap {
    /// Mapping from source byte sequence to destination Unicode codepoints.
    /// Uses `Vec<u8>` as key (source bytes) and `Vec<char>` as value (destination chars).
    mappings: HashMap<Vec<u8>, Vec<char>>,
}
```

- **Key**: `Vec<u8>` - source byte sequence from PDF
- **Value**: `Vec<char>` - destination Unicode characters (supports multi-codepoint mappings like ligatures)

### Call Sites

The `add_mapping()` function is called from three locations during CMap parsing:

1. **Single-character mapping** (`parse_beginbfchar()`, line 209):
   ```rust
   map.add_mapping(src, dst);
   ```

2. **Range mapping - explicit array form** (`parse_beginbfrange()`, line 279):
   ```rust
   map.add_mapping(current.clone(), dst);
   ```

3. **Range mapping - contiguous form** (`parse_beginbfrange()`, line 292):
   ```rust
   map.add_mapping(current.clone(), dst.clone());
   ```

## Context

### CMap Parser Structure

- **Parser**: `CMapParser` struct (line 118-121)
- **Entry point**: `CMapParser::parse()` (line 136-185)
- **Supported CMap syntax**:
  - `beginbfchar` / `endbfchar`: Single-character mappings
  - `beginbfrange` / `endbfrange`: Range mappings (contiguous and explicit array)
  - `usecmap`: Inheritance from named CMaps (stub - emits diagnostic)

### Module Structure

The font module is located at `crates/pdftract-core/src/font/` with these key files:
- `cmap.rs`: ToUnicode CMap parser (this file)
- `encoding.rs`: Font encoding handling
- `embedded.rs`: Embedded font support
- `resolver.rs`: Character resolution logic
- `predefined_cmap.rs`: Predefined CMap support

## Next Steps for Skip Logic Implementation

To implement skip logic for CMAP entries:

1. **Modification point**: The `add_mapping()` method is the ideal place to add skip logic
2. **Filter approach**: Add a conditional check before inserting into the HashMap
3. **Configuration**: May need to pass a filter/skip configuration to `ToUnicodeMap`
4. **Alternative**: Filter at call sites before calling `add_mapping()`

## References

- Module: `crates/pdftract-core/src/font/cmap.rs`
- Primary function: `ToUnicodeMap::add_mapping()` (lines 86-88)
- Data structure: `ToUnicodeMap.mappings: HashMap<Vec<u8>, Vec<char>>` (line 70)
- Call sites: lines 209, 279, 292

## Acceptance Criteria Status

✅ **PASS**: CMAP entry creation code location identified
✅ **PASS**: Function/module name documented (ToUnicodeMap::add_mapping)
✅ **PASS**: Line numbers noted (lines 86-88, plus call sites at 209, 279, 292)
✅ **PASS**: Understanding of how entries are currently added (via HashMap::insert)
