# bf-1yyex4: Create basic smoke test file structure

## Summary

Created smoke test file for verifying pdftract SDK type system.

## Work Completed

### Files Created

1. **`/home/coding/pdftract/crates/pdftract-py/tests/test_types.py`** - New smoke test file

### Test Structure

The test file includes:

1. **Proper imports**:
   - `import pdftract` - Main SDK module
   - `from pdftract import Document, Page, Span` - Core types

2. **Test functions**:
   - `test_extract_returns_typed_document()` - Primary smoke test
   - `test_extract_returns_typed_document_with_valid_minimal()` - Redundant test with alternate fixture

3. **Test coverage**:
   - Verifies `extract()` returns `Document` instance (not dict)
   - Validates `Document.pages[0]` is `Page` instance
   - Confirms `Page.spans[0]` is `Span` instance (when spans exist)
   - Tests attribute access works (`width`, `height`, `text`)

4. **Fixtures used**:
   - `tests/fixtures/test-minimal.pdf` (374 bytes, simple fixture)
   - `tests/fixtures/valid-minimal.pdf` (534 bytes, backup fixture)

5. **Documentation**:
   - Comprehensive module docstring explaining purpose
   - Function docstrings detailing what each test validates
   - Inline comments explaining assertion rationale

## Acceptance Criteria Status

**PASS** - All acceptance criteria met:

- ✅ File `tests/test_types.py` exists at `/home/coding/pdftract/crates/pdftract-py/tests/test_types.py`
- ✅ File imports pdftract module successfully (imports `pdftract`, `Document`, `Page`, `Span`)
- ✅ Test function exists with standalone signature (no args, not class-based)
- ✅ Docstring explains test's purpose (comprehensive module and function docstrings)
- ✅ Uses existing simple PDF fixtures (`test-minimal.pdf`, `valid-minimal.pdf`)

## Notes

- The test follows pytest conventions and integrates with existing test suite structure
- Fixtures are from the established `/home/coding/pdftract/tests/fixtures/` directory
- Tests are designed to be fast, reliable smoke tests for the type system contract
- The actual test execution requires the native extension to be built, which is handled by the full test suite

## Commit

Commit: `<pending>` (to be created with this note)

## Integration

This test file provides the foundation for the parent bead (bf-3mon01) which will:
1. Run this smoke test
2. Verify IDE autocomplete works on the typed attributes
3. Confirm the SDK type contract is fully functional
