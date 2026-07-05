# Truncated-Flate Test Scaffold Examination

## Summary

Examined the existing truncated-flate test scaffold and extraction result structure to understand where to add assertions for stream decode error diagnostics.

## Test Location

**File**: `/home/coding/pdftract/crates/pdftract-core/tests/error_recovery_integration.rs`

**Test function**: `test_truncated_mid_stream()` (lines 165-188)

## Test Fixture Structure

### Fixture Locations
- **PDF**: `/home/coding/pdftract/tests/error_recovery/fixtures/truncated_mid_stream.pdf`
- **Expected diagnostics JSON**: `/home/coding/pdftract/tests/error_recovery/fixtures/truncated_mid_stream.expected_diagnostics.json`

### Expected Diagnostics JSON Format
```json
{
  "description": "FlateDecode stream truncated mid-decompression",
  "expected_diagnostics": [
    {
      "code": "STREAM_DECODE_ERROR",
      "min_count": 1,
      "description": "Truncated FlateDecode stream should emit STREAM_DECODE_ERROR"
    }
  ],
  "expected_behavior": "partial output returned, no panic"
}
```

## Current Test Implementation

The current test at `test_truncated_mid_stream()` only verifies:
1. The fixture PDF exists and starts with `%PDF-`
2. The expected_diagnostics JSON structure contains STREAM_DECODE_ERROR

The test does NOT actually:
- Open the PDF with `PdfExtractor`
- Extract pages or streams
- Verify diagnostics are emitted during actual extraction
- Check for partial output behavior

## Extraction Result Structure

### ExtractionResult Fields (from `extract.rs`)
```rust
pub struct ExtractionResult {
    pub fingerprint: String,
    pub pages: Vec<PageResult>,
    pub metadata: ExtractionMetadata,
    pub signatures: Vec<SignatureJson>,
    pub form_fields: Vec<FormFieldJson>,
    pub links: Vec<LinkJson>,
    // ... other fields
}
```

### ExtractionMetadata Diagnostics Field
```rust
pub struct ExtractionMetadata {
    pub page_count: usize,
    pub error_count: usize,
    // ... other fields
    pub diagnostics: Vec<String>,  // ← This is where diagnostics appear
    // ... other fields
}
```

### Diagnostic Structure (from `diagnostics.rs`)
```rust
pub struct Diagnostic {
    pub code: DiagCode,              // e.g., DiagCode::StreamDecodeError
    pub byte_offset: Option<u64>,     // Where in the PDF the error occurred
    pub object_ref: Option<ObjRef>,   // Which object had the error
    pub message: Cow<'static, str>,   // Human-readable message
}
```

When serialized to JSON, diagnostics become String representations like:
- `"STREAM_DECODE_ERROR"`
- `"STRUCT_INVALID_NAME"`
- etc.

## STREAM_DECODE_ERROR Diagnostic Code

**Code**: `DiagCode::StreamDecodeError`  
**Category**: `STREAM_*`  
**Severity**: `Warning`  
**Recoverable**: `true`  
**Phase origin**: `1.5`  
**Description**: "Emitted when a stream decoder encounters corrupt data mid-decompression. Partial bytes decoded so far are returned."  
**String representation**: `"STREAM_DECODE_ERROR"`

## Where to Add the Assertion

When implementing the full extraction test for `test_truncated_mid_stream()`, the assertion should check:

```rust
// After extracting the PDF:
let result = extractor.extract()?;

// Check that STREAM_DECODE_ERROR appears in diagnostics:
let has_stream_error = result.metadata.diagnostics
    .iter()
    .any(|d| d.contains("STREAM_DECODE_ERROR"));

assert!(has_stream_error, "Expected STREAM_DECODE_ERROR diagnostic");
```

The assertion belongs in `test_truncated_mid_stream()` at line 169-186, replacing the placeholder fixture existence check with actual extraction and diagnostic verification.

## Related Fixtures

**Note**: There is also a `/home/coding/pdftract/tests/fixtures/malformed/truncated-flate.pdf` fixture, but it is not currently referenced by any test. This appears to be a different fixture than the one used in `error_recovery_integration.rs`.

## Test Pattern

Other tests in `error_recovery_integration.rs` follow this pattern:
1. Load fixture and expected diagnostics JSON
2. Perform extraction (currently TODO/placeholder)
3. Verify diagnostics count >= min_count using `assert_diagnostic_count_at_least()`
4. Verify no panic via `assert_no_panic()`
