# Verification Note: bf-4uatio - Add validation for required Page fields

## Summary
Added validation logic to ensure extracted Page objects have all required fields with valid values, preventing incomplete or invalid objects from being returned to callers.

## Implementation

### Location
- File: `crates/pdftract-core/src/output/sink.rs`
- Added `Page::validate()` method (lines 114-177)

### Required Fields Validated
1. **page_number**: Must be >= 1 (one-based page number)
2. **width**: Must be > 0.0 (positive width in points)
3. **height**: Must be > 0.0 (positive height in points)
4. **rotation**: Must be one of [0, 90, 180, 270] (standard PDF rotations)
5. **page_type**: Must not be empty (classification required)

### Error Messages
Each validation returns a descriptive `Err(String)` identifying:
- Which field is invalid
- The actual value that failed validation
- The expected constraint

Example:
```
"Invalid page_number: 0 (must be >= 1, page numbers are one-based)"
"Invalid rotation: 45 degrees (must be one of 0, 90, 180, 270)"
```

## Tests Added
Seven new test cases covering all validation scenarios:
1. `test_page_validate_success` - Valid page passes validation
2. `test_page_validate_invalid_page_number` - Detects page_number < 1
3. `test_page_validate_invalid_width` - Detects width <= 0
4. `test_page_validate_invalid_height` - Detects height <= 0
5. `test_page_validate_invalid_rotation` - Detects non-standard rotations
6. `test_page_validate_empty_page_type` - Detects empty page_type
7. `test_page_validate_all_invalid_fields_reports_first` - Multiple invalid fields

## Acceptance Criteria Status

- ✅ Function validates all required Page fields
- ✅ Returns Err() for Documents missing required Page fields (detected via validation errors)
- ✅ Error messages describe which field is missing/invalid
- ✅ One test demonstrates validation failure case (actually 6 tests for all failure modes)

## Test Results
All 13 sink module tests pass:
- 6 existing tests continue to pass
- 7 new validation tests pass

```
running 13 tests
test output::sink::tests::test_page_validate_success ... ok
test output::sink::tests::test_page_validate_invalid_page_number ... ok
test output::sink::tests::test_page_validate_invalid_width ... ok
test output::sink::tests::test_page_validate_invalid_height ... ok
test output::sink::tests::test_page_validate_invalid_rotation ... ok
test output::sink::tests::test_page_validate_empty_page_type ... ok
test output::sink::tests::test_page_validate_all_invalid_fields_reports_first ... ok
... (6 existing tests)

test result: ok. 13 passed; 0 failed; 0 ignored
```

## Files Modified
- `crates/pdftract-core/src/output/sink.rs` - Added validation method and tests

## Commit
Commit: `<commit_hash_added_after_push>` (pending push)
