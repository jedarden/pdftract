# Import Analysis for tests/integration_test.rs

## Task: Identify all required imports for test file

**Bead ID:** bf-flo4nv
**Analysis Date:** 2026-08-07
**Target File:** `tests/integration_test.rs`
**Status:** ANALYSIS COMPLETE - NO CODE CHANGES MADE

---

## Current State (from bead bf-26gozh)

The file `tests/integration_test.rs` currently contains:
- Basic module structure (`mod test_helpers;`, `mod test_cases;`)
- Comments indicating TODO imports for PyPdfProcessor

---

## Required Imports Analysis

### 1. **Primary Required Imports** (from acceptance criteria)

#### `std::path::{Path, PathBuf}`
- **Status:** ✅ ALREADY PRESENT
- **Purpose:** Path manipulation for test fixtures
- **Current code:** `use std::path::{Path, PathBuf};`

#### `pdftract_py::PyPdfProcessor`
- **Status:** ⚠️ DOES NOT EXIST YET
- **Purpose:** Main Python-PDF integration processor (to be implemented)
- **Note:** Comment in file indicates: `// TODO: Add PyPdfProcessor import when struct is created in pdftract-py crate`
- **Planned import:** `use pdftract::PyPdfProcessor;`
- **Important:** The `pdftract-py` crate uses lib name `"pdftract"` in Cargo.toml (line 10), so imports use `pdftract::` not `pdftract_py::`

### 2. **Supporting Imports from pdftract_core**

Based on analysis of `crates/pdftract-py/src/lib.rs` and existing test patterns:

```rust
// Core extraction types (used in pdftract-py)
use pdftract_core::{ExtractionOptions, OutputOptions};

// Additional core types that may be needed for integration tests
use pdftract_core::{AttachmentJson, PageResult, TableJson};

// SDK search functionality
use pdftract_core::sdk::{search as sdk_search, SearchMatch};

// Diagnostics for error testing
use pdftract_core::diagnostics::DIAGNOSTIC_CATALOG;
```

### 3. **PyO3 Imports for Python Integration Testing**

```rust
// Core PyO3 types
use pyo3::{Python, PyResult, PyDict};

// For more advanced Python object testing
use pyo3::types::PyDict;
use pyo3::types::PyAny;
```

### 4. **Exception Type Imports** (for error handling tests)

```rust
use pdftract::{
    PdftractError,
    EncryptionError,
    CorruptPdfError,
    SourceUnreachableError,
    RemoteFetchInterruptedError,
    TlsError,
    ReceiptVerifyError,
    UnsupportedOperationError,
};
```

**Note:** These exception types are defined in `pdftract-py/src/lib.rs` via `pyo3::create_exception!` macro and exported at the module level.

### 5. **Testing Infrastructure Imports**

```rust
// Standard test utilities
use std::fs;
use std::io::Read;

// Test environment
#[cfg(test)]
use crate::test_helpers::Fixtures;
```

---

## Import Organization Recommendations

### Recommended Import Order (Rust conventions):

1. **Standard library imports** (`std::*`)
2. **External crate imports** (`pdftract_core`, `pyo3`, etc.)
3. **Local crate imports** (pdftract types from pdftract-py)
4. **Test-specific imports** (`#[cfg(test)]` guarded)

### Suggested Structure for tests/integration_test.rs:

```rust
// ============================================================================
// Standard library imports
// ============================================================================
use std::path::{Path, PathBuf};

// ============================================================================
// pdftract core imports
// ============================================================================
use pdftract_core::{ExtractionOptions, OutputOptions};
use pdftract_core::{AttachmentJson, PageResult, TableJson};
use pdftract_core::sdk::{search as sdk_search, SearchMatch};
use pdftract_core::diagnostics::DIAGNOSTIC_CATALOG;

// ============================================================================
// PyO3 imports for Python integration testing
// ============================================================================
use pyo3::{Python, PyResult, PyDict};
use pyo3::types::{PyAny, PyDict};

// ============================================================================
// Exception type imports for error handling tests
// ============================================================================
use pdftract::{
    PdftractError,
    EncryptionError,
    CorruptPdfError,
    SourceUnreachableError,
    RemoteFetchInterruptedError,
    TlsError,
    ReceiptVerifyError,
    UnsupportedOperationError,
};

// ============================================================================
// PyPdfProcessor (to be added when implemented)
// ============================================================================
// use pdftract::PyPdfProcessor;  // TODO: Add when struct is created

// ============================================================================
// Test helpers
// ============================================================================
mod test_helpers;
mod test_cases;
```

---

## Dependencies Between Imports

1. **PyPdfProcessor cannot be imported yet** - struct needs to be created in pdftract-py crate first
2. **Exception types depend on PyO3** - must be imported after PyO3 imports
3. **Test fixtures depend on PathBuf** - must have std::path imports first

---

## Verification Notes

### PASS Items:
- ✅ `std::path::{Path, PathBuf}` already present in file
- ✅ File structure exists from bead bf-26gozh
- ✅ All required imports identified and documented
- ✅ Analysis completed without code changes (as required)

### WARN Items:
- ⚠️ `pdftract::PyPdfProcessor` does not exist yet - needs implementation in pdftract-py crate
- ⚠️ Some imports are commented out in current file waiting for implementation

### FAIL Items:
- ❌ None - this is analysis only, no code changes required

---

## References

1. **Existing test file:** `/home/coding/pdftract/tests/integration_test.rs`
2. **Python bindings crate:** `/home/coding/pdftract/crates/pdftract-py/src/lib.rs`
3. **Core crate exports:** `/home/coding/pdftract/crates/pdftract-core/src/lib.rs`
4. **Similar test pattern:** `/home/coding/pdftract/crates/pdftract-py/tests/test_search_integration.rs`
5. **pdftract-py Cargo.toml:** Line 10 shows `name = "pdftract"` (not "pdftract_py")

---

## Next Steps

This analysis (bead bf-flo4nv) informs the next bead (bf-31dioa: "Add primary imports to test file") which will:
1. Add the primary imports identified above
2. Keep PyPdfProcessor commented with TODO note
3. Verify all imports resolve with `cargo check`

After that, bead bf-3wxjmb will add any additional supporting imports and verify compilation.

---

**Analysis completed successfully. All required imports identified and documented. No code changes made as per task requirements.**
