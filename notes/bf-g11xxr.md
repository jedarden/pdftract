# Verification Note: bf-g11xxr - Wire pdftract binary invocation in classify_page

## Summary
Fixed the pdftract binary invocation argument construction in `classify_from_temp_file` function.

## Work Completed

### File Modified
- `crates/pdftract-core/src/sdk.rs` (line 438)

### Change Made
Removed erroneous `-` argument from pdftract binary invocation. The command was incorrectly:
```rust
let output = Command::new(&pdftract_binary)
    .arg("extract")
    .arg("--json")
    .arg("-")  // ❌ Erroneous - argument
    .arg(&temp_file)
    .output()
```

Fixed to:
```rust
let output = Command::new(&pdftract_binary)
    .arg("extract")
    .arg("--json")
    .arg(&temp_file)  // ✓ PDF input file
    .output()
```

### Commit
- Commit: `b1d6aa35` 
- Message: "fix(bf-g11xxr): correct pdftract binary invocation argument construction"

## Acceptance Criteria Verification

✓ **pdftract binary is invoked with correct arguments**
  - Command: `pdftract extract --json /path/to/temp/file.pdf`
  - Arguments: `["extract", "--json", temp_file_path]`

✓ **Temp file path is passed as input**
  - Line 438: `.arg(&temp_file)` passes the temp file as the input argument

✓ **Subprocess is configured correctly**
  - Line 439: `.output()` configures stdout/stderr capture
  - Stderr is captured and used in error reporting (line 444)

✓ **Command construction is correct**
  - No erroneous stdin marker (`-`)
  - Proper argument order: subcommand → flag → input file

✓ **Code compiles without errors**
  - `cargo check --package pdftract-core`: SUCCESS
  - No compilation errors (62 warnings, 0 errors)

## Test Results

### Test Suite: `smoke_test_classify_page`
```
running 5 tests
test test_classify_basic_scanned_page ... ok
test test_classify_page_fixture_exists ... ok  
test test_classify_basic_vector_page ... ok
test test_classify_page_output_format_comprehensive ... ok
test test_classify_page_returns_valid_result_for_valid_input ... ok

test result: ok. 5 passed; 0 failed; 0 ignored
```

**Result:** PASS (5/5 tests)

## Verification Details

### Command Structure
The corrected invocation follows the proper pdftract CLI structure:
1. Binary: `pdftract` (found via `find_pdftract_binary()`)
2. Subcommand: `extract`
3. Output format: `--json` 
4. Input: `<temp_file_path>` (the PDF file to process)

### Output Handling
- Stdout: Captured and parsed as JSON (line 453-457)
- Stderr: Captured and included in error messages (line 444)
- Exit code: Checked via `output.status.success()` (line 443)

### Integration Context
This function is called from `classify_page()` which creates a temp file for single-page PDFs, then invokes this function to process it via the pdftract binary.

## Related Work
- Parent bead: `bf-4ndidi` (classify_page implementation)
- Previous bead: `bf-cu80om` (temp file creation logic)
- Follows bead `bf-2e17h0` (standard library imports)

## Status
**COMPLETE** - All acceptance criteria met, tests pass, code compiles successfully.
