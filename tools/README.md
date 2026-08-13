# pdftract Tools & Generators

This directory contains utility scripts and generators for creating PDF test fixtures, debugging, and development workflow automation.

**Parent bead:** bf-6uh9a (Tools organization and documentation)

## PDF Fixture Generators

### Python Generators

#### `generate_encoding_fixtures.py`
**Purpose:** Generate Unicode recovery test fixtures for Phase 2.2–2.5

**Fixtures Generated:**
- `no-mapping.pdf` - Font with no ToUnicode and no standard encoding (worst case)
- `agl-only.pdf` - Font with only AGL glyph names (Level 2 recovery)
- `fingerprint-match.pdf` - Font embedded for fingerprint matching (Level 3)
- `shape-match.pdf` - Font for shape-based recognition (Level 4)

**Usage:**
```bash
python tools/generate_encoding_fixtures.py
```

**Requirements:** Python 3, standard library only

---

#### `generate_encrypted_pdf_fixtures.py`
**Purpose:** Generate encrypted PDF test fixtures for password handling

**Fixtures Generated:**
- `EC-04.pdf` - RC4-40 encrypted PDF (V=1, R=2)
- `EC-05.pdf` - AES-128 encrypted PDF (V=4, R=4)
- `EC-06.pdf` - AES-256 encrypted PDF (V=5, R=6)
- `EC-empty-password.pdf` - PDF with empty password

**Usage:**
```bash
python tools/generate_encrypted_pdf_fixtures.py
```

**Requirements:** Python 3, `pikepdf`

---

#### `generate_stress_pdf.py`
**Purpose:** Generate synthetic stress-test PDFs for memory ceiling testing

**Fixtures Generated:**
- Large-page-count PDFs for memory target validation
- 100-page vector PDF for buffered mode testing (target: < 512 MB)
- 10,000-page stress test for streaming mode validation (target: < 256 MB)

**Usage:**
```bash
python tools/generate_stress_pdf.py --pages 100 -o tests/fixtures/perf/100-page-vector.pdf
python tools/generate_stress_pdf.py --pages 10000 -o tests/fixtures/perf/10k-page.pdf
```

**Requirements:** Python 3, `reportlab`

---

#### `generate_invoice_pdf_fixtures.py`
**Purpose:** Generate invoice OCR test fixtures with proper DPI metadata

**Fixtures Generated:**
- Scanned PDF with correct 300 DPI settings
- Single invoice page with ground truth text for OCR testing

**Usage:**
```bash
python tools/generate_invoice_pdf_fixtures.py
```

**Requirements:** Python 3, `PIL` (Pillow)

---

#### `generate_markdown_structure_fixture.py`
**Purpose:** Generate markdown_structure.pdf fixture for testing extract_text() vs extract_markdown()

**Features:**
- Headings with # markers (# Main Title, ## Subtitle)
- Links with [text](url) syntax
- Lists (bullet points and numbered)
- Code blocks and inline code elements

**Usage:**
```bash
python tools/generate_markdown_structure_fixture.py
```

**Requirements:** Python 3, `reportlab`

---

#### `generate_unmapped_glyphs.py`
**Purpose:** Generate unmapped glyph PDF fixtures with custom glyph names and encodings

**Features:**
- Support for custom glyph names and encodings
- Default test set of 10 character codes
- Tests 4-level Unicode fallback chain failure path

**Usage:**
```bash
# Generate with default test glyphs
python3 generate_unmapped_glyphs.py

# Generate with custom glyphs
python3 generate_unmapped_glyphs.py --glyphs '{"0": "/CustomGlyph1", "1": "/CustomGlyph2"}'

# Generate to specific output file
python3 generate_unmapped_glyphs.py --output my-test.pdf --ground-truth my-test.txt
```

**Default glyph set (10 character codes):**
- Codes 0-2: /g001, /g002, /g003 (PUA unmapped)
- Codes 3-6: /CustomA, /CustomB, /NotAGlyph, /glyph_0041 (unmapped)
- Codes 7-9: /A, /B, /space (AGL mapped)

**Requirements:** Python 3, standard library only

**Related documentation:**
- Fixture design: notes/bf-68f9i-design.md
- Glyph selection: notes/bf-68f9i-glyphs.md

---

#### `generate_scanned_fixtures.py`
**Purpose:** Generate scanned PDF fixtures from ground truth text files

**Features:**
- Creates proper 300 DPI PDFs from ground truth text files
- Supports receipt, invoice, and document fixtures
- Configurable fonts, margins, and line spacing

**Usage:**
```bash
python3 generate_scanned_fixtures.py
```

**Requirements:**
- Python 3
- `reportlab` - for PDF generation
- `PIL` (Pillow) - for image processing
- `img2pdf` - for PDF creation from images

---

#### `generate_decompression_bomb.py`
**Purpose:** Generate TH-01 test fixture for decompression bomb protection

**Parent bead:** bf-6uh9a (relocated in bf-5bfr32, documented in bf-1l2z3q)

**Features:**
- Creates PDF with ~10 KB compressed stream that expands to ~10 MB (1000:1 ratio)
- Tests max_decompress_bytes enforcement (512 MB default)
- Safe alternative to 2GB bomb for CI environments

**Usage:**
```bash
python3 generate_decompression_bomb.py
```

**Output:** `tests/fixtures/malformed/bomb-10k-2g.pdf`

**Requirements:** Python 3, standard library only

**Security context:** TH-01 test fixture for Phase 1.5 stream decoder protection

---

#### `generate_embedded_js.py`
**Purpose:** Generate embedded-js.pdf fixture for TH-04 JavaScript detection testing

**Parent bead:** bf-6uh9a (relocated in bf-5bfr32, documented in bf-1l2z3q)

**Features:**
- Creates PDF with 3 JavaScript actions at different locations:
  1. Catalog /OpenAction → /JS containing app.alert("pwn")
  2. Page 0 /AA → /O (open action) → /JS containing second alert
  3. Page 1 annotation /A → /JS containing third snippet

**Usage:**
```bash
python3 generate_embedded_js.py
```

**Output:** `tests/fixtures/security/embedded-js.pdf`

**Requirements:** Python 3, standard library only

**Security context:** TH-04 test fixture for JavaScript presence detection (never executed)

---

#### `generate_vector_cer_corpus.py`
**Purpose:** Generate clean vector PDF fixtures for CER (Character Error Rate) testing

**Parent bead:** bf-6uh9a (relocated in bf-5bfr32, documented in bf-1l2z3q)

**Features:**
- Creates 5-10 clean LaTeX/Word-style PDFs with paired .txt ground-truth files
- Uses proper PDF structure with Type1 fonts and WinAnsiEncoding
- For AS-01 scenario and <0.5% CER Tier 1 gate

**Usage:**
```bash
python3 generate_vector_cer_corpus.py
```

**Requirements:** Python 3, standard library only

---

#### `create_unmapped_comprehensive.py`
**Purpose:** Generate comprehensive unmapped glyph fixture with advanced test cases

**Features:**
- Tests complex unmapped glyph scenarios
- Supports custom encoding patterns
- Generates ground truth files for validation

**Usage:**
```bash
python3 create_unmapped_comprehensive.py
```

**Requirements:** Python 3, standard library only

---

#### `create_degraded_200dpi.py`
**Purpose:** Generate degraded 200 DPI OCR test fixture

**Features:**
- Creates intentionally degraded PDF for OCR quality testing
- Degradation effects: Gaussian blur, noise, reduced contrast, JPEG compression artifacts
- Source text from Abraham Lincoln for WER measurement

**Usage:**
```bash
python3 create_degraded_200dpi.py
```

**Output:** `tests/fixtures/scanned/low-quality/degraded-200dpi.pdf`

**Requirements:**
- Python 3
- `reportlab` - for PDF generation
- `PIL` (Pillow) - for image processing
- `pdftoppm` (poppler-utils) - for PDF-to-image conversion

**Related documentation:**
- OCR generation process: notes/bf-3tedi-ocr-generation-docs.md
- WER measurement: Use calculate_wer.py with generated ground truth

---

#### `calculate_wer.py`
**Purpose:** Calculate Word Error Rate (WER) and Character Error Rate (CER) for OCR evaluation

**Features:**
- Levenshtein distance-based WER calculation
- Optional CER calculation with --cer flag
- Verbose output with word/character counts
- Exit code based on WER threshold (3%)

**Usage:**
```bash
# Basic WER calculation
python3 calculate_wer.py ground_truth.txt ocr_output.txt

# With CER and verbose output
python3 calculate_wer.py ground_truth.txt ocr_output.txt --cer --verbose

# Example with degraded fixture
python3 tools/calculate_wer.py tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt tests/fixtures/scanned/low-quality/degraded-200dpi-ocr.txt
```

**Requirements:**
- Python 3
- `jiwer` - for advanced WER calculation (optional, script has basic implementation)

**Output:** Prints WER/CER percentages and returns exit code 0 if WER < 3%

---

### Rust Generators

#### `generate_invoice_fixture.rs`
**Purpose:** Generate invoice fixture as a native Rust binary

**Usage:**
```bash
cargo run --bin generate_invoice_fixture
```

**Requirements:** Rust toolchain

---

#### `generate_encrypted_pdf_fixtures.rs`
**Purpose:** Generate encrypted PDF fixtures as a native Rust binary

**Usage:**
```bash
cargo run --bin generate_encrypted_pdf_fixtures
```

**Requirements:** Rust toolchain, `lopdf` crate

---

#### `generate_form_fixtures.rs`
**Purpose:** Generate AcroForm and XFA PDF test fixtures for Phase 7.4

**Fixtures Generated:**
- `acroform-text-fields.pdf`: AcroForm with text, checkbox, radio, and dropdown fields
- `acroform-readonly.pdf`: AcroForm with pre-filled read-only fields
- `acroform-submit.pdf`: AcroForm with a submit button
- `xfa-dynamic.pdf`: XFA dynamic form (placeholder for future XFA support)

Each fixture includes corresponding .json ground truth with expected field values.

**Usage:**
```bash
cargo run --bin generate_form_fixtures
```

**Requirements:** Rust toolchain, `lopdf` crate

---

#### `generate_sensitive_fixture.rs`
**Purpose:** Generate sensitive.pdf for TH-08 log audit test

**Features:**
- Creates password-protected PDF with unique, distinctive markers
- Body text contains "UNIQUE-MARKER-IN-BODY-TEXT-7f9a"
- Password value is "UNIQUE-PASSWORD-FOR-TH08-7f9a"
- Designed for reliable substring-based leak detection in log output

**Usage:**
```bash
cargo run --bin generate_sensitive_fixture
```

**Output:** `tests/fixtures/security/sensitive.pdf`

**Requirements:** Rust toolchain, `lopdf` crate

**Security context:** TH-08 test fixture for log audit validation

---

### Shell Scripts

#### `convert_pdf_to_scanned.sh`
**Purpose:** Convert text-embedded PDFs to scanned image-based PDFs at 300 DPI

**Fixtures Generated:**
- `invoice/invoice-300dpi.pdf` (and backup invoice-300dpi-text-embedded.pdf)
- `letter/letter-300dpi.pdf` (and backup letter-300dpi-text-embedded.pdf)
- `form/form-300dpi.pdf` (and backup form-300dpi-text-embedded.pdf)

**Usage:**
```bash
./tools/convert_pdf_to_scanned.sh
```

**Requirements:**
- `pdftoppm` (poppler-utils)
- ImageMagick (via nix-shell or direct installation)

**Process:**
1. Backs up original text-embedded versions with `-text-embedded.pdf` suffix
2. Converts PDF to PPM images at specified DPI using `pdftoppm`
3. Converts PPM images back to PDF using ImageMagick
4. Cleans up temporary files

---

#### `count_docs.sh`
**Purpose:** Wrapper script for documentation counting

**Usage:**
```bash
./tools/count_docs.sh
```

---

#### `extract-release-notes.sh`
**Purpose:** Extract release notes from git commits

**Usage:**
```bash
./tools/extract-release-notes.sh
```

---

## Debugging Tools

### `debug-fingerprint/`
**Purpose:** Debug tool for PDF fingerprint computation (Phase 1.7)

**Features:**
- Computes PDF structural fingerprint
- Displays fingerprint and computation time
- Useful for fingerprint validation and debugging

**Usage:**
```bash
cd tools/debug-fingerprint
cargo run -- -- <pdf-path>
```

**Output:** Displays PDF fingerprint and computation time

**Related:** Phase 1.7 PDF Structural Fingerprint

---

### `debug-fingerprint-diff/`
**Purpose:** Compare fingerprints between two PDFs

**Features:**
- Computes fingerprints for two PDFs
- Displays difference analysis
- Useful for fingerprint stability validation

**Usage:**
```bash
cd tools/debug-fingerprint-diff
cargo run -- -- <pdf1> <pdf2>
```

**Output:** Displays whether fingerprints match and detailed comparison

---

## Specialized Fixture Builders

### `build-objstm-fixture/`
**Purpose:** Generate object stream fixtures for testing

**Features:**
- Creates minimal PDFs with specific object stream structures
- Tests various compressed object configurations
- Validates object stream parsing and resolution

**Usage:**
```bash
cd tools/build-objstm-fixture
cargo run --bin build-objstm-fixture
```

**Fixtures Generated:**
- Minimal PDFs with specific object stream structures
- Various compressed object configurations

---

### `build-xref-fixture/`
**Purpose:** Generate xref testing fixtures for Phase 1.3

**Features:**
- Comprehensive xref structure testing
- Linearized PDF support
- Incremental update (/Prev chain) testing
- Corrupted xref recovery validation

**Usage:**
```bash
cd tools/build-xref-fixture
cargo run --bin build-xref-fixture
```

**Fixture Types:**
- Well-formed PDF with traditional xref table
- Well-formed PDF with xref stream (PDF 1.5)
- Hybrid file with traditional xref + `/XRefStm`
- PDF with 3 incremental revisions (`/Prev` chain)
- Linearized PDF (50 pages)
- File truncated at the start of xref
- File with `startxref` offset off by one
- File with corrupt xref entry
- File with circular `/Prev` reference

---

## Analysis Tools

#### `count_docs.py`
**Purpose:** Count rustdoc coverage for pdftract-core

**Features:**
- Analyzes public API surface
- Counts re-exports and public modules
- Identifies key public types for documentation

**Usage:**
```bash
python tools/count_docs.py
```

**Output:** Lists public modules, re-exports, and key types

**Requirements:** Python 3, standard library only

---

#### `count_public_api.py`
**Purpose:** Count public API coverage focusing on re-exports in lib.rs

**Features:**
- Analyzes pdftract-core/src/lib.rs
- Lists public modules and re-exports
- Shows key public types to document

**Usage:**
```bash
python tools/count_public_api.py
```

**Output:** 
- Public modules count and list
- Re-exports detail
- Key public types sorted alphabetically

**Requirements:** Python 3, standard library only

---

## Test Utilities

#### `test_rust_sdk.rs`
**Purpose:** Test the Rust SDK extract_markdown function to show correct behavior

**Features:**
- Demonstrates extract_markdown vs extract_text output
- Shows correct SDK usage patterns
- Validates markdown output structure

**Usage:**
```bash
cargo run --bin test_rust_sdk -- <pdf-path>
```

**Default PDF:** `tests/fixtures/remote_100page.pdf` if no argument provided

**Requirements:** Rust toolchain, pdftract-core SDK

---

#### `test_rust_markdown`
**Purpose:** Compiled binary for markdown extraction testing

**Usage:**
```bash
./tools/test_rust_markdown <pdf-path>
```

**Note:** This is a compiled Rust binary (built via cargo)

---

## Development Workflow

### Quick Documentation Coverage Check
```bash
python tools/count_docs.py
python tools/count_public_api.py
```

### Generate All Encoding Fixtures
```bash
python tools/generate_encoding_fixtures.py
```

### Generate All Encrypted PDF Fixtures
```bash
python tools/generate_encrypted_pdf_fixtures.py
```

### Generate Stress Test PDFs
```bash
python tools/generate_stress_pdf.py --pages 100 -o tests/fixtures/perf/100-page.pdf
python tools/generate_stress_pdf.py --pages 10000 -o tests/fixtures/perf/10k-page.pdf
```

### Generate Scanned Fixtures
```bash
python tools/generate_scanned_fixtures.py
```

### Debug Fingerprint Issues
```bash
cd tools/debug-fingerprint
cargo run -- -- tests/fixtures/your-file.pdf
```

### Calculate OCR Accuracy
```bash
python3 tools/calculate_wer.py ground_truth.txt ocr_output.txt
```

## Contributing

When adding new generators:
1. Use descriptive names with `generate_` prefix for generators
2. Add comprehensive docstring/comment block explaining:
   - What fixtures it generates
   - How to run it (command line, dependencies)
   - Any special requirements (input data, API keys, etc.)
   - Related test scenarios or threat model entries
3. Add `--help` support for command-line tools
4. Update this README with the new tool
5. Follow existing patterns (Python for encoding/OCR, Rust for security/crypto)
6. Cite parent bead if applicable

## Organization History

**Cleanup beads (2026-07-05):**
- **bf-1iefu**: Categorized 17 generator scripts (KEEP: 10, DELETE: 5, RELOCATE: 2)
- **bf-xqib3**: Removed 5 obsolete generators and compiled artifacts
- **bf-2yhak**: Relocated 2 general-purpose tools to tools/ with documentation
- **bf-620xp**: Verified and documented cleaned structure

**Current documentation update (bf-1l2z3q):**
- Added comprehensive documentation for all generators
- Standardized usage examples and requirements
- Added --help support recommendations
- Organized by category (Python/Rust/Shell/Debug/Test/Analysis)
- Cited parent bead bf-6uh9a

## Related Documentation

- [Test Fixtures Structure](../tests/fixtures/STRUCTURE.md) - Complete fixtures directory organization
- [Fixture Provenance Log](../tests/fixtures/PROVENANCE.md) - Generation history for each fixture
- [Generator Categorization (bf-1iefu)](../notes/bf-1iefu.md) - Rationale for KEEP/DELETE/RELOCATE decisions
- [Obsolete Generator Removal (bf-xqib3)](../notes/bf-xqib3.md) - Cleanup verification
- [Parent bead documentation (bf-6uh9a)](../notes/bf-6uh9a.md) - Tools organization and documentation