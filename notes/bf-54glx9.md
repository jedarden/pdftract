# Verification Note for bf-54glx9: Fixture Call and Document Type Assertion

## Summary
The fixture call and Document type assertion are already implemented in `smoke_test.py` and working correctly.

## Implementation Verification

### Acceptance Criteria Status

**✓ Test calls SDK method with real fixture**
- Location: `smoke_test.py:44`
- Code: `doc = pdftract.extract(str(fixture_path))`
- Fixture: `tests/fixtures/test-minimal.pdf`

**✓ First assertion checks isinstance(returned, Document)**
- Location: `smoke_test.py:47-48`
- Code: `assert isinstance(doc, pdftract.Document), f'Expected Document, got {type(doc).__name__}'`

**✓ Error message is clear and includes actual type**
- Message: `f'Expected Document, got {type(doc).__name__}'`
- Includes actual type name via `type(doc).__name__`

**✓ Test compiles and runs**
- Test execution: `python3 smoke_test.py`
- Result: All smoke tests passed ✓
- Output:
  ```
  ✓ extract() returns Document instance
  ✓ Document has 'pages' attribute
  ✓ Document has typed Metadata
  ✅ All smoke tests passed!
  ```

## Code Details

### Fixture Call (line 44)
```python
# Extract the document
doc = pdftract.extract(str(fixture_path))
```

### Document Type Assertion (lines 47-48)
```python
# Verify Document type
assert isinstance(doc, pdftract.Document), \
    f'Expected Document, got {type(doc).__name__}'
```

## Test Execution
```bash
$ cd /home/coding/pdftract/crates/pdftract-py/tests
$ python3 smoke_test.py
============================================================
pdftract SDK Smoke Test
============================================================

✓ extract() returns Document instance
✓ Document has 'pages' attribute
✓ Document has typed Metadata

✅ All smoke tests passed!
```

## Conclusion
All acceptance criteria for bead bf-54glx9 are met by the existing implementation in `smoke_test.py`. The fixture call correctly invokes the SDK with a real PDF fixture, and the isinstance assertion validates the Document type with a clear error message that includes the actual type received.
