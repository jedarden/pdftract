# CMAP and ToUnicode Entry Creation Points

**Bead ID**: bf-e4uvb-child-1
**Date**: 2026-07-05
**Status**: Complete

## Overview

This document identifies the exact locations in the pdftract codebase where CMAP and ToUnicode entries are created for glyphs. These are the critical points where skip logic for unmapped glyphs would need to be added.

## Detailed Entry Creation Points

### 1. ToUnicode Entry Creation (PRIMARY)

**File**: `crates/pdftract-core/src/font/cmap.rs`

#### Core Entry Creation Method
**Function**: `ToUnicodeMap::add_mapping()`
**Line**: 86-88
**Code**:
```rust
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
    self.mappings.insert(src, dst);
}
```

**MARKER**: This is where individual ToUnicode mappings are stored. Each mapping associates a source byte sequence (glyph identifier) with destination Unicode codepoints.

**Called from**:
- `parse_beginbfchar()` at line 209 - single-character mappings
- `parse_beginbfrange()` at lines 279 and 292 - range mappings (both explicit array and contiguous forms)

#### ToUnicode Parsing Flow
1. **Entry point**: `CMapParser::parse()` (line 136)
2. **beginbfchar blocks**: → `parse_beginbfchar()` (line 190)
   - Reads count
   - Loops through `<src> <dst>` pairs
   - Calls `map.add_mapping(src, dst)` for each pair (line 209)
3. **beginbfrange blocks**: → `parse_beginbfrange()` (line 223)
   - Contiguous form: expands range, calls `map.add_mapping()` (line 292)
   - Explicit array form: maps each array element, calls `map.add_mapping()` (line 279)

### 2. CMAP Entry Creation - Type1 Font Encoding

**File**: `crates/pdftract-core/src/font/encoding.rs`

#### Differences Overlay Parsing
**Function**: `DifferencesOverlay::parse()`
**Line**: 163-207
**Entry creation**: Line 195
**Code**:
```rust
overlay.entries.push((cursor as u8, Arc::clone(name)));
```

**MARKER**: This is where Type1 font encoding entries are created from `/Differences` arrays in font encoding dictionaries.

**Parsing Flow**:
1. **Input**: PDF object array like `[39 /quotesingle 96 /grave]`
2. **Loop**: Process each element
   - Integer: updates cursor position (lines 174-190)
   - Name: assigns glyph name to current cursor position (lines 192-197)
3. **Entry creation**: `overlay.entries.push((cursor as u8, Arc::clone(name)))` (line 195)
4. **Cursor**: auto-increments after each name assignment (line 197)

#### Named Encoding Tables
**Location**: Generated at build time from `OUT_DIR/named_encodings.rs`
**Supported encodings**:
- WinAnsiEncoding (Windows-1252)
- MacRomanEncoding
- MacExpertEncoding
- StandardEncoding
- SymbolEncoding
- ZapfDingbatsEncoding

### 3. Codespace Range Entry Creation

**File**: `crates/pdftract-core/src/font/codespace.rs`

#### Codespace Range Parsing
**Function**: `CodespaceParser::parse_codespace_block()`
**Line**: 222-278
**Range creation**: Line 256
**Code**:
```rust
if let Some(range) = CodespaceRange::new(lo, hi) {
    ranges.add(range);
    parsed += 1;
}
```

**MARKER**: This is where codespace range entries are created from `begincodespacerange...endcodespacerange` blocks in CMap streams.

**Parsing Flow**:
1. **Parse count** (optional, lines 224-234)
2. **Loop**: Parse pairs of hex strings: `<lo> <hi>`
3. **Create range**: `CodespaceRange::new(lo, hi)` (line 255)
4. **Add to collection**: `ranges.add(range)` (line 256)

## Key Findings

### 1. ToUnicode CMap Entry Creation

**File:** `crates/pdftract-core/src/font/cmap.rs`

#### Entry Point: `ToUnicodeMap::add_mapping()`
- **Location:** `cmap.rs:82-84`
- **Purpose:** Adds a single mapping from source byte sequence to destination Unicode characters
- **Signature:**
  ```rust
  pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>)
  ```
- **Data structure:** Uses `HashMap<Vec<u8>, Vec<char>>` for storage

#### ToUnicode Stream Parsing: `parse_beginbfchar()`
- **Location:** `cmap.rs:186-212`
- **Purpose:** Parses `beginbfchar...endbfchar` blocks (single-character mappings)
- **Format:** `beginbfchar <count> <src1> <dst1> <src2> <dst2> ... endbfchar`
- **Calls:** `map.add_mapping(src, dst)` for each mapping pair (line 205)

#### ToUnicode Range Parsing: `parse_beginbfrange()`
- **Location:** `cmap.rs:219-305`
- **Purpose:** Parses `beginbfrange...endbfrange` blocks (range mappings)
- **Two forms:**
  1. Contiguous: `<lo> <hi> <dst>` - auto-increments destination (lines 281-298)
  2. Explicit array: `<lo> <hi> [<d0> <d1> ...]` - explicit destination list (lines 243-279)
- **Calls:** `map.add_mapping()` for each expanded mapping (lines 275, 288)

#### Convenience Functions
- **Location:** `cmap.rs:489-501`
- `parse_to_unicode(input: &[u8]) -> ToUnicodeMap` - parses, discarding diagnostics
- `parse_to_unicode_with_diags(input: &[u8]) -> (ToUnicodeMap, Vec<Diagnostic>)` - parses with diagnostics

### 2. Font Encoding Entry Creation

**File:** `crates/pdftract-core/src/font/encoding.rs`

#### Named Encoding Tables
- **Location:** `encoding.rs:1-19` (module documentation)
- **Generated from:** `OUT_DIR/named_encodings.rs` (build-time code generation)
- **Supported encodings:**
  - WinAnsiEncoding (Windows-1252)
  - MacRomanEncoding
  - MacExpertEncoding
  - StandardEncoding
  - SymbolEncoding
  - ZapfDingbatsEncoding

#### Differences Overlay Parsing: `DifferencesOverlay::parse()`
- **Location:** `encoding.rs:163-207`
- **Purpose:** Parses `/Differences` arrays that override specific character codes
- **Format:** `[n /Name1 /Name2 ... m /OtherName ...]`
- **Data structure:** `Vec<(u8, Arc<str>)` - sparse list of (code, glyph_name) overrides
- **Entry creation:** Line 195 - `overlay.entries.push((cursor as u8, Arc::clone(name)))`

#### Font Encoding Construction: `FontEncoding::parse_from_font()`
- **Location:** `encoding.rs:288-331`
- **Purpose:** Parses `/Encoding` dictionary from font, combining base encoding + differences
- **Process:**
  1. Get `/Encoding` entry (line 294)
  2. If name → use named encoding directly (lines 301-304)
  3. If dict → parse `/BaseEncoding` (lines 309-313) and `/Differences` (lines 316-319)
  4. Return `FontEncoding { base, differences }`

### 3. Glyph Resolution (Usage, Not Creation)

**File:** `crates/pdftract-core/src/font/resolver.rs`

#### 4-Level Resolution Chain
- **Entry point:** `resolve_unicode()` (resolver.rs:296-377)
- **Level 1 (ToUnicode):** `resolve_level1()` (lines 383-399)
  - Calls `cmap.lookup(char_code)` (line 388)
- **Level 2 (Named encoding + AGL):** `resolve_level2()` (lines 404-434)
  - Calls `enc.glyph_name_for(code)` (line 417)
  - Then calls `unicode_for_glyph_name_multi()` or `unicode_for_glyph_name()` (lines 423, 428)
- **Level 3 (Fingerprint):** `resolve_level3()` (lines 444-464)
- **Level 4 (Shape recognition):** `resolve_level4()` (lines 472-479) - stub

### 4. Content Stream Processing (Where Mappings Are Used)

**File:** `crates/pdftract-core/src/content_stream.rs`

#### Text Operators
- **Tj operator:** Lines 571-595 (simplified), 1339-1363 (with CTM)
- **TJ operator:** Lines 596-620 (simplified), 1364-1402 (with CTM)
- **String processing:** `process_string_with_ctm()` (lines 1697-1736)
- **TJ array processing:** `process_tj_array()` (lines 1738-1850)

**Current implementation note:** As of line 1727, the code uses `String::from_utf8_lossy(bytes)` as a placeholder, not the full ToUnicode resolution path.

## Data Flow Summary

```
PDF Font Dictionary
    ↓
FontEncoding::parse_from_font() (encoding.rs:288)
    ├→ NamedEncoding::table() - static 256-element arrays
    └→ DifferencesOverlay::parse() - sparse overrides
    ↓
ToUnicode CMap Stream
    ↓
CMapParser::parse() (cmap.rs:132)
    ├→ parse_beginbfchar() - single mappings
    │   └→ ToUnicodeMap::add_mapping() (cmap.rs:82)
    └→ parse_beginbfrange() - range mappings
        └→ ToUnicodeMap::add_mapping() (cmap.rs:82)
    ↓
Text Extraction
    ↓
resolve_unicode() (resolver.rs:296)
    ├→ Level 1: cmap.lookup() - uses ToUnicodeMap
    ├→ Level 2: enc.glyph_name_for() + AGL lookup
    └→ Levels 3-4: fingerprint, shape recognition
```

## Future Work: Unmapped Glyph Skip Logic

**Parent bead:** `bf-e4uvb` (implement unmapped glyph skip logic)

To add skip logic for unmapped glyphs:

1. **Insertion point:** After `resolve_unicode()` returns `ResolvedGlyph` with `UnicodeSource::Unknown`
2. **Check location:** In text processing loops (Tj, TJ operators)
3. **Skip condition:** When `resolved_glyph.codepoint == '\u{FFFD}'` and confidence is 0.0
4. **Implementation:** Add early return or continue statement before glyph emission

## Related Files

- `crates/pdftract-core/src/font/cmap.rs` - ToUnicode CMap parsing and storage
- `crates/pdftract-core/src/font/encoding.rs` - Named encodings and differences overlay
- `crates/pdftract-core/src/font/resolver.rs` - 4-level resolution chain
- `crates/pdftract-core/src/content_stream.rs` - Text extraction operators
- `crates/pdftract-core/src/glyph/mod.rs` - Glyph struct definition
