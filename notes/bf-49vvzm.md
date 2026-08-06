# SDK Type Exports and Structure Exploration (bf-49vvzm)

## Summary

Explored the pdftract Python SDK structure to understand type exports, definitions, and import patterns for smoke test development.

## SDK Location

- **Primary path**: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/`
- **Module import**: `import pdftract` (after adding `crates/pdftract-py/python` to `sys.path`)

## Module Structure

```
crates/pdftract-py/python/pdftract/
├── __init__.py          # Main module exports and wrapper functions
├── types.py             # Dataclass type definitions (11 types)
├── exceptions.py        # Exception hierarchy (8 exception classes)
├── asyncio.py          # Async wrapper functions
├── fallback.py          # Subprocess fallback for when native module unavailable
└── _native.abi3.so      # Compiled Rust module (PyO3 bindings)
```

## Exported Types

All types are **frozen dataclasses** with `@classmethod from_native()` methods to convert from Rust dictionaries:

### Core Document Types
1. **Document** - Complete PDF extraction result
   - `pages: List[Page]`
   - `schema_version: Optional[str]`
   - `metadata: Optional[Metadata]`

2. **Page** - Single page from a PDF
   - `page: int` (1-based page number)
   - `width: int` (points)
   - `height: int` (points)
   - `rotation: int` (degrees: 0, 90, 180, 270)
   - `spans: List[Span]`
   - `blocks: List[Block]`

3. **Span** - Text span with font and position
   - `text: str`
   - `bbox: Tuple[float, float, float, float]` (x0, y0, x1, y1)
   - `font: str`
   - `size: float` (points)
   - `confidence: Optional[float]` (0.0-1.0 for OCR)

### Content Block Types
4. **Block** - Semantic block (text, heading, list, table, figure)
   - `kind: str` (block type)
   - `text: str`
   - `bbox: Tuple[float, float, float, float]`
   - `level: Optional[int]` (heading level 1-6)

### Table Types (not in `__all__` but available)
5. **Cell** - Table cell
   - `bbox: Tuple[float, float, float, float]`
   - `text: str`
   - `spans: List[int]`
   - `row: int`, `col: int`
   - `rowspan: int`, `colspan: int`
   - `is_header_row: bool`

6. **Row** - Table row
   - `bbox: Tuple[float, float, float, float]`
   - `cells: List[Cell]`
   - `is_header: bool`

7. **Table** - Complete table
   - `id: str`
   - `bbox: Tuple[float, float, float, float]`
   - `rows: List[Row]`
   - `header_rows: int`
   - `detection_method: str`
   - `continued: bool`
   - `continued_from_prev: bool`
   - `page_index: int`

### Search Result Types
8. **Match** - Regex search match
   - `text: str`
   - `page: int`
   - `bbox: Tuple[float, float, float, float]`
   - `context: Optional[Dict[str, str]]`

### Metadata Types
9. **Metadata** - Document metadata
   - `page_count: int`
   - `title: Optional[str]`
   - `author: Optional[str]`
   - `subject: Optional[str]`
   - `keywords: Optional[List[str]]`
   - `creator: Optional[str]`
   - `producer: Optional[str]`
   - `created: Optional[str]` (ISO 8601)
   - `modified: Optional[str]` (ISO 8601)

10. **Fingerprint** - PDF structural fingerprint
    - `hash: str` (SHA-256 hex)
    - `fast_hash: str` (BLAKE3 hex)
    - `page_count: int`
    - `metadata: Optional[Metadata]`

11. **Classification** - Page classification result
    - `category: str`
    - `confidence: float` (0.0-1.0)
    - `tags: List[str]`
    - `heuristics: Optional[Dict[str, bool]]`

## Exported Exceptions

All inherit from `PdftractError`:

1. **PdftractError** - Base exception
2. **CorruptPdfError** - Malformed PDF
3. **EncryptionError** - Missing/wrong password
4. **SourceUnreachableError** - File/URL not accessible
5. **RemoteFetchInterruptedError** - Network timeout/connection drop
6. **TlsError** - TLS certificate validation failure
7. **ReceiptVerifyError** - Receipt verification failed
8. **UnsupportedOperationError** - Method not supported by binary version

## Exported Functions

All functions are defined in `__init__.py` as wrappers around the native module:

1. **extract(source, **options) -> Document** - Full document extraction
2. **extract_text(source, **options) -> str** - Plain text extraction
3. **extract_markdown(source, **options) -> str** - Markdown extraction
4. **extract_stream(source, **options) -> Iterator[Page]** - Streaming page extraction
5. **search(source, pattern, **options) -> Iterator[Match]** - Regex search
6. **get_metadata(source, **options) -> Metadata** - Metadata only (cheap)
7. **hash(source, **options) -> Fingerprint** - Structural fingerprint
8. **classify(source) -> Classification** - Page classification
9. **verify_receipt(path, receipt) -> bool** - Receipt verification

## Async API

Available via `pdftract.asyncio` module with the same functions (async versions):

```python
import pdftract.asyncio as asyncio
await asyncio.extract("file.pdf")
```

## User Import Pattern

Users import and use types as follows:

```python
import pdftract

# Main extraction
doc = pdftract.extract("file.pdf")

# Access typed objects
for page in doc.pages:
    for span in page.spans:
        print(span.text)

# Access metadata
metadata = pdftract.get_metadata("file.pdf")
print(metadata.page_count)
```

## Type Implementation Details

- **All types are frozen dataclasses** with `slots=True` for performance
- **Each type has `@classmethod from_native(dict)`** to convert from Rust dictionaries
- **Wrapper functions handle conversion** from native dicts to typed objects
- **Fallback to subprocess** when native module unavailable

## Acceptance Criteria Verification

### ✅ List of all exported types from the SDK module
- **Document, Page, Span, Block, Match, Fingerprint, Classification, Metadata** (8 in `__all__`)
- **Cell, Row, Table** (available in types.py but not exported)

### ✅ File locations where each type is defined
- **All types**: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/types.py`
- **Main exports**: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/__init__.py`

### ✅ Documented import pattern for users
- **Pattern**: `import pdftract` → access types as `pdftract.Document`
- **NOT**: `from pdftract.types import Document` (internal implementation detail)

### ✅ Confirmation that types are classes or type aliases
- **All are frozen dataclasses** with `@dataclass(frozen=True, slots=True)`
- **Have `from_native()` classmethods** for conversion from Rust
- **Have `__repr__()` methods** for debugging
- **NOT just dicts** - proper typed objects

## References

- Main module: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/__init__.py`
- Type definitions: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/types.py`
- Exceptions: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/exceptions.py`
- Async wrappers: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/asyncio.py`
- Test file: `/home/coding/pdftract/test_sdk_types_smoke.py`
- Parent bead: bf-3mon01
- Depends on: bf-5b55jv child 4 (SDK methods return typed objects)

## Conclusion

The SDK is well-structured with:
- ✅ Clear type exports (11 types defined, 8 in `__all__`)
- ✅ Proper dataclass definitions with conversion methods
- ✅ Comprehensive exception hierarchy (8 exception types)
- ✅ Consistent import pattern (`import pdftract`)
- ✅ Both sync and async APIs
- ✅ Existing smoke test covering all main types

The smoke test `test_sdk_types_smoke.py` correctly targets the SDK's typed exports and will properly validate that methods return typed objects rather than raw dictionaries.
