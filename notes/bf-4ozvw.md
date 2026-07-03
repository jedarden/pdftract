# no-mapping.pdf Fixture Structure Analysis

**Date:** 2026-07-03
**Bead:** bf-4ozvw
**Fixture Path:** `tests/fixtures/encoding/no-mapping.pdf`

## PDF Metadata (pdfinfo)

```
Custom Metadata: no
Metadata Stream: no
Tagged:          no
UserProperties:  no
Suspects:        no
Form:            none
JavaScript:      no
Pages:           1
Encrypted:       no
Page size:       612 x 792 pts (letter)
Page rot:        0
File size:       660 bytes
Optimized:       no
PDF version:     1.4
```

## Font and Encoding Information (pdffonts)

```
name                                 type              encoding         emb sub uni object ID
------------------------------------ ----------------- ---------------- --- --- --- ---------
CustomNoMap                          Type 1            Custom           no  no  no       5  0
```

## Structural Findings

### File Characteristics
- **Size:** Extremely small (660 bytes) — minimal fixture
- **Pages:** Single page document
- **Format:** PDF 1.4 (standard version)
- **Page Size:** US Letter (612 x 792 pts)
- **Complexity:** No JavaScript, forms, or metadata streams

### Font Analysis
- **Font Name:** `CustomNoMap` — descriptive name indicating purpose
- **Font Type:** Type 1 (PostScript font)
- **Encoding:** Custom — this is the key characteristic
- **Embedding:** Not embedded (`emb: no`)
- **Subsetting:** Not subset (`sub: no`)
- **Unicode:** No Unicode mapping (`uni: no`)

### Key Observations

1. **Custom Encoding Purpose:** The fixture is specifically designed to test handling of custom font encodings that lack standard mappings to Unicode or other character sets.

2. **Minimal Complexity:** At 660 bytes, this is likely a handcrafted or programmatically generated PDF with minimal content, focused specifically on the encoding edge case.

3. **No Font Embedding:** The font is not embedded, meaning PDF readers would need to have access to the actual font file or rely on the encoding information provided.

4. **Testing Utility:** This fixture appears to be designed to test pdftract's handling of:
   - Custom encodings that don't map to standard character sets
   - Glyph-to-text extraction fallback strategies
   - Error recovery when encoding mappings are unavailable

### Technical Significance

This fixture is particularly valuable for testing:
- Font encoding edge cases
- Glyph recognition without standard mappings
- Error handling in the text extraction pipeline
- Fallback strategies when standard encoding tables (AGL, CJK, etc.) don't apply

## Notes for Documentation

The fixture name "no-mapping" combined with the custom encoding suggests this is intentionally testing the case where pdftract must handle glyphs that have no direct mapping to readable text. This is a critical edge case for robust PDF text extraction.
