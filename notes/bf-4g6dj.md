# bf-4g6dj: Truncated-Flate Test Scaffold Examination

## Summary

Examined the existing truncated-flate test scaffold and extraction result structure to understand where to add assertions for stream decode error diagnostics.

## Test Location

**File**: `/home/coding/pdftract/crates/pdftract-core/tests/stream_decoder_fixtures.rs`

**Test function**: `test_all_stream_decoder_fixtures()` (lines 264-364)

## Test Fixture Structure

### Fixture Locations
- **Binary fixture**: `/home/coding/pdftract/tests/stream_decoder/fixtures/flate_truncated.bin`
- **Expected output**: `/home/coding/pdftract/tests/stream_decoder/fixtures/flate_truncated.expected`
- **Metadata**: `/home/coding/pdftract/tests/stream_decoder/fixtures/flate_truncated.meta`

### Fixture Metadata Content
```
FlateDecode: mid-stream EOF; expects partial bytes + STREAM_DECODE_ERROR
```

## Current Test Implementation

### Current Fixture Definition (lines 61-65)
```rust
FixtureInfo {
    name: "flate_truncated",
    filter: FixtureFilter::Single("FlateDecode", None),
    expected_diags: vec![],  // ⚠️ EMPTY - but .meta file expects STREAM_DECODE_ERROR
    bomb_limit: None,
},
```

### Current Test Behavior
The `test_all_stream_decoder_fixtures()` function:
1. Loads each fixture from `.bin` file
2. Decodes using the specified filter (e.g., `FlateDecoder`)
3. Compares output against `.expected` file
4. **Checks `expected_diags` match** - but `flate_truncated` has empty expectations

### Key Issue Identified
The `.meta` file explicitly states the fixture should produce a `STREAM_DECODE_ERROR`, but the current test has an empty `expected_diags` vector. This is the core issue that needs to be fixed.

## Extraction Result Structure

### For Stream Decoder Tests
The stream decoder tests work at a lower level - they test individual `StreamDecoder` implementations directly:

```rust
pub trait StreamDecoder {
    fn decode(
        &self,
        data: &[u8],
        params: Option<&PdfObject>,
        bytes_written: &mut u64,
        max_bytes: u64
    ) -> Result<Vec<u8>, String>;
}
```

### For Full PDF Extraction Tests
When doing full PDF extraction (which might be needed for integration tests), the error structure is:

**Document-Level Errors**:
```rust
pub struct ExtractionResult {
    pub metadata: ExtractionMetadata,
    // ... other fields
}

pub struct ExtractionMetadata {
    pub diagnostics: Vec<String>,  // Document-level diagnostics
    pub error_count: usize,
    // ... other fields
}
```

**Page-Level Errors**:
```rust
pub struct PageResult {
    pub error: Option<String>,  // Individual page extraction error
    // ... other fields
}
```

## Where to Add the Assertion

### For Stream Decoder Test (Current Context)
The assertion should be added to the `FixtureInfo` definition:

```rust
FixtureInfo {
    name: "flate_truncated",
    filter: FixtureFilter::Single("FlateDecode", None),
    expected_diags: vec![DiagCode::StreamDecodeError],  // ← Add this
    bomb_limit: None,
},
```

The test framework already checks `expected_diags` in the test loop.

### For Full Extraction Test (Future Enhancement)
If a full extraction test is needed, it would check:

```rust
let result = extract_pdf(&pdf_path, &options)?;

assert!(
    result.metadata.diagnostics.iter().any(|d| d.contains("STREAM_DECODE_ERROR")),
    "Expected STREAM_DECODE_ERROR in extraction diagnostics"
);
```

## Diagnostic Code Reference

From `diagnostics.rs`:
- **Code**: `DiagCode::StreamDecodeError`
- **String representation**: `"STREAM_DECODE_ERROR"`
- **Description**: Emitted when a stream decoder encounters corrupt data mid-decompression

## Next Steps (for Dependent Beads)

1. **bf-2h1nt** - Research existing error assertion patterns in test suite
2. **bf-4bx00** - Add STREAM_DECODE_ERROR assertion to truncated-flate test (depends on bf-2h1nt)
3. **bf-2897m** - Add clear assertion failure messages and verify test compiles (depends on bf-4bx00)

## Conclusion

The test scaffold exists and is functional in `/home/coding/pdftract/crates/pdftract-core/tests/stream_decoder_fixtures.rs`. The `flate_truncated` fixture currently has an empty `expected_diags` vector when it should contain `DiagCode::StreamDecodeError` based on the fixture's `.meta` file specification.

The extraction result structure supports error reporting at both:
- **Document level**: `ExtractionResult.metadata.diagnostics: Vec<String>`
- **Page level**: Each `PageResult.error: Option<String>`

For stream decoder tests specifically, errors are handled through the `Result<Vec<u8>, String>` return type from the `StreamDecoder::decode()` trait method, and the test framework validates expected diagnostic codes via the `expected_diags` field in `FixtureInfo`.
