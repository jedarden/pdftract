# pdftract Tools & Generators

This directory contains utility scripts and generators for creating PDF test fixtures, debugging, and development workflow automation.

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

#### `count_docs.py`
**Purpose:** Count rustdoc coverage for pdftract-core

**Usage:**
```bash
python tools/count_docs.py
```

**Requirements:** Python 3, standard library only

---

### Rust Generators

#### `generate_invoice_fixture.rs`
**Purpose:** Generate invoice fixture as a native Rust binary

**Usage:**
```bash
cargo run --bin generate_invoice_fixture
```

---

#### `generate_encrypted_pdf_fixtures.rs`
**Purpose:** Generate encrypted PDF fixtures as a native Rust binary

**Usage:**
```bash
cargo run --bin generate_encrypted_pdf_fixtures
```

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
**Purpose:** Debug tool for PDF fingerprint computation

**Usage:**
```bash
cargo run --bin debug-fingerprint -- <pdf-path>
```

**Output:** Displays PDF fingerprint and computation time

---

### `debug-fingerprint-diff/`
**Purpose:** Compare fingerprints between two PDFs

**Usage:**
```bash
cargo run --bin debug-fingerprint-diff -- <pdf1> <pdf2>
```

---

## Specialized Fixture Builders

### `build-objstm-fixture/`
**Purpose:** Generate object stream fixtures for testing

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
**Purpose:** Generate xref testing fixtures

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

## Development Workflow

### Quick Documentation Coverage Check
```bash
python tools/count_docs.py
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

### Debug Fingerprint Issues
```bash
cargo run --bin debug-fingerprint -- tests/fixtures/your-file.pdf
```

## Contributing

When adding new generators:
1. Use descriptive names with `generate_` prefix
2. Add a docstring/comment block explaining:
   - What fixtures it generates
   - How to run it (command line, dependencies)
   - Any special requirements (input data, API keys, etc.)
3. Update this README with the new tool
4. Follow existing patterns (Python for encoding/OCR, Rust for security/crypto)

## Related Documentation

- [Test Fixtures Documentation](../tests/fixtures/README.md)
- [Generator Categorization (bf-1iefu)](../notes/bf-1iefu.md)
- [Obsolete Generator Removal (bf-xqib3)](../notes/bf-xqib3.md)
