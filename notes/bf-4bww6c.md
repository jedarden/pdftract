# bf-4bww6c: Implement STREAM_DECODE_ERROR assertion in test

## Summary
Implemented the actual assertion that `STREAM_DECODE_ERROR` appears in the errors/diagnostics array in the `test_truncated_flate_emits_stream_decode_error` test.

## Changes Made

### File: `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`

**Lines 383-403**: Replaced the commented-out assertion placeholder with the actual assertion code:

```rust
// Assert that STREAM_DECODE_ERROR appears in the diagnostics array.
// Pattern from bf-2h1nt: use .contains() on Vec<String> to check for specific codes
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

println!("✓ STREAM_DECODE_ERROR diagnostic found in {} diagnostics", diagnostics.len());
```

## Pattern Used
Following the pattern from bf-2h1nt research:
- Used `.iter().any(|d| d.contains("STREAM_DECODE_ERROR"))` to check for the specific error code
- Applied to `Vec<String>` diagnostics field from `extraction_result.metadata.diagnostics`
- Clear assertion message showing what was expected and what was found

## Test Results

**Compilation**: ✓ PASS (test compiles successfully)

**Test Execution**: ✓ FAIL (expected - diagnostics not yet emitted)
```
test test_truncated_flate_emits_stream_decode_error ... FAILED

Expected STREAM_DECODE_ERROR diagnostic not found. Got 0 diagnostics: []
```

The test properly fails with a clear message because the diagnostics infrastructure is in place but `STREAM_DECODE_ERROR` diagnostics are not yet emitted during extraction (that work belongs to a different bead).

## Acceptance Criteria Status

- ✓ Test asserts STREAM_DECODE_ERROR appears in diagnostics array
- ✓ Assertion follows existing test patterns from bf-2h1nt research
- ✓ Test compiles
- ✓ Assertion is placed correctly in test flow

All acceptance criteria PASS.

## Next Steps
The assertion is now in place and working. The test will pass once the underlying diagnostics emission is implemented to actually emit `STREAM_DECODE_ERROR` when truncated FlateDecode streams are encountered during extraction.

## Commit
Commit: (to be added after commit)
