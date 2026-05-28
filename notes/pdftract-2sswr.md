# pdftract-2sswr: JBIG2Decode passthrough + /JBIG2Globals reference recording + OCR_JBIG2_UNSUPPORTED diagnostic

## Summary

Verified that the JBIG2Decode passthrough filter implementation is complete and functional. The JBIG2 decoder module (`crates/pdftract-core/src/decoder/jbig2.rs`) was already implemented with all required functionality.

## Acceptance Criteria Status

### PASS
- JBIG2 stream with full-render feature → pass-through, no diagnostic (stream.rs:3542-3548)
- JBIG2 stream WITHOUT full-render → OCR_JBIG2_UNSUPPORTED diagnostic; pass-through anyway (stream.rs:3542-3548)
- /JBIG2Globals reference recorded on StreamMeta (stream.rs:3550-3556)
- Self-contained JBIG2 (no globals): StreamMeta.jbig2_globals_ref is None (field defaults to None)

### WARN
- Round-trip test with reference JBIG2 fixture: Unit tests in stream.rs (test_jbig2_passthrough, test_jbig2_extract_globals_ref, etc.) verify the passthrough and globals extraction functionality with mock data. No actual JBIG2 PDF fixture exists in the test suite.

## Changes Made

### Fixed compilation error in `parser/pages.rs`
- Added `Default` implementation for `PageDict` struct to fix compilation errors in `javascript.rs` tests
- The `PageDict::default()` method is used in javascript detection tests

### Verified existing implementation
The following components were already implemented and verified working:

**`crates/pdftract-core/src/decoder/jbig2.rs`** (225 lines):
- `Jbig2GlobalsRef` struct - captures ObjRef to globals stream
- `Jbig2Decoder` struct - handles passthrough and diagnostic emission
- `extract_globals_ref()` - extracts /JBIG2Globals reference from stream dict
- `emit_unsupported_diagnostic()` - emits OCR_JBIG2_UNSUPPORTED when full-render not available
- `has_full_render()` - checks cfg!(feature = "full-render") at compile time
- Read trait implementation for passthrough compatibility
- 6 unit tests (all passing)

**`crates/pdftract-core/src/parser/stream.rs`** (integration):
- Lines 3542-3548: Emit OCR_JBIG2_UNSUPPORTED diagnostic when full-render disabled
- Lines 3550-3556: Extract /JBIG2Globals reference and store in stream_meta
- Lines 5742-5831: 5 integration tests for JBIG2 passthrough (all passing)

**`crates/pdftract-core/src/diagnostics.rs`**:
- `DiagCode::OcrJbig2Unsupported` defined at line 633
- Diagnostic info at line 1951-1955 (Warning severity, recoverable)

## Test Results

All 11 JBIG2-related tests pass:
```
test decoder::jbig2::tests::test_emit_unsupported_diagnostic_when_feature_disabled ... ok
test decoder::jbig2::tests::test_extract_globals_ref_with_valid_ref ... ok
test decoder::jbig2::tests::test_extract_globals_ref_with_invalid_type ... ok
test decoder::jbig2::tests::test_extract_globals_ref_without_globals ... ok
test decoder::jbig2::tests::test_jbig2_decoder_const ... ok
test decoder::jbig2::tests::test_jbig2_globals_ref_const ... ok
test parser::stream::source_tests::test_jbig2_bomb_limit ... ok
test parser::stream::source_tests::test_jbig2_extract_globals_ref ... ok
test parser::stream::source_tests::test_jbig2_extract_globals_ref_invalid_type ... ok
test parser::stream::source_tests::test_jbig2_extract_globals_ref_missing ... ok
test parser::stream::source_tests::test_jbig2_passthrough ... ok
```

## Implementation Details

Per PDF spec 7.4.7:
- JBIG2Decode is a lossless compression format for bitonal images
- /JBIG2Globals is an indirect reference to a globally-shared symbol dictionary
- Without globals, the stream is self-contained (still decodable)

Passthrough behavior (EC-11):
- With full-render feature: Passthrough only, no diagnostic
- Without full-render: Emit OCR_JBIG2_UNSUPPORTED diagnostic, still passthrough

## Files Modified

- `crates/pdftract-core/src/parser/pages.rs` - Added Default impl for PageDict

## Files Verified (no changes needed)

- `crates/pdftract-core/src/decoder/jbig2.rs` - Complete implementation
- `crates/pdftract-core/src/decoder/mod.rs` - Module exports
- `crates/pdftract-core/src/parser/stream.rs` - Integration and diagnostics
- `crates/pdftract-core/src/diagnostics.rs` - Diagnostic code definition
- `crates/pdftract-core/src/lib.rs` - Public module export
