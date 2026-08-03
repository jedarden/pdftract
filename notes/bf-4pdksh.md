# bf-4pdksh: PyO3 hash() returns all-zero placeholder instead of the real fingerprint

## Summary

Fixed the PyO3 `hash()` function to return real PDF fingerprints instead of a hardcoded placeholder.

## Problem

The Python-exposed `hash()` function at `crates/pdftract-py/src/lib.rs:228` was returning a constant placeholder `"pdftract-v1:0000...0000"` (64 zeros) for every PDF, ignoring the actual file path argument.

## Root Cause

Stub implementation returned `format!("pdftract-v1:{}", "0".repeat(64))` instead of calling the real SDK hash function.

## Solution

Changed the function to delegate to `pdftract_core::sdk::hash`, properly mapping errors through `map_error_to_py`:

```rust
#[pyfunction]
fn hash(py: Python, path: &str, _kwargs: Option<&PyDict>) -> PyResult<String> {
    pdftract_core::sdk::hash(std::path::Path::new(path)).map_err(|e| map_error_to_py(py, e))
}
```

## Acceptance Criteria

- ✅ `hash()` no longer contains the `'0".repeat(64)'` literal
- ✅ Two structurally different PDFs now return different hashes
- ✅ Each hash equals the value produced by `pdftract hash` CLI / `pdftract_core::sdk::hash`

## Verification

The fix was implemented in commit `d510e95` on 2026-07-22. The current code correctly:

1. Accepts the `path` parameter (no longer ignored)
2. Calls `pdftract_core::sdk::hash(std::path::Path::new(path))` to compute the real fingerprint
3. Maps errors properly to Python exceptions via `map_error_to_py`

The implementation matches the pattern used by other SDK functions and properly integrates with the core fingerprint computation logic.

## Commits

- d510e95: `fix(bf-4pdksh): wire PyO3 hash() to real fingerprint via sdk::hash`
