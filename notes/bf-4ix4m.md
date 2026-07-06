# Analysis of Extraction Result Error Structure

## Overview
This document analyzes how errors are structured in pdftract-core's extraction result.

## Findings

### 1. Internal ExtractionResult Structure
**Location:** `crates/pdftract-core/src/extract.rs`

The `ExtractionResult` struct contains:
- `pages: Vec<PageResult>` - Each page may have page-level errors
- `metadata: ExtractionMetadata` - Document-level error metadata

### 2. PageResult Error Field
**Location:** `crates/pdftract-core/src/extract.rs:336`

Each `PageResult` includes:
```rust
/// Error message if extraction failed for this page.
#[serde(skip_serializing_if = "Option::is_none")]
pub error: Option<String>,
```

When a page extraction fails:
- `error` is set to `Some(e.to_string())` with the error message
- The page still appears in the pages array with empty spans/blocks/tables
- `metadata.error_count` is incremented

### 3. ExtractionMetadata Error Tracking
**Location:** `crates/pdftract-core/src/extract.rs:394-426`

```rust
pub struct ExtractionMetadata {
    /// Number of pages that failed to extract.
    pub error_count: usize,
    
    /// Diagnostics emitted during extraction (coverage warnings, etc.)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    
    // ... other fields
}
```

The `diagnostics` array contains string messages like:
- Tagged PDF deferred warnings
- Coverage check failures
- Page range warnings
- JavaScript detection notices

### 4. JSON Output Schema
**Location:** `crates/pdftract-core/src/schema/mod.rs:1487-1540`

The canonical `Output` schema includes:
```rust
pub struct Output {
    /// All diagnostics emitted during extraction.
    #[serde(default)]
    pub errors: Vec<DiagnosticJson>,
    
    // ... other fields
}
```

### 5. DiagnosticJson Structure
**Location:** `crates/pdftract-core/src/schema/mod.rs:813-834`

Each error in the `errors` array contains:
```rust
pub struct DiagnosticJson {
    /// Stable string identifier (e.g., "FONT_GLYPH_UNMAPPED")
    pub code: String,
    
    /// Human-readable description
    pub message: String,
    
    /// Severity level: "info", "warning", "error", or "fatal"
    pub severity: String,
    
    /// Page index where this diagnostic occurred (null for document-level)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<usize>,
    
    /// PDF object reference where the issue originated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<ObjectLocationJson>,
    
    /// Optional hint for resolving the diagnostic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}
```

### 6. Error Type Categories

Based on analysis of the codebase, errors fall into these categories:

1. **Page Extraction Errors** (captured in `PageResult.error`)
   - Content stream decoding failures
   - Panic/exception during page processing
   - Individual page extraction failures

2. **Document-Level Diagnostics** (captured in `ExtractionMetadata.diagnostics`)
   - Tagged PDF deferred notices
   - StructTree coverage warnings
   - JavaScript detection notices
   - Page range warnings
   - Cache status information

3. **Canonical JSON Output Errors** (in `Output.errors`)
   - Converted from `ExtractionMetadata.diagnostics` strings
   - Structured as `DiagnosticJson` objects with code, severity, and location

## Error Data Flow

```
Page Extraction Failure
    ↓
PageResult { error: Some("message"), spans: [], blocks: [], tables: [] }
    ↓
ExtractionMetadata { error_count: N }
    ↓
[Internal ExtractionResult]

String Diagnostics (ExtractionMetadata.diagnostics)
    ↓
convert_diagnostics() function (output/json.rs:146-183)
    ↓
Parse "CODE: message" format, determine severity from code
    ↓
Vec<DiagnosticJson> { code, message, severity, page_index, location, hint }
    ↓
Output { errors: Vec<DiagnosticJson> }
    ↓
[Canonical JSON Output]
```

## Key Insights

1. **Dual Error Representation**: Internally, page errors live on each `PageResult`, but externally they're elevated to document-level `errors` array

2. **String-to-Structured Conversion**: The `convert_diagnostics()` function attempts to parse error codes from string messages:
   - Format: `"CODE: message"` or just `"message"`
   - Severity inferred from code prefixes (`ERROR_*` → "error", `WARN_*` → "warning")

3. **Page-Level Errors Not in Output.errors**: Page extraction errors stored in `PageResult.error` are NOT automatically converted to `DiagnosticJson` - only document-level diagnostics are

## Files Analyzed

1. `/home/coding/pdftract/crates/pdftract-core/src/extract.rs` - Internal extraction result types
2. `/home/coding/pdftract/crates/pdftract-core/src/output/json.rs` - Conversion to JSON output
3. `/home/coding/pdftract/crates/pdftract-core/src/schema/mod.rs` - JSON schema definitions

## Acceptance Criteria Status

✅ Extraction result type understood
✅ Errors array structure identified  
✅ Error types and format documented
