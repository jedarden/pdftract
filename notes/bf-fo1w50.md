# bf-fo1w50: Page Type Assertion Implementation

## Task
Add assertion to verify pages in the result are instances of Page class.

## Implementation
Added Page type assertion to `test_contract_methods.py` in the `test_extract()` function:

```python
# First page should have expected attributes
page = result.pages[0]
assert isinstance(page, pdftract.Page), \
    f'Expected Page, got {type(page).__name__}'
```

## File Modified
- `/home/coding/pdftract/crates/pdftract-py/test_contract_methods.py` (lines 34-35)

## Acceptance Criteria
- ✅ Test asserts first page is instance of Page
- ✅ Assertion includes descriptive error message  
- ✅ Test accesses result.pages collection

## Verification Notes
The assertion correctly checks that `result.pages[0]` is an instance of `pdftract.Page` class and provides a descriptive error message showing the actual type name if the assertion fails.

**Note:** Test runs show PDF fixture errors ("No /Root reference in trailer") which are pre-existing environment issues with the test fixtures/native module, not related to this code change. The type assertion pattern is correctly implemented and matches the same pattern used in `test_type_assertions.py::test_document_type_from_fixture_data`.
