# Selected Unmapped Glyphs for Test Fixture

**Task:** Select target unmapped glyphs for fixture
**Bead ID:** bf-68f9i
**Date:** 2026-07-03
**Based on:** notes/bf-1cvmt.md

## Overview

This document lists the specific glyphs selected for inclusion in the PDF test fixture for unmapped glyph testing. The fixture will demonstrate the 4-level fallback chain failure and the resulting `GLYPH_UNMAPPED` diagnostic emission.

## Selected Unmapped Glyphs

### 1. `/g001`, `/g002`, `/g003` (PUA - Private Use Area)

**Category:** Private Use Area (PUA) Glyphs

**Source:** `tests/fixtures/encoding/no-mapping.pdf`

**Description:** Arbitrary glyph names created by a PDF generator using a custom encoding scheme. These follow a simple numeric pattern but are not recognized by the Adobe Glyph List.

**Why all 4 mapping levels fail:**
- **Level 1 (ToUnicode CMap):** No `/ToUnicode` CMap exists in the PDF
- **Level 2 (AGL lookup):** `/g001`, `/g002`, `/g003` are not in the Adobe Glyph List and do not match AGL algorithmic patterns (`uniXXXX`, `uXXXXXX`)
- **Level 3 (Font fingerprint):** The font is not in the fingerprint cache database
- **Level 4 (Shape recognition):** Feature-gated and disabled by default; even if enabled, these arbitrary glyphs likely have no match in the shape database

**Expected output:** `���` (three U+FFFD replacement characters)

**Content stream representation:** `<000102>` Tj (byte codes 0, 1, 2)

---

### 2. `/CustomA`, `/CustomB` (Custom Encoding)

**Category:** Custom Encodings Without Standard Names

**Source:** Hypothetical encoding scenario (illustrative)

**Description:** PDF generator creates a custom encoding dictionary with non-standard glyph names that appear meaningful but are not recognized by the Adobe Glyph List.

**Why all 4 mapping levels fail:**
- **Level 1 (ToUnicode CMap):** No `/ToUnicode` CMap exists
- **Level 2 (AGL lookup):** `/CustomA` and `/CustomB` are not in the Adobe Glyph List. They do not match the `uniXXXX` or `uXXXXXX` algorithmic patterns (require exactly 4 or up to 6 hex digits)
- **Level 3 (Font fingerprint):** Font is not in the fingerprint cache database
- **Level 4 (Shape recognition):** Feature-gated; no shape database match expected

**Expected output:** `��` (two U+FFFD replacement characters)

**Content stream representation:** Hypothetically `<0003>` for code 0 mapping to `/CustomA`

---

### 3. `/NotAGlyph` (Orphaned Character Code)

**Category:** Orphaned Character Codes

**Source:** Type3 font scenario

**Description:** A glyph name that appears in the encoding dictionary but does not correspond to an actual glyph definition in the font's `/CharProcs` dictionary.

**Why all 4 mapping levels fail:**
- **Level 1 (ToUnicode CMap):** No `/ToUnicode` CMap exists
- **Level 2 (AGL lookup):** `/NotAGlyph` is not in the Adobe Glyph List
- **Level 3 (Font fingerprint):** Type3 fonts are vector-based and don't have embedded font programs for fingerprinting
- **Level 4 (Shape recognition):** The glyph doesn't exist in `/CharProcs`, so there's no shape to recognize. The resolver explicitly checks for glyph presence and emits `GLYPH_UNMAPPED` before reaching Level 4

**Expected output:** `�` (one U+FFFD replacement character)

**Code path:** `resolver.rs` lines 627-636 explicitly handle this Type3 font case

---

### 4. `/glyph_0041` (Non-AGL Algorithmic Pattern)

**Category:** Algorithmic Patterns Outside AGL Convention

**Source:** Illustrative example

**Description:** A glyph name that appears algorithmic (contains hex digits) but does not follow the Adobe Glyph List convention for algorithmic names.

**Why all 4 mapping levels fail:**
- **Level 1 (ToUnicode CMap):** No `/ToUnicode` CMap exists
- **Level 2 (AGL lookup):** `/glyph_0041` is not in AGL direct lookup. The algorithmic parser in `agl.rs` only recognizes:
  - `uniXXXX` (exactly 4 hex digits)
  - `uXXXXXX` (up to 6 hex digits)
  The prefix `/glyph_` is not recognized, so the algorithmic match fails
- **Level 3 (Font fingerprint):** Font is not in the fingerprint cache database
- **Level 4 (Shape recognition):** Feature-gated; no match expected

**Expected output:** `�` (one U+FFFD replacement character)

**Note:** This demonstrates the importance of using the correct AGL algorithmic naming convention

---

## Selected Mapped AGL Glyphs (for Comparison)

These glyphs should be included in the same fixture to demonstrate successful mapping behavior when the fallback chain works:

### 1. `/A` (Standard AGL Glyph)

**Category:** Adobe Glyph List Direct Entry

**Description:** Standard uppercase letter A, direct entry in the Adobe Glyph List.

**Mapping path:**
- Level 1: If `/ToUnicode` present → maps to U+0041 (confidence 1.0)
- Level 2: If no `/ToUnicode` → glyph name `/A` → AGL lookup → U+0041 (confidence 0.9)

**Expected output:** `A`

**Content stream:** `<41>` Tj

---

### 2. `/B` (Standard AGL Glyph)

**Category:** Adobe Glyph List Direct Entry

**Description:** Standard uppercase letter B, direct entry in the Adobe Glyph List.

**Mapping path:**
- Level 1: If `/ToUnicode` present → maps to U+0042 (confidence 1.0)
- Level 2: If no `/ToUnicode` → glyph name `/B` → AGL lookup → U+0042 (confidence 0.9)

**Expected output:** `B`

**Content stream:** `<42>` Tj

---

### 3. `/space` (Standard AGL Glyph)

**Category:** Adobe Glyph List Direct Entry

**Description:** Standard space character, direct entry in the Adobe Glyph List.

**Mapping path:**
- Level 1: If `/ToUnicode` present → maps to U+0020 (confidence 1.0)
- Level 2: If no `/ToUnicode` → glyph name `/space` → AGL lookup → U+0020 (confidence 0.9)

**Expected output:** ` ` (space)

**Content stream:** `<20>` Tj

---

### 4. `/uni0041` (AGL Algorithmic Pattern)

**Category:** Adobe Glyph List Algorithmic Entry

**Description:** Algorithmic glyph name following the `uniXXXX` convention for direct Unicode codepoint mapping.

**Mapping path:**
- Level 1: If `/ToUnicode` present → maps directly (confidence 1.0)
- Level 2: If no `/ToUnicode` → glyph name `/uni0041` → algorithmic parser → U+0041 (confidence 0.9)

**Expected output:** `A`

**Content stream:** Hypothetically `<01>` mapping to glyph ID 1 with name `/uni0041`

---

## Recommended Fixture Structure

Based on these selections, the test fixture should include:

**Unmapped glyphs (to test failure path):**
- Character codes 0, 1, 2 → `/g001`, `/g002`, `/g003`
- Character code 3 → `/CustomA`
- Character code 4 → `/CustomB`
- Character code 5 → `/NotAGlyph`
- Character code 6 → `/glyph_0041`

**Mapped glyphs (to test success path):**
- Character code 65 (0x41) → `/A`
- Character code 66 (0x42) → `/B`
- Character code 32 (0x20) → `/space`
- Character code 99 → `/uni0041` (algorithmic)

**Expected extraction output:**
```
���ABCDE A
```

Where the first 6 positions are U+FFFD for the unmapped glyphs, followed by the mapped glyphs.

---

## Acceptance Criteria

- ✅ Created `notes/bf-68f9i-glyphs.md` documenting selected glyphs
- ✅ Listed 4 unmapped glyphs with categories:
  - PUA glyphs (`/g001`, `/g002`, `/g003`)
  - Custom encoding glyphs (`/CustomA`, `/CustomB`)
  - Orphaned code glyph (`/NotAGlyph`)
  - Non-AGL algorithmic pattern (`/glyph_0041`)
- ✅ Listed 4 mapped AGL glyphs for comparison
- ✅ Explained why each unmapped glyph fails all 4 mapping levels
- ✅ Provided recommended fixture structure with expected output

---

## Summary

The selected glyphs represent the most common unmapped glyph patterns encountered in real-world PDFs:

1. **Private Use Area (PUA) names** - Arbitrary numeric patterns
2. **Custom encoding names** - Meaningful-looking but non-standard
3. **Orphaned codes** - Missing glyph definitions
4. **Misformatted algorithmic names** - Almost correct but invalid prefixes

By including both unmapped and mapped glyphs in the same fixture, we can verify that:
- The 4-level fallback chain correctly identifies unmapped glyphs
- `GLYPH_UNMAPPED` diagnostics are emitted exactly once per (font_id, char_code)
- Mapped glyphs continue to resolve correctly through AGL lookup
- U+FFFD is output for unmapped glyphs while mapped glyphs render correctly
