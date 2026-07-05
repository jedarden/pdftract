# CMAP Data Flow and Glyph Processing Documentation

## Overview

This document describes how CMAP (Character Map) processes glyph names and creates encoding entries in the pdftract codebase. The CMAP system is a critical component of the 4-level encoding resolution chain for text extraction from PDFs.

## Data Flow Architecture

### Level 1: ToUnicode CMap (Confidence 1.0)

The primary and most reliable source for glyph-to-Unicode mapping is the `/ToUnicode` CMap stream embedded in PDF fonts.

#### Entry Creation Flow

1. **CMap Parsing** (`crates/pdftract-core/src/font/cmap.rs:86-88`)
   - **Entry Point**: `ToUnicodeMap::add_mapping()` method
   - **MARKER**: Line 85-88 explicitly marks this as the CMAP entry creation point
   - Called by:
     - `parse_beginbfchar()` (single character mappings)
     - `parse_beginbfrange()` (range mappings)

2. **Data Structure**: `ToUnicodeMap` (`cmap.rs:67-71`)
   ```rust
   pub struct ToUnicodeMap {
       /// Mapping from source byte sequence to destination Unicode codepoints.
       /// Uses Vec<u8> as key (source bytes) and Vec<char> as value (destination chars).
       mappings: HashMap<Vec<u8>, Vec<char>>,
   }
   ```

3. **Mapping Types Processed**:
   
   a. **Single Character Mappings** (`beginbfchar...endbfchar` blocks)
      - Format: `beginbfchar <count> <src1> <dst1> <src2> <dst2> ... endbfchar`
      - Example: `beginbfchar 1 <00> <0041> endbfchar` maps byte `0x00` to 'A'
      - Supports ligature expansion: `<00660069>` → `['f', 'i']` (two codepoints)
   
   b. **Range Mappings** (`beginbfrange...endbfrange` blocks)
      - **Contiguous form**: `<lo> <hi> <dst>` - maps range with incrementing destination
        - Example: `beginbfrange 1 <0041> <005A> <0041> endbfrange` maps A-Z to U+0041..U+005A
      - **Explicit array form**: `<lo> <hi> [<d0> <d1> ...]` - each destination explicitly specified
        - Example: `beginbfrange 1 <0001> <0003> [<FB01> <FB02> <FB03>] endbfrange`
        - Maps codes 1,2,3 to ligatures fi, fl, ffi

4. **Glyph Names Processed**:
   - CMap does NOT process glyph names directly
   - It processes **raw byte sequences** (character codes) as keys
   - Source byte sequences can be 1-4 bytes (variable-length)
   - Destination is always Unicode codepoints (UTF-16BE decoded)

5. **UTF-16BE Decoding** (`cmap.rs:329-367`)
   - Hex strings in CMap are UTF-16BE encoded
   - Decoded to `Vec<char>` supporting surrogate pairs and multi-codepoint results
   - Unpaired surrogates produce replacement character (U+FFFD)
   - Odd byte counts emit diagnostic but continue processing

#### Lookup Flow

1. **Resolution Entry** (`resolver.rs:302-377`)
   - `resolve_unicode()` is the main entry point
   - Checks per-font cache first (thread-safe LRU)
   - Calls `resolve_level1()` for ToUnicode lookup

2. **Level 1 Lookup** (`resolver.rs:383-399`)
   ```rust
   fn resolve_level1(char_code: &[u8], to_unicode: Option<&ToUnicodeMap>) -> ResolvedGlyph
   ```
   - Returns failure if ToUnicode CMap is missing
   - Returns failure if character code not in map
   - Returns failure if result is empty or U+FFFD only
   - Success: Returns `ResolvedGlyph` with chars, source=ToUnicode, confidence=1.0

### Level 2: Named Encoding + AGL (Confidence 0.9)

When ToUnicode is absent or fails, the system falls back to named encoding + AGL lookup.

#### Encoding Processing

1. **Named Encodings** (`encoding.rs:22-60`)
   - 6 standard encodings: WinAnsi, MacRoman, MacExpert, Standard, Symbol, ZapfDingbats
   - Map character codes (0-255) to glyph names
   - WinAnsiEncoding is most common (Windows-1252 superset)

2. **Differences Overlay** (`encoding.rs:129-150`)
   - Sparse overrides from `/Differences` array
   - Format: `[n /Name1 /Name2 ... m /OtherName ...]`
   - Each integer resets position, subsequent names assign to consecutive codes
   - Example: `[39 /quotesingle 96 /grave]` assigns code 39→"quotesingle", 96→"grave"

#### AGL Lookup (Glyph Name Processing)

1. **Adobe Glyph List** (`agl.rs:41-60`)
   - Compile-time phf::Map of ~4,400 glyph name → Unicode mappings
   - Generated from Adobe's glyphlist.txt at build time

2. **Algorithmic Patterns** (`agl.rs:96-100`)
   - `uniXXXX` (4 hex digits) → Unicode codepoint
   - `uXXXXXX` (up to 6 hex digits) → Unicode codepoint
   - Example: `uni20AC` → U+20AC (Euro sign)

3. **Variant Suffix Stripping** (`agl.rs:47-56`)
   - Strips suffixes like `.sc`, `.alt`, `.oldstyle` before lookup
   - Example: `A.sc` → `A` → U+0041

4. **Ligature Support** (`agl.rs:79-94`)
   - `unicode_for_glyph_name_multi()` handles multi-codepoint glyphs
   - Example: `fi` → `['f', 'i']`

#### Level 2 Resolution

- Character code → Encoding table → Glyph name → AGL lookup → Unicode
- Returns `ResolvedGlyph` with source=Agl, confidence=0.9

### Level 3: Font Fingerprint Cache (Confidence 0.85)

When both ToUnicode and AGL fail, the system uses font fingerprinting.

#### Fingerprint Processing (`fingerprint.rs`)

1. **Hash Computation**
   - SHA-256 hash of decoded font program bytes
   - Computed over embedded font program (post-stream decode)
   - Stable across different stream filters

2. **Lookup Database**
   - Compile-time phf::Map: `[u8; 32]` → `&'static [(u16, u32)]`
   - Maps glyph ID (GID) to Unicode codepoint
   - Generated from `build/font-fingerprints.json`

3. **Fallback Trigger**
- Used when:
  - `/ToUnicode` is missing or empty
  - Embedded font subset has stripped glyph names
  - Font binary matches a known fingerprint

### Level 4: Shape Recognition (Confidence 0.7, cfg-gated)

Final fallback: visual glyph shape matching.

#### Shape Processing (`shape.rs`)

1. **Rasterization**
   - Type 3 glyphs: Execute content stream to 32×32 bitmap
   - Embedded fonts: Extract glyph outline, render to bitmap

2. **Perceptual Hashing**
   - Compute pHash of bitmap
   - Look up in shape database (compile-time phf::Map)

3. **Feature-Gated**
   - Only available when `shape-db` feature is enabled
   - Requires `build/glyph-shapes.json` at compile time

## Key Data Structures

### 1. `ToUnicodeMap` (`cmap.rs:67-71`)
```rust
pub struct ToUnicodeMap {
    mappings: HashMap<Vec<u8>, Vec<char>>,
}
```
- **Purpose**: Stores character code → Unicode mappings from `/ToUnicode` stream
- **Key**: Variable-length byte sequence (1-4 bytes)
- **Value**: Unicode codepoints (1-4 chars for ligatures)
- **Entry point**: `add_mapping()` at line 86

### 2. `ResolvedGlyph` (`resolver.rs:168-175`)
```rust
pub struct ResolvedGlyph {
    pub chars: SmallVec<[char; 4]>,
    pub source: UnicodeSource,
    pub confidence: f32,
}
```
- **Purpose**: Result of encoding resolution at any level
- **chars**: 1-4 Unicode characters (supports ligature expansion)
- **source**: Which level produced the mapping (ToUnicode, Agl, Fingerprint, ShapeMatch, Unknown)
- **confidence**: Always 1.0, 0.9, 0.85, or 0.7 per INV-30

### 3. `FontEncoding` (`encoding.rs`)
```rust
pub enum FontEncoding {
    Named(NamedEncoding),
    Custom(DifferencesOverlay),
}
```
- **Purpose**: Level 2 character code → glyph name mapping
- **Named**: One of 6 standard encodings (WinAnsi, MacRoman, etc.)
- **Custom**: Base encoding + Differences array overlay

### 4. `ResolverCache` (`resolver.rs:223-278`)
```rust
pub struct ResolverCache {
    cache: DashMap<CacheKey, ResolvedGlyph>,
    emitted_misses: DashMap<(FontId, SmallVec<[u8; 4]>), ()>,
}
```
- **Purpose**: Per-font LRU cache for resolved glyphs
- **Thread-safe**: Uses DashMap for concurrent access
- **Miss tracking**: Ensures GLYPH_UNMAPPED diagnostic emitted once per (font, code)

### 5. `Font` (`resolver.rs:33-46`)
```rust
pub struct Font {
    id: FontId,
    to_unicode: Option<ToUnicodeMap>,
    encoding: Option<FontEncoding>,
    fingerprint: Option<CachedFingerprint>,
    has_embedded_program: bool,
    cache: ResolverCache,
}
```
- **Purpose**: Encapsulates all data for 4-level fallback chain
- **to_unicode**: Level 1 CMap (highest priority)
- **encoding**: Level 2 named encoding + AGL
- **fingerprint**: Level 3 font fingerprint cache
- **has_embedded_program**: Skip Level 3 if false (Type 3 fonts)

## Entry Addition Mechanism Details

### CMAP Entry Creation (`cmap.rs:82-88`)

```rust
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
    self.mappings.insert(src, dst);
}
```

**Call Sites**:

1. **From `parse_beginbfchar()` (line 209)**
   - Processes single-character mappings
   - Each `beginbfchar` block produces N calls to `add_mapping()`
   - Source: hex string from CMap
   - Destination: UTF-16BE decoded to `Vec<char>`

2. **From `parse_beginbfrange()` (lines 279, 292)**
   - Processes range mappings
   - **Contiguous form**: Calls `add_mapping()` in loop, incrementing both source and destination
   - **Explicit array form**: Calls `add_mapping()` once per array entry
   - Range expansion can produce hundreds of entries

### Encoding Entry Creation

**Named Encodings** (`encoding.rs`):
- Compile-time generated from JSON files in `build/named-encodings.json`
- Each encoding is a 256-element array: `[Option<&str>; 256]`
- Loaded at build time via `build.rs`

**Differences Overlay** (`encoding.rs:144-183`):
- Parsed at runtime from `/Differences` array
- Sparse vector of `(u8, Arc<str>)` pairs
- Created during `FontEncoding::parse_from_font()`

## Code Locations Reference

### CMAP Processing
- **ToUnicode parsing**: `crates/pdftract-core/src/font/cmap.rs:114-505`
- **Entry creation**: `cmap.rs:82-88` (MARKER at line 85)
- **bfchar parsing**: `cmap.rs:190-216`
- **bfrange parsing**: `cmap.rs:223-309`
- **UTF-16BE decoding**: `cmap.rs:329-367`

### Resolution Chain
- **Main resolver**: `crates/pdftract-core/src/font/resolver.rs:302-377`
- **Level 1 (ToUnicode)**: `resolver.rs:383-399`
- **Level 2 (AGL)**: `resolver.rs:405-432`
- **Level 3 (Fingerprint)**: `resolver.rs:438-451`
- **Level 4 (Shape)**: `resolver.rs:457-470` (cfg-gated)

### Glyph Name Processing
- **AGL lookup**: `crates/pdftract-core/src/font/agl.rs:41-94`
- **Algorithmic patterns**: `agl.rs:96-141`
- **Named encodings**: `crates/pdftract-core/src/font/encoding.rs:22-110`
- **Differences overlay**: `encoding.rs:144-183`

### Data Structures
- **ToUnicodeMap**: `cmap.rs:67-112`
- **ResolvedGlyph**: `resolver.rs:168-197`
- **Font**: `resolver.rs:33-104`
- **ResolverCache**: `resolver.rs:223-284`
- **FontEncoding**: `encoding.rs` (module)

## Glyph Name Summary

### Processed by CMAP
- **None directly** - CMAP processes byte sequences, not glyph names

### Processed by AGL (Level 2)
- Standard glyph names: `A`, `B`, `quotesingle`, `quotedblleft`, etc.
- Algorithmic patterns: `uniXXXX`, `uXXXXXX`
- Ligatures: `fi`, `fl`, `ffi`, `ffl`
- Variant suffixes: `A.sc`, `A.alt`, `one.oldstyle`
- Symbol font: `alpha`, `beta`, `gamma`, etc.
- ZapfDingbats: `a1`, `a2`, ..., `a202`

### Total Coverage
- AGL: ~4,400 entries (glyphlist.txt)
- AGL Multi: ~770 entries (aglfn.txt for ligatures)
- Algorithmic patterns: Unlimited (any valid Unicode codepoint)
- Named encodings: 6 × 256 = 1,536 potential mappings

## Confidence Hierarchy

Per INV-30, confidence is always one of {1.0, 0.9, 0.85, 0.7, 0.0}:

1. **1.0** - ToUnicode CMap (authoritative source)
2. **0.9** - Named encoding + AGL (reliable but indirect)
3. **0.85** - Font fingerprint (content-based, assumes standard fonts)
4. **0.7** - Shape recognition (visual similarity, error-prone)
5. **0.0** - Unknown (U+FFFD replacement character)

## Related Documentation

- **CMAP entry creation**: `notes/bf-e4uvb-child-1.md` (if exists)
- **ToUnicode entry creation code locations**: Commit `155e00ab` - `docs(bf-4hwap)`
- **4-level fallback chain**: `docs/plan/plan.md` (INV-30, EC-NN references)

## Testing References

- **CMap parsing tests**: `crates/pdftract-core/src/font/cmap.rs:507-732`
- **Resolver tests**: `crates/pdftract-core/src/font/resolver.rs` (mod tests)
- **AGL tests**: `crates/pdftract-core/src/font/agl.rs` (if present)

---

**Document Version**: 1.0  
**Last Updated**: 2026-07-05  
**Related Bead**: bf-3ztvp  
**Plan References**: (See plan.md for related EC-NN and INV-30 tags)
