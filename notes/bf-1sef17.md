# Bead bf-1sef17: Add essential keys missing detection helper

## Summary
Implemented `catalog_dict_missing_essential_keys()` helper function in `/home/coding/pdftract/crates/pdftract-core/src/parser/catalog.rs`.

## Implementation Details

### Function Signature
```rust
pub fn catalog_dict_missing_essential_keys(catalog_dict: &PdfObject) -> bool
```

### Behavior
- Takes a `&PdfObject` reference as input (following the pattern of existing helpers)
- Returns `true` if the catalog dictionary is missing `/Type` OR `/Pages` key (or both)
- Returns `false` if both `/Type` and `/Pages` are present
- Returns `false` if the input is not a dictionary (null, number, string, etc.)
- No panics on any input type

### Key Implementation Features
1. **Safe handling of non-dictionary types**: Returns `false` for non-dictionary objects (null, numbers, strings, arrays, references, etc.)
2. **Safe key presence checks**: Uses `contains_key()` which never panics
3. **Case-sensitive key matching**: Correctly checks for exact "Type" and "Pages" keys
4. **Minimal overhead**: Returns early for non-dictionary types

### Tests Added (16 tests)
All passing (87 catalog tests total):
- `test_catalog_dict_missing_essential_keys_complete_dict` - Both keys present → returns false
- `test_catalog_dict_missing_essential_keys_missing_type` - Missing /Type → returns true
- `test_catalog_dict_missing_essential_keys_missing_pages` - Missing /Pages → returns true
- `test_catalog_dict_missing_essential_keys_missing_both` - Missing both → returns true
- `test_catalog_dict_missing_essential_keys_empty_dict` - Empty dict (missing both) → returns true
- `test_catalog_dict_missing_essential_keys_null` - Null → returns false
- `test_catalog_dict_missing_essential_keys_boolean` - Boolean → returns false
- `test_catalog_dict_missing_essential_keys_integer` - Integer → returns false
- `test_catalog_dict_missing_essential_keys_real` - Real → returns false
- `test_catalog_dict_missing_essential_keys_string` - String → returns false
- `test_catalog_dict_missing_essential_keys_name` - Name → returns false
- `test_catalog_dict_missing_essential_keys_array` - Array → returns false
- `test_catalog_dict_missing_essential_keys_reference` - Reference → returns false
- `test_catalog_dict_missing_essential_keys_with_other_optional_keys` - Both essential + optional keys present → returns false
- `test_catalog_dict_missing_essential_keys_no_panic_on_non_dict` - No panic on non-dictionary types
- `test_catalog_dict_missing_essential_keys_no_panic_on_empty_dict` - No panic on empty dictionary
- `test_catalog_dict_missing_essential_keys_case_sensitive` - Case-sensitive key matching

## Acceptance Criteria Status

- ✅ Function exists and is named appropriately (`catalog_dict_missing_essential_keys`)
- ✅ Checks for /Type key presence
- ✅ Checks for /Pages key presence
- ✅ Returns true if either is missing
- ✅ Returns false if both present
- ✅ No panic on missing keys
- ✅ Compiles without errors

## Files Modified
- `/home/coding/pdftract/crates/pdftract-core/src/parser/catalog.rs` - Added function and 16 tests

## Test Results
```
test result: ok. 87 passed; 0 failed; 0 ignored; 0 measured; 3258 filtered out
```
All catalog tests pass, including 16 new tests for `catalog_dict_missing_essential_keys`.

## Git Commit
Commit: `feat(bf-1sef17): add catalog_dict_missing_essential_keys helper function`
