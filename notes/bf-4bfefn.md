# bf-4bfefn: Implement output parsing and error handling for classify_page

## Status: VERIFICATION COMPLETE

### Implementation Location
`crates/pdftract-core/src/sdk.rs:421-533` - `classify_from_temp_file` function

### Acceptance Criteria Status

#### 1. ✓ Captured output is parsed correctly
- **Lines 452-457**: Converts stdout to UTF-8 and parses JSON
- **Lines 460-477**: Extracts and validates pages array
- **Lines 479-487**: Extracts page_type and confidence fields
- **Lines 495-508**: Maps page_type strings to PageClass enum
- **Lines 511-526**: Extracts hybrid_cells for Hybrid pages

#### 2. ✓ Return value matches expected interface
- **Lines 528-532**: Returns `PageClassification` struct with:
  - `class: PageClass` (mapped from page_type string)
  - `confidence: f32` (from JSON or default 0.5)
  - `hybrid_cells: Option<BTreeSet<usize>>` (for Hybrid pages)

#### 3. ✓ Subprocess failures are properly reported
- **Lines 443-450**: Checks exit status and returns error with:
  - Exit code from output.status.code()
  - Full stderr output for diagnostics
  - Context about the pdftract binary path

#### 4. ✓ Errors are propagated with useful context
All error paths include `.with_context()` providing:
- **Line 443-450**: Subprocess failure with exit code and stderr
- **Line 453**: UTF-8 conversion failure context
- **Line 454**: JSON parsing failure context
- **Line 463**: Missing 'pages' array error
- **Line 467**: Empty pages array error
- **Lines 471-477**: Page index out of bounds with actual bounds
- **Lines 484-487**: Missing 'page_type' field error
- **Lines 502-507**: Unknown page_type value with valid options

#### 5. ✓ Code compiles without errors
Verified with `cargo check --lib -p pdftract-core` - no errors in sdk.rs

### Output Format Validation

The function validates the JSON structure:
- Ensures `pages` array exists and is non-empty
- Validates page_index is within bounds (0 to pages.len()-1)
- Requires `page_type` field to be present
- Handles all 6 valid page_type values: mixed, text, scanned, broken_vector, blank, figure_only
- Extracts optional `confidence` field (defaults to 0.5 if missing)

### Page Type Mapping

```
JSON page_type → PageClass enum:
- "mixed"         → PageClass::Hybrid
- "text"          → PageClass::Vector
- "scanned"       → PageClass::Scanned
- "broken_vector" → PageClass::BrokenVector
- "blank"         → PageClass::Vector (no content)
- "figure_only"   → PageClass::Scanned (image-only)
```

### Error Context Examples

```rust
// Subprocess failure
"pdftract extraction failed with exit code Some(1). stderr: <actual stderr output>"

// UTF-8 conversion failure
"Failed to convert pdftract output to UTF-8"

// JSON parsing failure
"Failed to parse pdftract JSON output"

// Missing pages array
"JSON output missing required 'pages' array"

// Page index out of bounds
"Page index 5 out of bounds (PDF has 3 pages)"

// Unknown page_type
"Unknown page_type 'invalid'. Expected one of: mixed, text, scanned, broken_vector, blank, figure_only"
```

### Integration Points

This function is called by:
1. `classify()` (line 369) - from file path
2. `classify_page()` (line 418) - from PDF bytes

Both use `create_temp_pdf_file()` to handle temporary file creation with RAII cleanup.

### Verification Method
- Code review of `classify_from_temp_file` function
- Compilation check: `cargo check --lib -p pdftract-core`
- Confirmed all error paths have proper context
- Verified return type matches `PageClassification` interface
- Validated output parsing handles all required JSON fields

### Dependencies
- Child bead 3 (output capture) must be complete for this bead to succeed
- The implementation assumes the pdftract binary is installed and accessible via `find_pdftract_binary()`
