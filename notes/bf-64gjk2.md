# bf-64gjk2: Set up test imports and basic structure

## Summary
Created the basic test file structure for Python SDK type assertion testing.

## Work completed

### Files created
- `tests/sdk/test_python_sdk.py` - New test file with proper imports and structure

### Acceptance criteria status
✅ **PASS** - All acceptance criteria met:
1. Test file exists at `tests/sdk/test_python_sdk.py`
2. Imports include pytest and required types from pdftract module
3. Basic test function follows pytest naming convention (`test_python_sdk_types`)
4. Function has clear docstring explaining what will be verified
5. Body contains only TODO comment for next step

### Test structure
- Module docstring explains the purpose of the test module
- Imports: `pytest`, `pdftract` (which includes Document, Page, Span types)
- Path setup to include the pdftract Python package
- Test function `test_python_sdk_types()` with clear docstring
- TODO placeholder for implementation in next bead (bf-ds6pdh)

### Python syntax validation
```bash
python3 -m py_compile tests/sdk/test_python_sdk.py
# ✓ Syntax is valid
```

## Next steps
The next bead (bf-ds6pdh) will implement the actual type assertions using fixture data.

## Verification
- File created: `tests/sdk/test_python_sdk.py`
- Syntax validation: PASS
- All acceptance criteria: PASS
