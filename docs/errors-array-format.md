# Errors Array Format and Test Integration Guide

This document explains the complete structure of the errors/diagnostics array in pdftract extraction results and how to integrate assertions in tests.

## Overview

The pdftract extraction system uses a unified diagnostic system to report errors, warnings, and informational messages. All diagnostics are emitted during PDF parsing and extraction without panicking (per INV-8), allowing the parser to always attempt recovery and continue processing.

## Extraction Result Structure

### Main Result Structure

```rust
pub struct ExtractionResult {
    pub fingerprint: String,
    pub pages: Vec<PageResult>,
    pub metadata: ExtractionMetadata,
    pub signatures: Vec<SignatureJson>,
    pub form_fields: Vec<FormFieldJson>,
    pub links: Vec<LinkJson>,
    pub attachments: Vec<AttachmentJson>,
    pub threads: Vec<ThreadJson>,
    pub javascript_actions: Vec<JavascriptActionJson>,
}
```

### Metadata Structure (Contains Errors/Diagnostics)

```rust
pub struct ExtractionMetadata {
    pub page_count: usize,
    pub receipts_mode: ReceiptsMode,
    pub span_count: usize,
    pub block_count: usize,
    pub cache_status: Option<String>,
    pub cache_age_seconds: Option<u64>,
    pub error_count: usize,              // Number of pages that failed to extract
    pub reading_order_algorithm: Option<String>,
    pub diagnostics: Vec<String>,        // THE ERRORS ARRAY
    pub profile_name: Option<String>,
    pub profile_version: Option<String>,
    pub profile_fields: Option<serde_json::Value>,
}
```

## Errors Array Format

### Location
- **Field**: `ExtractionResult.metadata.diagnostics`
- **Type**: `Vec<String>`
- **Description**: Array of diagnostic message strings emitted during extraction

### String Format

Each diagnostic in the array follows this format:

```
{DIAGNOSTIC_CODE}: {Human-readable message} (byte offset {offset})?
```

**Examples:**
- `STREAM_DECODE_ERROR: zlib stream truncated mid-inflation (byte offset 12345)`
- `STRUCT_INVALID_NAME: Name object exceeds 127-byte limit (byte offset 67890)`
- `FONT_GLYPH_UNMAPPED: Glyph could not be mapped to Unicode`
- `XREF_REPAIRED: Xref was reconstructed via forward scan`

### Components

1. **Diagnostic Code** (e.g., `STREAM_DECODE_ERROR`):
   - Identifies the type of error/warning/info
   - Follows naming convention: `CATEGORY_SPECIFIC_ISSUE`
   - Categories: `STRUCT_*`, `STREAM_*`, `XREF_*`, `ENCRYPTION_*`, `PAGE_*`, `FONT_*`, `OCR_*`, `REMOTE_*`, `GSTATE_*`, `LAYOUT_*`, `MCP_*`, `CACHE_*`, etc.

2. **Message**: Human-readable description of what occurred

3. **Byte Offset** (optional): Location in the PDF file where the error occurred

## Accessing Error Information

### From Extraction Result

```rust
use pdftract_core::extract::extract_pdf;

let result = extract_pdf("test.pdf", &Default::default())?;

// Access the errors array
let diagnostics = &result.metadata.diagnostics;

// Get error count
let error_count = result.metadata.error_count;

// Iterate through all diagnostics
for diagnostic in diagnostics {
    println!("{}", diagnostic);
}
```

### Checking for Specific Errors

```rust
// Check if any stream decode errors occurred
let has_stream_errors = diagnostics.iter()
    .any(|d| d.contains("STREAM_DECODE_ERROR"));

// Check for encryption errors
let has_encryption_errors = diagnostics.iter()
    .any(|d| d.contains("ENCRYPTION"));

// Count specific error types
let glyph_unmapped_count = diagnostics.iter()
    .filter(|d| d.contains("FONT_GLYPH_UNMAPPED"))
    .count();
```

## Diagnostic Categories and Codes

### Structure Errors (STRUCT_*)
- `STRUCT_INVALID_NAME` - Invalid name character or malformed name object
- `STRUCT_INVALID_HEX` - Invalid hex character in hex string
- `STRUCT_MISSING_KEY` - Missing required dictionary key
- `STRUCT_CIRCULAR_REF` - Circular reference detected
- `STRUCT_UNEXPECTED_BYTE` - Unexpected byte during parsing
- `STRUCT_UNEXPECTED_EOF` - Unexpected end of file

### Stream Errors (STREAM_*)
- `STREAM_DECODE_ERROR` - Stream decompression failed (corrupt data)
- `STREAM_BOMB` - Decompression bomb limit exceeded
- `STREAM_UNKNOWN_FILTER` - Unknown filter name
- `STREAM_INVALID_PARAMS` - Invalid filter parameters
- `STREAM_TRUNCATED` - Stream data truncated

### XRef Errors (XREF_*)
- `XREF_INVALID_HEADER` - Invalid xref keyword or header
- `XREF_REPAIRED` - Xref was reconstructed via forward scan
- `XREF_TRUNCATED` - Truncated xref table

### Encryption Errors (ENCRYPTION_*)
- `ENCRYPTION_UNSUPPORTED` - Unsupported encryption or no password supplied
- `ENCRYPTION_WRONG_PASSWORD` - Password incorrect
- `ENCRYPTION_INVALID_DICT` - Invalid encryption dictionary

### Font Errors (FONT_*)
- `FONT_GLYPH_UNMAPPED` - Glyph could not be mapped to Unicode
- `FONT_NOT_FOUND` - Font not found or couldn't be parsed
- `FONT_INVALID_CMAP` - Invalid CMap format
- `FONT_PARSE_FAILED` - Font program parsing failed

### Page Errors (PAGE_*)
- `PAGE_OUT_OF_RANGE` - Page number out of range
- `PAGE_INVALID_COUNT` - Invalid /Count key in /Pages tree
- `PAGE_INVALID_ROTATE` - Invalid /Rotate value (not multiple of 90)

### Graphics State Errors (GSTATE_*)
- `GSTATE_STACK_OVERFLOW` - Graphics state stack overflow
- `GSTATE_STACK_UNDERFLOW` - Graphics state stack underflow
- `CM_ARG_COUNT` - Invalid argument count for cm operator

### OCR Errors (OCR_*)
- `OCR_JBIG2_UNSUPPORTED` - JBIG2 decoder not available
- `OCR_TESSERACT_FAILED` - Tesseract OCR failed
- `OCR_LANGUAGE_UNAVAILABLE` - Requested language pack not available

## Assertion Patterns for Tests

### Pattern 1: Check for No Errors (Clean Extraction)

```rust
#[test]
fn test_clean_pdf_extracts_without_errors() {
    let result = extract_pdf("tests/fixtures/clean.pdf", &Default::default())
        .expect("Extraction should succeed");
    
    // Verify no diagnostics were emitted
    assert!(
        result.metadata.diagnostics.is_empty(),
        "Expected no diagnostics, but got: {:?}",
        result.metadata.diagnostics
    );
    
    // Verify error count is zero
    assert_eq!(result.metadata.error_count, 0);
}
```

### Pattern 2: Check for Specific Error Code

```rust
#[test]
fn test_truncated_stream_emits_decode_error() {
    let result = extract_pdf("tests/fixtures/truncated.pdf", &Default::default())
        .expect("Extraction should succeed (with recovery)");
    
    // Check for STREAM_DECODE_ERROR diagnostic
    let has_decode_error = result.metadata.diagnostics.iter()
        .any(|d| d.contains("STREAM_DECODE_ERROR"));
    
    assert!(
        has_decode_error,
        "Expected STREAM_DECODE_ERROR diagnostic, got: {:?}",
        result.metadata.diagnostics
    );
}
```

### Pattern 3: Check for Multiple Error Types

```rust
#[test]
fn test_malformed_pdf_emits_multiple_warnings() {
    let result = extract_pdf("tests/fixtures/malformed.pdf", &Default::default())
        .expect("Extraction should succeed (with recovery)");
    
    // Check for multiple expected diagnostics
    let diagnostics = &result.metadata.diagnostics;
    
    assert!(
        diagnostics.iter().any(|d| d.contains("STRUCT_INVALID_NAME")),
        "Expected STRUCT_INVALID_NAME"
    );
    
    assert!(
        diagnostics.iter().any(|d| d.contains("STREAM_DECODE_ERROR")),
        "Expected STREAM_DECODE_ERROR"
    );
    
    assert!(
        diagnostics.iter().any(|d| d.contains("XREF_REPAIRED")),
        "Expected XREF_REPAIRED"
    );
}
```

### Pattern 4: Count Specific Errors

```rust
#[test]
fn test_pdf_with_unmapped_glyphs() {
    let result = extract_pdf("tests/fixtures/custom-font.pdf", &Default::default())
        .expect("Extraction should succeed");
    
    // Count unmapped glyph diagnostics
    let unmapped_count = result.metadata.diagnostics.iter()
        .filter(|d| d.contains("FONT_GLYPH_UNMAPPED"))
        .count();
    
    assert!(
        unmapped_count > 0,
        "Expected at least one FONT_GLYPH_UNMAPPED diagnostic"
    );
    
    println!("Found {} unmapped glyphs", unmapped_count);
}
```

### Pattern 5: Verify Error Message Content

```rust
#[test]
fn test_encryption_error_message() {
    let result = extract_pdf_encrypted("tests/fixtures/encrypted.pdf", None)
        .expect("Should produce error result");
    
    // Find the encryption error diagnostic
    let encryption_diag = result.metadata.diagnostics.iter()
        .find(|d| d.contains("ENCRYPTION_UNSUPPORTED"))
        .expect("Expected ENCRYPTION_UNSUPPORTED diagnostic");
    
    // Verify message contains expected text
    assert!(
        encryption_diag.contains("no password supplied") || 
        encryption_diag.contains("Unsupported encryption"),
        "Encryption error message should mention password or unsupported algorithm"
    );
}
```

### Pattern 6: Check Error Count Matches

```rust
#[test]
fn test_partial_extraction_error_count() {
    let result = extract_pdf("tests/fixtures/partial-corrupt.pdf", &Default::default())
        .expect("Extraction should succeed");
    
    // Verify error_count field matches actual diagnostics
    let actual_error_count = result.metadata.diagnostics.iter()
        .filter(|d| d.contains("ERROR") || d.contains("FATAL"))
        .count();
    
    assert_eq!(
        result.metadata.error_count as usize,
        actual_error_count,
        "error_count field should match number of ERROR/FATAL diagnostics"
    );
}
```

### Pattern 7: Assert No Fatal Errors

```rust
#[test]
fn test_extraction_has_no_fatal_errors() {
    let result = extract_pdf("tests/fixtures/complex.pdf", &Default::default())
        .expect("Extraction should succeed");
    
    // Ensure no fatal-level diagnostics were emitted
    let has_fatal = result.metadata.diagnostics.iter()
        .any(|d| {
            // Fatal codes include ENCRYPTION_UNSUPPORTED, REMOTE_TLS_FAILED, etc.
            d.contains("ENCRYPTION_UNSUPPORTED") ||
            d.contains("REMOTE_TLS_FAILED") ||
            d.contains("REMOTE_DNS_FAILED")
        });
    
    assert!(
        !has_fatal,
        "Extraction should not have fatal errors: {:?}",
        result.metadata.diagnostics
    );
}
```

### Pattern 8: Verify Expected Number of Diagnostics

```rust
#[test]
fn test_specific_diagnostic_count() {
    let result = extract_pdf("tests/fixtures/known-issues.pdf", &Default::default())
        .expect("Extraction should succeed");
    
    // If we know this PDF produces exactly 3 warnings
    assert_eq!(
        result.metadata.diagnostics.len(),
        3,
        "Expected exactly 3 diagnostics, got {}: {:?}",
        result.metadata.diagnostics.len(),
        result.metadata.diagnostics
    );
}
```

## Test Integration Examples

### Example 1: Truncated Flate Stream Test

```rust
#[test]
fn test_truncated_flate_stream_recovery() {
    let fixture_path = "tests/fixtures/malformed/truncated-flate.pdf";
    let result = extract_pdf(fixture_path, &Default::default())
        .expect("Extraction should succeed with recovery");
    
    // Should have emitted STREAM_DECODE_ERROR
    let has_stream_error = result.metadata.diagnostics.iter()
        .any(|d| d.contains("STREAM_DECODE_ERROR") || d.contains("STREAM_TRUNCATED"));
    
    assert!(has_stream_error, 
        "Expected stream decode error for truncated-flate.pdf");
    
    // Should still produce some output (recovery)
    assert!(!result.pages.is_empty(), 
        "Should produce at least one page despite stream error");
    
    // Verify the error message mentions truncation
    let error_msg = result.metadata.diagnostics.iter()
        .find(|d| d.contains("STREAM"))
        .expect("Should have stream-related diagnostic");
    
    assert!(error_msg.to_lowercase().contains("truncat") ||
            error_msg.to_lowercase().contains("incomplete"),
        "Stream error message should mention truncation or incomplete data");
}
```

### Example 2: Encryption Error Test

```rust
#[test]
fn test_encrypted_pdf_without_password() {
    let fixture_path = "tests/fixtures/encrypted/livecycle.pdf";
    let result = extract_pdf_encrypted(fixture_path, None);
    
    // Should fail with encryption error
    assert!(result.is_err(), "Should fail when encrypted PDF lacks password");
    
    // Check the error contains ENCRYPTION_UNSUPPORTED
    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(error_str.contains("ENCRYPTION_UNSUPPORTED") ||
                error_str.contains("encryption"),
                "Error should mention unsupported encryption: {}", error_str);
    }
}
```

### Example 3: Font Glyph Unmapped Test

```rust
#[test]
fn test_custom_font_with_unmapped_glyphs() {
    let fixture_path = "tests/fixtures/fonts/custom-subset.pdf";
    let result = extract_pdf(fixture_path, &Default::default())
        .expect("Extraction should succeed");
    
    // Should have FONT_GLYPH_UNMAPPED diagnostics
    let unmapped_diagnostics: Vec<_> = result.metadata.diagnostics.iter()
        .filter(|d| d.contains("FONT_GLYPH_UNMAPPED"))
        .collect();
    
    assert!(!unmapped_diagnostics.is_empty(),
        "Custom font PDF should have unmapped glyph diagnostics");
    
    // Each unmapped glyph produces U+FFFD in output
    let text = result.pages.iter()
        .flat_map(|p| p.blocks.iter().flat_map(|b| b.spans.iter().map(|s| &s.text)))
        .collect::<String>();
    
    let fffd_count = text.matches('�').count();
    assert!(fffd_count > 0,
        "Output should contain replacement character for unmapped glyphs");
}
```

## Best Practices for Test Assertions

1. **Be Specific**: Check for exact diagnostic codes when testing specific error conditions
2. **Allow Recovery**: Most tests should expect extraction to succeed despite errors (per INV-8 recovery principle)
3. **Verify Output**: When errors occur, verify that meaningful output was still produced
4. **Count Errors**: Use counts to verify expected number of diagnostics
5. **Check Messages**: Verify diagnostic messages contain expected information
6. **Test Severity**: Ensure fatal errors only occur for truly unrecoverable conditions
7. **Use Helper Functions**: Create reusable assertion helpers for common patterns

## Helper Functions for Common Assertions

```rust
/// Assert that extraction produces a specific diagnostic code
pub fn assert_has_diagnostic(result: &ExtractionResult, code: &str) {
    assert!(
        result.metadata.diagnostics.iter().any(|d| d.contains(code)),
        "Expected diagnostic containing '{}', got: {:?}",
        code,
        result.metadata.diagnostics
    );
}

/// Assert that extraction has no diagnostics
pub fn assert_no_diagnostics(result: &ExtractionResult) {
    assert!(
        result.metadata.diagnostics.is_empty(),
        "Expected no diagnostics, got: {:?}",
        result.metadata.diagnostics
    );
}

/// Count diagnostics containing a specific code
pub fn count_diagnostics(result: &ExtractionResult, code: &str) -> usize {
    result.metadata.diagnostics.iter()
        .filter(|d| d.contains(code))
        .count()
}

/// Get all diagnostics containing a specific code
pub fn get_diagnostics(result: &ExtractionResult, code: &str) -> Vec<&String> {
    result.metadata.diagnostics.iter()
        .filter(|d| d.contains(code))
        .collect()
}
```

## Summary

The errors array (`ExtractionResult.metadata.diagnostics`) provides comprehensive visibility into the PDF extraction process:

- **Format**: `Vec<String>` with each string formatted as `CODE: message (offset)?`
- **Access**: `result.metadata.diagnostics`
- **Error Count**: `result.metadata.error_count`
- **Assertion Patterns**: Check for specific codes, count errors, verify messages, ensure recovery
- **Categories**: STRUCT, STREAM, XREF, ENCRYPTION, PAGE, FONT, OCR, REMOTE, GSTATE, LAYOUT, MCP, CACHE, etc.

This unified diagnostic system allows tests to verify that errors are properly detected, reported, and recovered from during PDF extraction.
