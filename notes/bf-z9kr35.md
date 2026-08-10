# bf-z9kr35: Catalog Dictionary Emptiness Check

## Summary
Implemented the first layer of catalog emptiness detection by checking if catalog.dictionary is empty or missing essential keys.

## Changes Made
Modified `validate_pages_structure()` in `crates/pdftract-core/src/document.rs`:
- Enhanced "Check 0" to detect catalog dictionary emptiness by checking for `StructMissingKey` diagnostics
- Any `StructMissingKey` diagnostic indicates the catalog dictionary is malformed/empty
- Returns `DocumentError::EmptyDocument` with source identifier when detected
- Error message includes source identifier for proper error reporting

## Acceptance Criteria Verification
✅ **Empty catalog.dictionary triggers DocumentError::EmptyDocument** - Covered by StructMissingKey diagnostics when dictionary has no keys
✅ **None catalog.dictionary triggers DocumentError::EmptyDocument** - Covered by StructMissingKey diagnostics when root object is not a dictionary  
✅ **Missing essential keys triggers DocumentError::EmptyDocument** - Covered by StructMissingKey diagnostics for /Type, /Pages, and other essential keys
✅ **Error message includes source identifier** - All EmptyDocument errors include source parameter
✅ **No panic on empty/None dictionary** - Graceful error handling without panics

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
The implementation leverages existing diagnostics infrastructure:
- During catalog parsing, `StructMissingKey` diagnostics are emitted when the catalog dictionary is empty or missing essential keys
- The check examines `catalog.diagnostics` for any `StructMissingKey` diagnostic
- This approach provides comprehensive coverage without duplicating parsing logic
- The check occurs early in `validate_pages_structure()` before other validation steps

## References
- Plan lines 3880-3890 (Edge case validation - catalog structure)
- Depends on: bf-6258c6 (catalog emptiness variants catalogued)
