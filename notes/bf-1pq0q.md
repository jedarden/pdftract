# ToUnicode Entry Creation Point - Location Analysis

**Bead ID**: bf-1pq0q
**Date**: 2026-07-06
**Status**: Complete

## Task Summary

Located and documented the exact locations where ToUnicode entries are created in the pdftract codebase. This is foundational work before implementing skip logic for unmapped glyphs.

## Key Findings

### 1. ToUnicode CMap Storage (Primary Entry Point)

**File**: `crates/pdftract-core/src/font/cmap.rs`
**Function**: `ToUnicodeMap::add_mapping()`
**Lines**: 86-88
**Purpose**: Core method where individual ToUnicode mappings are stored

```rust
/// MARKER: CMAP entry creation point - this is where individual ToUnicode
/// mappings are stored. Called by parse_beginbfchar() and parse_beginbfrange().
/// See notes/bf-e4uvb-child-1.md for documentation.
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
    self.mappings.insert(src, dst);
}
```

**Data Structure**: `HashMap<Vec<u8>, Vec<char>>`
- Key: Source byte sequence (1-4 bytes)
- Value: Destination Unicode codepoints (supports multi-codepoint mappings like ligatures)

**Call Sites**:
- `parse_beginbfchar()` (line 209) - Single character mappings
- `parse_beginbfrange()` (lines 279, 292) - Range mappings (explicit array and contiguous forms)

### 2. ToUnicode Resolution Entry Creation (Level 1)

**File**: `crates/pdftract-core/src/font/resolver.rs`
**Function**: `resolve_level1()`
**Lines**: 397-401
**Purpose**: Creates `ResolvedGlyph` with `UnicodeSource::ToUnicode` (confidence 1.0)

```rust
// MARKER: ToUnicode entry creation point - Level 1 CMap lookup success.
// Creates ResolvedGlyph with UnicodeSource::ToUnicode (confidence 1.0).
// See notes/bf-2nob5-child-1.md for documentation.
ResolvedGlyph::new(SmallVec::from_slice(chars), UnicodeSource::ToUnicode)
```

**Context**: This is in the 4-level encoding fallback chain:
- Level 1: ToUnicode CMap (confidence 1.0) - **THIS POINT**
- Level 2: Named encoding + AGL (confidence 0.9)
- Level 3: Font fingerprint cache (confidence 0.85)
- Level 4: Shape recognition (confidence 0.7)

## Data Flow

```
PDF Font Dictionary (/ToUnicode stream)
        │
        ▼
CMapParser::parse() (cmap.rs:136)
        │
   ┌────┴────┐
   │         │
   ▼         ▼
parse_beginbfchar  parse_beginbfrange
   │         │
   └────┬────┘
        │
        ▼
ToUnicodeMap::add_mapping() (cmap.rs:86) ← STORAGE
        │
        ▼
ToUnicodeMap.mappings (HashMap)
        │
        ▼
Content Stream Text Extraction
        │
        ▼
resolve_unicode() (resolver.rs:302)
        │
        ▼
resolve_level1() (resolver.rs:383)
        │
        ▼
cmap.lookup(char_code) (line 388)
        │
        ▼
ResolvedGlyph::new(..., UnicodeSource::ToUnicode) ← RESOLUTION ENTRY (resolver.rs:401)
```

## Existing Entry Filtering Logic

The current implementation already includes some filtering logic in `resolve_level1()`:

```rust
// Empty result or U+FFFD only -> fall through
if chars.is_empty() || (chars.len() == 1 && chars[0] == '\u{FFFD}') {
    return ResolvedGlyph::failure();
}
```

This means:
- Empty mappings fall through to Level 2
- U+FFFD-only mappings fall through to Level 2
- Only non-empty, non-U+FFFD entries are created as `UnicodeSource::ToUnicode`

## Where to Add Skip Logic

Based on this analysis, the appropriate location to add skip logic for unmapped glyphs would be:

1. **After resolution**: In text processing loops (Tj, TJ operators in `content_stream.rs`)
2. **Check condition**: When `resolved_glyph.source == UnicodeSource::Unknown` and `resolved_glyph.chars[0] == '\u{FFFD}'`
3. **Action**: Skip or filter the glyph before appending to output text

**Example locations to check**:
- `crates/pdftract-core/src/content_stream.rs` - `process_string_with_ctm()` function
- `crates/pdftract-core/src/content_stream.rs` - `process_tj_array()` function

## Related Documentation

- **Comprehensive CMAP/ToUnicode documentation**: `notes/bf-2nob5-child-1.md`
- **CMAP entry creation**: `notes/bf-e4uvb-child-1.md`
- **ToUnicode data flow**: `notes/bf-e4uvb-child-4-tounicode-flow.md`

## Verification

✅ Located primary ToUnicode entry creation point (`ToUnicodeMap::add_mapping()`)
✅ Located resolution entry creation point (`resolve_level1()`)
✅ Documented data flow from PDF to text extraction
✅ Identified existing filtering logic
✅ Identified where to add future skip logic
✅ All locations have inline MARKER comments for reference

## References

- **File**: `/home/coding/pdftract/crates/pdftract-core/src/font/cmap.rs`
- **File**: `/home/coding/pdftract/crates/pdftract-core/src/font/resolver.rs`
- **Function**: `ToUnicodeMap::add_mapping()` (cmap.rs:86-88)
- **Function**: `resolve_level1()` (resolver.rs:383-402)
- **Related note**: `notes/bf-2nob5-child-1.md` (comprehensive CMAP/ToUnicode documentation)
