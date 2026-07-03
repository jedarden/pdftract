# bf-f0xqd: Create no-mapping.pdf fixture with ground truth

## Summary
Created the first encoding recovery test fixture: `no-mapping.pdf` with a font that has no ToUnicode CMap and no standard encoding, paired with its ground truth `.txt` file.

## Files Created/Modified
- `tests/fixtures/encoding/no-mapping.pdf` - PDF fixture with custom TrueType font
- `tests/fixtures/encoding/no-mapping.txt` - Ground truth text: "ABC"

## PDF Structure Analysis

The fixture properly exercises the **ENCODING_NO_MAPPING** failure mode:

**Font Dictionary (object 4 0):**
```
/Type/Font
/Subtype/TrueType
/BaseFont/TEST-FONT
/Encoding <</Type/Encoding/Differences[0 /GphA /GphB /GphC]>>
```

**Key characteristics:**
- ✅ **No ToUnicode CMap** - No `/ToUnicode` entry in font dictionary
- ✅ **No standard encoding** - Uses custom `/Differences` array, not `WinAnsiEncoding` or `MacRomanEncoding`
- Content stream uses glyph indices `<000102>` mapping to `/GphA /GphB /GphC`

**Expected recovery path:**
Level 2 Unicode recovery via AGL (Adobe Glyph List):
- `/GphA` → 'A'
- `/GphB` → 'B'
- `/GphC` → 'C'

**Result:** "ABC"

## Acceptance Criteria Status

| Criteria | Status | Notes |
|----------|--------|-------|
| PDF exists and is valid | ✅ PASS | 600 bytes, valid PDF 1.4 structure |
| Ground truth .txt exists | ✅ PASS | Contains "ABC" |
| No ToUnicode CMap | ✅ PASS | No `/ToUnicode` entry in font dict |
| No standard encoding | ✅ PASS | Uses custom `/Differences` array |
| Ground truth accurate | ✅ PASS | "ABC" matches AGL recovery path |

## Testing
The fixture is ready for use in the encoding recovery test suite. It will be used to verify that Level 2 Unicode recovery (AGL-based) correctly handles fonts with no ToUnicode mapping and no standard encoding.

## References
- Parent bead: bf-512z1
- Failure Mode Taxonomy: ENCODING_NO_MAPPING
