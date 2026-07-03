# Selected Unmapped Glyphs for Fixture

**Task:** Select target unmapped glyphs for fixture
**Bead ID:** bf-41gtd
**Date:** 2026-07-03
**Based on:** notes/bf-1cvmt.md

## Overview

This document selects specific unmapped glyphs for the PDF fixture, drawn from the research in bf-1cvmt. The fixture will include 4 unmapped glyphs (from different categories) and 3 mapped AGL glyphs for comparison.

## Selected Unmapped Glyphs (4 examples)

### 1. `/g001` - Private Use Area (PUA) Glyph

**Category:** Private Use Area (PUA) Glyphs
**Source:** tests/fixtures/encoding/no-mapping.pdf

**Why unmapped (fails all 4 levels):**
- Level 1 (ToUnicode): No `/ToUnicode` CMap exists
- Level 2 (AGL): Glyph name `/g001` is not in the Adobe Glyph List
- Level 3 (Fingerprint): Font not in fingerprint database
- Level 4 (Shape): Shape recognition disabled or no match

**Context:** Uses custom encoding with `/Differences [0 /g001 /g002 /g003]`

---

### 2. `/CustomA` - Custom Encoding Without Standard Names

**Category:** Custom Encodings Without Standard Names
**Source:** Pattern from bf-1cvmt research

**Why unmapped (fails all 4 levels):**
- Level 1 (ToUnicode): No `/ToUnicode` CMap exists
- Level 2 (AGL): Glyph name `/CustomA` is not in the Adobe Glyph List
- Level 3 (Fingerprint): Font not in fingerprint database
- Level 4 (Shape): Shape recognition disabled or no match

**Context:** Would appear in custom encoding like `/Differences [65 /CustomA 66 /CustomB]`

---

### 3. `/glyph_0041` - Algorithmic Pattern Outside AGL Convention

**Category:** Algorithmic Patterns Outside AGL Convention
**Source:** Pattern from bf-1cvmt research

**Why unmapped (fails all 4 levels):**
- Level 1 (ToUnicode): No `/ToUnicode` CMap exists
- Level 2 (AGL): Glyph name `/glyph_0041` doesn't match AGL patterns (`uniXXXX` or `uXXXXXX`)
- Level 3 (Fingerprint): Font not in fingerprint database
- Level 4 (Shape): Shape recognition disabled or no match

**Context:** AGL only recognizes `uniXXXX` (exactly 4 hex digits) or `uXXXXXX` (up to 6 hex digits) patterns. The prefix `/glyph_` is not recognized.

---

### 4. `/NotAGlyph` - Orphaned Character Code

**Category:** Orphaned Character Codes
**Source:** Pattern from bf-1cvmt research

**Why unmapped (fails all 4 levels):**
- Level 1 (ToUnicode): No `/ToUnicode` CMap exists
- Level 2 (AGL): Glyph name `/NotAGlyph` is not in the Adobe Glyph List
- Level 3 (Fingerprint): Font not in fingerprint database
- Level 4 (Shape): Shape recognition disabled or no match

**Context:** Would appear in encoding like `/Differences [32 /space 999 /NotAGlyph]`, where the code may be outside valid range (0-255).

---

## Selected Mapped Glyphs (3 examples for comparison)

### 1. `/A` - Standard AGL Glyph

**Category:** Adobe Glyph List (AGL) mapped
**Unicode:** U+0041 (LATIN CAPITAL LETTER A)

**Why mapped:**
- Level 2 (AGL): Glyph name `/A` is found in the Adobe Glyph List with mapping to U+0041
- Confidence: 0.9 (if Level 1 fails)

**AGL entry:** From `crates/pdftract-core/src/font/agl.rs`

---

### 2. `/space` - Standard AGL Glyph

**Category:** Adobe Glyph List (AGL) mapped
**Unicode:** U+0020 (SPACE)

**Why mapped:**
- Level 2 (AGL): Glyph name `/space` is found in the Adobe Glyph List with mapping to U+0020
- Confidence: 0.9 (if Level 1 fails)

**AGL entry:** From `crates/pdftract-core/src/font/agl.rs`

---

### 3. `/B` - Standard AGL Glyph

**Category:** Adobe Glyph List (AGL) mapped
**Unicode:** U+0042 (LATIN CAPITAL LETTER B)

**Why mapped:**
- Level 2 (AGL): Glyph name `/B` is found in the Adobe Glyph List with mapping to U+0042
- Confidence: 0.9 (if Level 1 fails)

**AGL entry:** From `crates/pdftract-core/src/font/agl.rs`

---

## Fixture Design Implications

### Content Stream Byte Sequence

The fixture should use a content stream with byte codes that map to both unmapped and mapped glyphs:

```
/Differences [
  0 /g001           % code 0 → unmapped (PUA)
  1 /CustomA        % code 1 → unmapped (custom)
  32 /space         % code 32 → mapped (U+0020)
  65 /A             % code 65 → mapped (U+0041)
  66 /B             % code 66 → mapped (U+0042)
  99 /glyph_0041    % code 99 → unmapped (bad algorithmic)
  100 /NotAGlyph    % code 100 → unmapped (orphaned)
]
```

Content stream text: `<00 01 20 41 42 63 64> TJ`

**Expected output:** `��� ABC��` (U+FFFD for codes 0, 1, 99, 100; standard glyphs for 32, 65, 66)

### Expected Diagnostic Count

The fixture should emit **4 GLYPH_UNMAPPED diagnostics** (one per unmapped glyph):
1. `GLYPH_UNMAPPED` for glyph `/g001` at code 0x00
2. `GLYPH_UNMAPPED` for glyph `/CustomA` at code 0x01
3. `GLYPH_UNMAPPED` for glyph `/glyph_0041` at code 0x63
4. `GLYPH_UNMAPPED` for glyph `/NotAGlyph` at code 0x64

### Why These Particular Glyphs

1. **Coverage:** They represent 4 distinct unmapped glyph categories from bf-1cvmt research
2. **Test value:** Each fails all 4 mapping levels for a different reason
   - `/g001`: Arbitrary PUA naming
   - `/CustomA`: Custom encoding fabrication
   - `/glyph_0041`: Looks algorithmic but uses wrong prefix
   - `/NotAGlyph`: Orphaned name reference
3. **Comparison:** Mapped glyphs `/space`, `/A`, `/B` provide baseline for Level 2 (AGL) success
4. **Fixture simplicity:** All glyphs fit in single-byte encoding (0-255 range)

## Next Steps

After this bead, the fixture should be generated using the encoding fixture generator at `tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs` or a similar generator.

## References

- Research notes: notes/bf-1cvmt.md
- Font resolver: crates/pdftract-core/src/font/resolver.rs (lines 285-376)
- AGL lookup: crates/pdftract-core/src/font/agl.rs
- Diagnostics: crates/pdftract-core/src/diagnostics.rs (lines 569-576)
- Example fixture: tests/fixtures/encoding/no-mapping.pdf
