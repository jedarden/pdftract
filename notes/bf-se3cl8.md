# Verification Note: bf-se3cl8

## Task: Add None catalog dictionary detection helper

## Changes Made

### New Function: `is_catalog_dict_none`
- **Location**: `crates/pdftract-core/src/parser/catalog.rs:91`
- **Function signature**: `pub fn is_catalog_dict_none(catalog_dict: &PdfObject) -> bool`
- **Purpose**: Detects when `catalog.dictionary` is None/null (root object not a dictionary)

### Implementation Details
```rust
pub fn is_catalog_dict_none(catalog_dict: &PdfObject) -> bool {
    catalog_dict.as_dict().is_none()
}
```

The function uses the existing `as_dict()` method from `PdfObject`, which returns `None` when the object is not a dictionary (null, boolean, integer, real, string, name, array, or reference).

### Test Coverage
Added 11 comprehensive tests covering:
- `test_is_catalog_dict_none_null` - Null object returns true
- `test_is_catalog_dict_none_boolean` - Boolean objects return true
- `test_is_catalog_dict_none_integer` - Integer returns true
- `test_is_catalog_dict_none_real` - Real number returns true
- `test_is_catalog_dict_none_string` - String returns true
- `test_is_catalog_dict_none_name` - Name returns true
- `test_is_catalog_dict_none_array` - Array returns true
- `test_is_catalog_dict_none_reference` - Reference returns true
- `test_is_catalog_dict_none_empty_dict` - Empty dictionary returns false
- `test_is_catalog_dict_none_non_empty_dict` - Non-empty dictionary returns false
- `test_is_catalog_dict_none_no_panic_on_non_dict` - No panic on non-dictionary types
- `test_is_catalog_dict_none_no_panic_on_empty_dict` - No panic on empty dictionary

## Acceptance Criteria Status

✅ **PASS**: Function exists and is named appropriately
- Function name: `is_catalog_dict_none`
- Location: `crates/pdftract-core/src/parser/catalog.rs`

✅ **PASS**: Returns true when dictionary is None
- All non-dictionary PdfObject variants (Null, Bool, Integer, Real, String, Name, Array, Ref) return true

✅ **PASS**: Returns false when dictionary exists
- Both empty and non-empty dictionaries return false

✅ **PASS**: No panic on None dictionary
- Comprehensive panic tests verify no panics occur on any PdfObject variant

✅ **PASS**: Compiles without errors
- Built successfully with `cargo build --lib`
- All tests pass (12/12 tests passed)

## Test Results
```
test parser::catalog::tests::test_is_catalog_dict_none_no_panic_on_empty_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_none_no_panic_on_non_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_none_non_empty_dict ... ok
test parser::catalog::tests::test_is_catalog_dict_none_null ... ok
test parser::catalog::tests::test_is_catalog_dict_none_real ... ok
test parser::catalog::tests::test_is_catalog_dict_none_reference ... ok
test parser::catalog::tests::test_is_catalog_dict_none_string ... ok
[and 5 more tests...]

test result: ok. 12 passed; 0 failed; 0 ignored
```

## Integration
This helper complements the existing `is_catalog_dict_empty` function:
- `is_catalog_dict_empty`: Returns true if dictionary exists but has no keys
- `is_catalog_dict_none`: Returns true if dictionary is None/null (doesn't exist as a dictionary type)

Both functions handle Optional types safely and never panic on any PdfObject variant.

## Related Bead
This bead adds the None detection helper, complementing the empty dictionary detection from the dependent bead.
