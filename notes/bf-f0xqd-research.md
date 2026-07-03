# bf-f0xqd-research: Technical Approach for no-mapping.pdf Fixture

**Bead ID:** bf-2g412
**Parent Bead:** bf-f0xqd
**Failure Mode:** ENCODING_NO_MAPPING
**Date:** 2026-07-03

## Executive Summary

This document outlines the technical approach for creating a PDF fixture (`no-mapping.pdf`) that tests the ENCODING_NO_MAPPING failure mode — a font with no ToUnicode CMap and no standard encoding. This represents the worst-case scenario for Unicode recovery, forcing pdftract to use its Level 4 glyph shape recognition.

---

## 1. PDF Specification Background

### 1.1 Font Encoding Without ToUnicode CMap

According to PDF 1.7 specification (ISO 32000-1:2008), Section 9.6.3, a font dictionary may optionally include a `/ToUnicode` entry. When absent, text extraction must rely on fallback mechanisms:

1. **Standard Encoding Fallback** (Level 1): If `/Encoding` is a predefined name (`/WinAnsiEncoding`, `/MacRomanEncoding`, `/StandardEncoding`), map character codes to Unicode via the encoding table.

2. **Custom Encoding with `/Differences`** (Level 2): If `/Encoding` is a dictionary with `/Differences` array, map each code to a glyph name, then resolve glyph names via:
   - Adobe Glyph List (AGL) — Level 2 primary path
   - Font encoding tables — Level 2 secondary path

3. **Font Fingerprinting** (Level 3): SHA-256 hash of embedded font program matched against known corpus

4. **Glyph Shape Recognition** (Level 4): Visual matching against glyph shape database

### 1.2 The `/Differences` Array Structure

The `/Differences` array is specified in PDF 1.7 Section 9.8.1:

```
/Encoding << 
    /Type /Encoding 
    /Differences [code /name1 /name2 ...]
>>
```

Starting from the numeric `code`, each subsequent `/name` assigns that glyph name to the next sequential code point. Example:

```
/Differences [32 /space /exclam /quotedbl]
```

Assigns:
- Code 32 → `/space`
- Code 33 → `/exclam`  
- Code 34 → `/quotedbl`

---

## 2. ENCODING_NO_MAPPING Failure Mode

### 2.1 Definition

From the Failure Mode Taxonomy:

> **ENCODING_NO_MAPPING (Input → Glyph unmapped at Level 4)**  
> Detection: No ToUnicode, no AGL match, no fingerprint hit, no shape-DB hit  
> Recovery: Emit U+FFFD; `confidence: 0.0`; `unicode_source: "unknown"`; `GLYPH_UNMAPPED` diagnostic  
> Test Fixture: `tests/fixtures/encoding/no-mapping.pdf`

### 2.2 Why This Is the Worst Case

A font with:
- **No ToUnicode CMap** — No direct mapping from character codes to Unicode
- **No Standard Encoding** — `/Encoding` is a custom dictionary, not a predefined name
- **Non-AGL Glyph Names** — Glyph names in `/Differences` are not in the Adobe Glyph List
- **No Embedded Font Program** — Cannot fingerprint via SHA-256

Forces the extractor into Level 4 recovery (glyph shape recognition), which is computationally expensive and error-prone.

---

## 3. Fixture Creation Approach

### 3.1 Minimal PDF Structure (Manual Construction)

The fixture requires only 5 PDF objects:

**Object 1 (Catalog):**
```
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
```

**Object 2 (Pages):**
```
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
```

**Object 3 (Page):**
```
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Resources <<
    /Font <<
        /F1 4 0 R
    >>
>>
/Contents 5 0 R
>>
endobj
```

**Object 4 (Font with Custom Encoding):**
```
4 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /CustomNoMap
/Encoding <<
    /Type /Encoding
    /Differences [0 /g001 /g002 /g003]
>>
>>
endobj
```

**Object 5 (Content Stream):**
```
5 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
50 700 Td
<g001><g002><g003> Tj
ET
endstream
endobj
```

**XRef Table and Trailer:**
```
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000241 00000 n 
0000000380 00000 n 
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
[byte offset of xref]
%%EOF
```

### 3.2 Key Technical Decisions

#### Font Subtype: `/Type1`
- Type1 is the simplest font type that supports custom encodings
- Does NOT require an embedded font program (unlike TrueType/OpenType)
- Standard 14 fonts like `/Helvetica` are Type1, so we use `/CustomNoMap` to avoid ambiguity

#### BaseFont: `/CustomNoMap`
- Custom name ensures pdftract cannot match against standard 14 fonts
- No embedded program (`/FontDescriptor` absent) → prevents Level 3 fingerprinting

#### Encoding: `/Differences` Array
```
/Differences [0 /g001 /g002 /g003]
```
- Maps character codes: `0 → /g001`, `1 → /g002`, `2 → /g003`
- Glyph names `/g001`, `/g002`, `/g003` are **not in AGL** (Adobe Glyph List)
- This defeats Level 2 AGL-based recovery

#### Content Stream: Hex-Encoded Glyph Names
```
<g001><g002><g003> Tj
```
- Uses literal glyph names from the `/Differences` array
- Content stream shows 3 characters: codes 0, 1, 2

#### Ground Truth: Empty or U+FFFD
The `.txt` ground truth file should contain the expected output after all recovery levels:
- **Option 1:** Empty string (if glyph shape recognition also fails)
- **Option 2:** U+FFFD replacement characters (if Level 4 is expected to emit U+FFFD)

For our fixture: `tests/fixtures/encoding/no-mapping.txt` should contain `���` (three replacement characters).

---

## 4. Tooling and Libraries

### 4.1 Manual Construction (Recommended for This Fixture)

**Advantages:**
- Full control over PDF structure
- Minimal file size (~600 bytes)
- No external dependencies
- Byte-exact reproducibility

**Implementation:**
- Use the Rust implementation in `tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs`
- Or the Python version in `tests/fixtures/encoding/generate_encoding_fixtures.py`

### 4.2 ReportLab (Python Alternative)

If using ReportLab's `pdfgen`:

```python
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter

def create_no_mapping_pdf():
    c = canvas.Canvas("no-mapping.pdf", pagesize=letter)
    
    # Register a font with custom encoding
    # ReportLab requires embedding for custom encodings
    c.setFont("CustomFont", 12)
    c.drawString(100, 700, "ABC")
    
    c.save()
```

**Limitations:**
- ReportLab auto-generates ToUnicode CMaps in most versions
- Hard to suppress encoding generation
- Produces larger files (~3-5 KB)

### 4.3 Rust `lopdf` Crate

For programmatic generation:

```rust
use lopdf::::{Document, Object};

fn create_no_mapping_pdf() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::with_version("1.4");
    
    // Create catalog, pages, page objects...
    // Add font with custom encoding...
    // Add content stream...
    
    doc.save("no-mapping.pdf")?;
    Ok(())
}
```

**Limitations:**
- `lopdf` may add default structures that interfere with test isolation
- Less control over exact byte output vs manual construction

### 4.4 Existing Fixture Generation Scripts

The repository already has two working implementations:

1. **Rust:** `tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs`
   - Binary at: `tests/fixtures/encoding/generate_unicode_recovery_fixtures_bin`
   - Run: `cd tests/fixtures/encoding && ./generate_unicode_recovery_fixtures_bin`

2. **Python:** `tests/fixtures/encoding/generate_encoding_fixtures.py`
   - Run: `cd tests/fixtures/encoding && python3 generate_encoding_fixtures.py`

Both produce valid `no-mapping.pdf` fixtures.

---

## 5. Test Text Content Specification

### 5.1 Recommended Content

**Primary fixture (3 characters):**
- **Content:** `<g001><g002><g003>` → codes `[0, 1, 2]`
- **Glyph names:** `/g001`, `/g002`, `/g003` (non-AGL, arbitrary names)
- **Ground truth:** Three U+FFFD characters (`���`)
- **Rationale:** Simple, predictable, tests the core failure mode

**Alternative content variations:**

| Variation | Codes | Glyph Names | Purpose |
|-----------|-------|-------------|---------|
| Minimal single | `[0]` | `[/g001]` | Test single unmapped glyph |
| ASCII-aligned | `[65, 66, 67]` | `[/gA /gB /gC]` | Mimics ASCII but unmapped |
| Mixed AGL/non-AGL | `[65, 0, 66]` | `[/A /g001 /B]` | Test mixed recovery levels |
| Extended range | `[0-15]` | `[/g000- /g015]` | Stress test recovery engine |

### 5.2 Ground Truth File Format

The `.txt` file should contain **exactly** the extracted text after pdftract processes the PDF:

```
# tests/fixtures/encoding/no-mapping.txt
```

For the worst-case ENCODING_NO_MAPPING (no recovery at any level):
```
���
```

Or more explicitly in UTF-8 hex:
```
ef bf bd ef bf bd ef bf bd
```

---

## 6. Verification and Validation

### 6.1 PDF Structure Validation

After creating the fixture, verify it with:

```bash
# Check PDF validity
pdfinfo tests/fixtures/encoding/no-mapping.pdf

# Inspect font dictionary
pdffonts tests/fixtures/encoding/no-mapping.pdf

# Hex dump to verify structure
xxd tests/fixtures/encoding/no-mapping.pdf | head -20
```

Expected output:
- Valid PDF 1.4 structure
- Font: `/CustomNoMap` (Type1)
- Encoding: Custom (not standard)
- No ToUnicode entry

### 6.2 Extraction Test

```bash
# Run pdftract on the fixture
pdftract extract tests/fixtures/encoding/no-mapping.pdf

# Expected output includes:
# - text: "���" (U+FFFD × 3)
# - GLYPH_UNMAPPED diagnostic
# - confidence: 0.0
# - unicode_source: "unknown"
```

---

## 7. Failure Mode Exercise Confirmation

### 7.1 What This Fixture Tests

| Recovery Level | Expected Result |
|----------------|-----------------|
| **Level 1: Standard Encoding** | FAIL — No `/WinAnsiEncoding` or `/MacRomanEncoding` |
| **Level 2: AGL Lookup** | FAIL — Glyph names `/g001-3` not in AGL |
| **Level 3: Font Fingerprinting** | FAIL — No embedded font program |
| **Level 4: Shape Recognition** | FAIL/Partial — No shape DB match (intentional) |
| **Fallback** | Emit U+FFFD × 3 with `GLYPH_UNMAPPED` diagnostic |

### 7.2 Diagnostic Output

When pdftract processes this fixture, the JSON output should include:

```json
{
  "text": "���",
  "spans": [
    {
      "text": "���",
      "font": "CustomNoMap",
      "confidence": 0.0,
      "unicode_source": "unknown",
      "bbox": [50, 700, 86, 712]
    }
  ],
  "errors": [
    {
      "code": "GLYPH_UNMAPPED",
      "severity": "warn",
      "message": "No ToUnicode mapping for 3 glyphs in font CustomNoMap"
    }
  ]
}
```

---

## 8. Relationship to Other Encoding Fixtures

The `no-mapping.pdf` fixture is part of a 4-fixture suite that tests all recovery levels:

| Fixture | Level Tested | Key Characteristic |
|---------|--------------|-------------------|
| `agl-only.pdf` | Level 2 | Standard encoding, AGL glyph names |
| `fingerprint-match.pdf` | Level 3 | Embedded font program for SHA-256 fingerprinting |
| `no-mapping.pdf` | Level 4 | No ToUnicode, custom encoding, non-AGL names |
| `shape-match.pdf` | Level 4 | Type3 font with custom glyph drawing |

This progression ensures each recovery mechanism is tested in isolation.

---

## 9. Implementation Checklist

- [x] Document PDF specification for fonts without ToUnicode CMap
- [x] Determine approach for custom font with no standard encoding
- [x] Specify tooling/library requirements
- [x] Specify test text content for fixture
- [x] Create generation scripts (Rust and Python versions exist)
- [x] Validate fixture structure with `pdfinfo` / `pdffonts`
- [ ] Verify extraction produces expected GLYPH_UNMAPPED diagnostic
- [ ] Confirm ground truth `.txt` matches actual output
- [ ] Document fixture in test suite documentation

---

## 10. References

- PDF 1.7 Specification (ISO 32000-1:2008)
  - Section 9.6.3: Font Dictionaries
  - Section 9.8.1: Character Encoding
  - Section 9.10.3: ToUnicode CMaps
- Adobe Glyph List (AGL) — `/build/agl.json`
- pdftract plan: Failure Mode Taxonomy (ENCODING_NO_MAPPING)
- Existing fixture generation code:
  - `tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs`
  - `tests/fixtures/encoding/generate_encoding_fixtures.py`
- Internal research docs:
  - `docs/research/pdf-fonts-and-encoding.md`
  - `docs/research/cmap-format-and-cid-encoding.md`
  - `docs/research/glyph-recognition-and-unicode-recovery.md`

---

## Appendix A: Complete Fixture Source (Rust)

```rust
/// no-mapping.pdf: Custom encoding, no ToUnicode, no standard encoding
/// Tests Level 4 worst-case: Glyph shape recognition failure
fn generate_no_mapping_pdf() -> Result<(), Box<dyn std::error::Error>> {
    let objects = vec![
        // Catalog
        b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj".to_vec(),
        
        // Pages
        b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj".to_vec(),
        
        // Page
        b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Resources <<\n/Font <<\n/F1 4 0 R\n>>\n>>\n/Contents 5 0 R\n>>\nendobj".to_vec(),
        
        // Font with custom encoding (NO ToUnicode, NO standard encoding)
        b"4 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /CustomNoMap\n/Encoding <<\n/Type /Encoding\n/Differences [0 /g001 /g002 /g003]\n>>\n>>\nendobj".to_vec(),
        
        // Content stream: <g001><g002><g003> Tj (codes 0, 1, 2)
        b"5 0 obj\n<<\n/Length 44\n>>\nstream\nBT\n/F1 12 Tf\n50 700 Td\n<g001><g002><g003> Tj\nET\nendstream\nendobj".to_vec(),
    ];
    
    let pdf = build_pdf(objects);
    
    let mut file = File::create("tests/fixtures/encoding/no-mapping.pdf")?;
    file.write_all(&pdf)?;
    
    let mut txt_file = File::create("tests/fixtures/encoding/no-mapping.txt")?;
    txt_file.write_all("\u{FFFD}\u{FFFD}\u{FFFD}".as_bytes())?;
    
    Ok(())
}
```

---

## Appendix B: Validation Script

```bash
#!/bin/bash
# validate_no_mapping_fixture.sh

FIXTURE="tests/fixtures/encoding/no-mapping.pdf"

echo "=== PDF Structure Validation ==="
pdfinfo "$FIXTURE" || exit 1

echo ""
echo "=== Font Dictionary Check ==="
pdffonts "$FIXTURE" || exit 1

echo ""
echo "=== Extraction Test ==="
pdftract extract "$FIXTURE" > /tmp/extract_output.json

echo ""
echo "=== Diagnostic Check ==="
if grep -q "GLYPH_UNMAPPED" /tmp/extract_output.json; then
    echo "✓ GLYPH_UNMAPPED diagnostic present"
else
    echo "✗ GLYPH_UNMAPPED diagnostic MISSING"
    exit 1
fi

echo ""
echo "=== Ground Truth Comparison ==="
EXTRACTED_TEXT=$(jq -r '.text' /tmp/extract_output.json)
EXPECTED_TEXT=$(cat "tests/fixtures/encoding/no-mapping.txt")

if [ "$EXTRACTED_TEXT" == "$EXPECTED_TEXT" ]; then
    echo "✓ Extracted text matches ground truth"
else
    echo "✗ Text mismatch:"
    echo "  Expected: $EXPECTED_TEXT"
    echo "  Got:      $EXTRACTED_TEXT"
    exit 1
fi

echo ""
echo "✓ All validations passed!"
```
