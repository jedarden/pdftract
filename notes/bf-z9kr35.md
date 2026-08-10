# bf-z9kr35: Catalog Dictionary Emptiness Check

## Summary
Implemented the first layer of catalog emptiness detection by checking if catalog.dictionary is empty or missing essential keys.

## Changes Made
Modified `validate_pages_structure()` in `crates/pdftract-core/src/document.rs`:
- Added "Check 0" catalog dictionary emptiness detection (lines 741-770)
- Uses three helper functions from catalog.rs to detect catalog emptiness:
  - `is_catalog_dict_empty(&catalog.raw_dict)` - detects completely empty dictionaries
  - `is_catalog_dict_none(&catalog.raw_dict)` - detects None/null dictionaries
  - `catalog_dict_missing_essential_keys(&catalog.raw_dict)` - detects missing /Type or /Pages
- Returns `DocumentError::EmptyDocument` with source identifier when detected
- Error message includes source identifier for proper error reporting

Also fixed test cases to include `raw_dict` field in Catalog struct initialization:
- `test_validate_pages_structure_unresolvable_reference` (line 2559)
- `test_validate_pages_structure_truly_empty_catalog_no_panic` (line 2646)

## Acceptance Criteria Verification
✅ **Empty catalog.dictionary triggers DocumentError::EmptyDocument** - Check 0.1 (line 752) uses `is_catalog_dict_empty()`
✅ **None catalog.dictionary triggers DocumentError::EmptyDocument** - Check 0.2 (line 759) uses `is_catalog_dict_none()`
✅ **Missing essential keys triggers DocumentError::EmptyDocument** - Check 0.3 (line 766) uses `catalog_dict_missing_essential_keys()`
✅ **Error message includes source identifier** - All three checks return `EmptyDocument { source: source_identifier.to_string() }`
✅ **No panic on empty/None dictionary** - Helper functions handle all PdfObject variants safely

## Test Results
All catalog dictionary emptiness detection tests pass:
- `test_validate_pages_structure_catalog_dictionary_empty_detection` - PASS
- `test_validate_pages_structure_empty_catalog_returns_empty_document` - PASS  
- `test_validate_pages_structure_truly_empty_catalog_no_panic` - PASS
- `test_validate_pages_structure_missing_pages_ref` - PASS
- `test_validate_pages_structure_catalog_with_content_but_no_pages_returns_empty_document` - PASS
- `test_validate_pages_structure_minimal_catalog_with_content` - PASS

10/11 validate_pages_structure tests pass. The 1 failing test (`test_validate_pages_structure_valid_with_one_page`) is a pre-existing failure caused by a malformed test PDF file (`tests/fixtures/test-minimal.pdf`) that doesn't have a valid trailer with a Root entry.

## Technical Details
The implementation uses three helper functions from `catalog.rs`:

1. **`is_catalog_dict_empty(catalog_dict: &PdfObject) -> bool`** (catalog.rs:47-52)
   - Returns true if the object is a dictionary with zero keys
   - Safely handles all PdfObject variants using `as_dict()` and `unwrap_or(false)`

2. **`is_catalog_dict_none(catalog_dict: &PdfObject) -> bool`** (catalog.rs:90-92)
   - Returns true if the object is not a dictionary (null, number, string, etc.)
   - Uses `catalog_dict.as_dict().is_none()`

3. **`catalog_dict_missing_essential_keys(catalog_dict: &PdfObject) -> bool`** (catalog.rs:136-150)
   - Returns true if dictionary is missing /Type or /Pages (or both)
   - Returns false for non-dictionary types (handled by earlier checks)
   - Uses `dict.contains_key("Type")` and `dict.contains_key("Pages")`

The checks execute in order (empty dict → None dict → missing essential keys) at the start of `validate_pages_structure()`, ensuring catalog dictionary emptiness is caught before other validation steps.

## References
- Plan lines 3880-3890 (Edge case validation - catalog structure)
- Depends on: bf-6258c6 (catalog emptiness variants catalogued)
