# Import Inventory: Integration Test vs pdftract-py

**Generated:** 2026-08-08  
**Purpose:** Baseline inventory of current imports in `tests/integration_test.rs` compared to available exports from `crates/pdftract-py/src/lib.rs`

---

## Current Imports in `tests/integration_test.rs`

### Standard Library
```rust
use std::path::PathBuf;
```

### From pdftract crate (lib name in Cargo.toml)
```rust
use pdftract::PyPdfProcessor;
```

### From pdftract_core
```rust
use pdftract_core::{ExtractionOptions, PageResult, TableJson};
```

### Exception Types (from pdftract)
```rust
use pdftract::{
    CorruptPdfError,
    EncryptionError,
    PdftractError,
    ReceiptVerifyError,
    RemoteFetchInterruptedError,
    SourceUnreachableError,
    TlsError,
    UnsupportedOperationError,
};
```

### Commented-Out Imports
```rust
// Uncommented PyO3 imports for Python bindings testing
// use pyo3::{Python, PyResult, types::PyDict};
```

### Local Modules
```rust
mod test_helpers;
mod test_cases;
```

---

## Public Exports from `crates/pdftract-py/src/lib.rs`

### Core Types (re-exported from pdftract_core)
- `AttachmentJson` - NOT currently imported
- `ExtractionOptions` - ✅ **imported**
- `PageResult` - ✅ **imported**
- `TableJson` - ✅ **imported**

### Main Struct
- `PyPdfProcessor` - ✅ **imported**

### Exception Hierarchy (8 classes)
All created via `pyo3::create_exception!`:
- `PdftractError` - ✅ **imported**
- `EncryptionError` - ✅ **imported**
- `CorruptPdfError` - ✅ **imported**
- `SourceUnreachableError` - ✅ **imported**
- `RemoteFetchInterruptedError` - ✅ **imported**
- `TlsError` - ✅ **imported**
- `ReceiptVerifyError` - ✅ **imported**
- `UnsupportedOperationError` - ✅ **imported**

### SDK Functions/Types
- `SearchMatch` (from `pdftract_core::sdk::search`) - NOT currently imported
- `search` function - NOT currently imported (available as `pdftract::search` via module)
- `hash` function - NOT currently imported
- `classify` function - NOT currently imported
- `verify_receipt` function - NOT currently imported

### PyO3-Bound Functions (module exports via `#[pymodule]`)
- `extract` - NOT currently imported
- `extract_text` - NOT currently imported
- `extract_markdown` - NOT currently imported
- `extract_stream` - NOT currently imported
- `get_metadata` - NOT currently imported
- `search` (duplicate, see above) - NOT currently imported
- `hash` (duplicate, see above) - NOT currently imported
- `classify` (duplicate, see above) - NOT currently imported
- `verify_receipt` (duplicate, see above) - NOT currently imported

### Helper Types
- `StreamIterator` (from extract_stream module) - NOT currently imported

### Internal Types (not public, for reference only)
- `PyResultAny<'py>` - private type alias
- `map_error_to_py` - private function
- `kwargs_to_options` - private function
- `page_to_py` - private function
- `table_to_py` - private function
- `attachment_to_py` - private function
- `get_hint_for_code` - private function

---

## Comparison Matrix

| Category | Item | Status in integration_test.rs | Source in pdftract-py |
|----------|------|-------------------------------|----------------------|
| **Core Types** | | | |
| | `AttachmentJson` | ❌ NOT imported | `use pdftract_core::AttachmentJson` (line 24) |
| | `ExtractionOptions` | ✅ **imported** | `use pdftract_core::ExtractionOptions` (line 24) |
| | `PageResult` | ✅ **imported** | `use pdftract_core::PageResult` (line 24) |
| | `TableJson` | ✅ **imported** | `use pdftract_core::TableJson` (line 24) |
| **Main Struct** | | | |
| | `PyPdfProcessor` | ✅ **imported** | `pub struct PyPdfProcessor` (line 36) |
| **Exceptions** | | | |
| | `PdftractError` | ✅ **imported** | `create_exception!` (line 58) |
| | `EncryptionError` | ✅ **imported** | `create_exception!` (line 59) |
| | `CorruptPdfError` | ✅ **imported** | `create_exception!` (line 60) |
| | `SourceUnreachableError` | ✅ **imported** | `create_exception!` (line 61) |
| | `RemoteFetchInterruptedError` | ✅ **imported** | `create_exception!` (line 62) |
| | `TlsError` | ✅ **imported** | `create_exception!` (line 63) |
| | `ReceiptVerifyError` | ✅ **imported** | `create_exception!` (line 64) |
| | `UnsupportedOperationError` | ✅ **imported** | `create_exception!` (line 65) |
| **SDK Types** | | | |
| | `SearchMatch` | ❌ NOT imported | `use pdftract_core::sdk::SearchMatch` (line 25) |
| **PyO3 Functions** | | | |
| | `extract` | ❌ NOT imported | `wrap_pyfunction!(extract::extract)` (line 526) |
| | `extract_text` | ❌ NOT imported | `wrap_pyfunction!(py_extract_text)` (line 527) |
| | `extract_markdown` | ❌ NOT imported | `wrap_pyfunction!(py_extract_markdown)` (line 528) |
| | `extract_stream` | ❌ NOT imported | `wrap_pyfunction!(extract_stream_fn)` (line 522) |
| | `get_metadata` | ❌ NOT imported | `wrap_pyfunction!(get_metadata)` (line 530) |
| | `search` | ❌ NOT imported | `wrap_pyfunction!(search)` (line 529) |
| | `hash` | ❌ NOT imported | `wrap_pyfunction!(hash)` (line 531) |
| | `classify` | ❌ NOT imported | `wrap_pyfunction!(classify)` (line 532) |
| | `verify_receipt` | ❌ NOT imported | `wrap_pyfunction!(verify_receipt)` (line 533) |
| **Helper Types** | | | |
| | `StreamIterator` | ❌ NOT imported | `add_class::<StreamIterator>` (line 523) |
| **Commented Out** | | | |
| | `pyo3::Python` | ⚠️ Commented out | Not from pdftract-py (PyO3 crate) |
| | `pyo3::PyResult` | ⚠️ Commented out | Not from pdftract-py (PyO3 crate) |
| | `pyo3::types::PyDict` | ⚠️ Commented out | Not from pdftract-py (PyO3 crate) |

---

## Summary Statistics

- **Total public exports from pdftract-py:** 18 items
- **Currently imported in integration_test.rs:** 11 items (61%)
- **NOT imported:** 7 items (39%)
- **Commented out (non-pdftract-py):** 3 items (PyO3 types)

### Imported Items Coverage by Category
- **Core Types:** 3/4 (75%) - missing `AttachmentJson`
- **Exceptions:** 8/8 (100%) - complete
- **Main Structs:** 1/1 (100%) - complete
- **SDK Types:** 0/1 (0%) - missing `SearchMatch`
- **PyO3 Functions:** 0/9 (0%) - all functions not imported
- **Helper Types:** 0/1 (0%) - missing `StreamIterator`

---

## Import Gap Analysis

### High Priority (likely needed for testing)
1. **`AttachmentJson`** - For testing attachment extraction functionality
2. **`SearchMatch`** - For testing search contract method
3. **`StreamIterator`** - For testing stream extraction

### Medium Priority (may be needed for specific test cases)
4. **PyO3 function imports** (`extract`, `extract_text`, `extract_markdown`, etc.) - Only needed if testing Python bindings directly via `pyo3` (currently commented out)

### Low Priority (probably not needed for Rust tests)
5. **Additional PyO3 types** (`Python`, `PyResult`, `PyDict`) - Already commented out, suggests they're not needed for current Rust-only tests

---

## Notes

1. **Lib Name vs Package Name:** The integration test correctly uses `pdftract` (the `[lib]` name from Cargo.toml), not the package name `pdftract-py`. This is documented in line 10 of integration_test.rs.

2. **Exception Hierarchy:** All 8 exception classes are already imported, covering the full hierarchy from `PdftractError` (base) to specific error types.

3. **PyO3 Imports:** The commented-out PyO3 imports suggest these are only needed when testing Python integration directly. For pure Rust integration tests, they may remain commented.

4. **Module Functions:** The PyO3-bound functions (`extract_text`, `search`, etc.) are exported via the `#[pymodule]` but are not currently imported in integration tests. This suggests either:
   - They're tested through Python tests, not Rust integration tests
   - They'll be imported in a future task

5. **Re-exports:** The pdftract-py lib.rs re-exports several types from pdftract_core (lines 24-25). The integration test currently imports `ExtractionOptions`, `PageResult`, and `TableJson` from pdftract_core directly. It could optionally import them from pdftract instead for consistency.

---

## Recommendations for Future Import Work

1. **Add `AttachmentJson` import** if testing attachment extraction
2. **Add `SearchMatch` import** when implementing search tests
3. **Add `StreamIterator` import** when implementing stream extraction tests
4. **Consider consolidating imports** - import core types from `pdftract` instead of `pdftract_core` for consistency
5. **Leave PyO3 imports commented** unless specifically testing Python bindings (which would require a Python interpreter)
