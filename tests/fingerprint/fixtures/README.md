# Fingerprint Reproducibility Test Fixtures

This directory contains fixture pairs that verify the fingerprint algorithm's reproducibility and content-sensitivity properties.

## Fixture Provenance

All fixtures are generated from a clean source PDF (`.clean_source.pdf`) created using `pikepdf`, a Python library for PDF manipulation. The source is a 3-page PDF with Lorem Ipsum text, created with minimal metadata.

## Generation

Fixtures are generated using `generate_fingerprint_fixtures.py`, which requires:
- Python 3.11+
- `pikepdf` library (install via nix-shell or pip)

```bash
nix-shell --pure --packages python3 python3Packages.pikepdf --run \
  'python3 tests/fingerprint/fixtures/generate_fingerprint_fixtures.py'
```

## Fixture Pairs

Each fixture pair contains:
- `v1.pdf` - Original or first variant
- `v2.pdf` - Second variant (modified copy or re-saved version)
- `expected.txt` - Either "MATCH" (fingerprints should be identical) or "DIFFER" (fingerprints should differ)

### 1. byte_identical
**Expected: MATCH**
- Same PDF copied twice (verifies fingerprint determinism)

### 2. acrobat_resave
**Expected: MATCH**
- Simulates Acrobat re-save using qpdf
- Changes `/CreationDate`, `/ID`, and xref byte layout
- Preserves content (metadata-only changes should not affect fingerprint per ADR-008)

### 3. pdftk_resave
**Expected: MATCH**
- Simulates pdftk re-save using qpdf
- Changes object stream layout and compression
- Content should produce identical fingerprint

### 4. qpdf_resave
**Expected: MATCH**
- Same source through qpdf with `--object-streams=preserve --normalize-content=y`
- Verifies qpdf re-save produces same fingerprint

### 5. linearization_toggle
**Expected: MATCH (KU-7)**
- Unlinearized PDF vs `qpdf --linearize` output
- Different byte layouts but same content
- Verifies linearization independence (KU-7 requirement)

### 6. metadata_only
**Expected: MATCH (ADR-008)**
- Original vs copy with changed `/Title`, `/Author`, `/Producer`, `/CreationDate`
- Verifies metadata independence per ADR-008

### 7. content_edit_one_glyph
**Expected: DIFFER**
- "Hello World" vs "Hello Worl" (one character removed)
- Verifies content-sensitivity: removing a single glyph changes fingerprint

### 8. content_edit_one_paragraph
**Expected: DIFFER**
- Original paragraph vs variant with one word changed
- Verifies content-sensitivity: paragraph edit changes fingerprint

## License

The fixture PDFs are generated using MIT-licensed tools (pikepdf, qpdf) and contain public-domain text (Lorem Ipsum). Fixtures are MIT-licensed.

## References

- ADR-008: Metadata independence
- KU-7: Linearization independence
- INV-3: Fingerprint reproducibility (100 invocations produce identical results)
- INV-13: Fingerprint format (`^pdftract-v1:[0-9a-f]{64}$`)
