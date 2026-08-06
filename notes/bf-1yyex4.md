# bf-1yyex4: Create basic smoke test file structure (Python SDK)

## Summary
Created a standalone smoke test file for the pdftract Python SDK with proper imports, test function skeleton, and fixture-based testing setup.

## Work Completed

### 1. Created smoke test file
Created `/home/coding/pdftract/crates/pdftract-py/tests/smoke_test.py` with:
- Proper imports of pdftract module
- Standalone test function `test_extract_returns_typed_document()` with proper signature
- Comprehensive docstrings explaining the test's purpose
- Uses existing minimal PDF fixture (`test-minimal.pdf`)

### 2. Test structure
The smoke test verifies:
- `extract()` returns a `Document` instance (not a dict)
- Document has `pages` attribute
- Document has typed `Metadata` instance
- All core type contracts are met

### 3. Execution
The test runs successfully without requiring pytest or external dependencies:
```
$ python3 smoke_test.py
============================================================
pdftract SDK Smoke Test
============================================================

✓ extract() returns Document instance
✓ Document has 'pages' attribute
✓ Document has typed Metadata

✅ All smoke tests passed!
```

## Acceptance Criteria Status
- ✅ PASS - File exists: `/home/coding/pdftract/crates/pdftract-py/tests/smoke_test.py`
- ✅ PASS - File imports module successfully: Imports `pdftract` from `crates/pdftract-py/python`
- ✅ PASS - Test function exists with signature: `test_extract_returns_typed_document() -> None`
- ✅ PASS - Docstring explains the test's purpose: Comprehensive docstring included
- ✅ PASS - File uses an existing simple PDF fixture: Uses `fixtures/test-minimal.pdf`

## Files Created
- `/home/coding/pdftract/crates/pdftract-py/tests/smoke_test.py` - Standalone smoke test (2,502 bytes)

## Related Files (No Changes)
- `/home/coding/pdftract/crates/pdftract-py/tests/fixtures/test-minimal.pdf` - Existing fixture
- `/home/coding/pdftract/crates/pdftract-py/tests/fixtures/valid-minimal.pdf` - Alternative fixture

## Notes
- The smoke test is designed to be run standalone without pytest dependency
- Uses minimal PDF fixtures for fast execution
- Provides quick validation that the SDK is properly structured and functional
- Can be extended with additional test functions as needed
