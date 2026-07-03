# Unmapped Glyph PDF Fixture Design

**Task:** Design unmapped glyph PDF fixture structure  
**Bead ID:** bf-is1os  
**Design Document:** notes/bf-68f9i-design.md  
**Date:** 2026-07-03  
**Based on:** notes/bf-68f9i-glyphs.md

## Overview

This document specifies the complete PDF structure for the unmapped glyph test fixture. The fixture is designed to exercise the 4-level Unicode fallback chain failure path and verify proper `GLYPH_UNMAPPED` diagnostic emission.

The fixture includes:
- **6 unmapped glyphs** (testing failure path)
- **4 mapped glyphs** (testing success path for comparison)
- **Single-page PDF** with simple text layout
- **Type1 font** with custom encoding (no embedding)

## Design Decisions

### Font Type: Type1 (not Type3)

**Rationale:**
- Type1 fonts are simpler to generate and more commonly encountered in real-world PDFs
- Type1 with custom encoding is a well-supported PDF specification feature
- Type3 fonts require `/CharProcs` dictionary with drawing instructions, adding complexity
- The orphaned glyph case (`/NotAGlyph`) can still be tested with Type1 by using a glyph name that doesn't map to any standard character

**Alternative considered:** Type3 font would allow drawing arbitrary glyphs, but this adds significant complexity to fixture generation and testing. Type1 is sufficient for our testing needs.

### Font Embedding: None

**Rationale:**
- Embedded fonts would enable Level 3 (font fingerprinting) recovery
- We want all 4 levels to fail for unmapped glyphs
- Non-embedded fonts are commonly encountered in legacy PDFs
- Simpler fixture structure and smaller file size

### ToUnicode CMap: None

**Rationale:**
- Level 1 recovery bypasses the encoding dictionary entirely
- We want to test Level 2 (AGL lookup) as the primary success path
- ToUnicode presence would complicate the test matrix

### Page Size: US Letter (612 x 792 pts)

**Rationale:**
- Standard PDF default size
- Matches existing no-mapping.pdf fixture pattern
- Sufficient space for multi-line text layout

## PDF Structure Specification

### Overall Structure

The fixture will be a minimal PDF with 6 indirect objects:

```
Object 1 (0): Catalog
Object 2 (0): Pages node  
Object 3 (0): Page dictionary
Object 4 (0): Content stream
Object 5 (0): Font dictionary
Object 6 (0): (Optional) Metadata or additional resources
```

### Character Code to Glyph Name Mapping

The encoding `/Differences` array will map 10 character codes (0-9) to glyph names:

| Character Code | Byte Value | Glyph Name | Category | Expected Output |
|----------------|------------|------------|----------|-----------------|
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

**Note:** The additional mapped glyph `/uni0041` (algorithmic) can be added at code 10 (0x0A) if needed, but the 10 codes above provide sufficient coverage for the primary test scenarios.

### Font Dictionary (Object 5)

```pdf
5 0 obj
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
endobj
```

**Key characteristics:**
- **BaseFont name:** `/UnmappedTestFont` — descriptive, indicates test purpose
- **Subtype:** `/Type1` — standard PostScript font
- **Encoding:** Custom with `/Differences` array
- **No /FontDescriptor:** — font not embedded
- **No /ToUnicode:** — no Level 1 recovery

### Content Stream Layout (Object 4)

The content stream will display text in **3 lines** to test both simple positioning and the full glyph set:

```pdf
4 0 obj
<<
/Length 163
>>
stream
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
endstream
endobj
```

**Content stream breakdown:**
- **BT/ET:** Begin/End text block
- **/F1 12 Tf:** Set font to resource F1 at 12pt size
- **50 700 Td:** Move text position to (50, 700) 
- **<000102> Tj:** Show text string with byte codes [0, 1, 2]
- **<03040506> Tj:** Show text string with byte codes [3, 4, 5, 6]
- **<070809> Tj:** Show text string with byte codes [7, 8, 9]

**Visual layout:**
```
Line 1 (y=700): [g001][g002][g003]           → "���"
Line 2 (y=680): [CustomA][CustomB][NotAGlyph][glyph_0041]  → "����"
Line 3 (y=660): [A][B][space]                → "AB "
```

### Page Dictionary (Object 3)

```pdf
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
/Contents 4 0 R
>>
endobj
```

**Key elements:**
- **MediaBox:** US Letter size (612 x 792 points)
- **Resources:** References font F1 → object 5
- **Contents:** References content stream object 4

### Complete PDF Structure

```pdf
%PDF-1.4

1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
/Contents 4 0 R
>>
endobj

4 0 obj
<<
/Length 163
>>
stream
BT
/F1 12 Tf
50 700 Td
<000102> Tj
50 680 Td
<03040506> Tj
50 660 Td
<070809> Tj
ET
endstream
endobj

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

xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000241 00000 n 
0000000428 00000 n 
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
527
%%EOF
```

**Approximate file size:** ~1.2 KB (slightly larger than no-mapping.pdf due to more glyph names and content stream commands)

## Expected Extraction Output

### Text Content

The fixture should extract to:

```
���
����
AB 
```

Breaking down by line:
- **Line 1:** 3 × U+FFFD (for `/g001`, `/g002`, `/g003`)
- **Line 2:** 4 × U+FFFD (for `/CustomA`, `/CustomB`, `/NotAGlyph`, `/glyph_0041`)
- **Line 3:** "AB " (U+0041, U+0042, U+0020 for `/A`, `/B`, `/space`)

**Total:** 7 × U+FFFD + 3 mapped characters

### Diagnostic Output

The fixture should emit **7 distinct `GLYPH_UNMAPPED` diagnostics** (one per unique unmapped glyph):

```
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=0, glyph_name=/g001, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=1, glyph_name=/g002, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=2, glyph_name=/g003, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=3, glyph_name=/CustomA, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=4, glyph_name=/CustomB, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=5, glyph_name=/NotAGlyph, reason=not_in_agl
[WARN] GLYPH_UNMAPPED: font_id=5, char_code=6, glyph_name=/glyph_0041, reason=not_in_agl
```

**Important:** The diagnostic should be emitted **once per (font_id, char_code)** combination. If the same unmapped glyph appears multiple times in the content stream, it should only emit one diagnostic (not one per text position).

### Success Path Verification

The fixture should also verify that mapped glyphs resolve correctly:

```
[INFO] GLYPH_RESOLVED: font_id=5, char_code=7, glyph_name=/A, unicode=U+0041, source=agl, confidence=0.9
[INFO] GLYPH_RESOLVED: font_id=5, char_code=8, glyph_name=/B, unicode=U+0042, source=agl, confidence=0.9
[INFO] GLYPH_RESOLVED: font_id=5, char_code=9, glyph_name=/space, unicode=U+0020, source=agl, confidence=0.9
```

## Implementation Strategy

### Generation Approach

The fixture will be generated using a Rust binary similar to `generate_encoding_fixtures.rs`. The generator should:

1. **Use the `lopdf` crate** for PDF construction (already in dependencies)
2. **Build the encoding dictionary** with the exact `/Differences` array
3. **Write the content stream** with proper hex encoding for byte sequences
4. **Generate a valid xref table** with correct byte offsets
5. **Add PDF trailer** with proper `/Size` and `/Root` references

### File Location

The generated fixture should be placed at:

```
tests/fixtures/encoding/unmapped-comprehensive.pdf
```

**Rationale:** The name "unmapped-comprehensive" distinguishes it from the existing "no-mapping.pdf" (which tests only 3 PUA glyphs) and indicates broader coverage of unmapped glyph categories.

### Ground Truth File

Create corresponding ground truth:

```
tests/fixtures/encoding/unmapped-comprehensive.txt
```

Content:
```
���
����
AB 
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

## Advanced Scenarios (Optional Extensions)

### Extension 1: Algorithmic AGL Glyph

Add code 10 (0x0A) → `/uni0041` to test the algorithmic AGL pattern:

```pdf
/Differences [0 
  /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041 
  /A /B /space
  /uni0041      % code 10 → algorithmic AGL
]
```

**Expected output:** U+0041 (A) via algorithmic parser

### Extension 2: Repeated Glyphs

Test diagnostic deduplication by repeating `/g001` multiple times in the content stream:

```pdf
<000102000102> Tj  % /g001 and /g002 appear twice
```

**Expected behavior:** Only 2 `GLYPH_UNMAPPED` diagnostics total (one for `/g001`, one for `/g002`), not 4.

### Extension 3: Mixed Case

Add lowercase `/a`, `/b` to verify case sensitivity in AGL lookup.

### Extension 4: Type3 Font Variant

Create a second fixture with Type3 font and `/CharProcs` dictionary to test the orphaned glyph path more explicitly.

## References

### Related Documentation
- **notes/bf-68f9i-glyphs.md** — Selected glyph list and rationale
- **tests/fixtures/no-mapping.pdf** — Simpler 3-glyph fixture pattern
- **tests/fixtures/encoding/agl-only.pdf** — AGL-only success case
- **crates/pdftract-core/src/font/agl.rs** — AGL lookup implementation
- **crates/pdftract-core/src/font/resolver.rs** — 4-level fallback chain

### Plan Sections
- **Phase 2.3:** Encoding recovery and Unicode mapping (lines 1420-1650)
- **INV-14:** Unicode recovery levels and fallback strategies
- **TH-03:** Text extraction and encoding recovery tests

### Internal Links
- [[bf-41gtd]] — Glyph selection task (child bead)
- [[bf-68f9i]] — Parent coordinator bead for fixture work

## Acceptance Criteria

- ✅ Created `notes/bf-68f9i-design.md` documenting PDF structure
- ✅ Specified exact `/Encoding` dictionary structure with `/Differences` array
- ✅ Specified content stream commands (Tj, Td positioning, 3-line layout)
- ✅ Listed character code to glyph name mappings (10 codes)
- ✅ Included both unmapped (7) and mapped (3) glyphs
- ✅ Specified Type1 font (not Type3) with no embedding
- ✅ Specified expected extraction output (7 × U+FFFD + "AB ")
- ✅ Specified diagnostic emission behavior (7 unique GLYPH_UNMAPPED)
- ✅ Provided complete PDF structure with all 6 objects
- ✅ Specified file location (`tests/fixtures/encoding/unmapped-comprehensive.pdf`)

---

## Summary

This design specifies a comprehensive unmapped glyph fixture that:

1. **Tests all 4 mapping failure modes** — PUA, custom encoding, orphaned codes, non-AGL algorithmic
2. **Verifies success path** — Includes standard AGL glyphs for comparison
3. **Tests realistic PDF structure** — Type1 font, custom encoding, no ToUnicode
4. **Exercises content stream commands** — Multi-line layout with Td positioning
5. **Validates diagnostic behavior** — Unique diagnostics per (font_id, char_code)

The fixture builds on the existing no-mapping.pdf pattern while expanding coverage from 3 to 10 character codes across 7 unmapped glyph categories and 3 mapped glyphs for comprehensive testing of the Unicode recovery pipeline.
