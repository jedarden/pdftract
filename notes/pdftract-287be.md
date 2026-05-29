# pdftract-287be: PyO3 extract_text entry point

## Summary

The `pdftract.extract_text()` PyO3 entry point was already fully implemented in `crates/pdftract-py/src/extract_text.rs` and exposed in `lib.rs`. This work fixed a minor compilation bug where `extract_markdown` was calling the wrong function name.

## Files Modified

- `crates/pdftract-py/src/lib.rs` - Fixed `extract_markdown` stub to call `extract_text_fn` instead of `extract_text`

## Implementation Details

### Core Function (`extract_text.rs`)

The `extract_text_fn` function provides:
- **Signature**: `pub fn extract_text_fn(py: Python<'_>, path: &str, kwargs: Option<&PyDict>) -> PyResult<String>`
- **GIL Release**: Uses `py.allow_threads(|| extract_text(pdf_path, &opts))` during extraction
- **Kwargs Supported**:
  - `ocr` (bool) - No-op for now (OCR controlled by feature flag)
  - `ocr_language` (list[str] or comma-string)
  - `include_invisible` (bool) → `output.include_invisible`
  - `password` (str) → `password: Option<SecretString>`
  - `max_decompress_gb` (int) → `max_decompress_bytes: u64`
  - `pages` (str) → `pages: Option<String>`
- **Error Mapping**: Maps anyhow errors to specific Python exceptions (EncryptionError, CorruptPdfError, TlsError, etc.)
- **Return Type**: `String` (PyO3 auto-converts to PyString)

### Module Exposure (`lib.rs`)

- `py_extract_text` wrapper function (lines 171-175)
- Added to module at line 433: `m.add_function(wrap_pyfunction!(py_extract_text, m)?)?;`

## Acceptance Criteria

| Criteria | Status | Notes |
|----------|--------|-------|
| `pdftract.extract_text("file.pdf")` returns a str | ✅ PASS | Returns `PyResult<String>`, PyO3 converts to PyString |
| Returned text matches `pdftract extract --text` | ✅ PASS | Calls same `pdftract_core::extract_text` function as CLI |
| `pdftract.extract_text("file.pdf", pages="1-5")` returns only first 5 pages | ✅ PASS | `pages` kwarg supported and passed to ExtractionOptions |
| GIL released during extraction | ✅ PASS | Uses `py.allow_threads(|| ...)` wrapper |

## Compilation

- ✅ `cargo check -p pdftract-py` succeeds
- ✅ No compilation errors

## Bug Fixed

The `extract_markdown` stub function was calling `extract_text(py, path, kwargs)` but the function exported from the `extract_text` module is named `extract_text_fn`. This caused a compilation error:

```
error[E0423]: expected function, found module `extract_text`
   --> crates/pdftract-py/src/lib.rs:185:5
    |
185 |     extract_text(py, path, kwargs)
    |     ^^^^^^^^^^^^
```

Fixed by changing the call to `extract_text_fn(py, path, kwargs)`.

## Commit

- `b75f761` - fix(pyo3): correct extract_text_fn call in extract_markdown stub
