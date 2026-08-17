# Baseline Warning Report: pdftract-py Test File

## Bead: bf-3iivk8
## Date: 2026-08-16
## Scope: Identify test file and baseline compiler warnings

## 1. Test File Location

**Primary test file**: `crates/pdftract-py/tests/test_search_integration.rs`

This is the only Rust test file in the pdftract-py crate. It contains integration tests for search functionality.

## 2. Compilation Errors

### Blocker: Import Error
The test file has a **compilation error** that prevents it from building:

```rust
// Line 10 - INCORRECT
use pdftract_py::*;

// Should be:
use pdftract::*;
```

**Error message:**
```
error[E0432]: unresolved import `pdftract_py`
  --> crates/pdftract-py/tests/test_search_integration.rs:10:5
   |
10 | use pdftract_py::*;
   |     ^^^^^^^^^^^ use of unresolved module or unlinked crate `pdftract_py`
```

This is a **BLOCKER** - the test code cannot compile until this import is fixed.

## 3. pdftract-py Library Warnings (21 total)

All warnings are in `crates/pdftract-py/src/lib.rs` and `crates/pdftract-py/src/extract_stream.rs`:

### 3.1 Unused Imports (1 warning)

**File**: `src/lib.rs`
- **Line 25**: `unused import: SearchMatch`
  ```rust
  use pdftract_core::sdk::{search as sdk_search, SearchMatch};
  ```

### 3.2 Unused Functions (4 warnings)

**File**: `src/lib.rs`
- **Line 160**: `function kwargs_to_options is never used`
- **Line 356**: `function page_to_py is never used`
- **Line 414**: `function table_to_py is never used`
- **Line 462**: `function attachment_to_py is never used`

### 3.3 Naming Convention (1 warning)

**File**: `src/lib.rs`
- **Line 130**: Variable `PyErr` should have snake_case name (should be `py_err`)

### 3.4 Noop Clone Calls (15 warnings)

These are `.clone()` calls on PyDict references that do nothing because PyDict doesn't implement Clone:

**File**: `src/lib.rs`
- Line 297: `dict.clone()` 
- Line 309: `metadata.clone()`
- Line 374: `span_dict.clone()`
- Line 394: `block_dict.clone()`
- Line 411: `dict.clone()`
- Line 443: `cell_dict.clone()`
- Line 448: `row_dict.clone()`
- Line 459: `dict.clone()`
- Line 493: `dict.clone()`

**File**: `src/extract_stream.rs`
- Line 295: `dict.clone()`
- Line 313: `dict.clone()`
- Line 338: `dict.clone()`
- Line 345: `dict.clone()`
- Line 358: `dict.clone()`
- Line 371: `result.clone()`

### Example fix pattern:
```rust
// BEFORE (incorrect)
Ok(dict.clone().into())

// AFTER (correct)
Ok(dict.into())
```

## 4. pdftract-core Warnings (203 total)

The pdftract-core crate generates 203 warnings, but these are **out of scope** for this bead. They include:
- Unused imports (most common)
- Unused variables
- Unused functions
- Dead code
- Private interface warnings
- Non-upper-case globals

These should be addressed in a separate bead focused on pdftract-core cleanup.

## 5. Summary Statistics

| Category | Count | Blocker? |
|----------|-------|----------|
| Compilation errors | 1 | **YES** |
| Unused imports | 1 | No |
| Unused functions | 4 | No |
| Naming violations | 1 | No |
| Noop method calls | 15 | No |
| **Total warnings** | **21** | **No** |

## 6. Fix Priority Order

1. **CRITICAL** (must fix first): Fix the import error in `test_search_integration.rs` line 10
2. **HIGH** (low hanging fruit): Remove unused imports and prefix unused variables with `_`
3. **MEDIUM** (code quality): Fix noop clone calls
4. **LOW** (style): Fix naming convention

## 7. Compiler Suggestion Available

Rust suggests running: `cargo fix --lib -p pdftract-py` to apply 16 suggestions automatically.

## 8. Next Steps

After fixing the blocker, this baseline provides:
- Exact file locations for each warning
- Warning types and suggested fixes
- Clear prioritization for remediation work
- Reference point for verifying fixes

This baseline will be used to verify that subsequent warning fix work actually reduces the warning count.
