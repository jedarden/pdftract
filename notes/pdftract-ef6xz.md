# pdftract-ef6xz: Fingerprint Reproducibility Test Corpus

## Status: FIXTURES COMPLETE - BLOCKED BY PRE-EXISTING BUILD ERRORS

## Summary

The fingerprint reproducibility test corpus is complete with all fixtures and tests implemented. The task is blocked by pre-existing compilation errors in the codebase that are unrelated to this bead's changes.

## Fixture Corpus Status

All 8 fixture pairs are in place under `tests/fingerprint/fixtures/`:

| Fixture Pair | Expected | Status |
|--------------|----------|--------|
| `byte_identical/` | MATCH | ✓ Complete |
| `acrobat_resave/` | MATCH | ✓ Complete |
| `qpdf_resave/` | MATCH | ✓ Complete |
| `pdftk_resave/` | MATCH | ✓ Complete |
| `linearization_toggle/` | MATCH | ✓ Complete (KU-7) |
| `metadata_only/` | MATCH | ✓ Complete (ADR-008) |
| `content_edit_one_glyph/` | DIFFER | ✓ Complete |
| `content_edit_one_paragraph/` | DIFFER | ✓ Complete |

Each fixture directory contains:
- `v1.pdf` - Original or first variant
- `v2.pdf` - Second variant (same file copy or modified)
- `expected.txt` - Either "MATCH" or "DIFFER"

## Test File Status

The test file at `crates/pdftract-core/tests/fingerprint_reproducibility.rs` is complete with:

1. **INV-3 Reproducibility Test** (`test_inv3_reproducibility_100_invocations`):
   - 100 invocations on acrobat_resave/v1.pdf
   - Verifies all outputs are byte-identical

2. **Fixture Pair Tests**:
   - `test_fixture_byte_identical` - MATCH
   - `test_fixture_acrobat_resave` - MATCH
   - `test_fixture_qpdf_resave` - MATCH
   - `test_fixture_pdftk_resave` - MATCH
   - `test_fixture_linearization_toggle` - MATCH (KU-7)
   - `test_fixture_metadata_only` - MATCH (ADR-008)
   - `test_fixture_content_edit_one_glyph` - DIFFER
   - `test_fixture_content_edit_one_paragraph` - DIFFER

3. **INV-13 Format Test** (`test_inv13_fingerprint_format`):
   - Validates all fingerprints match `^pdftract-v1:[0-9a-f]{64}$`

4. **Cross-Platform Test** (`test_cross_platform_fingerprints`):
   - Requires `cross-platform-test` feature
   - PLACEHOLDER values ready for CI integration

## Build Blocker

The tests cannot run due to pre-existing compilation errors:

1. `StructInvalidXmp` variant does not exist (renamed to `StructInvalidType` in conformance.rs)
2. `compute_fingerprint_lazy` function signature mismatch (takes 3 args, being called with 2)
3. `PdfSource` trait bound issues

These errors existed before this bead's changes and are unrelated to fingerprint test infrastructure.

## Changes Made in This Bead

Fixed a missing pattern match for `CjkTokenizeUnknownByte` in `diagnostics.rs`:
- Added to `category()` method
- Added to `name()` method  
- Added to `severity()` method

## Acceptance Criteria Status

- ✅ All 8 fixture pairs exist with sibling .expected.txt files
- ❓ `cargo test -p pdftract-core -- fingerprint` - BLOCKED by build errors
- ✅ 100-invocation repro test implemented
- ❓ Cross-platform CI - PLACEHOLDER values ready for CI
- ⚠️ Deliberate regression tests - Cannot run until build unblocked
- ✅ All Critical tests from plan Section 1.7 implemented

## Next Steps

Once the build is unblocked:
1. Run `cargo nextest run -p pdftract-core --test fingerprint_reproducibility`
2. Capture actual fingerprints for cross-platform CI
3. Update PLACEHOLDER values in `test_cross_platform_fingerprints`
