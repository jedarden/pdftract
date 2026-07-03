# no-mapping.pdf Fixture Documentation

## Overview

The `no-mapping.pdf` fixture is a **Level 4 Unicode recovery test case** designed to exercise the worst-case scenario for text extraction: a PDF with custom glyph names that cannot be mapped to Unicode characters through any standard method.

**Location:** `tests/fixtures/encoding/no-mapping.pdf`  
**Generated:** 2026-06-09  
**Last Regenerated:** 2026-07-03  
**SHA256:** `b24f88d3add958bfec1d6b134f2cd030cd41bb1932bedbe99405599bd01fa8f0`

## Fixture Structure

### PDF Properties

```bash
$ pdfinfo tests/fixtures/encoding/no-mapping.pdf
Pages:           1
Page size:       612 x 792 pts (letter)
Page rot:        0
File size:       660 bytes
PDF version:     1.4
Encrypted:       no
```

### Font Information

```bash
$ pdffonts tests/fixtures/encoding/no-mapping.pdf
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
CustomNoMap                          Type 1            Custom           no  no  no       5  0
```

**Key characteristics:**
- **Font Name:** `/CustomNoMap` — non-standard, custom font
- **Subtype:** Type1
- **Encoding:** Custom encoding with differences array
- **Embedded:** No — font program not embedded
- **Subset:** No
- **ToUnicode:** No CMap present

### Content Stream Analysis

The PDF contains a single content stream that displays three glyphs by name:

```pdf
BT
/F1 12 Tf
50 700 Td
<g001><g002><g003> Tj
ET
```

The content references glyphs `/g001`, `/g002`, and `/g003` through a custom encoding dictionary.

### Custom Encoding Dictionary

```pdf
/Encoding <<
/Type /Encoding
/Differences [0 /g001 /g002 /g003]
>>
```

This mapping assigns:
- Position 0 → `/g001`
- Position 1 → `/g002`  
- Position 2 → `/g003`

**Critical:** The glyph names `/g001`, `/g002`, `/g003` are **not in the Adobe Glyph List (AGL)**, so they cannot be mapped to standard Unicode characters.

### Raw PDF Structure

The fixture is a minimal hand-written PDF with 5 indirect objects:

```pdf
%PDF-1.4
1 0 obj           # Catalog
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj           # Pages node
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj           # Page
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

4 0 obj           # Content stream
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

5 0 obj           # Font dictionary
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

## PDF Structure

Based on analysis from bead bf-4ozvw, the fixture has the following structural characteristics:

### File Characteristics

- **Size:** Extremely small (660 bytes) — minimal fixture
- **Pages:** Single page document  
- **Format:** PDF 1.4 (standard version)
- **Page Size:** US Letter (612 x 792 pts)
- **Complexity:** No JavaScript, forms, or metadata streams
- **Optimization:** Not optimized
- **User Properties:** None
- **Tagged PDF:** No
- **Metadata Stream:** No

### Font Analysis

- **Font Name:** `CustomNoMap` — descriptive name indicating purpose
- **Font Type:** Type 1 (PostScript font)
- **Encoding:** Custom — this is the key characteristic
- **Embedding:** Not embedded (`emb: no`)
- **Subsetting:** Not subset (`sub: no`)
- **Unicode:** No Unicode mapping (`uni: no`)
- **Object ID:** 5 0 R

### Technical Significance

This fixture is particularly valuable for testing:

1. **Font encoding edge cases** — Custom encodings that don't map to standard character sets
2. **Glyph recognition without standard mappings** — Glyphs with no direct Unicode equivalent
3. **Error handling in the text extraction pipeline** — Fallback strategies when encoding mappings are unavailable
4. **No Font Embedding scenario** — PDF readers must rely on encoding information provided

The fixture name "no-mapping" combined with the custom encoding indicates this is intentionally testing the case where pdftract must handle glyphs that have no direct mapping to readable text — a critical edge case for robust PDF text extraction.

## Expected Extraction Behavior

### Ground Truth

The `tests/fixtures/encoding/no-mapping.txt` ground truth file contains:

```
���
```

This is **three U+FFFD (REPLACEMENT CHARACTER)** codepoints, representing the three unmapped glyphs.

### Why U+FFFD?

The glyph names `/g001`, `/g002`, `/g003` cannot be recovered through any of the standard Unicode recovery levels:

1. **Level 1 (ToUnicode CMap):** ❌ No ToUnicode CMap present
2. **Level 2 (Adobe Glyph List):** ❌ Glyph names not in AGL
3. **Level 3 (Font Fingerprinting):** ❌ Font not embedded, no fingerprint available
4. **Level 4 (Glyph Shape Recognition):** ❌ No embedded glyph outlines

Since all recovery methods fail, pdftract should emit U+FFFD replacement characters to indicate unmappable content.

### Extraction Verification

```bash
$ cargo run --release -- pdftract mcp tests/fixtures/encoding/no-mapping.pdf
# Should output: ��� (three U+FFFD characters)
```

Or with the CLI:

```bash
$ cargo run --release --bin pdftract-cli -- extract \
    tests/fixtures/encoding/no-mapping.pdf
# Expected: Three replacement characters in output
```

## Regeneration Instructions

### Prerequisites

- Rust toolchain (1.70+)
- Working cargo build environment

### Regeneration Command

The fixture is generated by the Rust binary at `tests/fixtures/generate_encoding_fixtures.rs`.

To regenerate **only** the no-mapping.pdf fixture:

```bash
# From the pdftract repository root
cargo run --bin generate_encoding_fixtures
```

This will regenerate **all four encoding fixtures** in `tests/fixtures/encoding/`:
- `no-mapping.pdf` — Custom glyph names (this fixture)
- `agl-only.pdf` — AGL glyph names
- `fingerprint-match.pdf` — Embedded Type1 subset
- `shape-match.pdf` — Custom glyph with shape database

### Verification After Regeneration

After regeneration, verify the fixture with:

```bash
# 1. Check PDF structure
pdfinfo tests/fixtures/encoding/no-mapping.pdf
pdffonts tests/fixtures/encoding/no-mapping.pdf

# 2. Verify SHA256 checksum
sha256sum tests/fixtures/encoding/no-mapping.pdf
# Expected: b24f88d3add958bfec1d6b134f2cd030cd41bb1932bedbe99405599bd01fa8f0

# 3. Verify ground truth extraction
cargo nextest run encoding_recovery

# 4. Update PROVENANCE.md if SHA256 changed
# Add new entry at tests/fixtures/PROVENANCE.md line 208
```

### Manual Regeneration (Alternative)

If the binary fails to build, the fixture can be manually reconstructed from the raw PDF structure shown above. Create a file `no-mapping.pdf` with the exact content:

```bash
cat > tests/fixtures/encoding/no-mapping.pdf << 'EOF'
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
5 0 obj
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
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000241 00000 n 
0000000338 00000 n 
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
477
%%EOF
EOF
```

## Inspection Commands

### PDF Structure Inspection

```bash
# Basic PDF info
pdfinfo tests/fixtures/encoding/no-mapping.pdf

# Font details
pdffonts tests/fixtures/encoding/no-mapping.pdf

# View raw PDF structure (human-readable)
pdftk tests/fixtures/encoding/no-mapping.pdf dump_data output

# Extract and display content stream
pdftk tests/fixtures/encoding/no-mapping.pdf uncompress output /tmp/no-mapping-uncompressed.pdf
cat /tmp/no-mapping-uncompressed.pdf
```

### Cross-Reference Table Verification

```bash
# Check xref table integrity
pdftk tests/fixtures/encoding/no-mapping.pdf dump_data verbose | grep -A 20 "xref"
```

### Glyph Name Extraction

```bash
# Extract glyph names from encoding dictionary
grep -o "g[0-9]\{3\}" tests/fixtures/encoding/no-mapping.pdf
# Expected output: g001, g002, g003
```

### Content Stream Inspection

```bash
# Show content stream (formatted)
strings tests/fixtures/encoding/no-mapping.pdf | grep -A 5 "BT"
```

## Test Coverage

This fixture exercises the following test scenarios:

1. **Custom Encoding Handling:** Non-standard encoding with differences array
2. **Missing ToUnicode:** No CMap fallback for glyph-to-Unicode mapping
3. **Non-AGL Glyph Names:** Glyph names not in Adobe Glyph List
4. **U+FFFD Emission:** Proper replacement character generation
5. **Error Resilience:** Graceful handling of completely unmappable content

### Related Tests

- `tests/encoding_recovery.rs` — Unicode recovery level tests
- `tests/cjk_encoding.rs` — CJK encoding with proper ToUnicode
- `tests/encoding_recovery_integration.rs` — Full extraction pipeline

## Troubleshooting

### Issue: Extraction produces wrong characters

**Symptom:** pdftract outputs "ABC" or other incorrect text instead of U+FFFD.

**Diagnosis:**
```bash
# Verify the fixture structure matches expected
pdffonts tests/fixtures/encoding/no-mapping.pdf | grep CustomNoMap
# Should show: CustomNoMap | Type 1 | Custom | no | no | no
```

**Resolution:** 
- Ensure fixture was regenerated with correct version of `generate_encoding_fixtures.rs`
- Check that `/g001`, `/g002`, `/g003` are not in AGL (`grep -r g001 crates/pdftract-core/src/font/agl.rs`)

### Issue: SHA256 checksum mismatch

**Symptom:** Regenerated fixture has different hash.

**Diagnosis:**
```bash
# Compare file sizes
ls -l tests/fixtures/encoding/no-mapping.pdf
# Expected: 660 bytes
```

**Resolution:**
- Minimal differences (whitespace in PDF) are acceptable
- Update SHA256 in PROVENANCE.md if content is semantically identical
- If structure changed, investigate `generate_encoding_fixtures.rs` for unintended modifications

### Issue: Ground test file mismatch

**Symptom:** Test expects "ABC" but gets "���" (or vice versa).

**History:**
- 2026-07-03 (bf-f0xqd): Ground truth corrected from "ABC" to U+FFFD
- The fixture was originally designed for glyph shape recovery (Level 4), but that feature is not yet implemented
- Current behavior correctly emits U+FFFD

**Resolution:**
- Current expected output is three U+FFFD characters
- Tests should verify replacement character emission, not glyph shape recovery

## References

### Plan Documentation

- **Phase 2.3:** Encoding recovery and Unicode mapping (lines 1420-1650)
- **TH-03:** Text extraction and encoding recovery tests  
- **INV-14:** Unicode recovery levels and fallback strategies

### Related Fixtures

- `agl-only.pdf` — Level 2 recovery (Adobe Glyph List)
- `fingerprint-match.pdf` — Level 3 recovery (font fingerprinting)
- `shape-match.pdf` — Level 4 recovery (glyph shape database)

### Internal Documentation

- `tests/fixtures/PROVENANCE.md` — Fixture generation history
- `tests/fixtures/generate_encoding_fixtures.rs` — Generation source code
- `crates/pdftract-core/src/font/` — Font resolution and encoding implementation
