# Unmapped Glyph Generator Script Implementation

**Task:** Create generator script for unmapped glyph PDF
**Bead ID:** bf-3cwge
**Date:** 2026-07-03
**Status:** ✅ COMPLETE

## Implementation Summary

Created `tests/fixtures/encoding/generate_unmapped_glyphs.py` - a flexible Python script for generating unmapped glyph PDF fixtures with custom encodings.

## Script Features

### Command-Line Interface
```bash
python3 generate_unmapped_glyphs.py [OPTIONS]
```

**Options:**
- `--output PATH`: Output PDF path (default: unmapped-glyphs.pdf)
- `--ground-truth PATH`: Ground truth .txt path (default: unmapped-glyphs.txt)
- `--title NAME`: Font base name (default: UnmappedTestFont)
- `--glyphs JSON`: Glyph mappings as JSON string (default: built-in test set)
- `--no-ground-truth`: Skip generating ground truth file

### Usage Examples

1. **Generate with default test glyphs:**
   ```bash
   python3 generate_unmapped_glyphs.py
   ```

2. **Generate with custom glyphs:**
   ```bash
   python3 generate_unmapped_glyphs.py --glyphs '{"0": "/CustomGlyph1", "1": "/CustomGlyph2"}'
   ```

3. **Generate to specific output file:**
   ```bash
   python3 generate_unmapped_glyphs.py --output my-test.pdf --ground-truth my-test.txt
   ```

## Default Glyph Set

The script includes a built-in test fixture matching the design specification from notes/bf-68f9i-design.md:

| Character Code | Glyph Name | Category | Expected Output |
|----------------|------------|----------|-----------------|
| 0 | /g001 | PUA - Unmapped | U+FFFD |
| 1 | /g002 | PUA - Unmapped | U+FFFD |
| 2 | /g003 | PUA - Unmapped | U+FFFD |
| 3 | /CustomA | Custom - Unmapped | U+FFFD |
| 4 | /CustomB | Custom - Unmapped | U+FFFD |
| 5 | /NotAGlyph | Orphaned - Unmapped | U+FFFD |
| 6 | /glyph_0041 | Non-AGL - Unmapped | U+FFFD |
| 7 | /A | AGL - Mapped | U+0041 (A) |
| 8 | /B | AGL - Mapped | U+0042 (B) |
| 9 | /space | AGL - Mapped | U+0020 |

## Technical Details

### PDF Structure
- **Format:** PDF 1.4
- **Page Size:** US Letter (612 x 792 pts)
- **Font:** Type1 with custom encoding, no embedding
- **Objects:** 6 indirect objects (Catalog, Pages, Page, Content stream, Font)
- **File Size:** ~743 bytes (with default glyph set)

### Generated Content Stream
```pdf
BT
/F1 12 Tf
50 700 Td
<00010203040506070809> Tj
ET
```

All character codes are displayed in a single line with proper hex encoding.

### Encoding Dictionary
```pdf
/Encoding <<
/Type /Encoding
/Differences [0 /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041 /A /B /space]
>>
```

## Acceptance Criteria Verification

✅ **Generator script exists in tests/fixtures/encoding/ directory**
   - Created `tests/fixtures/encoding/generate_unmapped_glyphs.py` (8,256 bytes)
   - Executable permissions set (`chmod +x`)

✅ **Script accepts parameters for glyph names and encoding**
   - Supports `--glyphs` JSON parameter for custom glyph mappings
   - Supports `--title` parameter for custom font names
   - Supports `--output` and `--ground-truth` for file paths

✅ **Script generates a valid PDF with the specified structure**
   - Valid PDF 1.4 format verified
   - Proper Type1 font encoding with /Differences array
   - Correct xref table and trailer structure
   - Test runs produce parseable PDFs

✅ **Script tested with a simple dry run to verify it produces output**
   - Tested with default glyphs: `/tmp/final-test.pdf` (743 bytes)
   - Tested with custom glyphs: `/tmp/custom-test.pdf` (673 bytes)
   - Both tests generated valid PDFs and ground truth files

✅ **Script committed to git**
   - Commit: `feat(bf-3cwge): add unmapped glyph generator script`

## Design References

- **Design specification:** notes/bf-68f9i-design.md
- **Glyph selection:** notes/bf-68f9i-glyphs.md
- **Prerequisite decision:** bf-ad2pp (analyzed fixture requirements)

## Testing Results

### Dry Run 1: Default Glyphs
```bash
python3 tests/fixtures/encoding/generate_unmapped_glyphs.py --output /tmp/final-test.pdf
```
**Output:**
- PDF: `/tmp/final-test.pdf` (743 bytes)
- Ground truth: `/tmp/final-test.txt` (331 bytes)
- 10 character codes mapped correctly

### Dry Run 2: Custom Glyphs
```bash
python3 tests/fixtures/encoding/generate_unmapped_glyphs.py \
  --glyphs '{"0": "/TestGlyph1", "1": "/TestGlyph2", "2": "/TestGlyph3"}' \
  --output /tmp/custom-test.pdf \
  --title CustomFont
```
**Output:**
- PDF: `/tmp/custom-test.pdf` (673 bytes)
- Ground truth: `unmapped-glyphs.txt` (145 bytes)
- 3 custom character codes mapped correctly

## Files Created

1. **Generator script:** `tests/fixtures/encoding/generate_unmapped_glyphs.py`
2. **Verification note:** `notes/bf-3cwge.md`

## Expected Use Case

This generator will be used to:
1. Create test fixtures for the Unicode recovery pipeline
2. Test GLYPH_UNMAPPED diagnostic emission
3. Verify the 4-level fallback chain failure path
4. Generate custom fixtures for edge case testing

## Commit Details

**Commit Message:** `feat(bf-3cwge): add unmapped glyph generator script`

**Files in commit:**
- `tests/fixtures/encoding/generate_unmapped_glyphs.py` (new)
- `notes/bf-3cwge.md` (new)

**Commit Body:**
- Implements parameterized generator for unmapped glyph PDF fixtures
- Supports custom glyph names, font titles, and output paths
- Includes built-in test set from design specification
- Generates valid PDF 1.4 with Type1 custom encoding
- Passes all acceptance criteria for bead bf-3cwge

---

## Summary

✅ **All acceptance criteria met.**

The generator script provides a flexible, well-documented tool for creating unmapped glyph test fixtures. It supports both the predefined test set from the design specification and completely custom glyph mappings, making it suitable for a wide range of testing scenarios.
