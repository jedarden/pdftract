# SDK Type Exports and Structure Exploration

**Bead:** bf-49vvzm  
**Date:** 2026-08-06  
**Status:** COMPLETE

## Overview

Explored the Python SDK type system to understand exported types, their definitions, and user import patterns. This ensures smoke tests target the correct types and imports.

## SDK Module Structure

The SDK is located at `/home/coding/pdftract/crates/pdftract-py/python/pdftract/` with the following modules:

| Module | Purpose |
|--------|---------|
| `__init__.py` | Main API entry point with public exports |
| `types.py` | All type definitions (frozen dataclasses) |
| `exceptions.py` | Exception hierarchy |
| `asyncio.py` | Async wrappers for long-running methods |
| `fallback.py` | Subprocess fallback when native module unavailable |
| `_native.abi3.so` | Compiled Rust native module (PyO3 bindings) |

## Exported Types (Public API)

The main `__init__.py` exports these types via `__all__`:

### Core Data Types (from `pdftract.types`)
- **Document** - Complete PDF extraction result with pages, metadata
- **Page** - Single page with spans, blocks, dimensions
- **Span** - Text span with font, position, confidence
- **Block** - Semantic block (text/heading/list/table/figure)
- **Match** - Regex match result from search
- **Fingerprint** - PDF structural fingerprint for identity
- **Classification** - Page classification result
- **Metadata** - Document metadata (title, author, page count, etc.)

### Exception Types (from `pdftract.exceptions`)
- **PdftractError** - Base exception
- **CorruptPdfError** - Malformed PDF
- **EncryptionError** - PDF encrypted with wrong/missing password
- **SourceUnreachableError** - File or URL inaccessible
- **RemoteFetchInterruptedError** - Network timeout/interruption
- **TlsError** - TLS certificate validation failure
- **ReceiptVerifyError** - Receipt verification failed
- **UnsupportedOperationError** - Method not supported by binary version

### Functions
- `extract()` - Full extraction returning Document
- `extract_text()` - Plain text extraction
- `extract_markdown()` - Markdown extraction
- `extract_stream()` - Streaming page iterator
- `search()` - Regex search returning Match iterator
- `get_metadata()` - Metadata-only extraction (cheap)
- `hash()` - Fingerprint computation
- `classify()` - Page classification
- `verify_receipt()` - Receipt verification
- `asyncio` - Async wrappers module

## Internal Types (NOT in `__all__`)

These types in `types.py` are **not exported** in the public API:
- **Cell** - Table cell (used within Table/Row)
- **Row** - Table row (used within Table)  
- **Table** - Table extraction result (nested within Page/Block)

These are implementation details of the table structure and users access them through the Page.blocks hierarchy.

## Type Implementation Details

All types are implemented as **frozen dataclasses with slots**:

```python
@dataclass(frozen=True, slots=True)
class Document:
    pages: List[Page]
    schema_version: Optional[str] = None
    metadata: Optional[Metadata] = None
    
    @classmethod
    def from_native(cls, native_dict: dict) -> Self:
        # Convert Rust dict to typed object
        
    def __repr__(self) -> str:
        # Custom repr for debugging
```

Key characteristics:
- **Immutable** - `frozen=True` prevents modification after creation
- **Memory efficient** - `slots=True` reduces memory overhead
- **Type-safe** - Constructor enforces field types
- **Native conversion** - `from_native()` converts raw dicts from Rust
- **IDE friendly** - Custom `__repr__()` for debugging

## User Import Pattern

Users should **import from the main `pdftract` module**, NOT from submodules:

```python
# ✅ CORRECT - import from main module
import pdftract
from pdftract import Document, Page, Span

# ❌ INCORRECT - importing from submodules
from pdftract.types import Document  # Works but not recommended
from pdftract.exceptions import PdftractError  # Breaks if API changes
```

The main `__init__.py` re-exports all public types and functions, providing a stable import path.

## Native Module Integration

The SDK wraps a Rust native module (`_native.abi3.so`) via PyO3 bindings:

1. **Import fallback**: If native import fails, warns and uses subprocess fallback
2. **Type wrapping**: Functions check if native returns dicts, wrap in typed objects
3. **Streaming support**: Iterators yield typed objects by wrapping each item

Example from `extract()`:
```python
def extract(source, **options) -> Document:
    extractor = _get_extractor()
    result = extractor.extract(source, **options)
    # Wrap raw dict from native module in typed Document
    if isinstance(result, dict):
        return Document.from_native(result)
    return result
```

## Verification: Existing Test Coverage

The file `crates/pdftract-py/tests/test_types.py` already contains a smoke test that validates the type contract:

- `test_extract_returns_typed_document()` - Verifies Document/Page/Span hierarchy
- `test_extract_returns_typed_document_with_valid_minimal()` - Redundant check with different fixture

This confirms the SDK is correctly returning typed objects rather than raw dicts.

## Recommendations for Smoke Test Enhancement

Based on this exploration, the smoke test should:

1. ✅ **Already covered** - Test `extract()` returns Document instance
2. ✅ **Already covered** - Test Page/Span object hierarchy  
3. **Consider adding** - Test other extraction methods (`extract_text()`, `search()`)
4. **Consider adding** - Test exception types are raised correctly
5. **Consider adding** - Test Metadata.from_native() conversion

## File Locations Summary

| Component | Location |
|-----------|----------|
| Main API | `/home/coding/pdftract/crates/pdftract-py/python/pdftract/__init__.py` |
| Type definitions | `/home/coding/pdftract/crates/pdftract-py/python/pdftract/types.py` |
| Exception hierarchy | `/home/coding/pdftract/crates/pdftract-py/python/pdftract/exceptions.py` |
| Async wrappers | `/home/coding/pdftract/crates/pdftract-py/python/pdftract/asyncio.py` |
| Native module | `/home/coding/pdftract/crates/pdftract-py/python/pdftract/_native.abi3.so` |
| Existing tests | `/home/coding/pdftract/crates/pdftract-py/tests/test_types.py` |

## Conclusion

The SDK type system is well-structured with:
- ✅ Clear public API via `__all__` exports
- ✅ Immutable, memory-efficient dataclass types
- ✅ Proper type conversion from native Rust layer
- ✅ User-friendly import pattern from main module
- ✅ Existing smoke test coverage for core types

All acceptance criteria met:
- ✅ List of all exported types documented above
- ✅ File locations for each type identified
- ✅ Import pattern documented (use `from pdftract import ...`)
- ✅ Types are proper classes (frozen dataclasses), not dicts
