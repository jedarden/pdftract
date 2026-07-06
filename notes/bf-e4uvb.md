# CMAP and ToUnicode Entry Creation Points

## Overview
This document identifies all locations in the pdftract codebase where CMAP and ToUnicode entries are created for glyphs. These are the key insertion points for future skip logic for unmapped glyphs.

## Entry Creation Points

### 1. CMAP Codespace Range Creation
**File:** `crates/pdftract-core/src/cmap/codespace.rs`

#### Location 1: `CodespaceRange::new()` (line 60)
```rust
pub fn new(lo: [u8; 4], hi: [u8; 4], width: u8) -> Self
```
- **Purpose:** Creates individual codespace range entries for multi-byte CJK encodings
- **Called from:** `parse_codespace_block()` 
- **Data flow:** Parses PostScript `begincodespacerange`/`endcodespacerange` blocks, creates range entries with byte-width boundaries

#### Location 2: `parse_codespace_block()` (line 356)
```rust
ranges.push(CodespaceRange::new(lo_arr, hi_arr, width as u8));
```
- **Purpose:** Creates codespace range entries during CJK encoding parsing
- **Context:** Processes 1-4 byte hex string pairs, validates widths, creates range objects

### 2. ToUnicode Mapping Creation
**File:** `crates/pdftract-core/src/font/cmap.rs`

#### Location: `ToUnicodeMap::add_mapping()` (line 86)
```rust
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>)
```
- **Purpose:** Stores individual ToUnicode mappings (source bytes → destination Unicode)
- **Called from:**
  - `parse_beginbfchar()` (line 209) - single character mappings
  - `parse_beginbfrange()` (lines 279, 292) - range mappings (contiguous and explicit array forms)
- **Data flow:** 
  - Source: Variable-length byte sequences (1-4 bytes) from PDF
  - Destination: UTF-32 codepoint slices, supporting multi-codepoint mappings like ligatures

#### Parser Entry Points:
- `parse_beginbfchar()` - Processes `beginbfchar`/`endbfchar` blocks
- `parse_beginbfrange()` - Processes `beginbfrange`/`endbfrange` blocks
  - Contiguous form: `<lo> <hi> <dst>` - expands range
  - Explicit array form: `<lo> <hi> [<d0> <d1> ...]` - array must match range length

### 3. Type1 Font Encoding Differences
**File:** `crates/pdftract-core/src/font/encoding.rs`

#### Location: `DifferencesOverlay::parse()` (line 197)
```rust
overlay.entries.push((cursor as u8, Arc::clone(name)));
```
- **Purpose:** Creates encoding difference entries for Type1 fonts
- **Context:** Parses `/Differences` arrays from font encoding dictionaries
- **Data flow:** Integer codes reset position, subsequent names assigned to consecutive codes
- **Example:** `[39 /quotesingle 96 /grave]` creates entries for codes 39 and 96

### 4. ToUnicode Resolution Success
**File:** `crates/pdftract-core/src/font/resolver.rs`

#### Location: Level 1 CMap Lookup (line 398)
```rust
ResolvedGlyph::new(SmallVec::from_slice(chars), UnicodeSource::ToUnicode)
```
- **Purpose:** Creates `ResolvedGlyph` when Level 1 CMap lookup succeeds
- **Context:** After successful ToUnicode mapping, creates resolved glyph with confidence 1.0
- **Data flow:** Returns resolved glyph with Unicode source attribution

## Data Flow Summary

```
PDF Font → CMap Parser → ToUnicodeMap.add_mapping() → HashMap<Vec<u8>, Vec<char>>
                                                    ↓
                                            Resolver Lookup (Level 1)
                                                    ↓
                                    ResolvedGlyph (UnicodeSource::ToUnicode)
```

## Glyph Names Processed

The system processes:
1. **ToUnicode CMap entries** - Character codes from content streams mapped via `/ToUnicode` CMap
2. **Type1 font encodings** - Glyph names from encoding tables (WinAnsi, MacRoman, etc.)
3. **CJK encodings** - Multi-byte codespace ranges for CJK character sets
4. **Custom differences** - Overrides from `/Differences` arrays

## Key Data Structures

1. **ToUnicodeMap** - HashMap<Vec<u8>, Vec<char>> stores mappings
2. **CodespaceRanges** - SmallVec of CodespaceRange objects for byte-width boundaries
3. **DifferencesOverlay** - Sparse vector of (code, glyph_name) overrides
4. **ResolvedGlyph** - Result type with Unicode source attribution

## Future Skip Logic Integration

For adding skip logic for unmapped glyphs, the key insertion points are:
1. **ToUnicodeMap::add_mapping()** - Skip adding entries for glyphs without Unicode mappings
2. **parse_beginbfchar()** - Filter source bytes before calling add_mapping()
3. **parse_beginbfrange()** - Filter range entries before expansion
4. **DifferencesOverlay::parse()** - Skip encoding entries for unmapped glyph names

The resolver already emits `GLYPH_UNMAPPED` diagnostic once per (font, code) miss, which provides the signal for skip logic.
