# Bead bf-1yyex4: Basic Smoke Test File Structure (Rust)

## Status: PASS

## Summary

Verified the Rust smoke test file structure at `tests/smoke_test.rs` meets all acceptance criteria.

## Acceptance Criteria Status

All criteria PASS:

1. **PASS** - File `tests/smoke_test.rs` exists
2. **PASS** - File imports `pdftract_core` module successfully (line 12: `use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};`)
3. **PASS** - Test function `test_basic_pdf_extraction` exists with proper signature (line 15: `fn test_basic_pdf_extraction()`)
4. **PASS** - Docstring explains the test's purpose:
   - Module-level docstring (lines 1-11): explains the overall smoke test purpose
   - Function-level docstring (lines 16-24): explains what the specific test verifies
5. **PASS** - File uses existing simple PDF fixtures:
   - Primary fixture: `tests/fixtures/test-minimal.pdf` (374 bytes)
   - Secondary fixture: `tests/fixtures/sample.pdf` (534 bytes)

## Test Structure

The smoke test file includes:

1. **Module documentation** (lines 1-11): Explains the smoke test validates PDF extraction pipeline
2. **Import statement** (line 12): Imports `extract_pdf`, `ExtractionOptions`, and `OutputOptions` from `pdftract_core`
3. **Primary test function** `test_basic_pdf_extraction` (lines 15-67):
   - Verifies fixture exists
   - Runs extraction with default options
   - Validates extraction succeeds
   - Confirms at least one page is extracted
   - Validates page dimensions are positive
4. **Secondary test function** `test_sample_pdf_extraction` (lines 69-106):
   - Provides redundancy using a different fixture
   - Same validation structure as primary test

## Integration

The test file is ready to be run via:
```bash
cargo test test_basic_pdf_extraction
cargo test test_sample_pdf_extraction
cargo test --test smoke_test
```

## References

- Parent bead: bf-3mon01
- Depends on: bf-49vvzm (SDK type exploration complete)
