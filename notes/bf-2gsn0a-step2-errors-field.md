# Extraction Result Type and Errors Field Analysis

## Task
Identify extraction result type and errors field within the test flow.

## ExtractionResult Type Definition

**Location:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:232-286`

### Structure
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
    #[serde(default)]
    pub javascript_actions: Vec<JavascriptActionJson>,
}
```

### PageResult Error Field
**Location:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:287-337`

```rust
pub struct PageResult {
    pub index: usize,
    pub page_number: u32,
    pub page_label: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub rotation: Option<u16>,
    pub page_type: Option<String>,
    pub spans: Vec<SpanJson>,
    pub blocks: Vec<BlockJson>,
    pub tables: Vec<TableJson>,
    #[serde(default)]
    pub annotations: Vec<AnnotationJson>,
    /// Error message if extraction failed for this page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,  // <-- PER-PAGE ERROR FIELD
}
```

### ExtractionMetadata Error Count
**Location:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:393-426`

```rust
pub struct ExtractionMetadata {
    pub page_count: usize,
    pub receipts_mode: ReceiptsMode,
    pub span_count: usize,
    pub block_count: usize,
    pub cache_status: Option<String>,
    pub cache_age_seconds: Option<u64>,
    /// Number of pages that failed to extract.
    pub error_count: usize,  // <-- TOTAL ERROR COUNT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_order_algorithm: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_fields: Option<serde_json::Value>,
}
```

## Error Field Population Flow

### 1. Page Extraction Error Population
**Location:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:826-857`

```rust
match extract_result {
    Ok(Ok(mut page)) => {
        total_spans += page.spans.len();
        total_blocks += page.blocks.len();
        page.annotations = page_annotations;
        extracted_pages.push(page);
    }
    Ok(Err(e)) => {
        error_count += 1;  // <-- INCREMENT ERROR COUNT
        extracted_pages.push(PageResultInternal {
            index: page_index,
            spans: vec![],
            blocks: vec![],
            tables: vec![],
            annotations: page_annotations,
            error: Some(e.to_string()),  // <-- SET ERROR MESSAGE
            page_height,
        });
    }
    Err(_) => {
        error_count += 1;  // <-- INCREMENT ERROR COUNT FOR PANIC
        extracted_pages.push(PageResultInternal {
            index: page_index,
            spans: vec![],
            blocks: vec![],
            tables: vec![],
            annotations: page_annotations,
            error: Some(format!("Page {} extraction panicked", page_index)),  // <-- PANIC MESSAGE
            page_height,
        });
    }
}
```

### 2. Error Count Assignment to Metadata
**Location:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:1023-1036`

```rust
Ok(ExtractionResult {
    fingerprint,
    pages: extracted_pages,
    metadata: ExtractionMetadata {
        page_count,
        receipts_mode: options.receipts,
        span_count: total_spans,
        block_count: total_blocks,
        cache_status: None,
        cache_age_seconds: None,
        error_count,  // <-- TOTAL ERROR COUNT
        reading_order_algorithm: Some(final_reading_order_algorithm.as_str().to_string()),
        diagnostics: all_diagnostics_with_js,
        profile_name: None,
        profile_version: None,
        profile_fields: None,
    },
    // ... other fields
})
```

## Test Flow Error Representation

### TestResult Structure
**Location:** `/home/coding/pdftract/crates/pdftract-core/tests/conformance.rs:56-64`

```rust
#[derive(Debug)]
struct TestResult {
    id: String,
    passed: bool,
    skipped: bool,
    skip_reason: Option<String>,
    errors: Vec<String>,  // <-- TEST ERRORS ARRAY
}
```

### Error Array Population in Tests
**Location:** `/home/coding/pdftract/crates/pdftract-core/tests/conformance.rs:960-971`

```rust
match run_result {
    Ok((_actual, errors)) => {
        test_result.errors = errors;  // <-- ASSIGN ERRORS ARRAY
        test_result.passed = test_result.errors.is_empty();
    }
    Err(e) => {
        test_result
            .errors
            .push(format!("Test execution error: {}", e));  // <-- ADD EXECUTION ERROR
        test_result.passed = false;
    }
}
```

### Error Comparison Function
**Location:** `/home/coding/pdftract/crates/pdftract-core/tests/conformance.rs:173-272`

The `compare_with_tolerances` function recursively compares expected vs actual results and accumulates errors:

```rust
fn compare_with_tolerances(
    actual: &Value,
    expected: &Value,
    tolerances: &Value,
    path: &str,
) -> Vec<String> {
    let mut errors = Vec::new();  // <-- CREATE ERROR ARRAY
    
    match (expected, actual) {
        (Value::Object(exp_map), _) => {
            for (key, exp_value) in exp_map {
                let field_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                
                let act_value = match resolve_path(actual, &field_path) {
                    Some(v) => v,
                    None => {
                        errors.push(format!("Missing field: {}", field_path));  // <-- ADD ERROR
                        continue;
                    }
                };
                
                let field_errors = compare_with_tolerances(act_value, exp_value, tolerances, &field_path);
                errors.extend(field_errors);  // <-- EXTEND WITH SUB-FIELD ERRORS
            }
        }
        // ... more comparison logic
    }
    
    errors  // <-- RETURN ERROR ARRAY
}
```

## Error Representation Pattern

### PDF Extraction Level
- **Per-page errors:** `PageResult.error: Option<String>` - individual error message for each failed page
- **Total error count:** `ExtractionMetadata.error_count: usize` - count of failed pages
- **Diagnostics:** `ExtractionMetadata.diagnostics: Vec<String>` - warnings and info messages

### Test Conformance Level
- **Test errors:** `TestResult.errors: Vec<String>` - array of assertion failures from comparing expected vs actual results
- **Error types:**
  - Missing field errors
  - Value mismatch errors (with tolerance support)
  - Array length errors
  - Test execution errors

## Key Findings

1. **No top-level `errors` array in ExtractionResult** - The main extraction result type uses per-page `error: Option<String>` fields instead
2. **Two-tier error system** - Page-level errors (individual failures) + metadata-level count (total failures)
3. **Test errors are separate** - Test framework uses its own `Vec<String>` errors array for assertion failures
4. **Error accumulation pattern** - Tests recursively build error arrays by extending with sub-component errors
5. **Panic handling** - Panics during page extraction are caught and converted to error messages

## File Locations Summary

- **ExtractionResult definition:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:232-286`
- **PageResult.error field:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:336`
- **Error population logic:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs:826-857`
- **TestResult.errors array:** `/home/coding/pdftract/crates/pdftract-core/tests/conformance.rs:63`
- **Error comparison logic:** `/home/coding/pdftract/crates/pdftract-core/tests/conformance.rs:173-272`
