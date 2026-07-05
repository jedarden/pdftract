# Bead bf-fyfbf: Locate and read truncated-flate test file

## Summary

The test file `crates/pdftract-core/tests/test_truncated_flate_recovery.rs` **does not exist**.

## What was found

### Related test files that DO exist:

1. **`crates/pdftract-core/tests/error_recovery_integration.rs`**
   - Contains `test_truncated_mid_stream()` function (line 165)
   - Tests truncated streams with expected `STREAM_DECODE_ERROR` diagnostics
   - Uses `tests/error_recovery/fixtures/truncated_mid_stream.pdf`

2. **`crates/pdftract-core/tests/stream_decoder_fixtures.rs`**
   - Contains `flate_truncated` fixture test (line 61-65)
   - Tests flate decoder with truncated input
   - Uses `tests/stream_decoder/fixtures/flate_truncated.bin`

### Fixture files that exist:

1. **`tests/fixtures/malformed/truncated-flate.pdf`** (588 bytes)
   - This appears to be the PDF fixture that `test_truncated_flate_recovery.rs` would test
   - Not currently referenced by any existing test

2. **`tests/stream_decoder/fixtures/flate_truncated.bin`**
   - Binary fixture for stream decoder testing
   - Has `.meta` file: "FlateDecode: mid-stream EOF; expects partial bytes + STREAM_DECODE_ERROR"
   - Has `.expected` file (1 line, minimal output)

## Basic test structure understanding

From examining `error_recovery_integration.rs` and `stream_decoder_fixtures.rs`:

1. **Error recovery test pattern:**
   - Load fixture with `fs::read()`
   - Run extraction under `catch_unwind` for panic safety
   - Verify PDF starts with `%PDF-`
   - Check for expected diagnostics

2. **Stream decoder test pattern:**
   - Define fixture metadata (filter name, expected diagnostics, bomb limits)
   - Load binary fixture
   - Decode through appropriate decoder
   - Compare against `.expected` file

## Existing test functions documented

### error_recovery_integration.rs:
- `test_xref_30pct_bad_offsets()`
- `test_missing_mediabox_all_pages()`
- `test_missing_endobj()`
- `test_truncated_mid_stream()` - most relevant
- `test_int_overflow_bbox()`
- `test_nested_failure()`
- `test_combined_failures()`
- `test_inv_8_no_panics_across_all_fixtures()`

### stream_decoder_fixtures.rs:
- `test_all_stream_decoder_fixtures()` - includes flate_truncated
- `test_each_filter_exercised()`

## Conclusion

The file `test_truncated_flate_recovery.rs` does not exist and needs to be created. The parent bead `bf-4g6dj` assumes this file exists, but it appears this is a new test file that should be created to test the specific `truncated-flate.pdf` fixture.

The test file should likely follow the pattern from `error_recovery_integration.rs` but specifically for testing the `truncated-flate.pdf` fixture and validating its extraction results and error diagnostics.
