# bf-5seei4: Add test function and fixture loading

## Summary
Added comprehensive test functions to the existing `test_type_assertions.py` file for testing type assertions using real fixture data.

## Work Completed

### 1. Added `test_document_type_from_pdf_extraction()` function
This test function:
- Loads a real PDF fixture (`test-minimal.pdf`)
- Extracts the document using `pdftract.extract()`
- Validates that all returned objects match expected types:
  - Document instance
  - Metadata instance
  - List of Page instances
  - Individual Page instances

### 2. Added `test_metadata_field_types()` function
This test function:
- Extracts a PDF document
- Validates that Metadata fields have correct types
- Checks `page_count` is an `int`
- Verifies `title` and `author` are `str` or `None`

### 3. Verified syntax and imports
- Python syntax validation: ✓ PASS
- pdftract module import: ✓ PASS
- Test file follows pytest naming conventions (all functions prefixed with `test_`)

## Acceptance Criteria Status
- ✅ PASS - Test function exists and follows pytest naming conventions
- ✅ PASS - Test loads a real fixture file (`test-minimal.pdf`)
- ✅ PASS - Required types are imported from pdftract module
- ✅ PASS - Test file is syntactically valid Python
- ✅ PASS - Fix commit: 4e5b9de05871ef8e7866ded86035cbc4da5e5be0

## Files Modified
- `/home/coding/pdftract/crates/pdftract-py/tests/test_type_assertions.py` - Added two new test functions

## Related Files
- `/home/coding/pdftract/crates/pdftract-py/tests/fixtures/test-minimal.pdf` - PDF fixture used by tests
- `/home/coding/pdftract/crates/pdftract-py/tests/smoke_test.py` - Original test file from bf-1yyex4

## Notes
- The existing `test_type_assertions.py` file already had a solid foundation with imports, fixtures, and test structure
- Added tests build upon the existing pattern while adding more comprehensive type checking
- Tests follow pytest conventions and integrate seamlessly with the existing test suite
