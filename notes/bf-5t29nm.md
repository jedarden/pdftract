# Verification Note: bf-5t29nm

## Summary
Added type imports (Document, Page, Span) to smoke test file.

## Changes Made
- File: `/home/coding/pdftract/crates/pdftract-py/tests/smoke_test.py`
- Added import line: `from pdftract import Document, Page, Span`
- Import is placed at the top of the file after the basic `import pdftract` statement (line 23)

## Acceptance Criteria Status
✅ **PASS** - Test file imports Document, Page, Span from pdftract module
✅ **PASS** - Imports are at the top of the file (line 23)
✅ **PASS** - Test file runs without import errors

## Verification
Ran `python3 smoke_test.py` - all tests pass with no import errors:
```
============================================================
pdftract SDK Smoke Test
============================================================

✓ extract() returns Document instance
✓ Document has 'pages' attribute
✓ Document has typed Metadata

✅ All smoke tests passed!
```

## References
- Parent bead: bf-ds6pdh
- Depends on: bf-1yyex4 (test file structure exists)
