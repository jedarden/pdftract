# bf-5seei4: Add test function and fixture loading

## Summary
Added pytest-style test functions and fixture loading to the SDK type smoke test file.

## Work Completed

### File Changes
- **test_sdk_types_smoke.py**: Added pytest fixtures and test functions
  - Imported `json`, `os`, and `pathlib.Path` for fixture handling
  - Added conditional pytest import with `HAS_PYTEST` flag
  - Created pytest fixtures: `fixture_path`, `hybrid_fixture_metadata`, `sample_pdf_path`
  - Added `test_fixture_metadata_loading()` function
  - Added `test_pdf_document_with_fixture_validation()` function
  - Updated main test runner to include new test functions

### Test Functions Added

#### test_fixture_metadata_loading()
- Loads JSON metadata from hybrid fixture: `tests/fixtures/hybrid/hybrid-001-vector-header-over-scan.pdf.metadata.json`
- Validates metadata structure and required fields
- Verifies fixture metadata can be parsed correctly
- Tests fixture metadata structure validation

#### test_pdf_document_with_fixture_validation()
- Uses remote_100page.pdf as working PDF fixture
- Validates document extraction and type assertions
- Verifies Document and Metadata instances
- Tests Page type validation when pages are available

### Acceptance Criteria Status
- ✅ Test function exists and follows pytest naming conventions
- ✅ Test loads a real fixture file (JSON metadata and PDF)
- ✅ Required types are imported from pdftract module (pdftract.Document, pdftract.Metadata, pdftract.Page)
- ✅ Test file is syntactically valid Python (verified with `python3 -m py_compile`)
- ✅ Fix commit: 9666be3

## Verification

### Test Execution
```bash
python3 test_sdk_types_smoke.py
```

All tests passed successfully:
- ✅ Fixture metadata loading test passed
- ✅ PDF document with fixture validation test passed
- ✅ All original type assertion tests passed

### Syntax Validation
```bash
python3 -m py_compile test_sdk_types_smoke.py
# ✓ File is syntactically valid Python
```

### Commits
- `9666be3` - test(bf-5seei4): add pytest fixtures and test functions with fixture loading

## Notes
- The test file was previously gitignored due to the `test_*` pattern in `.gitignore` (line 35)
- Used `git add -f` to force-add the Python test file despite the ignore pattern
- The `test_*` pattern is intended for compiled binaries, not Python test files
- All test functions follow pytest naming conventions (prefixed with `test_`)
- Tests use real fixture files from `tests/fixtures/` directory
- Pytest fixtures are conditional on pytest availability (graceful fallback when not installed)
