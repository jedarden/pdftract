# Unmapped Glyph Fixture Verification

**Task:** bf-r4ydg - Add mapped glyphs and generate final fixture  
**Date:** 2026-08-12  
**Fixture:** tests/fixtures/encoding/unmapped-glyphs.pdf

## Overview

This fixture tests the 4-level Unicode fallback chain by including both unmapped and mapped glyphs in a single PDF. It verifies that:
1. Unmapped glyphs emit `GLYPH_UNMAPPED` diagnostics
2. Mapped glyphs resolve successfully via AGL lookup
3. The PDF structure matches the design specification

## Fixture Structure

### File Information
- **Path:** tests/fixtures/encoding/unmapped-glyphs.pdf
- **Size:** 755 bytes
- **Type:** Type1 font with custom encoding
- **Page Size:** US Letter (612 x 792 pts)
- **Design Reference:** notes/bf-68f9i-design.md

### Font Encoding

The fixture uses a custom `/Differences` array mapping 10 character codes:

| Code | Glyph Name | Category | Expected Output |
|------|------------|----------|-----------------|
| 0 | /g001 | PUA - Unmapped | U+FFFD (�) |
| 1 | /g002 | PUA - Unmapped | U+FFFD (�) |
| 2 | /g003 | PUA - Unmapped | U+FFFD (�) |
| 3 | /CustomA | Custom - Unmapped | U+FFFD (�) |
| 4 | /CustomB | Custom - Unmapped | U+FFFD (�) |
| 5 | /NotAGlyph | Orphaned - Unmapped | U+FFFD (�) |
| 6 | /glyph_0041 | Non-AGL - Unmapped | U+FFFD (�) |
| 7 | /A | AGL - Mapped | U+0041 (A) |
| 8 | /B | AGL - Mapped | U+0042 (B) |
| 9 | /space | AGL - Mapped | U+0020 (space) |

**Total:** 7 unmapped glyphs + 3 mapped glyphs

### Content Layout

The PDF displays text in 3 lines to test both positioning and the full glyph set:

```
Line 1 (y=700): <000102> Tj   → /g001, /g002, /g003
Line 2 (y=680): <03040506> Tj → /CustomA, /CustomB, /NotAGlyph, /glyph_0041  
Line 3 (y=660): <070809> Tj   → /A, /B, /space
```

## Expected Extraction Output

### Text Content

Based on tests/fixtures/encoding/unmapped-glyphs.txt:

```
���
����
AB 
```

**Breakdown:**
- Line 1: 3 × U+FFFD (for /g001, /g002, /g003)
- Line 2: 4 × U+FFFD (for /CustomA, /CustomB, /NotAGlyph, /glyph_0041)
- Line 3: "AB " (U+0041, U+0042, U+0020 for /A, /B, /space)

**Total:** 7 × U+FFFD + 3 mapped characters

### Expected Diagnostics

The fixture should emit 7 distinct `GLYPH_UNMAPPED` diagnostics:

```
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=0, glyph_name=/g001, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=1, glyph_name=/g002, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=2, glyph_name=/g003, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=3, glyph_name=/CustomA, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=4, glyph_name=/CustomB, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=5, glyph_name=/NotAGlyph, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=6, glyph_name=/glyph_0041, reason=not_in_agl
```

The fixture should also emit 3 `GLYPH_RESOLVED` diagnostics for the mapped glyphs:

```
[INFO] GLYPH_RESOLVED: font_id=5, char_code=7, glyph_name=/A, unicode=U+0041, source=agl
[INFO] GLYPH_RESOLVED: font_id=5, char_code=8, glyph_name=/B, unicode=U+0042, source=agl
[INFO] GLYPH_RESOLVED: font_id=5, char_code=9, glyph_name=/space, unicode=U+0020, source=agl
```

## Verification Results

### Acceptance Criteria Status

✅ **PASS:** PDF fixture file exists at tests/fixtures/encoding/unmapped-glyphs.pdf  
✅ **PASS:** Fixture contains 3+ unmapped glyphs (7 unmapped: /g001, /g002, /g003, /CustomA, /CustomB, /NotAGlyph, /glyph_0041)  
✅ **PASS:** Fixture contains 2-3 mapped glyphs for comparison (3 mapped: /A, /B, /space)  
✅ **PASS:** Fixture structure matches notes/bf-68f9i-design.md (3-line layout, proper encoding)  
✅ **PASS:** File is a valid PDF (755 bytes, starts with %PDF-1.4)  
✅ **PASS:** Fixture checked into git  
✅ **PASS:** Note file created (this document)

### Validation Commands

```bash
# Verify PDF validity
python3 -c "
with open('tests/fixtures/encoding/unmapped-glyphs.pdf', 'rb') as f:
    content = f.read()
    assert content.startswith(b'%PDF'), 'Invalid PDF format'
    print('✓ Valid PDF format')
"

# Verify file structure
python3 -c "
with open('tests/fixtures/encoding/unmapped-glyphs.pdf', 'rb') as f:
    content = f.read().decode('latin-1', errors='replace')
    assert '/UnmappedTestFont' in content
    assert '/Differences' in content
    assert '/g001' in content and '/A' in content
    print('✓ Contains both unmapped and mapped glyphs')
"

# Verify ground truth
test -f tests/fixtures/encoding/unmapped-glyphs.txt && echo "✓ Ground truth file exists"
```

## Generator

The fixture was generated using `tools/create_unmapped_comprehensive.py`:

```bash
python3 tools/create_unmapped_comprehensive.py
```

This creates both `unmapped-comprehensive.pdf` and `unmapped-glyphs.pdf` (copied).

## Related Documentation

- **Design spec:** notes/bf-68f9i-design.md
- **Glyph selection:** notes/bf-68f9i-glyphs.md
- **Generator:** tools/create_unmapped_comprehensive.py
- **Dependencies:** bf-ad2pp (requirements), bf-3cwge (generator), bf-84xr8 (unmapped glyphs)

## Test Coverage

This fixture provides coverage for:
- PUA glyphs (/g001, /g002, /g003)
- Custom encoding names (/CustomA, /CustomB)
- Orphaned glyph codes (/NotAGlyph)
- Non-AGL algorithmic patterns (/glyph_0041)
- Standard AGL success path (/A, /B, /space)
- Multi-line content positioning (Td commands)
- Diagnostic deduplication (one warning per unique font+code combination)

## Summary

The fixture successfully implements the comprehensive unmapped glyph test design with 7 unmapped glyphs for failure path testing and 3 mapped glyphs for success path comparison. All acceptance criteria have been met.
