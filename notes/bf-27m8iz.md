# Bead bf-27m8iz: Add from_native classmethods to all SDK type classes

## Summary
Added `@classmethod from_native(cls, native_dict: dict) -> Self` constructors to all 11 dataclass types in `types.py` (note: bead description mentioned 8 classes, but the file actually contains 11 classes).

## Changes Made

### File: `crates/pdftract-py/python/pdftract/types.py`

1. **Added import**: Added `Self` to the typing imports

2. **Added `from_native` methods to all classes**:
   - `Metadata.from_native()` - Handles all optional fields with `.get()` and defaults
   - `Span.from_native()` - Converts bbox list to tuple, handles optional confidence
   - `Block.from_native()` - Converts bbox list to tuple, handles optional level
   - `Cell.from_native()` - Converts bbox/spans lists to appropriate types, casts numeric fields
   - `Row.from_native()` - Recursively converts nested Cell list
   - `Table.from_native()` - Recursively converts nested Row list
   - `Page.from_native()` - Recursively converts nested Span and Block lists
   - `Document.from_native()` - Recursively converts Page list and Metadata
   - `Match.from_native()` - Converts bbox list to tuple, handles optional context
   - `Fingerprint.from_native()` - Handles optional Metadata with conditional conversion
   - `Classification.from_native()` - Converts tags list to tuple, handles optional heuristics

### Design Decisions

- **Nested structures**: All nested custom types (Page→Span/Block, Document→Page/Metadata, Table→Row→Cell) are recursively converted using `from_native()` rather than passing raw dicts
- **Tuples vs Lists**: Used `tuple()` comprehensions for list-to-tuple conversions on fields like `spans` and `blocks` in Page, `tags` in Classification
- **Type casting**: Added explicit `int()`, `float()`, `bool()` casts for numeric/boolean fields to ensure type correctness from potentially untyped dict values
- **Optional fields**: Used `.get()` with appropriate defaults for all optional fields
- **Optional nested objects**: For `Fingerprint.metadata`, check if the key exists and is not None before calling `Metadata.from_native()`

## Verification

### Compilation
```bash
python3 -m py_compile crates/pdftract-py/python/pdftract/types.py
# Result: PASS - no syntax errors
```

### Code Review Checklist
- ✅ All 11 classes have `from_native` classmethods
- ✅ Methods accept a single `native_dict: dict` argument
- ✅ Methods return an instance of the class (`Self`)
- ✅ Nested structures are recursively converted (Document→Page→Span/Block, Table→Row→Cell)
- ✅ Optional fields handled with `.get()` or default values
- ✅ Code compiles without errors

### Testing Notes
No existing tests were found for `from_native` methods. The implementation follows standard patterns for PyO3 native dict conversion. Full integration testing will be covered when the PyO3 bridge layer is implemented in parent bead bf-5b55jv.

## Acceptance Criteria Status

| Criteria | Status | Notes |
|----------|--------|-------|
| All 8 classes have `from_native` classmethods | PASS | All 11 classes (actual count) have methods |
| Methods accept single `native_dict: dict` argument | PASS | All methods follow this signature |
| Methods return instance of class (`Self`) | PASS | All methods return `cls(...)` |
| Nested structures recursively converted | PASS | Document, Page, Table, Row, Cell all handle nesting |
| Optional fields handled with `.get()` | PASS | All optional fields use `.get()` with defaults |
| Code compiles without errors | PASS | `python3 -m py_compile` successful |

## Commit
- File: `crates/pdftract-py/python/pdftract/types.py`
- Lines added: ~110 (11 classmethod implementations + import)
