<<<<<<< HEAD
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
=======
# SDK Type Structure Exploration

## Overview
The pdftract Python SDK is located at `/home/coding/pdftract/crates/pdftract-py/python/pdftract/` and provides a well-structured, type-safe API using frozen dataclasses.

## Module Structure

### Main Module (`__init__.py`)
- **Location**: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/__init__.py`
- **Purpose**: Public API entry point, exports all types and functions
- **Architecture**: Wraps native PyO3 bindings (`_native` module) with typed Python objects

### Supporting Modules
1. **`types.py`**: All dataclass type definitions
2. **`exceptions.py`**: Exception hierarchy (8 exception types)
3. **`asyncio.py`**: Async wrappers using `asyncio.to_thread`
4. **`fallback.py`**: Subprocess fallback when native module unavailable
5. **`_native.abi3.so`**: Compiled PyO3 Rust bindings

## Exported Types (9 total)

All types are **frozen dataclasses** with `@dataclass(frozen=True, slots=True)`:

### Core Document Types
1. **`Document`**: Complete PDF extraction result
   - Attributes: `pages: List[Page]`, `metadata: Optional[Metadata]`, `schema_version: Optional[str]`
   - Factory method: `from_native(native_dict: dict) -> Self`

2. **`Page`**: Single page with spans and blocks
   - Attributes: `page: int`, `width: int`, `height: int`, `rotation: int`, `spans: List[Span]`, `blocks: List[Block]`
   - Factory method: `from_native(native_dict: dict) -> Self`

### Content Types
3. **`Span`**: Text span with font and position
   - Attributes: `text: str`, `bbox: Tuple[float, float, float, float]`, `font: str`, `size: float`, `confidence: Optional[float]`

4. **`Block`**: Semantic block (text, heading, list, table, figure)
   - Attributes: `kind: str`, `text: str`, `bbox: Tuple[float, float, float, float]`, `level: Optional[int]`

### Table Types (Internal, not in `__all__`)
5. **`Cell`**: Table cell
   - Attributes: `bbox`, `text`, `spans`, `row`, `col`, `rowspan`, `colspan`, `is_header_row`

6. **`Row`**: Table row
   - Attributes: `bbox`, `cells: List[Cell]`, `is_header: bool`

7. **`Table`**: Complete table
   - Attributes: `id`, `bbox`, `rows: List[Row]`, `header_rows`, `detection_method`, `continued`, `continued_from_prev`, `page_index`

### Utility Types
8. **`Metadata`**: Document metadata
   - Attributes: `page_count`, `title`, `author`, `subject`, `keywords`, `creator`, `producer`, `created`, `modified`

9. **`Match`**: Regex search result
   - Attributes: `text: str`, `page: int`, `bbox`, `context: Optional[Dict[str, str]]`

10. **`Fingerprint`**: PDF structural fingerprint
    - Attributes: `hash: str`, `fast_hash: str`, `page_count`, `metadata: Optional[Metadata]`
    - Factory method: `from_string(hash_string: str) -> Self`

11. **`Classification`**: Page classification result
    - Attributes: `category: str`, `confidence: float`, `tags: List[str]`, `heuristics: Optional[Dict[str, bool]]`
    - Property: `class_name` (backward compatibility alias)

## Exported Exceptions (9 total)

All inherit from base `PdftractError`:
1. `PdftractError` - Base exception
2. `CorruptPdfError` - PDF file is corrupted
3. `EncryptionError` - PDF encryption issues
4. `SourceUnreachableError` - File/URL access issues
5. `RemoteFetchInterruptedError` - Network interruptions
6. `TlsError` - TLS/SSL certificate validation failures
7. `ReceiptVerifyError` - Receipt verification failures
8. `UnsupportedOperationError` - Binary version incompatibility

## Exported Functions (9 total)

Main extraction and utility functions:
- `extract(source, **options) -> Document`
- `extract_text(source, **options) -> str`
- `extract_markdown(source, **options) -> str`
- `extract_stream(source, **options) -> Iterator[Page]`
- `search(source, pattern, **options) -> Iterator[Match]`
- `get_metadata(source, **options) -> Metadata`
- `hash(source, **options) -> Fingerprint`
- `classify(source) -> Classification`
- `verify_receipt(path, receipt) -> bool`

## Async Module (`pdftract.asyncio`)

Re-exports async versions of all main functions:
- `AsyncExtractor` class (wraps sync extractor)
- `AsyncPageIterator` (async iterator for streaming)
- `AsyncMatchIterator` (async iterator for search)
- All async functions matching sync API

## User Import Pattern

Users should use **simple module import** (not sub-module imports):
>>>>>>> 02460305c17f3437a75a2750b71fd2760492b3de

```python
import pdftract

<<<<<<< HEAD
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
=======
# Access types
doc = pdftract.extract("file.pdf")
assert isinstance(doc, pdftract.Document)

# Access exceptions
try:
    pdftract.extract("file.pdf")
except pdftract.CorruptPdfError as e:
    # Handle corruption
    pass

# Async operations
async_doc = await pdftract.asyncio.extract("file.pdf")
```

**NOT**:
```python
from pdftract.types import Document  # ❌ Not the public API pattern
from pdftract.exceptions import CorruptPdfError  # ❌ Not the public API pattern
```

## Type Safety Characteristics

1. **All types are frozen dataclasses**: Immutable, hashable, memory-efficient
2. **Factory pattern**: Each type has `from_native(dict)` classmethod for conversion from Rust
3. **Not dicts**: SDK returns typed objects, not raw dictionaries (verified by existing tests)
4. **Proper annotations**: All attributes have type hints for IDE autocomplete
5. **`__repr__` methods**: Custom string representations for debugging

## Verification

Existing test files confirm the structure:
- `/home/coding/pdftract/test_sdk_types_smoke.py` - Comprehensive type verification
- `/home/coding/pdftract/tests/test_types.py` - IDE autocomplete verification

Both confirm that:
- Methods return typed objects (instances of dataclasses)
- Attribute access works correctly
- Types are properly exported from main module
- IDE autocomplete suggests correct attributes

## File Locations Summary

- **SDK root**: `/home/coding/pdftract/crates/pdftract-py/python/pdftract/`
- **Types**: `types.py` (9 exported types + Cell/Row/Table for internal use)
- **Exceptions**: `exceptions.py` (8 exception types)
- **Main API**: `__init__.py` (exports all types and functions)
- **Async**: `asyncio.py` (async wrappers)
- **Native bindings**: `_native.abi3.so` (compiled Rust)
>>>>>>> 02460305c17f3437a75a2750b71fd2760492b3de
