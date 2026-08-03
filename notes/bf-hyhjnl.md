# STREAM_DECODE_ERROR Test Verification (bf-hyhjnl)

## Test Result: FAIL

The test `test_truncated_flate_emits_stream_decode_error` is failing because:
- Expected: `STREAM_DECODE_ERROR` diagnostic in `extraction_result.metadata.diagnostics`
- Actual: Empty diagnostics array (0 diagnostics)

## Root Cause Analysis

The stream decoder (`decode_stream` in `parser/stream.rs`) generates diagnostics internally when encountering truncated/corrupt streams, but these diagnostics are not being collected and surfaced through the extraction API.

### Code Flow Issue

1. `extract_pdf` calls `decode_page_content_streams` (extract.rs:96)
2. `decode_page_content_streams` calls `decode_stream` (parser/stream.rs:3673)
3. `decode_stream` returns only `Vec<u8>` - it discards diagnostics
4. The internal `decode_stream_impl` returns `DecodeResult` with both bytes and diagnostics
5. But the public `decode_stream` API only exposes bytes: `decode_stream_impl(...).bytes`

### The Gap

```rust
// parser/stream.rs:3673
pub fn decode_stream(...) -> Vec<u8> {
    decode_stream_impl(...).bytes  // Diagnostics discarded here!
}

// Internal function that returns both
fn decode_stream_impl(...) -> DecodeResult {
    // ... generates diagnostics ...
}
```

### Contrast with Working Diagnostic Tests

The TH-04-js-presence test successfully validates diagnostics because JavaScript diagnostics are emitted during parsing (not stream decoding) and are properly collected and surfaced.

## Acceptance Criteria Status

- ❌ Test passes with new assertion - FAIL (test fails with empty diagnostics)
- ❌ No regressions in other tests - PARTIAL (need to run full suite)
- ✅ Test compiles cleanly - PASS
- ❌ Assertion correctly validates STREAM_DECODE_ERROR presence - FAIL (diagnostics not surfaced)
- ⚠️  Verification documented - This file

## Recommendation

The STREAM_DECODE_ERROR diagnostic infrastructure exists but is not connected to the extraction API. To fix this, one of the following approaches would be needed:

1. **Add `decode_stream_with_diagnostics`** that returns `DecodeResult`
2. **Modify `decode_page_content_streams`** to collect and return diagnostics
3. **Pass a diagnostics collector** through the stream decoding chain

The test assertion is correct, but the implementation needs to be updated to surface stream decoding diagnostics.

## Related Files

- Test: `/home/coding/pdftract/crates/pdftract-core/tests/test_truncated_flate_recovery.rs:360`
- Stream decoder: `/home/coding/pdftract/crates/pdftract-core/src/parser/stream.rs:3673`
- Extraction: `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:96`
- Diagnostics catalog: `/home/coding/pdftract/crates/pdftract-core/src/diagnostics.rs:465`
