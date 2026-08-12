# unmapped-comprehensive.pdf Fixture

## Purpose

Comprehensive GLYPH_UNMAPPED diagnostic test fixture covering all 4 mapping failure modes of the Unicode recovery pipeline, plus verification of the AGL success path. This fixture exercises the complete 4-level fallback chain with realistic unmapped glyph patterns.

Expected behavior:
- **7 unmapped glyphs** emit GLYPH_UNMAPPED diagnostics (one per unique glyph)
- **3 mapped glyphs** extract correctly via AGL Level 2 fallback
- Tests diagnostic deduplication (same glyph appears once, not once-per-occurrence)

## Structure

### PDF Properties
- **PDF Version:** 1.4
- **Pages:** 1
- **Page Size:** 612 x 792 pts (Letter)
- **File Size:** ~651 bytes
- **Encrypted:** No
- **Tagged:** No

### Font Details
```
Name: UnmappedTestFont
Type: Type 1
Encoding: Custom (Differences array)
Embedded: No
ToUnicode CMap: No
```

**Font Object (5 0 obj):**
```pdf
<<
/Type /Font
/Subtype /Type1
/BaseFont /UnmappedTestFont
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
>>
```

## Glyph Mappings

The fixture defines **10 character codes** with comprehensive coverage:

| Code | Glyph Name    | Category            | Has Mapping? | Expected Output |
|------|---------------|---------------------|--------------|-----------------|
| 0    | `/g001`       | PUA - Unmapped      | NO           | U+FFFD (�)       |
| 1    | `/g002`       | PUA - Unmapped      | NO           | U+FFFD (�)       |
| 2    | `/g003`       | PUA - Unmapped      | NO           | U+FFFD (�)       |
| 3    | `/CustomA`    | Custom - Unmapped   | NO           | U+FFFD (�)       |
| 4    | `/CustomB`    | Custom - Unmapped   | NO           | U+FFFD (�)       |
| 5    | `/NotAGlyph`  | Orphaned - Unmapped | NO           | U+FFFD (�)       |
| 6    | `/glyph_0041` | Non-AGL - Unmapped  | NO           | U+FFFD (�)       |
| 7    | `/A`          | AGL - Mapped        | YES          | U+0041 (A)       |
| 8    | `/B`          | AGL - Mapped        | YES          | U+0042 (B)       |
| 9    | `/space`      | AGL - Mapped        | YES          | U+0020 (space)   |

### Unmapped Glyph Categories

**1. Private Use Area (PUA) patterns** - `/g001`, `/g002`, `/g003`
- Arbitrary numeric names with `g` prefix
- Common pattern in legacy PDFs
- Not in Adobe Glyph List

**2. Custom encoding names** - `/CustomA`, `/CustomB`
- Meaningful semantic naming
- Not standard AGL glyphs
- Tests custom font encodings

**3. Orphaned glyph** - `/NotAGlyph`
- Name suggests missing CharProcs entry
- Tests invalid glyph reference handling
- Simulates corrupted PDF structures

**4. Non-AGL algorithmic** - `/glyph_0041`
- Wrong prefix for algorithmic AGL parsing
- Should be `/uni0041` for AGL algorithmic match
- Tests glyph name validation

### Mapped Glyph Categories

**Standard AGL glyphs** - `/A`, `/B`, `/space`
- All in Adobe Glyph List
- Should resolve via Level 2 fallback
- Provides success path verification

## Content Stream Layout

**Location:** Object 4 0 obj

**Visual Layout:**
```
Line 1 (y=700): [g001][g002][g003]              → "���"
Line 2 (y=680): [CustomA][CustomB][NotAGlyph][glyph_0041] → "����"
Line 3 (y=660): [A][B][space]                   → "AB "
```

**Raw Content Stream:**
```pdf
BT
/F1 12 Tf
% Line 1: Three PUA glyphs (codes 0, 1, 2)
50 700 Td
<000102> Tj
% Line 2: Custom and orphaned glyphs (codes 3, 4, 5, 6)
50 680 Td
<03040506> Tj
% Line 3: Mapped AGL glyphs (codes 7, 8, 9) 
50 660 Td
<070809> Tj
ET
```

**Content stream breakdown:**
- **BT/ET:** Begin/End text block
- **/F1 12 Tf:** Set font to resource F1 at 12pt size
- **50 700 Td:** Move text position to (50, 700) 
- **<000102> Tj:** Show text string with byte codes [0, 1, 2]
- **<03040506> Tj:** Show text string with byte codes [3, 4, 5, 6]
- **<070809> Tj:** Show text string with byte codes [7, 8, 9]

## Expected Extraction Output

### Text Content

The fixture should extract to **3 lines**:

```
���
����
AB 
```

**Breakdown by line:**
- **Line 1:** 3 × U+FFFD (for `/g001`, `/g002`, `/g003`)
- **Line 2:** 4 × U+FFFD (for `/CustomA`, `/CustomB`, `/NotAGlyph`, `/glyph_0041`)
- **Line 3:** "AB " (U+0041, U+0042, U+0020 for `/A`, `/B`, `/space`)

**Total:** 7 × U+FFFD + 3 mapped characters

### Diagnostic Output

The fixture should emit **7 distinct GLYPH_UNMAPPED diagnostics** (one per unique unmapped glyph):

```
[WARN] FONT_GLYPH_UNMAPPED: font_id=5, char_code=0, glyph_name=/g001, reason=not_in_agl
[WARN] FONT_GLYPH_UNMAPPED: font_id=5, char_code=1, glyph_name=/g002, reason=not_in_agl
[WARN] FONT_GLYPH_UNMAPPED: font_id=5, char_code=2, glyph_name=/g003, reason=not_in_agl
[WARN] FONT_GLYPH_UNMAPPED: font_id=5, char_code=3, glyph_name=/CustomA, reason=not_in_agl
[WARN] FONT_GLYPH_UNMAPPED: font_id=5, char_code=4, glyph_name=/CustomB, reason=not_in_agl
[WARN] FONT_GLYPH_UNMAPPED: font_id=5, char_code=5, glyph_name=/NotAGlyph, reason=not_in_agl
[WARN] FONT_GLYPH_UNMAPPED: font_id=5, char_code=6, glyph_name=/glyph_0041, reason=not_in_agl
```

**Important:** The diagnostic should be emitted **once per (font_id, char_code)** combination. If the same unmapped glyph appears multiple times in the content stream, it should only emit one diagnostic (not one per text position).

### Success Path Verification

The fixture should also verify that mapped glyphs resolve correctly:

```
[INFO] GLYPH_RESOLVED: font_id=5, char_code=7, glyph_name=/A, unicode=U+0041, source=agl, confidence=0.9
[INFO] GLYPH_RESOLVED: font_id=5, char_code=8, glyph_name=/B, unicode=U+0042, source=agl, confidence=0.9
[INFO] GLYPH_RESOLVED: font_id=5, char_code=9, glyph_name=/space, unicode=U+0020, source=agl, confidence=0.9
```

## Inspection Commands

### Basic PDF info:
```bash
pdfinfo tests/fixtures/encoding/unmapped-comprehensive.pdf
```

**Expected Output:**
```
Pages:           1
Page size:       612 x 792 pts (letter)
File size:       651 bytes
PDF version:     1.4
```

### Font details:
```bash
pdffonts tests/fixtures/encoding/unmapped-comprehensive.pdf
```

**Expected Output:**
```
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
UnmappedTestFont                     Type 1            Custom           no  no  no      [X]  0 R
```

### Test extraction:
```bash
# Build pdftract first if needed
cargo build --release

# Extract to see the mixed output
./target/release/pdftract extract tests/fixtures/encoding/unmapped-comprehensive.pdf \
  --format text -o /tmp/unmapped-comprehensive-output.txt
cat /tmp/unmapped-comprehensive-output.txt
```

**Expected Output:**
```
���
����
AB 
```

### Verify diagnostics:
```bash
# Extract with diagnostics enabled
./target/release/pdftract extract tests/fixtures/encoding/unmapped-comprehensive.pdf \
  --format json -o /tmp/unmapped-comprehensive-output.json

# Check for GLYPH_UNMAPPED in diagnostics
jq '.diagnostics[] | select(.code == "FONT_GLYPH_UNMAPPED")' /tmp/unmapped-comprehensive-output.json
```

## Regeneration Instructions

### Using the Rust generator (Recommended)

The fixture is generated by `xtask/src/bin/gen_unmapped_fixtures.rs`.

**Steps:**
```bash
# Run the generator
cargo run --bin gen_unmapped_fixtures
```

**Output location:** `tests/fixtures/encoding/unmapped-comprehensive.pdf`

**Ground truth:** `tests/fixtures/encoding/unmapped-comprehensive.txt`

### Manual reconstruction (Not recommended)

To manually reconstruct this PDF:

1. Create the 6 PDF objects as shown in the "Raw PDF Structure" section of the design doc
2. Build the xref table with correct byte offsets
3. Add the trailer with `/Size 6` and `/Root 1 0 R`
4. Write the complete PDF to `tests/fixtures/encoding/unmapped-comprehensive.pdf`

**Use the Rust generator instead** - it automatically generates the fixture with correct structure.

## Verification

After regeneration, verify:

```bash
# Check file size is approximately correct (should be ~651 bytes)
ls -la tests/fixtures/encoding/unmapped-comprehensive.pdf

# Verify extraction produces expected mixed output
./target/release/pdftract extract tests/fixtures/encoding/unmapped-comprehensive.pdf \
  --format text -o /tmp/verify-output.txt
diff /tmp/verify-output.txt tests/fixtures/encoding/unmapped-comprehensive.txt

# Count U+FFFD characters (should be 7)
cat /tmp/verify-output.txt | grep -o $'�' | wc -l
# Expected: 7
```

## Test Coverage Matrix

| Test Scenario | Glyphs | Coverage |
|--------------|--------|----------|
| PUA glyphs | `/g001`, `/g002`, `/g003` | Private Use Area patterns |
| Custom encoding | `/CustomA`, `/CustomB` | Non-standard meaningful names |
| Orphaned codes | `/NotAGlyph` | Missing glyph definitions |
| Non-AGL algorithmic | `/glyph_0041` | Invalid prefix patterns |
| Standard AGL | `/A`, `/B`, `/space` | Success path verification |
| Multiple text lines | 3 lines | Positioning commands (Td) |
| Diagnostic deduplication | 7 unmapped glyphs | One diagnostic per unique (font, code) |

## Test References

This fixture is used in the following tests:

- `tests/encoding_recovery.rs` — Unicode recovery integration tests
- `tests/test_glyph_unmapped_diagnostic.rs` — GLYPH_UNMAPPED diagnostic validation
- `tests/test_glyph_unmapped_diagnostic_content.rs` — Diagnostic content verification

## Related Fixtures

- `no-mapping.pdf` — Simpler 3-glyph fixture (PUA only)
- `unmapped-glyphs.pdf` — 10 unmapped glyph fixture (no mapped glyphs)
- `agl-only.pdf` — AGL-only success case (Level 2)
- `fingerprint-match.pdf` — Font fingerprinting (Level 3)
- `shape-match.pdf` — Glyph shape recognition (Level 4)

## Design Documentation

See **[`notes/bf-68f9i-design.md`](notes/bf-68f9i-design.md)** for complete design details:
- PDF structure specification with all 6 objects
- Character code to glyph name mapping rationale
- Content stream command breakdown
- Diagnostic emission behavior specification
- Test coverage scenarios
- Implementation strategy

See **[`notes/bf-68f9i.md`](notes/bf-68f9i.md)** for fixture creation details:
- Generation command output
- pdfinfo validation
- Expected output verification

## History

- **Created:** 2026-07-03 (bf-5taa6)
- **Regenerated:** 2026-08-12 (SHA256 updated after fixture regeneration)
- **Documented:** 2026-08-12 (bf-16ztr)
- **SHA256:** `a7effe0945c9120e5ab33ac84557755206dd10049d64a5dbd7ba5db4597ed3d3`

## Notes

The key insight of this fixture is that it represents the **most comprehensive unmapped glyph test**:

1. **Tests all 4 mapping failure modes:**
   - PUA patterns (`/g001`, `/g002`, `/g003`)
   - Custom encoding (`/CustomA`, `/CustomB`)
   - Orphaned glyphs (`/NotAGlyph`)
   - Non-AGL algorithmic (`/glyph_0041`)

2. **Verifies success path** — Includes standard AGL glyphs (`/A`, `/B`, `/space`) for comparison

3. **Tests realistic PDF structure** — Type1 font, custom encoding, no ToUnicode

4. **Exercises content stream commands** — Multi-line layout with Td positioning

5. **Validates diagnostic behavior** — Unique diagnostics per (font_id, char_code)

This fixture is the most complete test of the Unicode recovery pipeline's failure modes while still providing a clear success path for validation. It builds on the simpler `no-mapping.pdf` and `unmapped-glyphs.pdf` patterns to provide comprehensive coverage of encoding recovery edge cases.
