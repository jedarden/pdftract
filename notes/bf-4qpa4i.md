# Verification Note: bf-4qpa4i — Export SDK types from pdftract module

## Summary
Enhanced the pdftract module docstring to explicitly document all available SDK types, making the public API more discoverable for users. The types were already properly imported and re-exported in `__init__.py`.

## Acceptance Criteria Verification

### ✓ PASS: types.py classes are imported in __init__.py
Location: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/__init__.py:43-52`
```python
from pdftract.types import (
    Document,
    Page,
    Span,
    Block,
    Match,
    Fingerprint,
    Classification,
    Metadata,
)
```

### ✓ PASS: Classes are re-exported at module level
The types are imported at module level (not inside a function or conditional), making them directly accessible as `pdftract.Document`, etc.

### ✓ PASS: All 8 types are importable as pdftract.Document, pdftract.Page, etc.
Verified with import test:
```bash
PYTHONPATH=python python3 -c "from pdftract import Document, Page, Span, Block, Match, Fingerprint, Classification, Metadata; print('✓ All 8 types imported successfully')"
```
Output: `✓ All 8 types imported successfully`

### ✓ PASS: Module compiles without errors
```bash
python3 -m py_compile python/pdftract/__init__.py  # PASS
python3 -m py_compile python/pdftract/types.py    # PASS
```

### ✓ PASS: from pdftract import Document works in a Python REPL
Verified that all 8 types are importable:
```
Document: <class 'pdftract.types.Document'>
Page: <class 'pdftract.types.Page'>
Span: <class 'pdftract.types.Span'>
Block: <class 'pdftract.types.Block'>
Match: <class 'pdftract.types.Match'>
Fingerprint: <class 'pdftract.types.Fingerprint'>
Classification: <class 'pdftract.types.Classification'>
Metadata: <class 'pdftract.types.Metadata'>
```

### ✓ PASS: Types included in __all__
The `__all__` list (lines 71-102) properly exports all 8 types along with other public API members.

### ✓ PASS: Module docstring documents available types
Added explicit "Available types" section to module docstring listing all 8 SDK types with brief descriptions.

## Implementation Details
- Enhanced the module docstring in `__init__.py` to add "Available types" section
- The type imports were already present at module level for direct access
- All types have `from_native` classmethods for conversion from native Rust dictionaries

## Files Modified
- `crates/pdftract-py/python/pdftract/__init__.py`: Enhanced docstring with explicit type documentation

## Commits
- `4f8fd89` docs(bf-4qpa4i): enhance module docstring with available types documentation

## References
- Bead: bf-4qpa4i
- Parent bead: bf-5b55jv
- Plan: `/home/coding/pdftract/docs/plan/plan.md`
