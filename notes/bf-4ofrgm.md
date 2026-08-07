# Verification Note: bf-4ofrgm - Implement Document type assertion

## Task
Add assertion to verify the top-level returned object is an instance of Document class.

## Implementation verified

The Document type assertion has been implemented in two locations:

### 1. `crates/pdftract-py/tests/test_type_assertions.py`

#### In `test_extract_returns_document_type()` (lines 58-59):
```python
assert isinstance(doc, pdftract.Document), \
    f'Expected Document, got {type(doc).__name__}'
```

#### In `test_document_type_from_fixture_data()` (lines 230-231):
```python
assert isinstance(result, pdftract.Document), \
    f'Expected Document, got {type(result).__name__}'
```

### 2. `test_sdk_types_smoke.py` (line 230):
```python
assert isinstance(doc, Document), f'Expected Document, got {type(doc).__name__}'
```

## Acceptance criteria status
✅ **PASS** - Test asserts result is instance of Document
✅ **PASS** - Assertion includes descriptive error message (`'Expected Document, got {type(result).__name__}'`)
✅ **PASS** - Test calls the function being tested (`pdftract.extract()` and `Document.from_native()`)

## Implementation guidance compliance
The assertions match the implementation guidance format exactly:
```python
assert isinstance(result, Document), f'Expected Document, got {type(result).__name__}'
```

Both implementations use:
- `isinstance()` to verify the type
- Descriptive error message showing actual type received
- Clear variable naming (`doc` or `result` for the returned Document)

## Test functions covered
1. `test_extract_returns_document_type()` - Tests `pdftract.extract()` returns Document
2. `test_document_type_from_fixture_data()` - Tests `Document.from_native()` returns Document
3. `test_type_assertions_from_fixture_data()` - Tests `pdftract.extract()` with imported Document type

## Related commits
- a3a12cf - add Document type assertion
- c4a9cdd - use imported Document class in type assertion
- 6d2d1e9 - implement Document type assertion

## Related beads
- Parent: bf-ds6pdh (Implement type assertion tests)
- Dependency: bf-5t29nm (Add type imports to test file) - CLOSED
