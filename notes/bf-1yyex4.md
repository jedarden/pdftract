# Bead bf-1yyex4: Basic Smoke Test File Structure

## Summary
Created basic smoke test file structure at `tests/smoke_test.rs` with proper imports, test function skeletons, and fixture-based testing setup.

## Implementation
Created `/home/coding/pdftract/tests/smoke_test.rs` with:

### Imports
```rust
use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};
use std::path::Path;
```

### Test Functions
1. **`test_basic_pdf_extraction`**: Uses `test-minimal.pdf` (374 bytes)
   - Verifies PDF extraction succeeds
   - Validates at least one page extracted
   - Checks page dimensions are valid

2. **`test_sample_pdf_extraction`**: Uses `sample.pdf` (534 bytes)
   - Provides redundancy across different minimal PDFs
   - Same validation pattern

### Documentation
- Comprehensive module-level docstring explaining test purpose
- Individual function docstrings for each test
- Comments explaining each assertion

## Acceptance Criteria Status
✅ PASS - File `tests/smoke_test.rs` exists (107 lines)
✅ PASS - File imports `pdftract_core` module successfully
✅ PASS - Test functions exist with proper `#[test]` signatures (standalone, no args)
✅ PASS - Docstrings explain each test's purpose
✅ PASS - Uses existing simple PDF fixtures (test-minimal.pdf, sample.pdf)

## Artifacts
- File: `/home/coding/pdftract/tests/smoke_test.rs` (107 lines)
- Fixtures used: `tests/fixtures/test-minimal.pdf` (374 bytes), `tests/fixtures/sample.pdf` (534 bytes)
- Note file: `/home/coding/pdftract/notes/bf-1yyex4.md`

## Notes
- Test structure follows patterns from existing test files like `test_assertion_methods.rs`
- Uses two different minimal fixtures for redundancy
- Tests verify basic extraction pipeline without complex assertions (those come later)
- File compiles successfully and integrates with existing test suite
- Smoke test validates: basic PDF parsing, page extraction, output structure, page geometry

## Next Steps
The smoke test file structure is now in place. Subsequent beads can:
1. Add more comprehensive assertions
2. Test edge cases and error conditions
3. Add performance benchmarks
4. Test with more complex fixtures
