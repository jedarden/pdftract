# bf-5b55jv - Python SDK Type Classes Implementation

## Task
Define language-native type classes for SDK contract types in the Python SDK.

## Implementation Summary

The type classes were already implemented in `crates/pdftract-py/python/pdftract/types.py`. This verification confirms all acceptance criteria are met.

## Type Classes Verified

All 7 required type classes exist as frozen dataclasses with slots:

1. **Document** - Complete PDF extraction result
   - Fields: `pages: List[Page]`, `metadata: Metadata`
   - Methods: `from_native()`, `from_dict()`, `__repr__()`

2. **Page** - Single page from PDF
   - Fields: `page_index: int`, `spans: List[Span]`, `blocks: List[Block]`, `tables: List[Table]`, `error: Optional[str]`
   - Methods: `from_native()`, `from_dict()`, `__repr__()`

3. **Span** - Text span extracted from PDF
   - Fields: `text: str`, `bbox: List[float]`, `font: str`, `size: float`, `confidence: Optional[float]`
   - Methods: `from_native()`, `__repr__()`

4. **Block** - Semantic block extracted from PDF
   - Fields: `kind: str`, `text: str`, `bbox: List[float]`, `level: Optional[int]`, `table_index: Optional[int]`
   - Methods: `from_native()`, `__repr__()`

5. **Match** - Regex match result from search
   - Fields: `text: str`, `page_index: int`, `span_index: int`, `bbox: List[float]`, `match_start: int`, `match_end: int`
   - Methods: `from_native()`, `__repr__()`

6. **Fingerprint** - PDF structural fingerprint
   - Fields: `value: str`, `version: str`, `fast_hash: Optional[str]`
   - Methods: `from_native()`, `from_string()`, `__repr__()`

7. **Classification** - Page classification result
   - Fields: `category: str`, `confidence: float`, `hybrid_cells: Optional[set[int]]`
   - Methods: `from_native()`, `__repr__()`, `class_name` property

8. **Metadata** - Document metadata (bonus type)
   - Fields: `page_count: int`, `title: Optional[str]`, `author: Optional[str]`, `subject: Optional[str]`, `keywords: Optional[str]`, `creator: Optional[str]`, `producer: Optional[str]`, `creation_date: Optional[str]`, `mod_date: Optional[str]`, `fingerprint: Optional[str]`, `outline: Optional[dict]`
   - Methods: `from_native()`, `__repr__()`

## Additional Supporting Types

- **Cell** - Table cell
- **Row** - Table row
- **Table** - Extracted table

## Acceptance Criteria Status

### ✅ PASS - Type Classes Exist
All 7 type classes exist as `@dataclass(frozen=True, slots=True)` with proper field definitions.

### ✅ PASS - from_native() Classmethod
Each class has a `from_native(cls, native_dict: dict) -> ClassName` classmethod that converts from PyO3 native layer dict representation.

### ✅ PASS - Module Exports
All types are properly imported and re-exported from `pdftract/__init__.py`:
```python
from pdftract.types import (
    Document, Page, Span, Block, Match,
    Fingerprint, Classification, Metadata,
)
```

### ✅ PASS - API Method Integration
The 9 sync methods properly wrap raw dict results in typed objects:

1. `extract()` → wraps with `Document.from_native()`
2. `extract_text()` → returns `str` (no wrapping needed)
3. `extract_markdown()` → returns `str` (no wrapping needed)
4. `extract_stream()` → wraps with `Page.from_native()`
5. `search()` → wraps with `Match.from_native()`
6. `get_metadata()` → wraps with `Metadata.from_native()`
7. `hash()` → wraps with `Fingerprint.from_native()` or `Fingerprint.from_string()`
8. `classify()` → wraps with `Classification.from_native()`
9. `verify_receipt()` → returns `bool` (no wrapping needed)

### ✅ PASS - IDE Autocomplete
All type attributes are accessible via attribute access (e.g., `document.pages`, `page.blocks`, `span.text`), enabling IDE autocomplete and mypy type checking.

### ✅ PASS - Smoke Test
```python
import pdftract
span = pdftract.Span(text='Hello', bbox=[0,0,100,20], font='Arial', size=12.0)
assert isinstance(span, pdftract.Span)
assert span.text == 'Hello'
```

## Files Modified

**`crates/pdftract-py/python/pdftract/fallback.py`**: Fixed inconsistent type conversion (5 changes)
- Line 167: `Document.from_dict()` → `Document.from_native()`
- Line 243: `Page.from_dict()` → `Page.from_native()`
- Line 271: Direct `Match()` construction → `Match.from_native()`
- Line 303: Direct `Metadata()` construction → `Metadata.from_native()`
- Line 367: Dict return → `Classification.from_native()`

This ensures the subprocess fallback uses the same `from_native` classmethods as the native module for consistency.

## Verification Commands

```bash
# Test type class instantiation
cd /home/coding/pdftract/crates/pdftract-py
python -c "
import sys
sys.path.insert(0, 'python')
from pdftract.types import Document, Page, Span, Block, Match, Fingerprint, Classification, Metadata
span = Span(text='Test', bbox=[0,0,100,20], font='Arial', size=12.0)
print(f'✓ Type classes work: {span}')
"

# Test module exports
python -c "
import sys
sys.path.insert(0, 'python')
import pdftract
assert hasattr(pdftract, 'Document')
assert hasattr(pdftract, 'Page')
print('✓ All types exported')
"

# Test API wrapping
python -c "
import sys
sys.path.insert(0, 'python')
import inspect
import pdftract
source = inspect.getsource(pdftract.extract)
assert 'Document.from_native' in source
print('✓ API methods wrap results')
"
```

## References

- Parent bead: pdftract-2nu0s (Python SDK surface area)
- Depends on: child bead 1 (sync API surface) 
- Plan section: SDK Acceptance Criteria, lines 3581-3589
