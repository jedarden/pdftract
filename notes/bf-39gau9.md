# bf-39gau9: STREAM_DECOMPRESS_ERROR Assertion Verification

## Task
Verify STREAM_DECOMPRESS_ERROR assertion fires correctly in truncated-flate test.

## Findings

### Assertion Implementation
The assertion is implemented in `test_truncated_flate_emits_stream_decode_error()` (lines 383-395):

```rust
let has_stream_decode_error = diagnostics
    .iter()
    .any(|d| d.contains("STREAM_DECODE_ERROR"));

assert!(
    has_stream_decode_error,
    "Expected STREAM_DECODE_ERROR diagnostic not found. \
     Got {} diagnostics: {:?}",
    diagnostics.len(),
    diagnostics
);
```

### Test Execution Results
Running the test produces the following output:

```
Testing STREAM_DECODE_ERROR emission for: /home/coding/pdftract/crates/pdftract-core/../../tests/fixtures/malformed/truncated-flate.pdf
✓ extract_pdf() succeeded
  Fingerprint: pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
  Page count: 0
  Total diagnostics: 0

thread 'test_truncated_flate_emits_stream_decode_error' panicked at crates/pdftract-core/tests/test_truncated_flate_recovery.rs:389:5:
Expected STREAM_DECODE_ERROR diagnostic not found. Got 0 diagnostics: []
```

### Verification Results

#### ✅ PASS: Assertion Logic Correct
- The assertion correctly uses `.iter().any(|d| d.contains("STREAM_DECODE_ERROR"))` to check for the error code
- The assertion message properly includes the diagnostics count and contents for debugging
- The pattern follows the established pattern from bf-2h1nt research

#### ✅ PASS: Assertion Fires When Expected Condition Is Met
- The assertion correctly fires when STREAM_DECODE_ERROR is not present
- The panic message is clear and informative: "Expected STREAM_DECODE_ERROR diagnostic not found. Got 0 diagnostics: []"

#### ❌ FAIL: Expected Diagnostic Not Present
- The extraction completes successfully but produces **0 diagnostics**
- STREAM_DECODE_ERROR is NOT being emitted by the extraction pipeline
- This is a pre-existing issue (documented in parent bead bf-hyhjnl) - the test was written to document the expected behavior, but the implementation does not yet emit this diagnostic

### Conclusion
The **assertion is correctly implemented and working as designed**. It successfully detects that STREAM_DECODE_ERROR is not being emitted when the truncated-flate.pdf fixture is processed. The test failure is due to a missing implementation in the extraction pipeline, not a defect in the test assertion logic.

## Test Command
```bash
cargo test --test test_truncated_flate_recovery test_truncated_flate_emits_stream_decode_error -- --show-output
```

## Related Files
- Test: `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
- Fixture: `tests/fixtures/malformed/truncated-flate.pdf`
