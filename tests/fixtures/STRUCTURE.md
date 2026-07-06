# tests/fixtures/ Structure

This document describes the structure and organization of test fixtures in the pdftract project.

## Overview

The `tests/fixtures/` directory contains PDF test data files organized by test category. As of 2026-07-05, the directory contains both fixture data files and a small set of actively maintained generator scripts co-located with the fixtures they generate.

## Cleanup History (2026-07-05)

Three beads completed the cleanup and organization:

1. **bf-1iefu**: Categorized all generator scripts into KEEP/DELETE/RELOCATE
2. **bf-xqib3**: Removed obsolete generators and compiled artifacts (5 files deleted)
3. **bf-2yhak**: Relocated general-purpose tools to tools/ with documentation

## Directory Structure

```
tests/fixtures/
├── cjk/                      # CJK encoding fixtures
├── classifier/               # Classifier training data
├── encoding/                 # Unicode recovery test fixtures
│   ├── generate_unmapped_glyphs.py        [KEEP: generates unmapped glyph tests]
│   └── create_unmapped_comprehensive.py   [KEEP: generates comprehensive encoding tests]
├── encrypted/                # (deprecated - moved to security/)
├── encrypted-old/            # Legacy encrypted PDF tests
├── fonts/                    # Font subset fixtures
├── forms/                    # AcroForm and XFA fixtures
│   └── generate_form_fixtures.rs         [KEEP: generates form field fixtures]
├── grep-corpus/              # CER testing corpus
├── json_schema/              # JSON schema validation fixtures
├── lzw_*.bin                 # LZW compression test data (binary fixtures)
├── malformed/                # Malformed PDF fixtures
│   ├── random_bytes.bin                   [fixture: random corruption data]
│   ├── compression-bomb.bin               [fixture: decompression bomb test]
│   └── gen_bomb.py                        [KEEP: generates malformed fixtures]
├── no-mapping.md              # Encoding mapping reference
├── ocr/                      # OCR ground truth data
├── page_class/               # Page classification fixtures
├── perf/                     # Performance test fixtures
├── preprocess/               # Preprocessing test data
├── profiles/                 # Color profile fixtures
├── PROVENANCE.md             # Fixture generation log
├── remote_100page.pdf        # Remote testing fixture
├── sample.pdf                # Minimal valid PDF
├── scanned/                  # Scanned document fixtures
│   ├── generate_scanned_fixtures.py       [KEEP: generates scan simulation fixtures]
│   ├── run_gen.sh                         [KEEP: nix-shell wrapper for generator]
│   ├── calculate_wer.py                   [KEEP: WER/CER calculation utility]
│   └── wer_gate_stub.rs                  [KEEP: WER gate test stub]
├── security/                 # Security and encryption fixtures
│   └── generate_sensitive_fixture.rs      [KEEP: generates security test fixtures]
├── STRUCTURE.md              # This file
├── tagged-*.pdf               # Tagged PDF fixtures
├── test-minimal.pdf           # Minimal test fixture
├── valid-minimal.pdf         # Valid minimal PDF
└── vector/                   # Vector text extraction fixtures
    └── generate_vector_cer_corpus.py      [KEEP: generates CER test corpus]
```

## Generator Scripts Remaining in tests/fixtures/

As of 2026-07-05, **10 generator scripts** remain in tests/fixtures/. These were categorized as **KEEP** in bf-1iefu because they generate fixtures for specific test categories and are actively maintained:

| Script | Category | Purpose |
|--------|----------|---------|
| `encoding/generate_unmapped_glyphs.py` | KEEP | Unmapped glyph test generation |
| `encoding/create_unmapped_comprehensive.py` | KEEP | Comprehensive encoding tests |
| `forms/generate_form_fixtures.rs` | KEEP | AcroForm/XFA fixtures |
| `malformed/gen_bomb.py` | KEEP | Decompression bomb fixtures |
| `scanned/generate_scanned_fixtures.py` | KEEP | OCR scan simulation fixtures |
| `scanned/run_gen.sh` | KEEP | Nix-shell dependency wrapper |
| `scanned/calculate_wer.py` | KEEP | WER/CER calculation utility |
| `scanned/wer_gate_stub.rs` | KEEP | WER gate test stub |
| `security/generate_sensitive_fixture.rs` | KEEP | TH-08 log audit fixtures |
| `vector/generate_vector_cer_corpus.py` | KEEP | CER testing corpus |

These scripts are **actively maintained** (last modified May-July 2026) and are **not redundant** - each generates unique fixtures for specific test scenarios.

## Removed Scripts (bf-xqib3)

The following obsolete generators were removed on 2026-07-05:

- `scanned/generate_scanned_fixtures.rs` (Rust stub, use Python version)
- `security/generate_sensitive_fixture.py` (duplicate, use Rust version)
- `encoding/generate_unmapped_glyphs.rs` (superseded by Python version)
- `malformed/gen-bomb-10k-2g.sh` (superseded by Python version)
- `forms/generate_form_fixtures.d` (orphaned Makefile dependency)

## Relocated Scripts (bf-2yhak)

The following general-purpose tools were relocated to `tools/` on 2026-07-05:

- `scanned/convert_to_scanned.sh` → `tools/convert_pdf_to_scanned.sh` (general PDF conversion utility)
- `grep-corpus/regenerate.sh` → deleted (incomplete stub)

## Binary Test Data

The `.bin` files in the fixtures root are **legitimate test fixtures**, not compiled artifacts:

- `lzw_*.bin` - LZW compression stream test data (14 files)
- `malformed/random_bytes.bin` - Random corruption data
- `malformed/compression-bomb.bin` - Decompression bomb test fixture

## Fixture Categories

### Encoding Tests (`encoding/`)
Unicode recovery test fixtures with custom encodings and glyph mappings.

### Form Tests (`forms/`)
AcroForm and XFA fixtures for form field extraction.

### Malformed PDFs (`malformed/`)
Intentionally malformed PDFs for robustness testing.

### OCR Tests (`scanned/`)
Scan simulation fixtures with ground truth text for OCR validation.

### Security Tests (`security/`)
Encrypted PDFs and security policy validation fixtures.

### Vector Tests (`vector/`)
Clean vector PDFs with embedded text for CER/accuracy testing.

### Performance Tests (`perf/`)
Large-page-count PDFs for memory ceiling validation.

### CJK Tests (`cjk/`)
Chinese, Japanese, Korean text encoding fixtures.

## Maintenance Guidelines

### When Adding New Fixtures

1. Place fixtures in the appropriate category subdirectory
2. If writing a generator, add it to the same subdirectory as the fixtures it generates
3. Update `PROVENANCE.md` with generation details
4. Add documentation comments explaining the generator's purpose

### When Modifying Existing Fixtures

1. Regenerate fixtures using the co-located generator script
2. Update `PROVENANCE.md` with new generation date and SHA256
3. Commit with message referencing the bead ID

### Generator Script Guidelines

- Generators should be co-located with the fixtures they generate
- General-purpose tools belong in `tools/` (not `tests/fixtures/`)
- Add docstring/comments explaining: what it generates, how to run it, dependencies
- Prefer Python for encoding/OCR fixtures, Rust for security/crypto fixtures

## Related Documentation

- [`PROVENANCE.md`](PROVENANCE.md) - Detailed generation log for each fixture
- [`tools/README.md`](../tools/README.md) - General-purpose tools catalog
- [`notes/bf-1iefu.md`](../notes/bf-1iefu.md) - Generator categorization rationale
- [`notes/bf-xqib3.md`](../notes/bf-xqib3.md) - Obsolete generator removal
- [`notes/bf-2yhak.md`](../notes/bf-2yhak.md) - Tool relocation documentation

## Verification Status

Last verified: 2026-07-05 (bf-620xp)

✅ All DELETE-category generators removed
✅ All RELOCATE-category generators moved to tools/
✅ No compiled artifacts (`.o`, `.so`, `.a`, `.dylib`, `.dll`) present
✅ No obsolete scripts remaining
✅ 10 KEEP generators remain (actively maintained, co-located with fixtures)
✅ tools/README.md comprehensive documentation present
