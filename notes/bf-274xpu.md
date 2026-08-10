# Bead bf-274xpu: catalog_dict_missing_essential_keys helper function

## Summary
The `catalog_dict_missing_essential_keys` helper function already existed in `crates/pdftract-core/src/parser/catalog.rs`. Fixed a compilation error in `document.rs` where the function was being called incorrectly.

## Work Done

### Fixed Compilation Error
**File:** `crates/pdftract-core/src/document.rs` (line 766)

**Before:**
```rust
if catalog_dict_missing_essential_keys(&catalog.raw_dict) {
```

**After:**
```rust
if catalog_dict_missing_essential_keys(&catalog) {
```

The function signature is `catalog_dict_missing_essential_keys(catalog: &Catalog) -> bool`, not `catalog_dict_missing_essential_keys(catalog_dict: &PdfObject) -> bool`. The caller was passing `&catalog.raw_dict` (a `&PdfObject`) instead of `&catalog`.

### Function Details
The function exists in two forms:

1. **Low-level function:** `is_catalog_dict_missing_essential_keys(catalog_dict: &PdfObject) -> bool`
   - Lines 137-151 in catalog.rs
   - Operates directly on PdfObject

2. **Convenience wrapper:** `catalog_dict_missing_essential_keys(catalog: &Catalog) -> bool`
   - Lines 269-271 in catalog.rs
   - Wraps the low-level function for use with Catalog structs

### Function Behavior
- Returns `true` if catalog.dictionary exists but is missing essential keys (/Type or /Pages)
- Returns `false` if both /Type and /Pages are present
- Returns `false` if the object is not a dictionary (graceful handling of None/null)
- No side effects (pure function)
- Standalone and fully testable

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Function exists in appropriate module | ✅ PASS | catalog.rs, lines 269-271 |
| Returns true when dictionary missing /Type | ✅ PASS | test_is_catalog_dict_missing_essential_keys_missing_type |
| Returns true when dictionary missing /Pages | ✅ PASS | test_is_catalog_dict_missing_essential_keys_missing_pages |
| Returns false when all essential keys present | ✅ PASS | test_is_catalog_dict_missing_essential_keys_complete_dict |
| Function handles None dictionary gracefully | ✅ PASS | test_is_catalog_dict_missing_essential_keys_null |
| Function is standalone and testable | ✅ PASS | test_catalog_dict_missing_essential_keys_standalone_and_testable |
| No side effects | ✅ PASS | Multiple calls return same result (no mutation) |

## Test Results
All 27 tests passing:
- 16 tests for `is_catalog_dict_missing_essential_keys` (low-level function)
- 11 tests for `catalog_dict_missing_essential_keys` (Catalog wrapper)

```
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_default_catalog ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_no_panic ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_standalone_and_testable ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_complete_catalog ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_empty_catalog ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_integer_raw_dict ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_missing_pages ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_missing_type ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_non_dictionary ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_optional_keys ... ok
test parser::catalog::tests::test_catalog_dict_missing_essential_keys_with_string_raw_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_array ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_boolean ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_case_sensitive ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_complete_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_empty_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_integer ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_missing_both ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_missing_pages ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_missing_type ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_name ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_no_panic_on_empty_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_no_panic_on_non_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_null ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_real ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_reference ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_string ... ok
test parser::catalog::tests::test_is_catalog_dict_missing_essential_keys_with_other_optional_keys ... ok
```

## Verification Steps
1. ✅ Confirmed function exists in `crates/pdftract-core/src/parser/catalog.rs`
2. ✅ Fixed compilation error in `crates/pdftract-core/src/document.rs`
3. ✅ Ran `cargo build --lib` - compiles successfully
4. ✅ Ran all related tests - all 27 tests passing
5. ✅ Verified function is standalone (no dependencies on global state)
6. ✅ Verified function has no side effects (pure function)

## Commit Information
- **Files changed:** 1 file (document.rs)
- **Lines changed:** 1 line (fixed function call)
- **Tests:** 27 tests passing (all existing tests)
