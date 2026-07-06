# Errors Array Format and Test Integration Guide

## Overview

This document provides comprehensive documentation of the errors array format in pdftract extraction results and how to write test assertions for error conditions.

## Errors Array Structure

### JSON Output Format (Canonical)

When extraction results are converted to JSON via `result_to_output()`, errors are exposed in the top-level `errors` array:

```rust
// In Output struct (schema/mod.rs line 1539)
pub struct Output {
    pub schema_version: String,
    pub metadata: DocumentMetadata,
    pub pages: Vec<PageJson>,
    pub extraction_quality: ExtractionQuality,
    pub errors: Vec<DiagnosticJson>,  // ← Errors array here
}
```

### DiagnosticJson Structure

Each error in the `errors` array has the following structure:

```rust
pub struct DiagnosticJson {
    /// Stable string identifier (e.g., "STREAM_DECODE_ERROR")
    pub code: String,

    /// Human-readable description
    pub message: String,

    /// Severity level: "info", "warning", "error", or "fatal"
    pub severity: String,

    /// Page index where diagnostic occurred (null for document-level)
    pub page_index: Option<usize>,

    /// PDF object reference where issue originated
    pub location: Option<ObjectLocationJson>,

    /// Optional hint for resolution
    pub hint: Option<String>,
}
```

### ObjectLocationJson Structure

When a diagnostic includes a PDF object reference:

```rust
pub struct ObjectLocationJson {
    pub object_number: u32,      // Zero-based xref index
    pub generation_number: u16,  // Incremented on each save
}
```

## Internal ExtractionResult Format

Before JSON conversion, errors exist in two locations within `ExtractionResult`:

### 1. Document-Level Diagnostics

```rust
pub struct ExtractionResult {
    pub metadata: ExtractionMetadata,
    // ... other fields
}

pub struct ExtractionMetadata {
    pub error_count: usize,                    // Number of failed pages
    pub diagnostics: Vec<String>,              // ← Document-level diagnostics
    // ... other fields
}
```

**Purpose**: Contains warnings and errors affecting the entire extraction.

**Examples**:
- "XREF_REPAIRED: xref reconstructed via forward scan"
- "TAGGED_PDF_STRUCT_TREE_DEFERRED: StructTree traversal deferred to Phase 7.1"
- "COVERAGE_WARNING: Some content not fully analyzed"

### 2. Per-Page Errors

```rust
pub struct PageResult {
    pub index: usize,
    pub page_number: u32,
    pub error: Option<String>,  // ← Per-page error if extraction failed
    // ... other fields
}
```

**Purpose**: Contains error messages when a specific page fails to extract.

**Example**: `"PAGE_EXTRACTION_FAILED: Mediabox missing or invalid"`

## Severity Levels

Diagnostics have four severity levels that determine their impact:

| Severity | Impact | Examples |
|----------|--------|----------|
| `info` | Does not affect output validity | `XREF_REPAIRED`, `TAGGED_PDF_STRUCT_TREE_DEFERRED` |
| `warning` | Output usable but degraded | `STRUCT_INVALID_NAME`, `FONT_GLYPH_UNMAPPED`, `STREAM_DECODE_ERROR` |
| `error` | This region/page invalid; others OK | `STREAM_BOMB`, `PAGE_OUT_OF_RANGE`, `REMOTE_FETCH_INTERRUPTED` |
| `fatal` | Extraction aborted, no usable output | `ENCRYPTION_UNSUPPORTED`, `REMOTE_TLS_FAILED` |

## Diagnostic Code Categories

Error codes follow naming conventions with prefixes indicating the layer:

| Category | Prefix | Phase Origin | Examples |
|----------|--------|--------------|----------|
| Structure | `STRUCT_*` | 1.1 | `STRUCT_INVALID_NAME`, `STRUCT_MISSING_KEY` |
| Stream | `STREAM_*` | 1.5 | `STREAM_DECODE_ERROR`, `STREAM_BOMB`, `STREAM_UNKNOWN_FILTER` |
| XREF | `XREF_*` | 1.3 | `XREF_REPAIRED`, `XREF_INVALID_STREAM_FORMAT` |
| Encryption | `ENCRYPTION_*` | 1.4 | `ENCRYPTION_UNSUPPORTED` |
| Page | `PAGE_*` | 2.x | `PAGE_MISSING_MEDIABOX` |
| Font | `FONT_*` | 3.2 | `FONT_GLYPH_UNMAPPED` |
| Graphics State | `GSTATE_*` | 3.1 | `GSTATE_INVALID_MATRIX` |
| Layout | `LAYOUT_*` | 4.x | `LAYOUTReadingOrderBroken` |
| OCR | `OCR_*` | 5.x | `OCR_LANGUAGE_INVALID` |
| Remote | `REMOTE_*` | 1.8 | `REMOTE_FETCH_INTERRUPTED` |
| MCP | `MCP_*` | 6.7 | `MCP_REQUEST_TIMEOUT` |
| Cache | `CACHE_*` | 6.9 | `CACHE_WRITE_FAILED` |

## Accessing Errors from Extraction Results

### Method 1: From JSON Output (Recommended for Integration Tests)

```rust
use pdftract_core::{extract_pdf, ExtractionOptions, output::json::result_to_output};

let result = extract_pdf(&pdf_path, &ExtractionOptions::default())?;
let output = result_to_output(&result);

// Access all errors
for error in &output.errors {
    println!("Code: {}", error.code);
    println!("Message: {}", error.message);
    println!("Severity: {}", error.severity);
    if let Some(page_idx) = error.page_index {
        println!("Page: {}", page_idx);
    }
}

// Check for specific error codes
let has_decode_errors = output.errors
    .iter()
    .any(|e| e.code == "STREAM_DECODE_ERROR");
```

### Method 2: From ExtractionResult (For Unit Tests)

```rust
use pdftract_core::{extract_pdf, ExtractionOptions};

let result = extract_pdf(&pdf_path, &ExtractionOptions::default())?;

// Document-level diagnostics
for diag in &result.metadata.diagnostics {
    println!("{}", diag);
}

// Check error count
assert_eq!(result.metadata.error_count, 0, "Expected no failed pages");

// Per-page errors
for page in &result.pages {
    if let Some(error) = &page.error {
        eprintln!("Page {} error: {}", page.index, error);
    }
}
```

## Assertion Patterns for Error Conditions

### Pattern 1: Assert No Errors (Clean Extraction)

```rust
let result = extract_pdf(&pdf_path, &ExtractionOptions::default())?;
let output = result_to_output(&result);

// Assert no errors with severity >= warning
let has_warnings_or_errors = output.errors
    .iter()
    .any(|e| e.severity != "info");

assert!(!has_warnings_or_errors,
    "Expected clean extraction but found errors: {:?}",
    output.errors);
```

### Pattern 2: Assert Specific Error Code Present

```rust
let result = extract_pdf(&truncated_pdf, &ExtractionOptions::default())?;
let output = result_to_output(&result);

// Check that STREAM_DECODE_ERROR is present
let has_decode_error = output.errors
    .iter()
    .any(|e| e.code == "STREAM_DECODE_ERROR");

assert!(has_decode_error,
    "Expected STREAM_DECODE_ERROR in output");
```

### Pattern 3: Assert Error Count

```rust
let output = result_to_output(&result);

// Should have exactly one error
assert_eq!(output.errors.len(), 1,
    "Expected exactly 1 error, found {}",
    output.errors.len());

// First error should be warning severity
assert_eq!(output.errors[0].severity, "warning");
```

### Pattern 4: Assert Error on Specific Page

```rust
let output = result_to_output(&result);

// Find errors on page 2
let page_2_errors: Vec<_> = output.errors
    .iter()
    .filter(|e| e.page_index == Some(2))
    .collect();

assert!(!page_2_errors.is_empty(),
    "Expected errors on page 2");
```

### Pattern 5: Assert Error Message Contains Text

```rust
let output = result_to_output(&result);

// Find error with specific message content
let has_truncation_error = output.errors
    .iter()
    .any(|e| e.message.contains("truncated"));

assert!(has_truncation_error,
    "Expected error message to contain 'truncated'");
```

### Pattern 6: Assert No Fatal Errors

```rust
let output = result_to_output(&result);

// Check extraction didn't fail completely
let has_fatal = output.errors
    .iter()
    .any(|e| e.severity == "fatal");

assert!(!has_fatal,
    "Expected no fatal errors but found: {:?}",
    output.errors.iter().filter(|e| e.severity == "fatal").collect::<Vec<_>>());
```

## Complete Test Example

Here's a complete test demonstrating error assertion patterns:

```rust
#[test]
fn test_truncated_flate_recovery() {
    use pdftract_core::{extract_pdf, ExtractionOptions, output::json::result_to_output};
    use std::path::PathBuf;

    let pdf_path = PathBuf::from("tests/fixtures/malformed/truncated-flate.pdf");
    let result = extract_pdf(&pdf_path, &ExtractionOptions::default())
        .expect("Extraction should succeed (recovery mode)");

    let output = result_to_output(&result);

    // Pattern 1: Extraction should succeed (no fatal errors)
    let has_fatal = output.errors
        .iter()
        .any(|e| e.severity == "fatal");
    assert!(!has_fatal, "Extraction should not fail fatally");

    // Pattern 2: Should detect the stream decode error
    let has_decode_error = output.errors
        .iter()
        .any(|e| e.code == "STREAM_DECODE_ERROR");
    assert!(has_decode_error, "Should detect truncated flate stream");

    // Pattern 3: Should have at least one warning/error
    assert!(!output.errors.is_empty(), "Should have diagnostics");

    // Pattern 4: Message should mention truncation
    let has_truncation_msg = output.errors
        .iter()
        .any(|e| e.message.to_lowercase().contains("truncat"));
    assert!(has_truncation_msg, "Error message should mention truncation");

    // Pattern 5: Should still extract some pages (recovery)
    assert!(output.pages.len() > 0, "Should extract at least one page");
}
```

## Diagnostic Code Reference

### Common Structure Errors
- `STRUCT_INVALID_NAME` - Invalid name character or length > 127 bytes
- `STRUCT_INVALID_HEX` - Invalid hex character in hex string
- `STRUCT_INVALID_OCTAL` - Invalid octal escape sequence
- `STRUCT_INVALID_STREAM_HEADER` - Stream keyword not followed by proper newline
- `STRUCT_UNEXPECTED_BYTE` - Lexer encountered unexpected byte
- `STRUCT_UNEXPECTED_EOF` - File ended mid-token
- `STRUCT_UNTERMINATED_STRING` - Missing closing paren on literal string
- `STRUCT_MISSING_KEY` - Required dictionary key absent

### Common Stream Errors
- `STREAM_DECODE_ERROR` - Decompression failed (corrupt data)
- `STREAM_BOMB` - Decompressed size exceeds max_decompress_bytes
- `STREAM_UNKNOWN_FILTER` - Unsupported filter name
- `STREAM_INVALID_PARAMS` - Invalid /DecodeParms dictionary
- `STREAM_INVALID_JPEG` - Missing SOI/EOI markers

### Common XREF Errors
- `XREF_REPAIRED` - Xref reconstructed via forward scan (EC-07)
- `XREF_LINEARIZED_NO_FORWARD_SCAN` - Forward scan disabled for linearized PDF
- `XREF_REMOTE_NO_FORWARD_SCAN` - Forward scan disabled for HTTP sources
- `XREF_INVALID_STREAM_FORMAT` - Malformed xref stream
- `XREF_INVALID_STREAM_ENTRY` - Unparseable xref stream entry

## Adding Assertions to Existing Tests

### Step 1: Identify Expected Error Conditions

Before adding assertions, determine what error conditions the test should verify:

1. **Does the fixture have known issues?** (e.g., truncated streams, corrupt xref)
2. **What diagnostic codes should be emitted?** (check fixture `.meta` files)
3. **What severity level is expected?** (info/warning/error/fatal)
4. **Which pages are affected?** (document-level vs page-specific)

### Step 2: Add Error Collection to Test

Modify the test to collect errors:

```rust
// Before
let result = extract_pdf(&pdf_path, &options)?;
assert!(result.pages.len() > 0);

// After
let result = extract_pdf(&pdf_path, &options)?;
let output = result_to_output(&result);

assert!(output.pages.len() > 0);
// Now we can also assert on output.errors
```

### Step 3: Add Targeted Assertions

Add assertions that verify the expected error conditions:

```rust
// Example: Assert expected warning is present
let expected_warning = output.errors
    .iter()
    .find(|e| e.code == "STREAM_DECODE_ERROR");
assert!(expected_warning.is_some(), "Missing expected STREAM_DECODE_ERROR");
```

### Step 4: Document Expected Behavior

Add comments explaining why the assertion exists:

```rust
// This fixture has a truncated flate stream, so we expect
// STREAM_DECODE_ERROR but extraction should still succeed
// (recovery per EC-07)
assert!(has_decode_error);
assert!(output.pages.len() > 0);
```

## Conversion Flow: Internal → JSON

Understanding how errors flow from internal diagnostics to JSON output:

```
1. Emission (during parsing)
   emit!(diagnostics, STREAM_DECODE_ERROR, offset = 12345);

2. Internal Diagnostic
   Diagnostic {
       code: DiagCode::StreamDecodeError,
       byte_offset: Some(12345),
       object_ref: None,
       message: Cow::Borrowed("zlib stream truncated mid-inflation"),
   }

3. String conversion (for ExtractionResult)
   "STREAM_DECODE_ERROR: zlib stream truncated mid-inflation (offset 12345)"

4. JSON conversion (result_to_output)
   DiagnosticJson {
       code: "STREAM_DECODE_ERROR".to_string(),
       message: "zlib stream truncated mid-inflation".to_string(),
       severity: "warning".to_string(),
       page_index: None,
       location: None,
       hint: None,
   }
```

## Key Files Reference

- `crates/pdftract-core/src/diagnostics.rs` - Diagnostic system, `Diagnostic` struct, `DiagCode` enum
- `crates/pdftract-core/src/extract.rs` - `ExtractionResult`, `ExtractionMetadata`
- `crates/pdftract-core/src/schema/mod.rs` - `Output`, `DiagnosticJson` structs
- `crates/pdftract-core/src/output/json.rs` - `result_to_output()`, `convert_diagnostics()`
- `crates/pdftract-core/tests/error_recovery_integration.rs` - Existing error test patterns

## Test Fixture Convention

When adding new test fixtures that expect errors:

1. Create `.meta` file documenting expected diagnostics:
   ```
   filter: FlateDecode
   expected_diagnostics: STREAM_DECODE_ERROR
   description: Mid-stream EOF during decompression
   ```

2. Add assertions to match expected diagnostics:
   ```rust
   let meta = load_fixture_meta("truncated-flate.meta");
   for expected_code in meta.expected_diagnostics {
       let found = output.errors.iter().any(|e| e.code == expected_code);
       assert!(found, "Missing expected diagnostic: {}", expected_code);
   }
   ```

## Acceptance Criteria Status

✅ **Complete errors array documentation created**
- Documented JSON `Output.errors` array structure
- Documented `DiagnosticJson` fields and types
- Documented severity levels and impact

✅ **Assertion integration guide written**
- Provided 6 common assertion patterns with examples
- Included complete working test example
- Documented step-by-step process for adding assertions

✅ **Examples provided for common error checking patterns**
- Pattern 1: Assert no errors (clean extraction)
- Pattern 2: Assert specific error code present
- Pattern 3: Assert error count
- Pattern 4: Assert error on specific page
- Pattern 5: Assert error message contains text
- Pattern 6: Assert no fatal errors
