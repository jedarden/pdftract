# Understanding of Unmapped Glyph Requirements

**Task:** Read and understand glyph requirements
**Bead ID:** bf-3ogvjq
**Date:** 2026-08-14
**Parent:** bf-84xr8

## Understanding Summary

I have read and understood the glyph requirements from the design documents.

### 7 Unmapped Glyphs Identified

1. **`/g001`** - Private Use Area (PUA) glyph
2. **`/g002`** - Private Use Area (PUA) glyph
3. **`/g003`** - Private Use Area (PUA) glyph
4. **`/CustomA`** - Custom encoding (non-standard meaningful name)
5. **`/CustomB`** - Custom encoding (non-standard meaningful name)
6. **`/NotAGlyph`** - Orphaned character code (missing glyph definition)
7. **`/glyph_0041`** - Non-AGL algorithmic pattern (invalid prefix)

### 3 Mapped Glyphs Identified (for comparison)

1. **`/A`** - Standard AGL glyph → U+0041 (A)
2. **`/B`** - Standard AGL glyph → U+0042 (B)
3. **`/space`** - Standard AGL glyph → U+0020 (space)

### Expected Fixture Structure

**PDF Type:** Type1 font (not Type3)
**Encoding:** Custom `/Differences` array
**Font Embedding:** None
**ToUnicode CMap:** None
**Page Size:** US Letter (612 x 792 pts)
**Content:** 3 lines of text

**Character Code to Glyph Name Mapping (codes 0-9):**

| Code | Byte | Glyph Name | Category | Expected Output |
|------|------|------------|----------|-----------------|
| 0 | 0x00 | `/g001` | PUA - Unmapped | U+FFFD (�) |
| 1 | 0x01 | `/g002` | PUA - Unmapped | U+FFFD (�) |
| 2 | 0x02 | `/g003` | PUA - Unmapped | U+FFFD (�) |
| 3 | 0x03 | `/CustomA` | Custom - Unmapped | U+FFFD (�) |
| 4 | 0x04 | `/CustomB` | Custom - Unmapped | U+FFFD (�) |
| 5 | 0x05 | `/NotAGlyph` | Orphaned - Unmapped | U+FFFD (�) |
| 6 | 0x06 | `/glyph_0041` | Non-AGL - Unmapped | U+FFFD (�) |
| 7 | 0x07 | `/A` | AGL - Mapped | U+0041 (A) |
| 8 | 0x08 | `/B` | AGL - Mapped | U+0042 (B) |
| 9 | 0x09 | `/space` | AGL - Mapped | U+0020 (space) |

### Expected Output

**Text Content:**
```
���
����
AB 
```

**Total:** 7 × U+FFFD + "AB " (3 mapped characters)

**Diagnostic Output:**
- 7 distinct `GLYPH_UNMAPPED` warnings (one per unique unmapped glyph)
- 3 `GLYPH_RESOLVED` info messages (for `/A`, `/B`, `/space`)

### Type1 Font /Differences Encoding Understanding

The `/Differences` encoding array maps character codes to glyph names:

```pdf
/Encoding <<
/Type /Encoding
/Differences [0 
  /g001         % code 0  → PUA glyph
  /g002         % code 1  → PUA glyph
  /g003         % code 2  → PUA glyph
  /CustomA      % code 3  → custom encoding
  /CustomB      % code 4  → custom encoding
  /NotAGlyph    % code 5  → orphaned glyph
  /glyph_0041   % code 6  → non-AGL algorithmic
  /A            % code 7  → standard AGL
  /B            % code 8  → standard AGL
  /space        % code 9  → standard AGL
]
>>
```

This encoding is used in the content stream with byte codes:
- `<000102>` → displays `/g001/g002/g003` → "���"
- `<03040506>` → displays `/CustomA/CustomB/NotAGlyph/glyph_0041` → "����"
- `<070809>` → displays `/A/B/space` → "AB "

### Fixture File Location

**PDF:** `tests/fixtures/encoding/unmapped-comprehensive.pdf`
**Ground Truth:** `tests/fixtures/encoding/unmapped-comprehensive.txt`

## Acceptance Criteria Status

- ✅ Design documents (notes/bf-68f9i-*.md) have been read and understood
- ✅ List of 7 unmapped glyphs identified and documented
- ✅ List of 3 mapped glyphs identified and documented
- ✅ Expected fixture structure documented
- ✅ Understanding of Type1 font /Differences encoding confirmed
- ✅ Verification note created at notes/bf-84xr8-child-1.md

## References

- Design doc: notes/bf-68f9i-design.md
- Glyph list: notes/bf-68f9i-glyphs.md
- Parent bead: bf-84xr8
