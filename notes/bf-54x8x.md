# bf-54x8x: Document no-mapping.pdf fixture structure and regeneration

## Task Completed

Documented the `no-mapping.pdf` fixture with comprehensive technical documentation at `tests/fixtures/no-mapping.md`.

## Work Performed

### 1. Fixture Structure Analysis

Examined the fixture using standard PDF tools:
- `pdfinfo`: Confirmed 660 bytes, PDF 1.4, single page, Letter size
- `pdffonts`: Identified custom Type1 font `/CustomNoMap` with custom encoding
- `sha256sum`: Verified checksum `b24f88d3add958bfec1d6b134f2cd030cd41bb1932bedbe99405599bd01fa8f0`

### 2. Raw PDF Structure Documentation

Documented the complete PDF structure including:
- 5 indirect objects (Catalog, Pages, Page, Content stream, Font dictionary)
- Custom encoding with differences array: `[0 /g001 /g002 /g003]`
- Content stream: `<g001><g002><g003> Tj` (glyph references)
- No ToUnicode CMap, no embedded font program

### 3. Generation Source Identification

Located and examined the generation script:
- `tests/fixtures/generate_encoding_fixtures.rs` (function `generate_no_mapping_pdf`)
- Script generates 4 encoding fixtures: no-mapping.pdf, agl-only.pdf, fingerprint-match.pdf, shape-match.pdf
- Buildable via: `cargo run --bin generate_encoding_fixtures`

### 4. Regeneration Instructions

Provided three approaches for regeneration:
1. **Standard**: `cargo run --bin generate_encoding_fixtures`
2. **Manual reconstruction**: Complete bash heredoc with raw PDF bytes
3. **Verification steps**: pdfinfo, pdffonts, sha256sum checks

### 5. Inspection Commands

Documented working commands for fixture inspection:
- `pdfinfo` - Basic PDF metadata
- `pdffonts` - Font encoding details
- `pdftk` - Content stream extraction
- `grep` - Glyph name extraction (verified: g001, g002, g003)

### 6. Expected Behavior Documentation

Explained why the fixture emits U+FFFD:
- Level 1 (ToUnicode): ❌ No CMap present
- Level 2 (AGL): ❌ Glyph names /g001, /g002, /g003 not in AGL
- Level 3 (Fingerprinting): ❌ Font not embedded
- Level 4 (Glyph shapes): ❌ No embedded outlines
- **Result**: Three U+FFFD replacement characters

### 7. Historical Context

Noted the fixture history from PROVENANCE.md:
- Generated: 2026-06-09
- Regenerated: 2026-07-02 (bf-512z1)
- Regenerated: 2026-07-03 (bf-f0xqd: corrected ground truth from "ABC" to U+FFFD)
- Regenerated: 2026-07-03 (bf-1m30m: regenerated via generate_encoding_fixtures.rs)

### 8. Troubleshooting Guide

Added troubleshooting section covering:
- Extraction producing wrong characters
- SHA256 checksum mismatches
- Ground truth file mismatches
- Historical notes about the "ABC" vs U+FFFD correction

## Acceptance Criteria

All criteria met:
- ✅ `tests/fixtures/no-mapping.md` includes fixture structure section
- ✅ Regeneration instructions are complete and reproducible
- ✅ Includes example commands for pdfinfo/pdffonts inspection

## Files Modified

- `tests/fixtures/no-mapping.md` - Created comprehensive documentation (235 lines)

## Verification

Tested all documentation commands:
```bash
$ sha256sum tests/fixtures/encoding/no-mapping.pdf
b24f88d3add958bfec1d6b134f2cd030cd41bb1932bedbe99405599bd01fa8f0  tests/fixtures/encoding/no-mapping.pdf

$ grep -o "g[0-9]\{3\}" tests/fixtures/encoding/no-mapping.pdf | sort -u
g001
g002
g003
```

Both commands match documentation expectations.

## References

- Plan: Phase 2.3 Encoding recovery and Unicode mapping (lines 1420-1650)
- TH-03: Text extraction and encoding recovery tests
- INV-14: Unicode recovery levels and fallback strategies
- PROVENANCE.md: Fixture generation history
