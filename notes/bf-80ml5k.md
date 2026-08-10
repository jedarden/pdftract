# Verification Note: bf-80ml5k - Position catalog validation before pages access

## Task
Ensure all catalog emptiness checks happen BEFORE any pages.array() access to prevent panic.

## Implementation Summary

### Changes Made
1. **Added critical ordering documentation** (`crates/pdftract-core/src/document.rs:734-746`)
   - Added comprehensive comment explaining WHY catalog checks must precede pages access
   - Documented the strict ordering: Check 0 (catalog) → Check 1+ (pages)
   - Specified that any modification MUST preserve this ordering

2. **Added comprehensive test** (`crates/pdftract-core/tests/catalog_emptiness_checks.rs:538-656`)
   - `test_catalog_checks_before_pages_access`: New test with 4 test cases
   - Test Case 1: Empty catalog fails at Check 0.1, no pages access
   - Test Case 2: Catalog with /Pages=null fails at Check 0.4, prevents resolver.resolve()
   - Test Case 3: Invalid pages_ref fails cleanly (Check 1/2), no panic
   - Test Case 4: Empty /Kids detected quickly, no pages array access

### Verification of Ordering

The validation function follows this strict sequence:

**Catalog-Level Checks (Check 0)** - Lines 741-808:
- Check 0.1: Empty dictionary detection
- Check 0.2: None dictionary detection
- Check 0.3: Missing essential keys (/Type or /Pages)
- Check 0.4: Specific /Pages entry validation (null, wrong type)

**Pages Reference Validation (Check 1+)** - Lines 810+:
- Check 1: Validate catalog.pages_ref is non-zero
- Check 2: Resolve pages reference
- Check 3: Verify resolved object is dictionary
- Check /Kids: Access pages array
- Check 5: Count pages tree

**Critical Protection**: All catalog checks execute BEFORE:
- Any `resolver.resolve(catalog.pages_ref)` call
- Any `pages_obj.as_dict()` call
- Any `pages_dict.get("Kids")` array access
- Any `count_pages_tree()` traversal

This prevents panic when:
- catalog.pages_ref is null/zero
- catalog.raw_dict is malformed
- /Pages entry is invalid type
- Pages reference doesn't resolve

## Acceptance Criteria Status
- ✅ **Catalog checks execute before pages.array() call**: Verified - Check 0 (741-808) precedes all pages access
- ✅ **Comment documents why early placement is critical**: Added comprehensive comment at lines 734-746
- ✅ **Test demonstrates no panic with empty catalog**: New test covers 4 scenarios, all pass cleanly
- ✅ **Code review confirms proper ordering**: Verified by inspecting validation flow

## Test Results
All 16 tests in catalog_emptiness_checks.rs pass:
```
running 16 tests
test test_catalog_checks_before_pages_access ... ok          <-- NEW
test test_catalog_with_optional_fields_missing_essential ... ok
test test_catalog_with_pages_null_value_triggers_empty_document ... ok
test test_catalog_with_pages_wrong_type_array_triggers_empty_document ... ok
test test_catalog_with_pages_wrong_type_integer_triggers_empty_document ... ok
test test_catalog_with_pages_wrong_type_dictionary_triggers_empty_document ... ok
test test_catalog_with_pages_wrong_type_string_triggers_empty_document ... ok
test test_detection_order_empty_then_none ... ok
test test_empty_catalog_dict_triggers_empty_document_error ... ok
test test_error_message_includes_source_identifier ... ok
test test_missing_essential_keys_triggers_empty_document_error ... ok
test test_no_panic_or_hang_on_empty_catalog ... ok
test test_none_catalog_dict_triggers_empty_document_error ... ok
test test_no_panic_when_pages_absent_or_invalid ... ok
test test_valid_catalog_passes_through_normally ... ok
test test_various_none_catalog_types_trigger_empty_document ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Commit Details
- **Commit**: Pending
- **Files modified**:
  - `crates/pdftract-core/src/document.rs` (+14 lines)
  - `crates/pdftract-core/tests/catalog_emptiness_checks.rs` (+119 lines)
- **Total changes**: 133 insertions

## Verification Steps Performed
1. ✅ Verified catalog checks (Check 0) execute before all pages access
2. ✅ Verified no resolver.resolve() calls before catalog validation completes
3. ✅ Added comprehensive comment documenting the critical ordering requirement
4. ✅ Added test demonstrating no panic with various empty catalog scenarios
5. ✅ All 16 tests pass (15 existing + 1 new)
6. ✅ Verified new test completes instantly for empty catalogs (<100ms)

## Integration Notes
This bead completes the validation ordering work started in bf-3cqxcw:
- bf-3cqxcw: Added Check 0.4 (specific /Pages entry validation)
- bf-80ml5k: Documented and verified the critical ordering requirement

The ordering is now:
1. Check 0.1: Empty dictionary (fastest check)
2. Check 0.2: None dictionary
3. Check 0.3: Missing essential keys
4. Check 0.4: Specific /Pages entry validation (prevents resolver.resolve() on invalid refs)
5. Check 1+: Pages structure access (safe to proceed if Check 0 passes)

This ensures robust protection against panics while maintaining clear failure modes.
