# bf-1m30m: Generate no-mapping.pdf Fixture

**Date:** 2026-07-03
**Parent Bead:** bf-f0xqd
**Prerequisite:** bf-2g412 (Research and design approach)

## Implementation

Generated the `no-mapping.pdf` fixture using the existing Rust script:
`tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs`

## Verification Results

### Acceptance Criteria - All PASS

1. **✓ Valid PDF exists**: `tests/fixtures/encoding/no-mapping.pdf` (660 bytes)
   - Valid PDF 1.4 structure
   - Single page with 612x792 pts (letter size)

2. **✓ Font has no ToUnicode CMap**: Verified with `grep -a "ToUnicode"`
   - No `/ToUnicode` entry in font dictionary

3. **✓ Font has no standard encoding**: Verified with `pdffonts` and `grep`
   - `pdffonts` shows: `Encoding: Custom`
   - No `/WinAnsiEncoding`, `/MacRomanEncoding`, or `/StandardEncoding` present

4. **✓ PDF contains test text content**: Verified in content stream
   - Content stream: `<g001><g002><g003> Tj`
   - Codes: `[0, 1, 2]` mapped to non-AGL glyph names `/g001`, `/g002`, `/g003`
   - Ground truth file: Three U+FFFD replacement characters (0xEF 0xBF 0xBD × 3)

### Font Dictionary Structure

```
/Type /Font
/Subtype /Type1
/BaseFont /CustomNoMap
/Encoding <<
    /Type /Encoding
    /Differences [0 /g001 /g002 /g003]
>>
```

**Key characteristics:**
- Type1 font (simplest for custom encodings)
- Custom base font name (prevents standard 14 font matching)
- Custom encoding with `/Differences` array
- Non-AGL glyph names (`/g001-3` not in Adobe Glyph List)
- No embedded font program (prevents Level 3 fingerprinting)
- No ToUnicode CMap (prevents direct Unicode mapping)

## Testing Purpose

This fixture tests the ENCODING_NO_MAPPING failure mode — the worst case for Unicode recovery:
- **Level 1 (Standard Encoding)**: FAIL — No standard encoding
- **Level 2 (AGL Lookup)**: FAIL — Glyph names not in AGL
- **Level 3 (Font Fingerprinting)**: FAIL — No embedded font program
- **Level 4 (Shape Recognition)**: FAIL/Partial — Intentionally no shape DB match
- **Fallback**: Emit U+FFFD × 3 with `GLYPH_UNMAPPED` diagnostic

## Files Modified

- `tests/fixtures/encoding/no-mapping.pdf` - Generated fixture (660 bytes)
- `tests/fixtures/encoding/no-mapping.txt` - Ground truth (3 × U+FFFD)

## Files Unchanged

- `tests/fixtures/encoding/generate_unicode_recovery_fixtures.rs` - Generation script (already existed)
- `tests/fixtures/encoding/agl-only.pdf` - Companion fixture
- `tests/fixtures/encoding/fingerprint-match.pdf` - Companion fixture
- `tests/fixtures/encoding/shape-match.pdf` - Companion fixture

## Reference

- Technical approach documented in `notes/bf-f0xqd-research.md`
- Failure Mode Taxonomy: ENCODING_NO_MAPPING
