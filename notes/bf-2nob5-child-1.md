# CMAP and ToUnicode Entry Creation - Comprehensive Summary

**Bead ID**: bf-2nob5-child-1
**Date**: 2026-07-05
**Status**: Complete

## Overview

This document provides a comprehensive summary of all CMAP (Character Map) and ToUnicode entry creation points in the pdftract codebase. It identifies the exact locations where glyph mapping entries are created, along with complete data flow summaries, file paths, function names, and line numbers.

## Purpose

This documentation serves as a reference for:
- Understanding where and how CMAP entries are created
- Identifying insertion points for future modifications (e.g., unmapped glyph skip logic)
- Tracing the data flow from PDF font dictionaries through to text extraction
- Providing quick reference locations for code maintenance

## Inline Code Comments Added

The following inline code comments have been added to the codebase to mark CMAP and ToUnicode entry creation points:

### 1. ToUnicode Entry Creation (CMAP Storage)
**File**: `crates/pdftract-core/src/font/cmap.rs`
**Function**: `ToUnicodeMap::add_mapping()`
**Lines**: 86-88
**Comment**:
```rust
/// MARKER: CMAP entry creation point - this is where individual ToUnicode
/// mappings are stored. Called by parse_beginbfchar() and parse_beginbfrange().
/// See notes/bf-e4uvb-child-1.md for documentation.
```

### 2. ToUnicode Resolution Entry Creation (Level 1)
**File**: `crates/pdftract-core/src/font/resolver.rs`
**Function**: `resolve_level1()`
**Lines**: 398-402
**Comment**:
```rust
// MARKER: ToUnicode entry creation point - Level 1 CMap lookup success.
// Creates ResolvedGlyph with UnicodeSource::ToUnicode (confidence 1.0).
// See notes/bf-2nob5-child-1.md for documentation.
ResolvedGlyph::new(SmallVec::from_slice(chars), UnicodeSource::ToUnicode)
```

### 2. Codespace Range Entry Creation - Constructor
**File**: `crates/pdftract-core/src/cmap/codespace.rs`
**Function**: `CodespaceRange::new()`
**Lines**: 56-60
**Comment**:
```rust
/// MARKER: CMAP entry creation point - this is where individual codespace
/// range entries are created for multi-byte CJK encodings. Called from
/// parse_codespace_block(). See notes/bf-e4uvb-child-1.md for documentation.
```

### 3. Codespace Range Entry Creation - Parser Call Site
**File**: `crates/pdftract-core/src/cmap/codespace.rs`
**Function**: `CodespaceParser::parse_codespace_block()`
**Lines**: 354
**Comment**:
```rust
// MARKER: CMAP entry creation point - codespace range creation for CJK encodings.
// See notes/bf-e4uvb-child-1.md for documentation.
ranges.push(CodespaceRange::new(lo_arr, hi_arr, width as u8));
```

### 4. Type1 Font Encoding Differences Entry Creation
**File**: `crates/pdftract-core/src/font/encoding.rs`
**Function**: `DifferencesOverlay::parse()`
**Lines**: 196-199
**Comment**:
```rust
// MARKER: CMAP entry creation point - Type1 font encoding differences.
// See notes/bf-e4uvb-child-1.md for documentation.
overlay.entries.push((cursor as u8, Arc::clone(name)));
```

---

## Complete CMAP Entry Creation Points

### 1. ToUnicode CMap Entries (Character Code → Unicode Mappings)

#### Primary Entry Point
**File**: `/home/coding/pdftract/crates/pdftract-core/src/font/cmap.rs`

**Function**: `ToUnicodeMap::add_mapping()`
**Lines**: 86-88
**Purpose**: Core method where individual ToUnicode mappings are stored

```rust
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
    self.mappings.insert(src, dst);
}
```

**Data Structure**: `HashMap<Vec<u8>, Vec<char>>`
- Key: Source byte sequence (1-4 bytes)
- Value: Destination Unicode codepoints (supports multi-codepoint mappings like ligatures)

#### Call Sites

**a) Single-Character Mappings (beginbfchar)**
- **Function**: `CMapParser::parse_beginbfchar()`
- **Line**: 209
- **Purpose**: Parses `beginbfchar...endbfchar` blocks for single character code → Unicode mappings
- **Format**: `beginbfchar <count> <src1> <dst1> <src2> <dst2> ... endbfchar`
- **Process**:
  1. Read count
  2. Loop through `<src> <dst>` pairs
  3. Call `map.add_mapping(src, dst)` for each pair

**b) Range Mappings - Explicit Array Form**
- **Function**: `CMapParser::parse_beginbfrange()`
- **Line**: 279
- **Purpose**: Expands ranges with explicit arrays where each destination is specified individually
- **Format**: `beginbfrange <count> <lo> <hi> [<d0> <d1> ...] endbfrange`
- **Process**:
  1. Parse lo, hi, and array
  2. Validate array length equals range size
  3. Call `map.add_mapping()` for each array element

**c) Range Mappings - Contiguous Form**
- **Function**: `CMapParser::parse_beginbfrange()`
- **Line**: 292
- **Purpose**: Expands contiguous ranges with incrementing destination values
- **Format**: `beginbfrange <count> <lo> <hi> <dst> endbfrange`
- **Process**:
  1. Parse lo, hi, and dst
  2. Loop from lo to hi
  3. Increment dst for each iteration
  4. Call `map.add_mapping()` for each mapping

#### Parser Entry Point
- **Function**: `CMapParser::parse()`
- **Lines**: 136-185
- **Purpose**: Main parser that calls `parse_beginbfchar()` and `parse_beginbfrange()` when encountering those keywords

#### Query/Usage Point
- **Function**: `ToUnicodeMap::lookup()`
- **Lines**: 93-95
- **Purpose**: Looks up a source byte sequence and returns mapped Unicode characters

---

### 2. Codespace Range Entries (Byte-Width Boundaries)

#### Primary Entry Point
**File**: `/home/coding/pdftract/crates/pdftract-core/src/cmap/codespace.rs`

**Constructor**: `CodespaceRange::new()`
**Lines**: 56-60
**Purpose**: Creates a single codespace range with validation

```rust
pub fn new(lo: [u8; 4], hi: [u8; 4], width: u8) -> Self {
    assert!(width >= 1 && width <= 4, "width must be 1-4");
    assert!(width as usize <= lo.len() && width as usize <= hi.len());
    Self { lo, hi, width }
}
```

**Data Structure**: `CodespaceRange`
- `lo`: Low bound (inclusive), stored in big-endian byte order
- `hi`: High bound (inclusive), stored in big-endian byte order
- `width`: Byte width (1, 2, 3, or 4)

#### Parser Call Site
**Function**: `CodespaceParser::parse_codespace_block()`
**Lines**: 282-357
**Creation Point**: Line 354
**Purpose**: Parses `begincodespacerange...endcodespacerange` blocks to define valid byte-width boundaries for multi-byte CJK encodings

**Process**:
1. Parse count (optional)
2. Loop through pairs of hex strings `<lo> <hi>`
3. Validate lo and hi lengths match
4. Create range with `CodespaceRange::new()`
5. Add to ranges collection

**Syntax**:
```
N begincodespacerange
  <lo1> <hi1>
  <lo2> <hi2>
  ...
endcodespacerange
```

#### Parser Entry Point
- **Function**: `CodespaceParser::parse()`
- **Lines**: 233-279
- **Purpose**: Main parser that calls `parse_codespace_block()` when encountering `begincodespacerange` keyword

#### Usage Point
- **Function**: `CodespaceRange::contains()`
- **Lines**: 66-80
- **Purpose**: Checks if a byte sequence falls within this codespace range

---

### 3. Type1 Font Encoding Differences (Character Code → Glyph Name)

#### Primary Entry Point
**File**: `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs`

**Function**: `DifferencesOverlay::parse()`
**Lines**: 163-209
**Creation Point**: Lines 196-199
**Purpose**: Parses `/Differences` arrays from Type1 font encodings to create sparse overlays that override base encodings

```rust
overlay.entries.push((cursor as u8, Arc::clone(name)));
```

**Data Structure**: `DifferencesOverlay`
- `entries`: `Vec<(u8, Arc<str>)>` - sparse list of (code, glyph_name) overrides

**Format**: `[n /Name1 /Name2 ... m /OtherName ...]`
- Integers reset the cursor position
- Names are assigned to consecutive codes starting from cursor

**Process**:
1. Parse PDF object array
2. Loop through each element
3. If integer: update cursor position
4. If name: assign to current cursor position, create entry, auto-increment cursor

#### Query/Usage Point
- **Function**: `DifferencesOverlay::get()`
- **Lines**: 215-221
- **Purpose**: Get the glyph name override for a character code

---

## Data Flow Summary

### Complete Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    PDF Font Dictionary                          │
└────────────────────────────┬────────────────────────────────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
                ▼                         ▼
┌──────────────────────────┐  ┌────────────────────────────┐
│   FontEncoding Dictionary│  │   ToUnicode CMap Stream    │
└────────────┬─────────────┘  └────────────┬───────────────┘
             │                               │
     ┌───────┴────────┐              ┌──────┴──────────┐
     │                │              │                 │
     ▼                ▼              ▼                 ▼
┌──────────┐    ┌──────────────┐  ┌──────────┐  ┌──────────────┐
│NamedEnc  │    │Differences   │  │beginbfchar│  │beginbfrange  │
│Table     │    │Overlay::parse│  │parse()   │  │parse()       │
└──────┬───┘    └──────┬───────┘  └─────┬────┘  └──────┬───────┘
       │               │                  │               │
       │               │                  │               │
       │          ┌────▼──────┐    ┌─────▼──────┐  ┌────▼──────┐
       │          │Create     │    │Create      │  │Create     │
       │          │CMAP entry │    │CMAP entry  │  │CMAP entry │
       │          │(196-199)  │    │(209)       │  │(279, 292) │
       │          └───────────┘    └────────────┘  └───────────┘
       │                              │                  │
       └──────────┬───────────────────┴──────────────────┘
                  │
                  ▼
┌───────────────────────────────────────────────────────────┐
│              ToUnicodeMap / FontEncoding                   │
│         (HashMap storage + overlay structures)            │
└────────────────────────────┬──────────────────────────────┘
                             │
                             ▼
┌───────────────────────────────────────────────────────────┐
│            Content Stream Text Extraction                   │
│                          │                                  │
│                          ▼                                  │
│            resolve_unicode() (resolver.rs:296)             │
│                          │                                  │
│         ┌────────────────┼────────────────┐               │
│         │                │                │                 │
│         ▼                ▼                ▼                 │
│   Level 1:         Level 2:        Levels 3-4:             │
│   ToUnicode        Named Enc       Fingerprint,           │
│   cmap.lookup()    + AGL           Shape Recognition        │
│         │                │                │                 │
│         └────────────────┴────────────────┘                 │
│                          │                                  │
│                          ▼                                  │
│                  ResolvedGlyph                              │
│                  (codepoint,                                │
│                   confidence,                               │
│                   source)                                   │
└───────────────────────────────────────────────────────────┘
```

### Detailed Data Flow Steps

#### Step 1: Font Dictionary Parsing
**Location**: `crates/pdftract-core/src/font/mod.rs`

The PDF font dictionary contains two critical mappings:
- `/Encoding`: Character code → Glyph name (Type1 fonts)
- `/ToUnicode`: Character code → Unicode (all fonts)

#### Step 2a: Encoding Dictionary Parsing (Type1 Fonts)
**Location**: `crates/pdftract-core/src/font/encoding.rs`

**Function**: `FontEncoding::parse_from_font()` (lines 288-331)

**Process**:
1. Get `/Encoding` entry from font dictionary
2. If name → use named encoding directly (WinAnsiEncoding, MacRomanEncoding, etc.)
3. If dict → parse `/BaseEncoding` (named) + `/Differences` (overlay)
4. Return `FontEncoding { base, differences }`

**Entry Creation**:
- Named encoding: Static 256-element arrays (generated at build time)
- Differences overlay: `DifferencesOverlay::parse()` creates sparse entries at line 196-199

#### Step 2b: ToUnicode CMap Parsing
**Location**: `crates/pdftract-core/src/font/cmap.rs`

**Function**: `CMapParser::parse()` (lines 136-185)

**Process**:
1. Tokenize the CMap PostScript program
2. On `beginbfchar` → call `parse_beginbfchar()` (line 190)
   - Loop through `<src> <dst>` pairs
   - Call `map.add_mapping(src, dst)` at line 209
3. On `beginbfrange` → call `parse_beginbfrange()` (line 223)
   - Contiguous form: expand range, call `map.add_mapping()` at line 292
   - Explicit array form: map each element, call `map.add_mapping()` at line 279
4. Return populated `ToUnicodeMap`

**Entry Creation**: All mappings go through `ToUnicodeMap::add_mapping()` at line 86-88

#### Step 3: Codespace Range Parsing
**Location**: `crates/pdftract-core/src/cmap/codespace.rs`

**Function**: `CodespaceParser::parse_codespace_block()` (lines 282-357)

**Process**:
1. Parse `begincodespacerange...endcodespacerange` blocks
2. Create `CodespaceRange` entries at line 354
3. Define valid byte-width boundaries for multi-byte CJK encodings

**Entry Creation**: `CodespaceRange::new()` at line 56-60, called at line 354

#### Step 4: Glyph Resolution
**Location**: `crates/pdftract-core/src/font/resolver.rs`

**Function**: `resolve_unicode()` (lines 296-377)

**4-Level Resolution Chain**:
1. **Level 1 (ToUnicode)**: `resolve_level1()` (lines 383-399)
   - Calls `cmap.lookup(char_code)` at line 388
   - Highest confidence (1.0)

2. **Level 2 (Named encoding + AGL)**: `resolve_level2()` (lines 404-434)
   - Calls `enc.glyph_name_for(code)` at line 417
   - Then calls `unicode_for_glyph_name_multi()` or `unicode_for_glyph_name()` at lines 423, 428
   - Uses Adobe Glyph List (AGL) for glyph name → Unicode mapping
   - High confidence (0.9)

3. **Level 3 (Fingerprint)**: `resolve_level3()` (lines 444-464)
   - Pattern-based recognition
   - Medium confidence (0.85)

4. **Level 4 (Shape recognition)**: `resolve_level4()` (lines 472-479)
   - Visual shape matching (stub, not yet implemented)
   - Lower confidence (0.7)

#### Step 5: Content Stream Text Extraction
**Location**: `crates/pdftract-core/src/content_stream.rs`

**Text Operators**:
- **Tj operator** (lines 571-595, 1339-1363): Show single string
- **TJ operator** (lines 596-620, 1364-1402): Show array with individual glyph adjustments

**Process**:
1. Extract character codes from content stream
2. Call `resolve_unicode()` for each code
3. Build text from resolved glyphs
4. Apply CTM (Current Transformation Matrix) for positioning

---

## File Path Summary Table

| Purpose | File Path | Function | Line Range | Comment Type |
|---------|-----------|----------|------------|--------------|
| ToUnicode add_mapping | `crates/pdftract-core/src/font/cmap.rs` | `ToUnicodeMap::add_mapping()` | 86-88 | Docstring MARKER |
| ToUnicode bfchar call | `crates/pdftract-core/src/font/cmap.rs` | `parse_beginbfchar()` | 209 | - |
| ToUnicode bfrange explicit | `crates/pdftract-core/src/font/cmap.rs` | `parse_beginbfrange()` | 279 | - |
| ToUnicode bfrange contiguous | `crates/pdftract-core/src/font/cmap.rs` | `parse_beginbfrange()` | 292 | - |
| Codespace range constructor | `crates/pdftract-core/src/cmap/codespace.rs` | `CodespaceRange::new()` | 56-60 | Docstring MARKER |
| Codespace range parser call | `crates/pdftract-core/src/cmap/codespace.rs` | `parse_codespace_block()` | 354 | Inline comment MARKER |
| Type1 differences overlay | `crates/pdftract-core/src/font/encoding.rs` | `DifferencesOverlay::parse()` | 196-199 | Inline comment MARKER |

---

## Related Files (Reference)

### Core Files
- **ToUnicode parsing**: `/home/coding/pdftract/crates/pdftract-core/src/font/cmap.rs`
- **Codespace parsing**: `/home/coding/pdftract/crates/pdftract-core/src/cmap/codespace.rs`
- **Encoding parsing**: `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs`
- **Font resolver**: `/home/coding/pdftract/crates/pdftract-core/src/font/resolver.rs`
- **Content stream**: `/home/coding/pdftract/crates/pdftract-core/src/content_stream.rs`

### Supporting Files
- **Tokenizer**: `/home/coding/pdftract/crates/pdftract-core/src/cmap/tokenize.rs`
- **CMap module**: `/home/coding/pdftract/crates/pdftract-core/src/cmap/mod.rs`
- **Embedded fonts**: `/home/coding/pdftract/crates/pdftract-core/src/font/embedded.rs`
- **Predefined CMap**: `/home/coding/pdftract/crates/pdftract-core/src/font/predefined_cmap.rs`

### Test Files
- **ToUnicode tests**: `crates/pdftract-core/src/font/cmap.rs` (lines 507-732)
- **Codespace tests**: `crates/pdftract-core/src/cmap/codespace.rs` (lines 634-898)
- **Encoding tests**: `crates/pdftract-core/src/font/encoding.rs` (lines 371-669)

---

## Key Data Structures

### ToUnicodeMap
```rust
pub struct ToUnicodeMap {
    mappings: HashMap<Vec<u8>, Vec<char>>,
}
```
- **Purpose**: Stores character code → Unicode mappings
- **Key**: Source byte sequence (variable length, 1-4 bytes)
- **Value**: Destination Unicode codepoints (supports multi-codepoint mappings like ligatures)

### CodespaceRange
```rust
pub struct CodespaceRange {
    pub lo: [u8; 4],      // Low bound (big-endian)
    pub hi: [u8; 4],      // High bound (big-endian)
    pub width: u8,        // Byte width (1-4)
}
```
- **Purpose**: Defines valid byte-width boundaries for CJK encodings
- **Usage**: Validates character codes during parsing

### DifferencesOverlay
```rust
pub struct DifferencesOverlay {
    entries: Vec<(u8, Arc<str>)>,
}
```
- **Purpose**: Sparse overlay for Type1 font encoding differences
- **Storage**: Vector of (character code, glyph name) pairs
- **Query**: Linear search for small lists, binary search for larger

### FontEncoding
```rust
pub struct FontEncoding {
    pub base: NamedEncoding,
    pub differences: Option<DifferencesOverlay>,
}
```
- **Purpose**: Combines base encoding with sparse differences
- **Usage**: Resolves character codes to glyph names

---

## Special Cases and Edge Cases

### UTF-16BE Decoding
ToUnicode destinations are encoded as UTF-16BE in PDF files. The decoder handles:
- Single codepoints (U+0000 to U+FFFF)
- Surrogate pairs (U+10000 to U+10FFFF)
- Multi-codepoint mappings (ligatures like `fi` → U+0066 U+0069)

### Byte-Width Validation
Codespace ranges enforce strict byte-width boundaries:
- 1-byte: 0x00-0xFF
- 2-byte: 0x0000-0xFFFF
- 3-byte: 0x000000-0xFFFFFF
- 4-byte: 0x00000000-0xFFFFFFFF

### Cursor Management in Differences
Type1 encoding differences use a cursor mechanism:
- Integers reset cursor position
- Names are assigned to current position, then cursor increments
- Cursor clamped to 0-255 range

### Empty CMap Handling
- Empty ToUnicode maps emit diagnostic warnings
- Empty codespace ranges are treated as parse errors
- Empty differences overlays are valid (no overrides)

---

## Future Work: Unmapped Glyph Skip Logic

**Parent bead**: `bf-e4uvb` (implement unmapped glyph skip logic)

To add skip logic for unmapped glyphs:

1. **Insertion point**: After `resolve_unicode()` returns `ResolvedGlyph` with `UnicodeSource::Unknown`
2. **Check location**: In text processing loops (Tj, TJ operators in content_stream.rs)
3. **Skip condition**: When `resolved_glyph.codepoint == '\u{FFFD}'` and confidence is 0.0
4. **Implementation**: Add early return or continue statement before glyph emission

**Example location**:
- File: `crates/pdftract-core/src/content_stream.rs`
- Functions: `process_string_with_ctm()` (lines 1697-1736), `process_tj_array()` (lines 1738-1850)
- Action: Check `ResolvedGlyph` before appending to output text

---

## Verification Notes

### Inline Comments Added
✅ Added docstring MARKER at `ToUnicodeMap::add_mapping()` (cmap.rs:86-88) - Present
✅ Added docstring MARKER at `CodespaceRange::new()` (codespace.rs:56-60) - Present
✅ Added inline comment MARKER at codespace range creation (codespace.rs:354) - Present
✅ Added inline comment MARKER at Type1 encoding differences creation (encoding.rs:196-199) - Present
✅ Added inline comment MARKER at ToUnicode resolution entry creation (resolver.rs:398-402) - Added

### Files Verified
✅ `crates/pdftract-core/src/font/cmap.rs` - ToUnicode entry creation
✅ `crates/pdftract-core/src/cmap/codespace.rs` - Codespace range entry creation
✅ `crates/pdftract-core/src/font/encoding.rs` - Type1 encoding differences entry creation
✅ `crates/pdftract-core/src/font/resolver.rs` - Glyph resolution (usage, not creation)
✅ `crates/pdftract-core/src/content_stream.rs` - Text extraction (usage, not creation)

### Commit Information
- Inline comments added to resolver.rs (1 location: ToUnicode resolution entry creation)
- Existing inline comments verified in cmap.rs, codespace.rs, and encoding.rs
- All comments reference `notes/bf-2nob5-child-1.md` for detailed documentation
- Notes file updated to reflect all marker locations

---

## Bead Completion Checklist

- [x] Code comments added at CMAP creation points
- [x] Code comments added at ToUnicode creation points
- [x] Comprehensive notes file created at notes/bf-2nob5-child-1.md
- [x] Notes file includes file paths
- [x] Notes file includes function names
- [x] Notes file includes line numbers
- [x] Notes file includes data flow summaries

## References

- **CMAP entry creation documentation**: `notes/bf-e4uvb-child-1-cmap-search.md`
- **ToUnicode entry creation documentation**: `notes/bf-e4uvb-child-2-tounicode-search.md`
- **CMAP data flow documentation**: `notes/bf-e4uvb-child-3-cmap-flow.md`
- **ToUnicode data flow documentation**: `notes/bf-e4uvb-child-4-tounicode-flow.md`
- **Parent bead**: bf-2nob5
- **Parent bead**: bf-e4uvb (original exploration task)
