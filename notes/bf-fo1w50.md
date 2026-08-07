# bf-fo1w50: Page Type Assertion

## Task
Add assertion to verify pages in the result are instances of Page class.

## Implementation

The Page type assertion was already implemented in `test_contract_methods.py` at lines 55-58:

```python
# Check first page is Page instance (bf-fo1w50: Page type assertion)
assert isinstance(result.pages[0], pdftract.Page), \
    f'Expected Page, got {type(result.pages[0]).__name__}'
print(f"  ✓ First page is Page instance (bf-fo1w50)")
```

## Verification

Test execution confirms the assertion works:
```
Testing extract()...
  ✓ Created Document from fixture with 1 pages
  ✓ First page is Page instance (bf-fo1w50)
  ✓ First page has 0 blocks (no spans)
```

## Acceptance Criteria

- ✅ Test asserts first page is instance of Page
- ✅ Assertion includes descriptive error message
- ✅ Test accesses result.pages collection

## Files Modified

- `crates/pdftract-py/test_contract_methods.py` - lines 55-58 (already in place)

## Status

**PASS** - All acceptance criteria met. The Page type assertion is working correctly in the test suite.
