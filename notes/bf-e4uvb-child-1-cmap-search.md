# CMAP Entry Creation Code Locations

## Search Date
2026-07-05

## Summary
This document identifies all locations in the pdftract codebase where CMAP (Character Map) entries are created for glyphs.

## 1. ToUnicode CMap Entries (Character Code → Unicode Mappings)

### Primary Entry Point
**File:** `/home/coding/pdftract/crates/pdftract-core/src/font/cmap.rs`

**Function:** `ToUnicodeMap::add_mapping()`  
**Lines:** 86-88

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

### Call Sites

#### a) Single-Character Mappings (beginbfchar)
**Function:** `CMapParser::parse_beginbfchar()`  
**Line:** 209  
**Purpose:** Parses `beginbfchar...endbfchar` blocks for single character code → Unicode mappings

#### b) Range Mappings - Contiguous Form
**Function:** `CMapParser::parse_beginbfrange()`  
**Line:** 292  
**Purpose:** Expands contiguous ranges `<lo> <hi> <dst>` where each code in range maps to incrementing Unicode values

#### c) Range Mappings - Explicit Array Form
**Function:** `CMapParser::parse_beginbfrange()`  
**Line:** 279  
**Purpose:** Expands ranges with explicit arrays `<lo> <hi> [<d0> <d1> ...]` where each destination is specified individually

### Parser Entry Point
**Function:** `CMapParser::parse()`  
**Lines:** 136-185  
**Purpose:** Main parser that calls `parse_beginbfchar()` and `parse_beginbfrange()` when encountering those keywords

---

## 2. Codespace Range Entries (Byte-Width Boundaries)

### Primary Entry Point
**File:** `/home/coding/pdftract/crates/pdftract-core/src/cmap/codespace.rs`

**Function:** `CodespaceParser::parse_codespace_block()`  
**Lines:** 282-357

**Creation Point:** Line 350
```rust
ranges.push(CodespaceRange::new(lo_arr, hi_arr, width as u8));
```

### Purpose
Parses `begincodespacerange...endcodespacerange` blocks to define valid byte-width boundaries for multi-byte CJK encodings. Each `CodespaceRange` defines:
- `lo`: Low bound (inclusive)
- `hi`: High bound (inclusive)  
- `width`: Byte width (1, 2, 3, or 4)

### Constructor
**Function:** `CodespaceRange::new()`  
**Lines:** 56-60  
**Purpose:** Creates a single codespace range with validation

### Parser Entry Point
**Function:** `CodespaceParser::parse()`  
**Lines:** 233-279  
**Purpose:** Main parser that calls `parse_codespace_block()` when encountering `begincodespacerange` keyword

---

## 3. Type1 Font Encoding Differences (Character Code → Glyph Name)

### Primary Entry Point
**File:** `/home/coding/pdftract/crates/pdftract-core/src/font/encoding.rs`

**Function:** `DifferencesOverlay::parse()`  
**Lines:** 163-209

**Creation Point:** Lines 196-199
```rust
PdfObject::Name(name) => {
    // Assign this name to the current cursor position
    // MARKER: CMAP entry creation point - Type1 font encoding differences.
    // See notes/bf-e4uvb-child-1.md for documentation.
    if cursor <= 255 {
        overlay.entries.push((cursor as u8, Arc::clone(name)));
    }
    cursor = cursor.saturating_add(1);
}
```

### Purpose
Parses `/Differences` arrays from Type1 font encodings to create sparse overlays that override base encodings. Format: `[n /Name1 /Name2 ... m /OtherName ...]` where integers reset the position and subsequent names are assigned to consecutive codes.

### Data Structure
**Type:** `DifferencesOverlay`  
**Lines:** 129-232  
**Purpose:** Stores sparse list of `(code, glyph_name)` overrides

---

## File Path Summary

| Purpose | File Path | Function | Line Range |
|---------|-----------|----------|------------|
| ToUnicode add_mapping | `crates/pdftract-core/src/font/cmap.rs` | `ToUnicodeMap::add_mapping()` | 86-88 |
| ToUnicode bfchar call | `crates/pdftract-core/src/font/cmap.rs` | `parse_beginbfchar()` | 209 |
| ToUnicode bfrange explicit | `crates/pdftract-core/src/font/cmap.rs` | `parse_beginbfrange()` | 279 |
| ToUnicode bfrange contiguous | `crates/pdftract-core/src/font/cmap.rs` | `parse_beginbfrange()` | 292 |
| Codespace range creation | `crates/pdftract-core/src/cmap/codespace.rs` | `parse_codespace_block()` | 350 |
| Type1 differences overlay | `crates/pdftract-core/src/font/encoding.rs` | `DifferencesOverlay::parse()` | 196-199 |

---

## Related Files (Reference)

- **Codespace module:** `/home/coding/pdftract/crates/pdftract-core/src/cmap/mod.rs`
- **Tokenizer:** `/home/coding/pdftract/crates/pdftract-core/src/cmap/tokenize.rs`
- **Font resolver:** `/home/coding/pdftract/crates/pdftract-core/src/font/resolver.rs`
- **Embedded fonts:** `/home/coding/pdftract/crates/pdftract-core/src/font/embedded.rs`
- **Predefined CMap:** `/home/coding/pdftract/crates/pdftract-core/src/font/predefined_cmap.rs`

---

## Test Files

- ToUnicode tests: `crates/pdftract-core/src/font/cmap.rs` (lines 507-732)
- Codespace tests: `crates/pdftract-core/src/cmap/codespace.rs` (lines 634-898)
- Encoding tests: `crates/pdftract-core/src/font/encoding.rs` (lines 371-669)

---

## Bead Reference
This search was performed for bead **bf-4hwap** (explore task for CMAP entry creation).
