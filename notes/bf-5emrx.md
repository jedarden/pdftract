# Verification Note: bf-5emrx

## Task
Write failing test case for assert_exit_code

## Summary
Added comprehensive test `test_extraction_result_assert_exit_code_error_message` that verifies `assert_exit_code` returns an error when the exit code does not match the expected value.

## Implementation

### File Modified
- `crates/pdftract-core/src/extract.rs` (lines 3169-3213)

### Test Added
```rust
#[test]
fn test_extraction_result_assert_exit_code_error_message()
```

This test:
1. Creates an `ExtractionResult` with exit code 1 (error_count = 1)
2. Calls `assert_exit_code(0)` expecting a different value
3. Asserts the method returns an `Err(...)`
4. Verifies the error message contains useful information about the mismatch

### Test Coverage
- Verifies that `assert_exit_code(0)` returns `Err` when actual exit code is 1
- Validates error.expected = 0
- Validates error.actual = 1
- Checks error.description contains "extraction result had"
- Checks error.description contains "error"
- Validates error message Display formatting contains "expected 0" and "got 1"

## Test Results

### Command
```bash
cargo test --package pdftract-core --lib test_extraction_result_assert_exit_code_error_message
```

### Output
```
running 1 test
test extract::tests::test_extraction_result_assert_exit_code_error_message ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2903 filtered out
```

### Related Tests
All 4 assert_exit_code tests pass:
- `test_extraction_result_assert_exit_code_error_message` (new)
- `test_extraction_result_assert_exit_code_mismatch`
- `test_extraction_result_assert_exit_code_success`
- `test_extraction_result_assert_exit_code_with_errors`

## Acceptance Criteria

### PASS Criteria
✅ Test case exists for failing scenario
✅ Test creates ExtractionResult with exit code 1 (error_count = 1)
✅ Test calls assert_exit_code(0) and expects Err(...)
✅ Test verifies error message contains useful information
✅ Test passes when run (detecting the failure correctly)

### WARN Criteria
None - all acceptance criteria met successfully

## Git Commit
Commit: `test(bf-5emrx): add assert_exit_code error message verification test`

## Verification Notes
The test successfully verifies that `assert_exit_code` properly returns an `AssertionError` with:
- Correct expected value (0)
- Correct actual value (1)
- Descriptive error message mentioning the extraction result had errors

The test integrates seamlessly with the existing test suite and provides comprehensive coverage for the failure scenario.
