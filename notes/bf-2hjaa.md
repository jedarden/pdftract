# Bead bf-2hjaa: Assertion Location for Error Validation

## Summary

This analysis identifies where in the stream decoder fixture test the error assertion should be added to validate that `STREAM_DECODE_ERROR` diagnostics are emitted for the `flate_truncated` fixture.

## Problem Statement

The `flate_truncated.meta` file specifies that decoding this fixture should:
1. Return partial bytes (what was decoded before the truncation point)
2. Emit a `STREAM_DECODE_ERROR` diagnostic

However, the current test scaffold does not validate diagnostics at all.

## Current State

### 1. Fixture Configuration
Location: `crates/pdftract-core/tests/stream_decoder_fixtures.rs:60-64`

```rust
FixtureInfo {
    name: "flate_truncated",
    filter: FixtureFilter::Single("FlateDecode", None),
    expected_diags: vec![],  // <-- Should be vec![DiagCode::StreamDecodeError]
    bomb_limit: None,
},
```

### 2. Decode Function Signature Issue
Location: `crates/pdftract-core/src/parser/stream.rs:89-95`

The `StreamDecoder::decode()` trait:
```rust
fn decode(
    &self,
    input: &[u8],
    params: Option<&PdfObject>,
    doc_counter: &mut u64,
    max_bytes: u64,
) -> Result<Vec<u8>, FilterError>;
```

**Problem:** The trait returns only `Result<Vec<u8>, FilterError>` - no diagnostics!
- When `UnexpectedEof` occurs, it returns `Ok(partial_bytes)` but cannot emit diagnostics
- See line 542-544 in `FlateDecoder::decode_impl()`: truncated streams just return partial bytes with no diagnostic

### 3. Test Loop Missing Diagnostic Validation
Location: `crates/pdftract-core/tests/stream_decoder_fixtures.rs:254-360`

The test loop:
1. ✓ Reads fixture and expected files
2. ✓ Decodes the fixture
3. ✓ Compares output bytes
4. ✗ **Does NOT validate diagnostics**

## Where the Assertion Should Be Added

**Location:** In `test_all_stream_decoder_fixtures()`, after the byte comparison check (after line 326)

**What to validate:**
- If `fixture.expected_diags` is non-empty, verify that matching diagnostics were emitted
- For `flate_truncated`: assert that exactly one `STREAM_DECODE_ERROR` diagnostic was emitted

## How to Fix

### Option 1: Use higher-level `decode_stream()` function
The `decode_stream()` function returns `DecodeResult` which includes diagnostics:
```rust
pub struct DecodeResult {
    pub bytes: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,  // <-- We need this!
    pub meta: StreamMeta,
}
```

### Option 2: Add diagnostic support to `StreamDecoder::decode()` trait
Modify the trait to return diagnostics alongside the decoded bytes.

## Recommended Assertion Implementation

After line 326 (byte comparison), add:

```rust
// Validate expected diagnostics
if !fixture.expected_diags.is_empty() {
    // Check if each expected diagnostic code is present in the actual diagnostics
    for expected_code in &fixture.expected_diags {
        let found = result.diagnostics.iter().any(|d| d.code() == *expected_code);
        assert!(
            found,
            "{}: expected diagnostic {} not found. Got: {:?}",
            fixture.name,
            expected_code,
            result.diagnostics
        );
    }
}
```

## Edge Cases and Considerations

1. **Partial bytes vs error**: Truncated streams should return partial bytes AND emit a diagnostic (not `Err()`)
2. **Multiple diagnostics**: Some fixtures may emit multiple diagnostics - check all expected ones
3. **Bomb limit tests**: `flate_bomb_3gb` should emit `StreamBomb` diagnostic
4. **Unknown filter**: `unknown_filter` fixture should emit `StreamUnknownFilter` diagnostic

## Files to Modify

1. **Test scaffold** (`crates/pdftract-core/tests/stream_decoder_fixtures.rs`):
   - Fix `flate_truncated.expected_diags` to `vec![DiagCode::StreamDecodeError]`
   - Add diagnostic validation loop after byte comparison
   - Change `decode_fixture()` to return `DecodeResult` instead of `Vec<u8>`

2. **Stream decoder trait** (`crates/pdftract-core/src/parser/stream.rs`):
   - Either: use `decode_stream()` which already returns `DecodeResult` with diagnostics
   - Or: modify `StreamDecoder::decode()` trait to support diagnostics

## Acceptance Criteria Status

- [x] Assertion purpose and scope defined
- [x] Optimal assertion location identified (after line 326 in test loop)
- [x] Assertion requirements documented (validate `STREAM_DECODE_ERROR` for `flate_truncated`)
