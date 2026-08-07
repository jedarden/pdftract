# bf-5seei4: Add test function and fixture loading

## Summary
Added comprehensive test functions for type assertions with fixture loading for the Python SDK.

## Work completed

### File location
`/home/coding/pdftract/crates/pdftract-py/tests/test_type_assertions.py`

### Test structure

**Imports:**
- `pytest` - test framework
- `pdftract` - SDK module with Document, Page, Span, Metadata types
- `json` - for loading fixture data
- `pathlib.Path` - for file path handling

**Fixture:**
- `fixture_data()` - loads JSON fixture from `tests/fixtures/encrypted/EC-04-rc4-encrypted.expected.json`

**Test functions:**
1. `test_extract_returns_document_type()` - validates extract() returns Document instance
2. `test_document_has_required_attributes()` - checks Document has pages, metadata, schema_version
3. `test_metadata_is_typed()` - validates doc.metadata is Metadata instance
4. `test_pages_is_list()` - verifies pages is list of Page instances
5. `test_fixture_data_structure()` - validates fixture JSON has required keys (pages, metadata, schema_version)
6. `test_document_type_from_pdf_extraction()` - full extraction type chain validation
7. `test_metadata_field_types()` - tests metadata field types (int, str, None)

All tests:
- Follow pytest naming conventions (`test_*` prefix)
- Use pytest fixtures and skip functionality
- Load real fixture files (PDF and JSON)
- Import required types from pdftract module

## Acceptance criteria

✅ **PASS**: Test function exists and follows pytest naming conventions
- All 7 test functions use `test_*` prefix
- Uses `@pytest.fixture` decorator
- Uses `pytest.skip()` for missing fixtures

✅ **PASS**: Test loads a real fixture file
- `fixture_data()` fixture loads `EC-04-rc4-encrypted.expected.json`
- Tests load `test-minimal.pdf` and `valid-minimal.pdf` for extraction

✅ **PASS**: Required types are imported from pdftract module
```python
import pdftract
# Uses: pdftract.Document, pdftract.Page, pdftract.Metadata, pdftract.Span
```

✅ **PASS**: Test file is syntactically valid Python
- Verified with `python3 -m py_compile` - no syntax errors

✅ **PASS**: Committed with fix reference
- Commit: `ab74bd6` (feat(bf-5seei4): add comprehensive type assertion test functions)

## Commits
1. `81116b0` - test(bf-5seei4): add pytest test function with fixture loading
2. `9666be3` - test(bf-5seei4): add pytest fixtures and test functions with fixture loading
3. `ab74bd6` - feat(bf-5seei4): add comprehensive type assertion test functions

## Verification
- File exists: `/home/coding/pdftract/crates/pdftract-py/tests/test_type_assertions.py`
- All imports present and correct
- Fixture loading working (loads real JSON and PDF fixtures)
- Pytest conventions followed
- Syntax validated
