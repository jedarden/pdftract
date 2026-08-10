# bf-32vbc6: Error Path Tests for classify_page

## Summary

Created comprehensive error path tests for `classify_page` function in `pdftract_core::sdk`.

## Implementation

### File Created
- `/home/coding/pdftract/crates/pdftract-core/tests/classify_page_error_paths.rs`

### Tests Implemented

All tests verify error paths return the correct `Result::Err` variant with appropriate diagnostic text:

1. **test_classify_page_error_invalid_pdf_missing_signature**
   - Tests PDF file missing `%PDF` signature
   - Verifies error message contains "Invalid PDF" or "missing PDF signature"
   - Status: ✅ PASS

2. **test_classify_page_error_empty_pdf**
   - Tests empty PDF file (zero bytes)
   - Verifies error message contains "empty", "0 bytes", or "no data"
   - Status: ✅ PASS

3. **test_classify_page_error_corrupted_pdf_header**
   - Tests PDF with wrong magic bytes (e.g., `%PNG` instead of `%PDF`)
   - Verifies error message mentions PDF signature issue
   - Status: ✅ PASS

4. **test_classify_page_error_nonexistent_pdf_file**
   - Tests when PDF file does not exist
   - Verifies error message contains "Failed to read PDF file" or file not found
   - Status: ✅ PASS

5. **test_classify_page_error_invalid_pdf_truncated**
   - Tests truncated PDF (only `%PDF-1.4` header, no content)
   - Verifies pdftract binary rejects invalid PDF structure
   - Status: ✅ PASS (correctly rejected)

6. **test_classify_page_error_pdftract_binary_not_found**
   - Tests when pdftract binary cannot be found
   - Uses `#[cfg_attr(not(feature = "error-path-tests"), ignore)]` to skip in normal dev
   - Documented expected error message format
   - Status: ⚠️ IGNORED (expected - binary exists in dev environment)

7. **test_classify_page_error_page_index_out_of_bounds_negative**
   - Documents that Rust's type system prevents negative `usize` indices
   - Compile-time safety check
   - Status: ✅ PASS

### Test Utilities

- Used `tempfile::NamedTempFile` for automatic cleanup of temporary test files
- Each test creates its own isolated temporary file that auto-deletes
- No manual cleanup required

## Acceptance Criteria Verification

- ✅ Error path tests exist for all failure modes
- ✅ Each test verifies the correct error is returned
- ✅ Error messages contain expected diagnostic text
- ✅ Tests properly mock failure conditions using temp files
- ✅ All tests pass (6 passed, 1 ignored)
- ✅ Module compiles without errors

## Actual Error Messages Verified

### Invalid PDF Signature
```
Invalid PDF: missing PDF signature (expected to start with '%PDF')
```

### Empty PDF
```
PDF input is empty
```

### Nonexistent File
```
Failed to read PDF file: /tmp/test-nonexistent-pdf-12345.pdf
```

### Binary Not Found
```
pdftract binary not found. Tried the following paths: [...]. Ensure pdftract is built (run 'cargo build --release') and available in PATH.
```

## Dependencies

- Uses existing `tempfile` crate (already in dev-dependencies)
- No new dependencies required

## Compilation and Test Results

```bash
cargo test -p pdftract-core --test classify_page_error_paths
```

Result: **ok. 6 passed; 0 failed; 1 ignored; 0 measured**

## Notes

- Tests use `tempfile::NamedTempFile` for automatic cleanup instead of manual RAII guards
- Binary not found test is environment-dependent and properly marked to ignore in normal dev setup
- All error messages contain clear diagnostic text matching acceptance criteria
- Tests are isolated and can run independently

## Commit Information

- Files modified: 1 (created new test file)
- Test coverage: All documented error paths for `classify_page`
