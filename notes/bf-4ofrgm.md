# Verification Note: bf-4ofrgm - Implement Document type assertion

## Task
Add assertion to verify the top-level returned object is an instance of Document class.

## What was done
Updated the Document type assertion in `test_sdk_types_smoke.py` (line 230) to use the imported `Document` class instead of `pdftract.Document`, making it consistent with the imports at the top of the file.

**Change:**
- Before: `assert isinstance(doc, pdftract.Document), f'Expected Document, got {type(doc).__name__}'`
- After: `assert isinstance(doc, Document), f'Expected Document, got {type(doc).__name__}'`

## Acceptance criteria
✅ **PASS** - Test asserts result is instance of Document
✅ **PASS** - Assertion includes descriptive error message (`'Expected Document, got {type(doc).__name__}'`)
✅ **PASS** - Test calls the function being tested (`pdftract.extract(pdf_path)`)

## Test results
All tests pass successfully:
- `test_type_assertions_from_fixture_data()` - ✓ Document type assertion passed
- `test_extract_returns_typed_document()` - ✓ extract() returns Document instance
- `test_pdf_document_with_fixture_validation()` - ✓ extract() returns Document instance

## Implementation guidance compliance
The assertion matches the implementation guidance format:
```python
assert isinstance(result, Document), f'Expected Document, got {type(result).__name__}'
```

Note: The test uses `doc` as the variable name instead of `result`, which is more semantically meaningful in this context. The type assertion logic is identical.

## Files modified
- `test_sdk_types_smoke.py` (line 230)

## Related beads
- Parent: bf-ds6pdh (Implement type assertion tests)
- Dependency: bf-5t29nm (Add type imports to test file) - CLOSED
