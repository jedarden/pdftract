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
