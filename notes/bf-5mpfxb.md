# Bead bf-5mpfxb: Create types.py with 7 SDK type dataclass definitions

## Status: PASS

Verification performed on 2026-08-06.

## Summary
The `types.py` file already existed at `crates/pdftract-py/python/pdftract/types.py` with all required type definitions present and properly implemented.

## Acceptance Criteria Verification

### PASS: File exists at correct path
- Location: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/types.py`
- File exists and is readable

### PASS: All 8 classes defined (7 SDK types + Metadata)
1. Document - Complete PDF document extraction result
2. Page - Individual page with spans, blocks, and tables
3. Span - Text span with font, size, bbox, and confidence
4. Block - Semantic block (text, heading, list, table, figure)
5. Match - Regex match result from search
6. Fingerprint - PDF structural fingerprint
7. Classification - Page classification result
8. Metadata - Document metadata (title, author, page_count, etc.)

Additional helper types also defined:
- Cell - Table cell
- Row - Table row
- Table - Extracted table

### PASS: All classes use @dataclass(frozen=True, slots=True)
Verified via Python inspection:
```
Document: dataclass=True, frozen=True, slots=True
Page: dataclass=True, frozen=True, slots=True
Span: dataclass=True, frozen=True, slots=True
Block: dataclass=True, frozen=True, slots=True
Match: dataclass=True, frozen=True, slots=True
Fingerprint: dataclass=True, frozen=True, slots=True
Classification: dataclass=True, frozen=True, slots=True
Metadata: dataclass=True, frozen=True, slots=True
```

### PASS: All classes have appropriate type annotations
All fields are properly typed using:
- `List[T]` for collections
- `Optional[T]` for nullable fields
- `int`, `str`, `float`, `bool` for primitives
- `set[int]` for the Classification.hybrid_cells field

### PASS: All classes have __repr__ methods
Verified via inspection - all 8 classes have custom `__repr__` implementations that provide useful debugging output.

### PASS: File compiles without syntax errors
```bash
python3 -c "from pdftract.types import Document, Page, Span, Block, Match, Fingerprint, Classification, Metadata"
# Import successful
```

## Notes
- The implementation includes additional helper types (Cell, Row, Table) beyond the minimum 7 required by the SDK contract
- All types include `from_native()` class methods for constructing from native layer dict representations
- Some types include additional convenience methods (e.g., Classification has a `class_name` property for backward compatibility)
- The bbox fields use `List[float]` rather than `Tuple[int, int, int, int]` as suggested in the task description, which is appropriate for PDF coordinates

## Conclusion
The bead is complete - all acceptance criteria are met. The types.py file provides a comprehensive, well-typed SDK interface with immutable dataclasses.
