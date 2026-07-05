# ToUnicode Data Flow and Glyph Processing

**Bead ID**: bf-5thkf  
**Date**: 2026-07-05  
**Status**: Complete

## Overview

This document specifically focuses on ToUnicode CMap processing and how it handles glyph identifiers to create encoding entries. ToUnicode is Level 1 of the 4-level encoding resolution chain and provides the highest confidence (1.0) mappings.

## Critical Clarification: ToUnicode Does NOT Process Glyph Names

**Key Finding**: ToUnicode CMap does **NOT** process glyph names directly. It processes **raw byte sequences** (character codes) that serve as glyph identifiers.

- **Input**: Variable-length byte sequences (1-4 bytes) representing character codes in the font's encoding space
- **Output**: Unicode codepoint sequences (1-4 chars for ligature expansions)
- **Mapping**: `Vec<u8>` → `Vec<char>` (stored in HashMap)

Glyph names are processed by **Level 2 (AGL)** of the resolution chain, not by ToUnicode.

## ToUnicode Processing Flow

### 1. CMap Stream Entry

**Location**: `crates/pdftract-core/src/font/cmap.rs:489-505`

**Convenience functions**:
```rust
pub fn parse_to_unicode(input: &[u8]) -> ToUnicodeMap
pub fn parse_to_unicode_with_diags(input: &[u8]) -> (ToUnicodeMap, Vec<Diagnostic>)
```

**Input**: Raw bytes from the `/ToUnicode` stream in a PDF font dictionary  
**Output**: Populated `ToUnicodeMap` structure + optional diagnostics

### 2. CMap Parser Initialization

**Location**: `cmap.rs:124-130`

```rust
pub fn new(input: &'a [u8]) -> Self {
    Self {
        lexer: Lexer::new(input),
        diagnostics: Vec::new(),
    }
}
```

The lexer tokenizes the PostScript CMap program syntax.

### 3. Main Parse Loop

**Location**: `cmap.rs:136-185`

**Process**:
1. Read tokens until EOF
2. On `beginbfchar`: → `parse_beginbfchar()`
3. On `beginbfrange`: → `parse_beginbfrange()`
4. On `usecmap`: → `handle_usecmap()` (stub, emits diagnostic)
5. Unknown keywords: silently skipped
6. Errors: emit diagnostic, attempt recovery by skipping to matching end keyword

### 4. Single Character Mapping: beginbfchar

**Location**: `cmap.rs:190-216`

**Syntax**: `beginbfchar <count> <src1> <dst1> <src2> <dst2> ... endbfchar`

**Algorithm**:
1. Read count (integer)
2. Loop `count` times:
   - Read source hex string (byte sequence)
   - Read destination hex string (UTF-16BE encoded)
   - Decode destination to `Vec<char>`
   - Call `map.add_mapping(src, dst)` ← **ENTRY CREATION**
3. Expect `endbfchar` keyword

**Example**:
```
beginbfchar 2
<00> <0041>      % Maps byte 0x00 to 'A'
<01> <00660069>  % Maps byte 0x01 to ['f', 'i'] (ligature expansion)
endbfchar
```

**Entry creation call** (line 209):
```rust
map.add_mapping(src, dst);
```

### 5. Range Mapping: beginbfrange

**Location**: `cmap.rs:223-309`

**Two forms supported**:

#### 5a. Contiguous Range Form

**Syntax**: `beginbfrange <count> <lo> <hi> <dst> ... endbfrange`

**Algorithm**:
1. Read count
2. For each range:
   - Read `<lo>` and `<hi>` (source range bounds)
   - Validate `lo <= hi` (as byte sequences)
   - Read `<dst>` (starting destination)
   - Loop from `lo` to `hi`:
     - Call `map.add_mapping(current_src, dst)` ← **ENTRY CREATION**
     - Increment `current_src`
     - Increment `dst` (only last codepoint for multi-codepoint destinations)

**Example**:
```
beginbfrange 1
<0041> <005A> <0041>  % Maps 0x0041→'A', 0x0042→'B', ..., 0x005A→'Z'
endbfrange
```

**Entry creation call** (line 292):
```rust
map.add_mapping(current.clone(), dst.clone());
```

#### 5b. Explicit Array Form

**Syntax**: `beginbfrange <count> <lo> <hi> [<d0> <d1> ...] ... endbfrange`

**Algorithm**:
1. Read count
2. For each range:
   - Read `<lo>` and `<hi>`
   - Read array of destination strings
   - Validate array length equals `(hi - lo + 1)`
   - Loop through array:
     - Call `map.add_mapping(current_src, dst_array[i])` ← **ENTRY CREATION**
     - Increment `current_src`

**Example**:
```
beginbfrange 1
<0001> <0003> [<FB01> <FB02> <FB03>]  % Maps 1→'ﬁ', 2→'ﬂ', 3→'ﬃ'
endbfrange
```

**Entry creation call** (line 279):
```rust
map.add_mapping(current.clone(), dst);
```

### 6. UTF-16BE Decoding

**Location**: `cmap.rs:329-367`

**Purpose**: Convert destination hex strings to Unicode codepoints.

**Algorithm**:
1. Process input bytes in pairs (big-endian 16-bit units)
2. For each pair, assemble `u16` code unit: `(hi << 8) | lo`
3. Use `char::decode_utf16()` to handle surrogate pairs
4. Unpaired surrogates → replacement character (U+FFFD)
5. Odd byte counts → emit diagnostic but continue

**Result**: `Vec<char>` supporting:
- Single codepoints (most common)
- Surrogate pairs (for characters outside BMP)
- Multi-codepoint sequences (ligature expansions)

## Entry Addition Mechanism

### Core Entry Creation Point

**Location**: `cmap.rs:82-88`

**MARKER**: This is the exact location where ToUnicode entries are created.

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

**Call sites**:
1. `parse_beginbfchar()` - line 209 (one call per mapping pair)
2. `parse_beginbfrange()` - line 279 (explicit array form)
3. `parse_beginbfrange()` - line 292 (contiguous form)

### Entry Storage Structure

**Location**: `cmap.rs:67-71`

```rust
pub struct ToUnicodeMap {
    /// Mapping from source byte sequence to destination Unicode codepoints.
    /// Uses Vec<u8> as key (source bytes) and Vec<char> as value (destination chars).
    mappings: HashMap<Vec<u8>, Vec<char>>,
}
```

**Key characteristics**:
- **Key type**: `Vec<u8>` (variable-length byte sequence, 1-4 bytes)
- **Value type**: `Vec<char>` (Unicode codepoints, supports ligatures)
- **Collision handling**: Later mappings overwrite earlier ones (HashMap semantics)
- **Ordering**: No ordering guarantee (HashMap)

### Lookup Mechanism

**Location**: `cmap.rs:90-95`

```rust
pub fn lookup(&self, src: &[u8]) -> Option<&[char]> {
    self.mappings.get(src).map(|v| v.as_slice())
}
```

**Usage**: Called by `resolver.rs:388` during Level 1 resolution.

## Which Glyph Identifiers Are Processed?

### Byte Sequences, Not Names

ToUnicode processes **character codes** (byte sequences), not glyph names:

| Character Code Type | Example | Byte Sequence |
|-------------------|---------|--------------|
| Single-byte        | 0x00    | `[0x00]` |
| Double-byte        | 0x0100  | `[0x01, 0x00]` |
| Triple-byte        | 0x010203 | `[0x01, 0x02, 0x03]` |
| Quad-byte          | 0x01020304 | `[0x01, 0x02, 0x03, 0x04]` |

### Variable-Length Character Codes

The byte sequences in ToUnicode maps represent **character codes in the font's encoding space**, which can be:

1. **Single-byte (0-255)**: Most common for 8-bit encodings
2. **Multi-byte (CID fonts)**: For CJK fonts with large character sets
3. **TrueType glyph indices**: For TrueType fonts using `/ToUnicode`

### Font-Specific Encoding Spaces

**Important**: The same byte sequence can map to different characters in different fonts:

```
Font A: 0x0041 → 'A' (Latin)
Font B: 0x0041 → 'あ' (CJK)
```

This is why ToUnicode maps are **per-font** and stored in the `Font` struct.

## Key Data Structures

### 1. ToUnicodeMap

**Location**: `cmap.rs:67-112`

```rust
pub struct ToUnicodeMap {
    mappings: HashMap<Vec<u8>, Vec<char>>,
}
```

**Purpose**: Stores all ToUnicode mappings for a single font.  
**Lifetime**: Created once per font during font loading, reused for all text extraction.  
**Thread-safety**: Not thread-safe (font-local data).

### 2. CMapParser

**Location**: `cmap.rs:118-487`

```rust
pub struct CMapParser<'a> {
    lexer: Lexer<'a>,
    diagnostics: Vec<Diagnostic>,
}
```

**Purpose**: One-shot parser for ToUnicode streams.  
**Lifetime**: Created temporarily during parsing, dropped after parsing completes.

### 3. ResolvedGlyph (Usage)

**Location**: `resolver.rs:168-175`

```rust
pub struct ResolvedGlyph {
    pub chars: SmallVec<[char; 4]>,
    pub source: UnicodeSource,
    pub confidence: f32,
}
```

**Purpose**: Result of ToUnicode lookup (and other resolution levels).  
**From ToUnicode**: `source = UnicodeSource::ToUnicode`, `confidence = 1.0`.

## Data Flow Diagram

```
PDF Font Dictionary
    ↓
/ToUnicode stream (raw bytes)
    ↓
parse_to_unicode_with_diags()
    ↓
CMapParser::new(input)
    ↓
CMapParser::parse()
    ├→ Lexer tokenization
    ├→ parse_beginbfchar()
    │   └→ map.add_mapping(src, dst)  [ENTRY CREATION]
    └→ parse_beginbfrange()
        ├→ Contiguous form:
        │   └→ map.add_mapping(current, dst)  [ENTRY CREATION]
        └→ Explicit array form:
            └→ map.add_mapping(current, dst)  [ENTRY CREATION]
    ↓
ToUnicodeMap { HashMap<Vec<u8>, Vec<char>> }
    ↓
Font.to_unicode (stored per-font)
    ↓
resolve_unicode() → resolve_level1()
    ↓
ToUnicodeMap::lookup(char_code)
    ↓
Option<&[char]> (Unicode characters or None)
```

## Comparison with Glyph Name Processing

| Aspect | ToUnicode (Level 1) | AGL (Level 2) |
|--------|-------------------|---------------|
| **Input** | Byte sequences | Glyph names |
| **Example input** | `[0x00, 0x41]` | `"A"`, `"fi"`, `"uni20AC"` |
| **Processing** | CMap parsing | AGL table lookup |
| **Coverage** | Font-specific | Standard (~4400 entries) |
| **Confidence** | 1.0 (highest) | 0.9 |
| **When used** | Always preferred | Fallback when ToUnicode missing/empty |

## Special Cases and Edge Cases

### 1. Empty Mappings

**Syntax**: `<00> <>`

**Result**: `dst` is empty `Vec<char>`, maps to empty slice.  
**Meaning**: Character code exists in font but should not produce text output.  
**Lookup**: Returns `Some(&[])` (empty slice, not None).

### 2. Ligature Expansions

**Syntax**: `<00> <00660069>` (UTF-16BE for "fi")

**Decoding**: Two UTF-16 code units → two `char` values.  
**Result**: `dst = ['f', 'i']`  
**Lookup**: Returns `Some(&['f', 'i'])` (two characters from one source).

**Examples**:
- `fi` → `['f', 'i']`
- `ffi` → `['f', 'f', 'i']`
- `æ` → `['æ']` (single ligature codepoint U+00E6)

### 3. Unpaired Surrogates

**Syntax**: `<00> <D800>` (D800 is lone high surrogate)

**Decoding**: `char::decode_utf16()` returns `Err` for unpaired surrogates.  
**Result**: Replacement character (U+FFFD, '�')  
**Diagnostic**: Not emitted (replacement is standard behavior).

### 4. Odd-Length Hex Strings

**Syntax**: `<00> <0041>` (5 hex digits → 3 decoded bytes)

**Decoding**: Processes complete pairs, ignores trailing byte.  
**Result**: `['A']` (from first 2 bytes)  
**Diagnostic**: "UTF-16BE string has odd number of bytes" emitted.

### 5. Multi-Codepoint Destinations in Ranges

**Syntax**: `beginbfrange 1 <0001> <0002> <00660069> endbfrange`

**Decoding**: For contiguous ranges with multi-codepoint destinations, **only the last codepoint** is incremented.

**Expansion**:
- `0x0001 → ['f', 'i']`
- `0x0002 → ['f', 'j']` (last codepoint incremented)

**Spec justification**: PDF spec note H.2 (ToUnicode CMaps).

## Error Handling and Recovery

### Parse Errors

**Strategy**: Emit diagnostic, attempt recovery, continue parsing.

**Examples**:
1. **Invalid range** (`lo > hi`): Emit error, skip to `endbfrange`
2. **Array length mismatch**: Emit error, skip to `endbfrange`
3. **Missing keyword**: Emit error, skip to expected keyword
4. **Unexpected token**: Emit error, skip token

**Result**: Partial map (mappings before error are preserved).

### usecmap Directive

**Current behavior**: Emit diagnostic "predefined CMap loading not yet implemented (Phase 2.3)"  
**Future**: Will load and merge predefined CMaps (Adobe-Japan1-UCS2, Adobe-CNS1-UCS2, etc.).

## Performance Considerations

### Map Size

- Typical Latin font: 100-500 mappings
- CJK font: 1,000-10,000+ mappings
- Worst case: All 65,536 CID fonts

### Lookup Performance

- HashMap lookup: O(1) average, O(n) worst case
- Byte sequence comparison: O(k) where k = length (1-4 bytes)
- Cache-friendly: Yes (small keys)

### Memory Usage

Per-font overhead:
- HashMap structure: ~100-500 bytes
- Per entry: ~50 bytes (key + value + overhead)
- Total for 10,000 mappings: ~500 KB

## Testing Coverage

**Location**: `cmap.rs:507-732`

**Test categories**:
1. Single bfchar mappings (lines 520-567)
2. Ligature expansions (lines 532-567)
3. Contiguous bfrange (lines 570-583)
4. Explicit array bfrange (lines 586-597)
5. Comments (lines 600-610)
6. Variable-width sources (lines 636-646)
7. Error cases (lines 650-731)

## Related Documentation

- **CMAP entry creation**: `notes/bf-e4uvb-child-1.md`
- **CMAP data flow**: `notes/bf-e4uvb-child-3-cmap-flow.md`
- **ToUnicode entry creation code locations**: Commit `155e00ab` - `docs(bf-4hwap)`

## Code Location Summary

| Function | Location | Purpose |
|----------|----------|---------|
| `parse_to_unicode_with_diags()` | cmap.rs:502-505 | Entry point |
| `CMapParser::parse()` | cmap.rs:136-185 | Main parse loop |
| `parse_beginbfchar()` | cmap.rs:190-216 | Single mappings |
| `parse_beginbfrange()` | cmap.rs:223-309 | Range mappings |
| `decode_utf16be()` | cmap.rs:329-367 | Decode destinations |
| `ToUnicodeMap::add_mapping()` | cmap.rs:82-88 | **ENTRY CREATION** |
| `ToUnicodeMap::lookup()` | cmap.rs:90-95 | Resolution lookup |

---

**Document Version**: 1.0  
**Last Updated**: 2026-07-05  
**Related Beads**: bf-5thkf, bf-e4uvb  
**Plan References**: INV-30 (4-level resolution chain), EC-NN (CMap parsing requirements)
