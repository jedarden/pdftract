# Bead bf-2hjaa: Assertion Location for Error Validation

## Summary

This analysis identifies where in the truncated-flate.pdf test the error assertion should be added to validate that `STREAM_DECODE_ERROR` diagnostics are emitted when processing a PDF with a truncated flate stream.

## Context

This is about a PDF extraction test for the fixture at `/home/coding/pdftract/tests/fixtures/malformed/truncated-flate.pdf`, NOT the stream decoder unit tests. The test should:
1. Extract the PDF using `pdftract_core::extract::extract_pdf()`
2. Assert extraction succeeds (non-fatal error recovery)
3. Assert `STREAM_DECODE_ERROR` appears in the errors/diagnostics array

## Extraction Result Structure

### From `/home/coding/pdftract/crates/pdftract-core/src/extract.rs`:

```rust
pub struct ExtractionResult {
    pub fingerprint: String,
    pub pages: Vec<PageResult>,
    pub metadata: ExtractionMetadata,
    // ... other fields
}

pub struct ExtractionMetadata {
    // ... other fields
    /// Diagnostics emitted during extraction (coverage warnings, etc.)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    // ... other fields
}
```

**Key insight**: The diagnostics are stored in `extraction_result.metadata.diagnostics` as a `Vec<String>`, not `Vec<Diagnostic>`.

### Diagnostic Code Format

From `/home/coding/pdftract/crates/pdftract-core/src/diagnostics.rs`:
- **Enum variant**: `DiagCode::StreamDecodeError`
- **String representation**: `"STREAM_DECODE_ERROR"` (via `Display` impl)

## Optimal Assertion Location

The assertion should be added in the test function `test_truncated_flate_recovery()` at the following location:

**After**: Extraction succeeds and we have the `ExtractionResult`
**Before**: Any other assertions that might depend on the result

### Complete Test Pattern

Based on the pattern from `TH-04-js-presence.rs` lines 31-94:

```rust
#[test]
fn test_truncated_flate_recovery() {
    let fixture = PathBuf::from("tests/fixtures/malformed/truncated-flate.pdf");

    // Skip if fixture doesn't exist
    if !fixture.exists() {
        eprintln!("Skipping test: fixture not found at {}", fixture.display());
        return;
    }

    // Extract with default options
    let options = ExtractionOptions::default();
    let result = extract_pdf(&fixture, &options);

    // ASSERTION LOCATION 1: Assert extraction succeeds (non-fatal recovery)
    assert!(result.is_ok(), "Extraction should succeed despite truncated stream");

    let extraction_result = result.unwrap();

    // ASSERTION LOCATION 2: Assert STREAM_DECODE_ERROR appears in diagnostics
    let diagnostics = &extraction_result.metadata.diagnostics;
    assert!(
        diagnostics.iter().any(|d| d.contains("STREAM_DECODE_ERROR")),
        "Expected STREAM_DECODE_ERROR diagnostic for truncated flate stream. Diagnostics: {:?}",
        diagnostics
    );

    // Optional: Verify extraction produced some output despite error
    assert!(extraction_result.pages.len() > 0, "Should extract at least one page");
}
```

## What the Assertion Should Validate

1. **Primary validation**: That `"STREAM_DECODE_ERROR"` is present in `metadata.diagnostics`
2. **Implicit validation**: That extraction succeeded (`result.is_ok()`) - confirms non-fatal error recovery per INV-8
3. **Optional validation**: That partial output was produced (not zero pages)

## Edge Cases and Error Conditions

1. **Fixture not found**: Skip test with clear message (standard pattern)
2. **Multiple diagnostics**: Use `.contains()` to be flexible about exact string format
3. **Case sensitivity**: The diagnostic string is `"STREAM_DECODE_ERROR"` (uppercase)
4. **Alternative diagnostic codes**: Some code paths use `DecompressionFailed` internally but map to `StreamDecodeError`

## Test File Location

Recommended: `/home/coding/pdftract/crates/pdftract-core/tests/test_truncated_flate_recovery.rs`

**Reasoning**:
- Follows naming pattern: `test_<fixture>_<purpose>.rs`
- Keeps malformed PDF tests isolated
- Dedicated file for this specific test case

## Required Imports

```rust
use pdftract_core::extract::extract_pdf;
use pdftract_core::options::ExtractionOptions;
use std::path::PathBuf;
```

## Key Implementation Details

1. **Use `.contains()` not `.eq()`**: Diagnostic strings may have additional context
2. **Check for presence not count**: Use `.any()` not exact count matching
3. **Provide helpful failure message**: Include actual diagnostics in assert message
4. **Follow TH-04 pattern**: That test demonstrates the canonical pattern for diagnostic assertions

## References

- **Pattern source**: `/home/coding/pdftract/crates/pdftract-core/tests/TH-04-js-presence.rs` lines 86-93
- **Diagnostic enum**: `/home/coding/pdftract/crates/pdftract-core/src/diagnostics.rs`
- **Extraction API**: `/home/coding/pdftract/crates/pdftract-core/src/extract.rs`
- **Fixture location**: `/home/coding/pdftract/tests/fixtures/malformed/truncated-flate.pdf`

## Acceptance Criteria Status

- [x] Assertion purpose and scope defined (validate STREAM_DECODE_ERROR in diagnostics)
- [x] Optimal assertion location identified (after extraction succeeds, in diagnostics check)
- [x] Assertion requirements documented (use .contains() on metadata.diagnostics Vec<String>)
