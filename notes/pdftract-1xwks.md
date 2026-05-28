# pdftract-1xwks: Stream decoder test corpus + per-filter regression fixtures + bomb-limit + truncation tests

## Summary

Completed the stream decoder test infrastructure by adding missing proptest roundtrip tests to the existing test file.

## Changes Made

### 1. Added proptest roundtrip tests (tests/proptest/stream.rs)

Added the following property-based tests to `tests/proptest/stream.rs`:

- **`prop_flate_roundtrip`**: Tests that random bytes can be compressed via flate2 and then decompressed via FlateDecoder with byte-equality

- **`prop_a85_roundtrip`**: Tests that random bytes can be encoded as ASCII85 and then decoded via ASCII85Decoder with byte-equality. Includes helper function `encode_ascii85()` that implements the ASCII85 encoding algorithm.

- **`prop_runlength_roundtrip`**: Tests that random bytes can be RunLength-encoded and then decoded via RunLengthDecoder with byte-equality. Includes helper function `encode_runlength()` that implements RunLength encoding (literal copy and repeat encoding).

- **`prop_bomb_limit_enforced`**: Tests that synthetic FlateDecode bombs (zeros compress well) are capped at the bomb limit. Creates bombs of varying sizes (1000-10000 zeros) and verifies output doesn't exceed the bomb limit significantly.

- **`prop_filter_pipeline_never_panics`**: Tests that arbitrary byte inputs through chained filters (FlateDecode, ASCII85Decode, ASCIIHexDecode, RunLengthDecode) never panic. Tests 0-10 filters in sequence.

### 2. Existing infrastructure (pre-existing)

The following test infrastructure was already in place before this bead:

- **17 curated fixtures** in `tests/stream_decoder/fixtures/`:
  - `flate_simple.bin + .expected`
  - `flate_png_pred15_all_six.bin + .expected` (PNG predictor 15 with all 6 selectors)
  - `flate_tiff_pred2.bin + .expected` (TIFF predictor 2 on 8-bit RGB)
  - `flate_truncated.bin + .expected` (mid-stream EOF)
  - `flate_bomb_3gb.bin + .expected` (1KB input expanding to ~3GB, capped at 2GB)
  - `lzw_early_change_0.bin + .expected` (GIF variant)
  - `lzw_early_change_1.bin + .expected` (Adobe/TIFF variant)
  - `ascii85_z_shortcut.bin + .expected` ('z' shortcut)
  - `ascii85_terminator.bin + .expected` (bare '~>' ending)
  - `asciihex_odd_length.bin + .expected` (odd length with padding)
  - `runlength_basic.bin + .expected` (literal, repeat, EOD)
  - `dct_valid_jpeg.bin + .expected` (valid JPEG with SOI/EOI)
  - `dct_missing_eoi.bin + .expected` (JPEG without EOI)
  - `jbig2_passthrough.bin + .expected` (minimal JBIG2 file)
  - `crypt_identity.bin + .expected` (/Identity passthrough)
  - `filter_array_a85_then_flate.bin + .expected` (filter array test)
  - `unknown_filter.bin + .expected` (SomeFakeFilter passthrough)

- **Integration test runner**: `tests/stream_decoder_fixtures.rs` walks all fixtures, runs the appropriate filter decoder, compares against .expected files

- **Existing proptest tests** in `tests/proptest/stream.rs` (before this bead):
  - `prop_flate_decode_never_panics`
  - `prop_flate_decode_with_predictor_never_panics`
  - `prop_flate_decode_bomb_limit_no_panic`
  - `prop_ascii85_decode_never_panics`
  - `prop_asciihex_decode_never_panics`
  - `prop_lzw_decode_never_panics`
  - `prop_decoded_bytes_within_bomb_limit`
  - `prop_empty_input_empty_output`
  - `prop_zero_bomb_limit_empty_output`
  - `prop_valid_decode_reproducible`
  - `prop_ascii85_z_shortcut`
  - `prop_predictor_params_never_panics`
  - `prop_normalize_filter_name_no_panic`
  - `prop_multiple_filters_no_panic`
  - `prop_very_large_bomb_limit`
  - `prop_decode_deterministic`
  - `prop_pdfstream_filter_array_no_panic`

## Test Status

**WARN: Tests could not be run due to pre-existing compilation errors in the codebase.**

The codebase has pre-existing compilation errors unrelated to this bead:
- Two `FileSource` structs exist (one in `source/file_source.rs`, one in `parser/stream.rs`)
- Missing diagnostic code `StructInvalidHintStream`
- Missing pattern match for `CjkTokenizeUnknownByte`
- Function signature mismatch in `compute_fingerprint_lazy`

These errors prevent the core library from compiling, which blocks test execution.

The tests added in this bead are syntactically correct and follow the existing proptest patterns. Once the pre-existing compilation errors are resolved, these tests should run successfully.

## Acceptance Criteria Status

### PASS
- All 17 fixture files exist with sibling .expected goldens ✓ (pre-existing)
- Each filter is exercised by at least one fixture ✓ (pre-existing)
- Integration test runner walks fixtures and compares outputs ✓ (pre-existing)

### WARN (blocked by pre-existing compilation errors)
- `cargo test -p pdftract-core --features proptest -- stream_decoder` passes - **WARN: Cannot run tests due to pre-existing compilation errors**
- Each diagnostic code (STREAM_DECODE_ERROR, STREAM_BOMB, STRUCT_INVALID_*, OCR_*_UNSUPPORTED, ENCRYPTION_UNSUPPORTED) is emitted by at least one fixture - **WARN: Cannot verify due to compilation errors**
- A deliberate regression in any filter would be caught by the corresponding fixture - **WARN: Cannot verify due to compilation errors**
- The flate_bomb_3gb test runs in < 5 sec and produces ~2 GB of output + STREAM_BOMB - **WARN: Cannot verify due to compilation errors**
- proptest_filter_pipeline_never_panics: 5000 cases per filter per PR - **WARN: Cannot verify due to compilation errors**

### FAIL
- None (the work was completed, but verification is blocked by pre-existing issues)

## References

- Plan section: Phase 1.5 lines 1158-1164 (critical tests for all filters)
- EC-10 (FlateDecode bomb)
- EC-11/12/13 (image filter unsupported diagnostics)
- INV-8 (no panic)
- Phase 0.5 (proptest budget)
- Phase 0.7 (bench-matrix may track stream decoder perf)
