# Bead bf-4ofrgm: Document Type Assertion

## Changes Made

Updated the `test_document_type_from_fixture_data` function in `/home/coding/pdftract/crates/pdftract-py/tests/test_type_assertions.py` to match the required assertion format:

### Changes:
1. Changed variable name from `doc` to `result` for consistency with the specification
2. Updated assertion format to use `type(result).__name__` for cleaner error messages
3. Updated error message from `'Expected Document type, got {type(doc)}'` to `'Expected Document, got {type(result).__name__}'`

### Acceptance Criteria Status:
- ✅ Test asserts result is instance of Document (uses `isinstance(result, pdftract.Document)`)
- ✅ Assertion includes descriptive error message (`f'Expected Document, got {type(result).__name__}'`)
- ✅ Test calls the function being tested (`pdftract.Document.from_native(fixture_data)`)

### Test Results:
```
tests/test_type_assertions.py::test_document_type_from_fixture_data PASSED [100%]
============================== 1 passed in 0.04s ===============================
```

The Document type assertion is now working correctly and validates that `Document.from_native()` returns a properly typed `Document` object rather than a raw dict.
