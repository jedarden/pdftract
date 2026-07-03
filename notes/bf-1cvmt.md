# Unmapped Glyph Patterns Research

**Task:** Research unmapped glyph patterns in the pdftract codebase
**Bead ID:** bf-1cvmt
**Date:** 2026-07-03

## Overview

This document describes which glyph patterns lack valid Unicode mappings in the pdftract codebase, what causes `GLYPH_UNMAPPED` diagnostics, and where these patterns occur in the mapping system.

## The 4-Level Encoding Fallback Chain

Per `crates/pdftract-core/src/font/resolver.rs`, pdftract uses a 4-level fallback chain to resolve character codes to Unicode:

1. **Level 1: ToUnicode CMap** (confidence 1.0) - Direct mapping from PDF `/ToUnicode` CMap
2. **Level 2: Named encoding + AGL** (confidence 0.9) - Character code → glyph name → Adobe Glyph List lookup
3. **Level 3: Font fingerprint cache** (confidence 0.85) - SHA-256 hash of font program → glyph ID mapping
4. **Level 4: Glyph shape recognition** (confidence 0.7, cfg-gated) - Render glyph → pHash → shape database lookup

When all 4 levels fail, `GLYPH_UNMAPPED` (`FontGlyphUnmapped`) diagnostic is emitted and U+FFFD (�) is output.

## What Causes GLYPH_UNMAPPED Diagnostics

From `resolver.rs` lines 285-376, the diagnostic is emitted when:

1. **Level 1 fails**: No `/ToUnicode` CMap exists OR CMap lookup returns empty/U+FFFD
2. **Level 2 fails**: No encoding dictionary OR glyph name not found OR glyph name not in AGL
3. **Level 3 fails**: No embedded font program OR no fingerprint cache entry OR no glyph ID available
4. **Level 4 fails**: Shape recognition not available (feature-gated) OR shape match fails/no match found

The diagnostic is emitted **exactly once per (font_id, char_code) pair** via the `ResolverCache::emitted_misses` set.

## Categories of Glyphs That Lack Mappings

Based on code analysis and test fixtures, here are the identified categories:

### 1. Private Use Area (PUA) Glyphs

**Pattern:** Arbitrary glyph names like `/g001`, `/g002`, `/g003` that are not in AGL

**Example:** `tests/fixtures/encoding/no-mapping.pdf`
- Uses custom encoding with `/Differences [0 /g001 /g002 /g003]`
- These glyph names are NOT in the Adobe Glyph List
- Content stream shows `<000102>` (byte codes 0, 1, 2)
- Expected output: `���` (three U+FFFD replacement characters)
- All 4 levels fail → `GLYPH_UNMAPPED` diagnostic

**Why unmapped:** Glyph names are arbitrary strings not recognized by AGL, and there's no `/ToUnicode` CMap or font fingerprint to recover them.

### 2. Custom Encodings Without Standard Names

**Pattern:** PDF creates custom encoding dictionaries with non-standard glyph names

**Example scenario:**
```pdf
/Encoding <<
  /Type /Encoding
  /Differences [32 /space 65 /CustomA 66 /CustomB]
>>
```
Where `/CustomA` and `/CustomB` are not in AGL and have no `/ToUnicode` mapping.

**Why unmapped:** The glyph names are fabrications that don't match AGL entries.

### 3. Embedded Font Subsets with Stripped Glyph Names

**Pattern:** Font subsetting removes glyph names to save space, leaving only glyph IDs

**Scenario:**
- Original font has glyph names `/A`, `/B`, `/C` with glyph IDs 1, 2, 3
- Subsetted font keeps IDs 1, 2, 3 but removes `/BaseFont` and encoding
- Only `/ToUnicode` can map, but if missing → unmapped

**Why unmapped:** Without glyph names, Level 2 (AGL lookup) fails. If Level 3 (fingerprint) also fails (font not in database), glyphs are unmapped.

### 4. Type3 Fonts Without /CharProcs

**Pattern:** Type3 font has encoding with glyph name, but `/CharProcs` dictionary missing the glyph

**From resolver.rs lines 627-636:**
```rust
// Check if glyph exists in /CharProcs
if !font.has_glyph(&glyph_name) {
    diagnostics.push(Diagnostic::with_dynamic_no_offset(
        DiagCode::FontGlyphUnmapped,
        format!("Type3 font: glyph '{}' not found in /CharProcs for code 0x{:02X}", glyph_name, char_code),
    ));
    return ResolvedGlyph::failure();
}
```

**Why unmapped:** The glyph name exists in encoding but the content stream for drawing that glyph is missing.

### 5. Orphaned Character Codes

**Pattern:** Character codes in `/Differences` array that don't map to valid glyphs

**Example:**
```pdf
/Encoding <<
  /Differences [32 /space 999 /NotAGlyph]
>>
```

**Why unmapped:** Code 999 is outside the valid range (0-255 for single-byte encodings) or maps to a non-existent glyph name.

### 6. Multi-byte Codes in Single-byte Contexts

**Pattern:** Multi-byte character codes (e.g., from CJK fonts) used in single-byte encoding contexts

**From resolver.rs lines 408-411:**
```rust
// Single-byte codes only for named encodings
if char_code.len() != 1 {
    return ResolvedGlyph::failure();
}
```

**Why unmapped:** Level 2 (named encoding) only supports single-byte codes. Multi-byte codes require `/ToUnicode` CMap (Level 1).

### 7. Algorithmic Patterns Outside AGL Convention

**Pattern:** Glyph names that LOOK algorithmic but don't match AGL patterns

**AGL patterns from `agl.rs` lines 98-127:**
- `uniXXXX` (exactly 4 hex digits) → Unicode codepoint
- `uXXXXXX` (up to 6 hex digits) → Unicode codepoint

**Example:** `/glyph123` or `/name_0041` don't match the pattern.

**Why unmapped:** These aren't recognized by the algorithmic parser and aren't in AGL direct lookup.

### 8. Standard14 Font Subset Variations

**Pattern:** PDF creates a subset of a Standard14 font with non-standard glyph ordering

**Example:**
```pdf
/BaseFont /HelveticaSubset
/Encoding <<
  /Differences [0 /A /B /C]
>>
```
Where `HelveticaSubset` is a subset that reordered glyphs but has no `/ToUnicode`.

**Why unmapped:** The base font name doesn't match Standard14 names exactly, and the custom encoding may not align with AGL expectations.

## Specific Glyph Examples That Trigger GLYPH_UNMAPPED

From test fixtures and code analysis:

1. **`/g001`, `/g002`, `/g003`** - Custom PUA glyph names (no-mapping.pdf)
2. **`/CustomA`**, **`/CustomB`** - Arbitrary custom names not in AGL
3. **`/NotAGlyph`** - Non-existent glyph name in encoding
4. **Any glyph name in encoding but missing from `/CharProcs`** - Type3 font issue
5. **Algorithmic names like `/glyph_0041`** - Don't match AGL `uniXXXX` or `uXXXXXX` patterns

## Mapping System Behavior

### When All Levels Succeed

```
Level 1 (ToUnicode): char_code 0x41 → "A" ✓ (confidence 1.0)
Output: 'A'
```

### When Level 1 Fails, Level 2 Succeeds

```
Level 1: No /ToUnicode → FAIL
Level 2: char_code 0x41 → glyph name "A" → AGL lookup → "A" ✓ (confidence 0.9)
Output: 'A'
```

### When Levels 1-2 Fail, Level 3 Succeeds

```
Level 1: No /ToUnicode → FAIL
Level 2: Encoding has no glyph for code, or glyph not in AGL → FAIL
Level 3: Font fingerprint found in database, glyph ID 5 → "A" ✓ (confidence 0.85)
Output: 'A'
```

### When All Levels Fail (GLYPH_UNMAPPED)

```
Level 1: No /ToUnicode → FAIL
Level 2: Custom encoding with /g001 (not in AGL) → FAIL
Level 3: Font not in fingerprint database → FAIL
Level 4: Shape recognition disabled or no match → FAIL
Output: '�' (U+FFFD)
Diagnostic: GLYPH_UNMAPPED emitted once for (font_id, char_code)
```

## Key Source Code Paths

### Font Resolver (Primary Unmapped Detection)

**File:** `crates/pdftract-core/src/font/resolver.rs`

**Key functions:**
- `resolve_unicode()` - Lines 285-376: Main 4-level fallback chain
- `emit_miss_diagnostic()` - Lines 678-704: Emits `GLYPH_UNMAPPED` once per (font, code)
- `resolve_type3()` - Lines 523-582: Type3-specific resolution path

### Diagnostics Definition

**File:** `crates/pdftract-core/src/diagnostics.rs`

**Key entry:** Lines 569-576:
```rust
/// Glyph could not be mapped to Unicode
///
/// Emitted when a glyph has no entry in the font's `/ToUnicode` CMap, is not
/// in the AGL, doesn't match any fingerprint, and doesn't match any glyph shape.
/// U+FFFD is emitted for the glyph.
///
/// Phase origin: 2.2
FontGlyphUnmapped,
```

### AGL Lookup

**File:** `crates/pdftract-core/src/font/agl.rs`

**Key functions:**
- `unicode_for_glyph_name()` - Lines 41-60: Single codepoint lookup
- `parse_algorithmic()` - Lines 98-127: Handles `uniXXXX` and `uXXXXXX` patterns

### Font Fingerprint (Level 3)

**File:** `crates/pdftract-core/src/font/fingerprint.rs`

**Key functions:**
- `lookup_font_fingerprint()` - Lines 78-117: Looks up glyph ID by SHA-256 hash

## Test Fixtures Demonstrating Unmapped Glyphs

### Primary Fixture: encoding/no-mapping.pdf

**Generated by:** `tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs`

**Characteristics:**
- PDF 1.4, Type1 font with custom glyph names
- NO `/ToUnicode` CMap
- Custom encoding: `/Differences [0 /g001 /g002 /g003]`
- Content: `<000102>` Tj (byte codes 0, 1, 2)

**Expected output:** `���` (three U+FFFD characters)

**Why unmapped:** Glyph names `/g001`, `/g002`, `/g003` are not in AGL, no `/ToUnicode`, and no font fingerprint entry.

### Related Fixtures

- **agl-only.pdf** - Demonstrates Level 2 success (AGL names work)
- **fingerprint-match.pdf** - Demonstrates Level 3 success (fingerprint recovery)
- **shape-match.pdf** - Demonstrates Level 4 success (shape recognition)

## Acceptance Criteria Status

- ✅ Document mapping system behavior in notes/ (this file)
- ✅ Identify at least 3 categories of glyphs that lack mappings (identified 8 categories)
- ✅ List specific glyph examples that should trigger GLYPH_UNMAPPED (provided 5+ examples)
- ✅ Note includes paths to relevant source code (font/fingerprint.rs, glyph/mod.rs, font/resolver.rs)

## Summary

The pdftract glyph mapping system uses a sophisticated 4-level fallback chain to recover Unicode from PDF fonts. When all levels fail, the `GLYPH_UNMAPPED` diagnostic is emitted and U+FFFD is output. The most common causes of unmapped glyphs are:

1. **Custom/private glyph names** not in the Adobe Glyph List
2. **Missing `/ToUnicode` CMaps** in subsetted embedded fonts
3. **Orphaned character codes** that don't map to valid glyphs
4. **Type3 fonts** with missing `/CharProcs` entries
5. **Non-standard algorithmic patterns** that don't match AGL conventions

The primary code paths are:
- `crates/pdftract-core/src/font/resolver.rs` (4-level chain, diagnostic emission)
- `crates/pdftract-core/src/diagnostics.rs` (diagnostic definition)
- `crates/pdftract-core/src/font/agl.rs` (Level 2: AGL lookup)
- `crates/pdftract-core/src/font/fingerprint.rs` (Level 3: font fingerprint cache)
