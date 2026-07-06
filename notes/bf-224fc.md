# bf-224fc: Add basic test function skeleton to forms_integration.rs

## Status: COMPLETE

## Summary

The file `tests/forms_integration.rs` already contains multiple test functions that meet all acceptance criteria.

## Acceptance Criteria Verification

### ✅ File contains at least one `#[test]` function
The file contains 3 test functions:
1. `test_discover_pdf_fixtures` (line 42) - Tests PDF fixture discovery
2. `test_cli_extract_json_on_fixtures` (line 127) - Tests CLI invocation with bounded timeout
3. `test_forms_extraction` (line 233) - Tests forms extraction and JSON serialization

### ✅ Test function compiles without syntax errors
Verified with `cargo test --test forms_integration --no-run` - compilation succeeds.

### ✅ Test function does something non-empty
All three test functions contain:
- Assertions (e.g., `assert!(bin.exists(), ...)`)
- Logic for discovering fixtures
- CLI invocation with timeout protection
- JSON serialization verification
- Print statements for debugging

## Additional Notes

The test file includes:
- `discover_pdf_fixtures()` helper function for recursive PDF discovery
- `pdftract_bin()` helper to locate the compiled binary
- `wait_with_timeout()` helper to prevent hanging tests (critical for test hygiene)
- Graceful handling when no fixtures exist (tests return early rather than fail)

## Test Hygiene Considerations

The tests implement the timeout protection pattern recommended in CLAUDE.md:
- Uses `wait_with_timeout()` with bounded waits (10 seconds per fixture)
- Falls back to release binary if debug binary not found
- Returns early when fixtures directory is empty
- Does not fail the entire test suite if fixtures are missing

This prevents the exact issue described in the test hygiene section where hung tests can stall the entire marathon.

## Verification

- File path: `tests/forms_integration.rs`
- Compilation: PASS (cargo check --tests)
- Test function count: 3
- Code review: All tests are non-empty and functional
