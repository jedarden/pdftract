# Unmapped Comprehensive PDF Fixture Creation

**Bead ID:** bf-5taa6
**Task:** Create unmapped glyph PDF fixture file
**Date:** 2026-07-03

## Summary

Created the `unmapped-comprehensive.pdf` fixture for testing unmapped glyph handling and the 4-level Unicode fallback chain failure path.

## Files Created

### 1. Generator Script
**Path:** `tests/fixtures/encoding/create_unmapped_comprehensive.py`

Python script that generates the unmapped-comprehensive.pdf fixture according to the design specification in `notes/bf-68f9i-design.md`.

### 2. PDF Fixture
**Path:** `tests/fixtures/encoding/unmapped-comprehensive.pdf`
**Size:** 755 bytes
**Structure:** Minimal PDF with 5 objects (Catalog, Pages, Page, Content stream, Font dictionary)

### 3. Ground Truth
**Path:** `tests/fixtures/encoding/unmapped-comprehensive.txt`
**Size:** 27 bytes
**Content:**
```
���
����
AB 
```

## Fixture Structure

### Character Code to Glyph Name Mapping

| Character Code | Glyph Name | Category | Expected Output |
|----------------|------------|----------|-----------------|
| 0 (0x00) | `/g001` | PUA - Unmapped | U+FFFD (�) |
| 1 (0x01) | `/g002` | PUA - Unmapped | U+FFFD (�) |
| 2 (0x02) | `/g003` | PUA - Unmapped | U+FFFD (�) |
| 3 (0x03) | `/CustomA` | Custom - Unmapped | U+FFFD (�) |
| 4 (0x04) | `/CustomB` | Custom - Unmapped | U+FFFD (�) |
| 5 (0x05) | `/NotAGlyph` | Orphaned - Unmapped | U+FFFD (�) |
| 6 (0x06) | `/glyph_0041` | Non-AGL - Unmapped | U+FFFD (�) |
| 7 (0x07) | `/A` | AGL - Mapped | U+0041 (A) |
| 8 (0x08) | `/B` | AGL - Mapped | U+0042 (B) |
| 9 (0x09) | `/space` | AGL - Mapped | U+0020 (space) |

### Content Stream Layout

The fixture displays text in 3 lines:

- **Line 1 (y=700):** `<000102>` Tj → `/g001`, `/g002`, `/g003` → "���"
- **Line 2 (y=680):** `<03040506>` Tj → `/CustomA`, `/CustomB`, `/NotAGlyph`, `/glyph_0041` → "����"
- **Line 3 (y=660):** `<070809>` Tj → `/A`, `/B`, `/space` → "AB "

### Font Specification

```pdf
5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /UnmappedTestFont
/Encoding <<
/Type /Encoding
/Differences [0 /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041 /A /B /space]
>>
>>
endobj
```

## Acceptance Criteria Status

- ✅ PDF fixture file exists in `tests/fixtures/encoding/` directory
- ✅ Fixture named descriptively as `unmapped-comprehensive.pdf`
- ✅ Fixture contains 7 unmapped glyphs (exceeds requirement of 3):
  - `/g001`, `/g002`, `/g003` (PUA)
  - `/CustomA`, `/CustomB` (Custom)
  - `/NotAGlyph` (Orphaned)
  - `/glyph_0041` (Non-AGL algorithmic)
- ✅ Fixture contains 3 mapped glyphs for comparison: `/A`, `/B`, `/space`
- ✅ Fixture uses the exact structure from `notes/bf-68f9i-design.md`
- ✅ Ground truth file created with expected extraction output

## Expected Behavior

### Text Extraction

The fixture should extract to:
```
���
����
AB 
```

Total: 7 × U+FFFD + 3 mapped characters

### Diagnostic Output

The fixture should emit **7 distinct `GLYPH_UNMAPPED` diagnostics**:

```
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=0, glyph_name=/g001, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=1, glyph_name=/g002, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=2, glyph_name=/g003, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=3, glyph_name=/CustomA, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=4, glyph_name=/CustomB, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=5, glyph_name=/NotAGlyph, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=6, glyph_name=/glyph_0041, reason=not_in_agl
```

Additionally, 3 `GLYPH_RESOLVED` diagnostics for the mapped AGL glyphs (`/A`, `/B`, `/space`).

## Testing Coverage

This fixture provides comprehensive coverage of:

1. **PUA glyphs** — Arbitrary numeric patterns (`/g001`, `/g002`, `/g003`)
2. **Custom encoding** — Non-standard meaningful names (`/CustomA`, `/CustomB`)
3. **Orphaned codes** — Missing glyph definitions (`/NotAGlyph`)
4. **Non-AGL algorithmic** — Invalid prefix patterns (`/glyph_0041`)
5. **Standard AGL** — Success path verification (`/A`, `/B`, `/space`)
6. **Multiple text lines** — Positioning commands (Td)
7. **Diagnostic deduplication** — One diagnostic per unique (font, char_code)

## References

- **Design document:** `notes/bf-68f9i-design.md`
- **Glyph selection:** `notes/bf-68f9i-glyphs.md`
- **Parent bead:** bf-68f9i (fixture design coordinator)
- **Related fixture:** `tests/fixtures/encoding/no-mapping.pdf` (simpler 3-glyph version)

## Commits

- `fixtures(bf-5taa6): add unmapped-comprehensive.pdf fixture and generator script`

## Verification

The fixture structure was verified by:
1. Examining the PDF object structure (5 objects: Catalog, Pages, Page, Content, Font)
2. Confirming the encoding `/Differences` array contains the correct glyph mappings
3. Verifying the content stream contains the correct hex-encoded byte sequences
4. Checking the ground truth file contains the expected UTF-8 output
5. Confirming the PDF trailer has correct xref table and startxref offset

The fixture is ready for use in TH-03 encoding recovery tests to verify:
- GLYPH_UNMAPPED diagnostic emission
- U+FFFD replacement character output
- Proper AGL lookup for mapped glyphs
- Diagnostic deduplication behavior
