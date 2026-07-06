# bf-3erm0: Read and understand truncated-flate test scaffold

## Summary

The test file `crates/pdftract-core/tests/test_truncated_flate_recovery.rs` **does not exist** in the codebase. This bead and its parent chain (bf-4g6dj → bf-mzf4i → bf-23y18) were written assuming this test scaffold already exists, but it has not been created yet.

## Key Findings

### 1. Missing Test File
- **Expected path**: `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
- **Actual status**: File does not exist
- **Implication**: The test scaffold needs to be created before assertions can be added

### 2. Error Code Correction
The bead chain references `STREAM_DECOMPRESS_ERROR`, but the actual diagnostic code in the codebase is:
- **Enum variant**: `DiagCode::StreamDecodeError`
- **Emitted string**: `STREAM_DECODE_ERROR`
- **Definition**: `crates/pdftract-core/src/diagnostics.rs` line ~100-110

### 3. Extraction Result Structure
From examining `crates/pdftract-core/src/output/json.rs`:
- Errors are stored in the `Output` struct's `errors: Vec<DiagnosticJson>` field
- Each `DiagnosticJson` contains:
  - `code: String` - The diagnostic code (e.g., "STREAM_DECODE_ERROR")
  - `message: String` - Human-readable message
  - `severity: String` - "error", "warning", or "info"
  - `page_index: Option<u32>` - Affected page (if applicable)
  - `location: Option<String>` - Location in PDF
  - `hint: Option<String>` - Suggested remediation

### 4. Test Patterns for Error Assertions
From examining existing tests (`error_recovery_integration.rs`):

```rust
// Pattern 1: Filter diagnostics by code substring
fn assert_diagnostic_count_at_least(diagnostics: &[String], code: &str, min_count: usize) {
    let actual_count = diagnostics.iter().filter(|d| d.contains(code)).count();
    assert!(actual_count >= min_count, ...);
}

// Pattern 2: Check if any diagnostic contains a code
diagnostics.iter().any(|d| d.contains("STREAM_DECODE"))
```

For JSON output schema tests:
```rust
// Access error codes from Output.errors array
assert!(output.errors.iter().any(|e| e.code == "STREAM_DECODE_ERROR"));
```

### 5. StreamDecodeError Context
From `diagnostics.rs`:
- **Purpose**: "Stream decompression failed (corrupt data)"
- **Severity**: Error
- **Triggered when**: A stream decoder encounters corrupt data mid-decompression
- **Related**: There's also `DiagCode::StreamBomb` for decompression bomb limits

## Implications for Parent Beads

1. **bf-4g6dj** (Examine truncated-flate test scaffold): Cannot examine a file that doesn't exist
2. **bf-mzf4i** (Add STREAM_DECOMPRESS_ERROR assertion): 
   - File needs to be created first
   - Error code should be `STREAM_DECODE_ERROR`, not `STREAM_DECOMPRESS_ERROR`
3. **bf-23y18** (likely parent): Dependency chain assumes test scaffold exists

## Recommendation

Before proceeding with the parent beads:
1. Create the basic test scaffold `test_truncated_flate_recovery.rs`
2. Create a truncated-flate test fixture (PDF with incomplete/corrupt flate stream)
3. Use the correct error code `STREAM_DECODE_ERROR`
4. Follow the existing test patterns from `error_recovery_integration.rs`

## References

- Diagnostics definition: `crates/pdftract-core/src/diagnostics.rs`
- Output schema: `crates/pdftract-core/src/output/json.rs`
- Test patterns: `crates/pdftract-core/tests/error_recovery_integration.rs`
- SDK extraction pattern: `crates/pdftract-core/tests/test_sdk_extraction_simple.rs`
