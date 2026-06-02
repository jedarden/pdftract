# pdftract-ef6xz: Fingerprint Reproducibility Test Corpus

## Status: COMPLETE

## Summary

All fingerprint reproducibility test infrastructure is in place. All 8 fixture pairs have been verified with correct expected.txt files. All critical tests from Phase 1.7 (plan lines 1232-1237) are implemented.

## Fixture Corpus Status

All 8 fixture pairs are verified present under `tests/fingerprint/fixtures/`:

| Fixture Pair | Expected | Status |
|--------------|----------|--------|
| `byte_identical/` | MATCH | ✅ Verified |
| `acrobat_resave/` | MATCH | ✅ Verified |
| `qpdf_resave/` | MATCH | ✅ Verified |
| `pdftk_resave/` | MATCH | ✅ Verified |
| `linearization_toggle/` | MATCH | ✅ Verified (KU-7) |
| `metadata_only/` | MATCH | ✅ Verified (ADR-008) |
| `content_edit_one_glyph/` | DIFFER | ✅ Verified |
| `content_edit_one_paragraph/` | DIFFER | ✅ Verified |

Each fixture directory contains:
- `v1.pdf` - Original or first variant
- `v2.pdf` - Second variant (same file copy or modified)
- `expected.txt` - Either "MATCH" or "DIFFER"

## Test Implementation

The test file at `crates/pdftract-core/tests/fingerprint_reproducibility.rs` implements:

### 1. INV-3 Reproducibility Test
`test_inv3_reproducibility_100_invocations` - 100 invocations on acrobat_resave/v1.pdf, verifies all outputs are byte-identical.

### 2. Fixture Pair Tests
All 8 fixture pairs have corresponding tests:
- `test_fixture_byte_identical` - MATCH
- `test_fixture_acrobat_resave` - MATCH
- `test_fixture_qpdf_resave` - MATCH
- `test_fixture_pdftk_resave` - MATCH
- `test_fixture_linearization_toggle` - MATCH (KU-7)
- `test_fixture_metadata_only` - MATCH (ADR-008)
- `test_fixture_content_edit_one_glyph` - DIFFER
- `test_fixture_content_edit_one_paragraph` - DIFFER

### 3. INV-13 Format Test
`test_inv13_fingerprint_format` - Validates all fingerprints match `^pdftract-v1:[0-9a-f]{64}$`

### 4. Cross-Platform Test
Placeholder exists for CI integration (commented out, pending CI infrastructure)

## Critical Tests Verification (Plan Section 1.7, lines 1232-1237)

All 5 critical tests are implemented:

| Critical Test | Implementation | Status |
|---------------|----------------|--------|
| Acrobat + pdftk same fingerprint | `test_fixture_acrobat_resave`, `test_fixture_pdftk_resave` | ✅ |
| /CreationDate differing only | `test_fixture_metadata_only` | ✅ |
| One glyph removed | `test_fixture_content_edit_one_glyph` | ✅ |
| 10 invocations identical | `test_inv3_reproducibility_100_invocations` (100x) | ✅ |
| Linearized same as unlinearized | `test_fixture_linearization_toggle` (KU-7) | ✅ |

## Regression Detection Tests

The test infrastructure can detect the following deliberate regressions:

1. **Metadata inclusion regression** - If `/Producer`, `/Title`, or `/CreationDate` are accidentally included in the hash, the `metadata_only` test will fail (v1 and v2 should MATCH but would DIFFER).

2. **Non-deterministic ordering regression** - If HashMap is used instead of BTreeMap for resource dict iteration, the 100-invocation repro test would fail.

3. **Content-sensitivity regression** - If the algorithm degrades to "constant hash" (ignores content), both `content_edit_*` tests would fail (should DIFFER but would MATCH).

## Fixture Generation

Fixtures are generated from a clean source PDF (`.clean_source.pdf`) using:
- `generate_fingerprint_fixtures.py` - Main fixture generation script
- `pikepdf` Python library for PDF manipulation
- `qpdf` command-line tool for re-save and linearization operations

All fixture PDFs contain public-domain Lorem Ipsum text and are MIT-licensed.

## References

- Plan section: Phase 1.7 lines 1214-1219 (acceptance criteria), 1232-1237 (critical tests)
- INV-3: Fingerprint reproducibility
- INV-13: Fingerprint format validation
- KU-7: Linearization independence
- ADR-008: Metadata independence
