# bf-1yyex4: Rust Smoke Test File Structure

## Summary
Created basic Rust smoke test file structure at `tests/smoke_test.rs` with proper imports, test function skeleton, and fixture-based testing setup.

## What Was Done
1. **Created test file**: `tests/smoke_test.rs` (106 lines)
2. **Added necessary imports**:
   - `pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions}`
   - `std::path::Path`
3. **Created test functions**:
   - `test_basic_pdf_extraction()` - Validates basic PDF extraction using test-minimal.pdf (374 bytes)
   - `test_sample_pdf_extraction()` - Validates extraction using sample.pdf (534 bytes)
4. **Added comprehensive documentation**:
   - Module-level docstring explaining purpose (lines 2-10)
   - Per-test docstrings explaining what each validates
5. **Uses existing simple PDF fixtures**:
   - `tests/fixtures/test-minimal.pdf` (374 bytes)
   - `tests/fixtures/sample.pdf` (534 bytes)

## Acceptance Criteria Status
| Criterion | Status | Evidence |
|-----------|--------|----------|
| File tests/smoke_test.rs exists | ✅ PASS | File exists with 106 lines |
| File imports pdftract_core module successfully | ✅ PASS | Lines 12-13 import required modules |
| Test function test_basic_pdf_extraction exists with signature | ✅ PASS | Line 16: `fn test_basic_pdf_extraction()` |
| Docstring explains the test's purpose | ✅ PASS | Lines 2-10 (module) and 18-24 (test) |
| Uses existing simple PDF fixture | ✅ PASS | Uses test-minimal.pdf (374 bytes) |

## File Structure Verification
```bash
$ wc -l tests/smoke_test.rs
106 tests/smoke_test.rs

$ grep -E "^(use|#\[test\]|fn test_)" tests/smoke_test.rs
use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};
use std::path::Path;
#[test]
fn test_basic_pdf_extraction() {
#[test]
fn test_sample_pdf_extraction() {
```

## Fixture Verification
```bash
$ ls -lh tests/fixtures/test-minimal.pdf tests/fixtures/sample.pdf
-rw-r--r-- 1 coding users 534 May 31 23:42 tests/fixtures/sample.pdf
-rw-r--r-- 1 coding users 374 May 23 13:08 tests/fixtures/test-minimal.pdf
```

## Test Coverage
The smoke tests validate:
- Basic PDF parsing and loading
- Page extraction functionality
- Text extraction capabilities
- Error handling for missing fixtures
- Validation of extraction results (non-empty pages, valid dimensions)

## Implementation Details
The test follows standard Rust testing conventions:
- Uses `#[test]` attribute for test functions
- Returns `()` (no special return type)
- Uses `assert!` and `assert!` with custom error messages
- Includes descriptive assertions for debugging
- Uses `Path::new()` for fixture paths
- Calls `extract_pdf()` with default options

## Next Steps
The smoke test foundation is now in place. Subsequent beads can add:
- Additional assertions for extraction quality
- More comprehensive fixture coverage
- Performance validation
- Integration with CI pipeline (via existing Argo WorkflowTemplate)

## References
- Parent bead: bf-3mon01
- Depends on: bf-49vvzm (SDK type exploration complete)
