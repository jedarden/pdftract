# Unmapped Glyph PDF Fixture

**Task:** Create unmapped glyph PDF fixture  
**Bead ID:** bf-68f9i  
**Date:** 2026-07-03  

## Overview

Created a new PDF test fixture containing glyphs without valid Unicode mappings to trigger GLYPH_UNMAPPED diagnostics in pdftract.

## Fixture Details

**File:** `tests/fixtures/encoding/unmapped-glyphs.pdf`

**Validation:**  
```bash
$ pdfinfo tests/fixtures/encoding/unmapped-glyphs.pdf
Pages:           1
Encrypted:       no
Page size:       612 x 792 pts (letter)
PDF version:     1.4
File size:       688 bytes
```

**Font Information:**  
```
name: UnmappedGlyphs
type: Type 1
encoding: Custom
embedded: no
subset: no
unicode: no
```

## Unmapped Glyphs

The fixture contains **4 distinct unmapped glyphs**:

1. **`/CustomAlpha`** - Custom glyph name not in AGL
2. **`/CustomBeta`** - Custom glyph name not in AGL
3. **`/CustomGamma`** - Custom glyph name not in AGL
4. **`/CustomDelta`** - Custom glyph name not in AGL

**Byte codes:** `00 01 02 03` (character codes 0-3)

**Custom encoding:** `/Differences [0 /CustomAlpha /CustomBeta /CustomGamma /CustomDelta]`

**Why unmapped:** These glyph names are arbitrary custom names not recognized by the Adobe Glyph List (AGL), and the PDF has no `/ToUnicode` CMap or embedded font program for fingerprint recovery. All 4 levels of the encoding fallback chain will fail.

## Content Stream

```
BT
/F1 12 Tf
50 700 Td
<00010203> Tj
ET
```

## Expected Output

**Expected text output:** `����` (4 × U+FFFD replacement characters)

**Expected diagnostics:** 4 × `GLYPH_UNMAPPED` (FontGlyphUnmapped)

## Acceptance Criteria Status

- ✅ PDF fixture exists in tests/fixtures/encoding/ directory
- ✅ Fixture contains at least 3 unmapped glyphs (contains 4: CustomAlpha, CustomBeta, CustomGamma, CustomDelta)
- ✅ Fixture is valid (passes pdfinfo validation)
- ✅ Fixture is named descriptively (unmapped-glyphs.pdf)
- ✅ Expected output file created (unmapped-glyphs.txt)

## Generation Script

**File:** `tests/fixtures/encoding/generate_unmapped_glyphs.rs`

**To regenerate:**  
```bash
cd tests/fixtures/encoding
rustc -o generate_unmapped_glyphs_bin generate_unmapped_glyphs.rs
./generate_unmapped_glyphs_bin
```

## Technical Implementation

The fixture uses the same structure as the existing `no-mapping.pdf` but with:
- More descriptive glyph names (`/CustomAlpha` instead of `/g001`)
- 4 unmapped glyphs instead of 3 (exceeds the "at least 3" requirement)
- Clear semantic naming for easier debugging

The PDF structure follows the standard minimal PDF pattern:
- Catalog → Pages → Page → Content stream
- Type1 font with custom encoding
- NO `/ToUnicode` CMap
- NO embedded font program
- NO standard encoding reference

## Relationship to Other Fixtures

This fixture complements the existing encoding recovery fixtures:
- **agl-only.pdf**: Tests Level 2 success (AGL names work)
- **no-mapping.pdf**: Tests unmapped glyphs with `/g001`, `/g002`, `/g003` pattern
- **unmapped-glyphs.pdf** (this fixture): Tests unmapped glyphs with descriptive custom names
- **fingerprint-match.pdf**: Tests Level 3 success (font fingerprinting)
- **shape-match.pdf**: Tests Level 4 success (glyph shape recognition)

## References

- Research from child bead bf-1cvmt: `notes/bf-1cvmt.md`
- Font resolver code: `crates/pdftract-core/src/font/resolver.rs`
- AGL lookup code: `crates/pdftract-core/src/font/agl.rs`
- Existing fixture generation: `tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs`

## Summary

Successfully created a valid PDF test fixture containing 4 unmapped glyphs that will trigger GLYPH_UNMAPPED diagnostics in pdftract. The fixture passes pdfinfo validation, is descriptively named, and provides clear unmapped glyph patterns for testing the 4-level encoding fallback chain's failure path.
