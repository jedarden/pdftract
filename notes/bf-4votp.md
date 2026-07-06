# Fixtures Directory Structure Exploration

Bead: bf-4votp
Date: 2026-07-06
Task: Explore fixtures directory structure

## Overview

The `tests/fixtures/` directory contains **1,301 PDF files** organized into **16 top-level categories** with a maximum depth of 2 levels (very flat structure).

## Directory Structure

### Top-level Categories

```
tests/fixtures/
├── cjk/                    # 4 PDFs - Chinese, Japanese, Korean
├── classifier/             # 200 PDFs - Document type classification fixtures
│   ├── contract/          
│   ├── invoice/            # 50 numbered invoice PDFs (01-50.pdf)
│   ├── misc/
│   └── scientific_paper/
├── encoding/               # 6 PDFs - Font encoding edge cases
├── encrypted/              # 5 PDFs - RC4, AES-128, AES-256 encrypted PDFs
├── fonts/                  # 0 PDFs (empty)
├── forms/                  # 0 PDFs (has generate_form_fixtures.rs)
├── grep-corpus/            # 1,000 PDFs - Folder search benchmark corpus
│   └── corpus/
├── json_schema/            # 6 PDFs - Schema validation fixtures
├── malformed/              # 15 PDFs - Truncated, corrupt xref, circular refs
├── ocr/                    # 2 PDFs - OCR test fixtures
│   ├── brokenvector_aligned/
│   ├── brokenvector_misaligned/
│   ├── clean_lorem_ipsum/
│   ├── eng_fra_mixed/
│   └── perf_10_page/
├── page_class/             # 4 PDFs - Page classification test fixtures
│   ├── brokenvector_pdfa/
│   ├── hybrid_header_body/
│   ├── scanned_single/
│   └── vector_pure/
├── perf/                   # 2 PDFs - Performance benchmarking
│   └── 10k-page.pdf (2.4 MB)
├── portfolios/             # 0 PDFs (empty)
├── preprocess/             # 0 PDFs (empty)
│   ├── clean_digital/
│   ├── jbig2_scan/
│   ├── skewed_2deg/
│   ├── uneven_lighting/
├── profiles/               # 20 PDFs + 52 symlinks - Document profile fixtures
│   ├── invoice/            # Symlinks → ../../classifier/invoice/
│   ├── receipt/
│   ├── bank_statement/
│   ├── book_chapter/
│   ├── contract/
│   ├── form/
│   ├── invalid/            # 5 YAML files (malformed profiles)
│   ├── legal_filing/
│   ├── resolution/
│   ├── scientific_paper/
│   ├── slide_deck/
│   └── valid/              # 2 YAML files
├── scanned/                # 16 PDFs - Physical scans at various DPI
│   ├── documents/
│   ├── form/
│   ├── invoice/
│   ├── letter/
│   ├── low-quality/
│   ├── multi-page/         # Large files up to 2.2 MB
│   └── receipt/
├── security/               # 4 PDFs - Security test fixtures
│   └── xss-payload.pdf
├── vector/                 # 10 PDFs - Clean LaTeX/Word/InDesign outputs
│   ├── academic-paper/
│   ├── code-documentation/
│   ├── conference-proceedings/
│   ├── financial-report/
│   ├── legal-contract/
│   ├── medical-research/
│   ├── multi-page-academic/
│   ├── scientific-report/
│   ├── technical-documentation/
│   └── user-manual/
├── sample.pdf              # Root-level test PDF
├── remote_100page.pdf      # 5.9 MB - Remote source testing
├── test-minimal.pdf
├── tagged-suspects-*.pdf   # 3 files - Tagged PDF variants
└── valid-minimal.pdf
```

## Symlinks (Structural Oddity)

**53 symlinks** exist, primarily in the `profiles/` directory:

1. **Profile fixtures reference classifier fixtures:**
   - `tests/fixtures/profiles/invoice/*.pdf` → `../../classifier/invoice/*.pdf`
   - This allows profile validation tests to use the same underlying PDFs as classifier tests

2. **Self-referential directory symlink:**
   - `tests/fixtures/classifier/scientific_paper/scientific_paper` → `/home/coding/pdftract/tests/fixtures/classifier/scientific_paper`
   - Purpose unclear, appears to be a legacy artifact

## Non-PDF Files

### Binary Fixtures (LZW compression tests)
- `tests/fixtures/lzw_*.bin` (13 files) - Test fixtures for LZW stream decoder

### Markdown Files
- `STRUCTURE.md` - Fixtures documentation
- `PROVENANCE.md` - Source/origin tracking
- `no-mapping.md` - Encoding test notes

### YAML Profile Files
- 6 YAML files for document profile validation:
  - `profiles/valid/*.yaml` (2 files)
  - `profiles/invalid/*.yaml` (5 files)
  - `profiles/resolution/custom-invoice.yaml`

### Code Files
- `forms/generate_form_fixtures.rs` - Form fixture generator

### Expected Output Files
- `encrypted/EC-05-aes128-encrypted.expected.json` - Expected JSON output

## Size Distribution

| Category | Size Range | Largest File |
|----------|------------|--------------|
| Root-level | 534 B - 5.9 MB | `remote_100page.pdf` (5.9 MB) |
| scanned/ | 416K - 2.2 MB | `multi-page/report-300dpi.pdf` (2.2 MB) |
| perf/ | 2.4 MB | `10k-page.pdf` |
| Most categories | < 500 KB | N/A |

## Structural Characteristics

### Positive Attributes
1. **Flat hierarchy** - Max depth of 2 levels makes navigation simple
2. **No empty directories** - All categories contain files
3. **Clear naming** - Directory names self-document their purpose
4. **Logical organization** - Grouped by feature/test type

### Potential Issues
1. **Symlink complexity** - 53 symlinks could complicate:
   - File discovery algorithms
   - Test fixture copying/archiving
   - Docker volume mounts (symlink handling varies)
   
   The symlinks from `profiles/` to `classifier/` are intentional for test reuse but may break tools that don't follow symlinks.

2. **Large grep-corpus** - 1,000 PDFs in a single flat directory could:
   - Impact filesystem performance on some systems
   - Cause name collisions if more added
   - Benefit from sharding (e.g., `grep-corpus/batch-1/`, `grep-corpus/batch-2/`)

3. **Mixed content at root level** - 7 PDFs at the root (`sample.pdf`, `test-minimal.pdf`, etc.) lack clear categorization

## Recommendations for Discovery Implementation

1. **Follow symlinks** - Use `followSymlinks: true` or equivalent when walking the directory tree
2. **Handle grepcorpus specially** - Consider batching or streaming for the 1,000-PDF directory
3. **Canonicalize paths** - Resolve symlinks to their targets before deduplication
4. **Root-level special case** - Either categorize root PDFs or document them as "general purpose"

## Summary

- **Total PDFs:** 1,301
- **Total categories:** 16 top-level directories
- **Max depth:** 2 levels
- **Symlinks:** 53 (mostly in `profiles/` → `classifier/`)
- **Largest single file:** `remote_100page.pdf` (5.9 MB)
- **Most populated category:** `grep-corpus/` with 1,000 PDFs

The structure is well-organized and flat, with symlink-based fixture reuse being the primary complexity to handle in discovery implementation.
