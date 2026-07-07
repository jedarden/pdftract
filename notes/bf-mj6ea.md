# Verification Note: bf-mj6ea

## Task
Update encoding.rs assertions part 2 (tests 7-15) with newly designed diagnostic messages.

## Investigation Findings

### Tests 7-15: Do Not Exist
The task description mentions the following tests should be updated with assertions 45-72:

7. **test_differences_overlay_with_slashed_names** (assertions 42-44)
8. **test_differences_overlay_with_uni_names** (assertions 45-48)
9. **test_differences_overlay_preserves_order** (assertions 49-52)
10. **test_differences_overlay_with_duplicate_codes** (assertions 53-56)
11. **test_differences_overlay_with_mixed_formats** (assertions 57-61)
12. **test_differences_overlay_with_empty_differences** (assertions 62-63)
13. **test_differences_overlay_with_all_unmapped** (assertions 64-67)
14. **test_differences_overlay_case_sensitivity** (assertions 68-71)
15. **test_differences_overlay_with_invalid_codes** (assertion 72)

However, none of these tests exist in the current `crates/pdftract-core/src/font/encoding.rs` file.

### Current File State
The current encoding.rs file contains 1345 lines and has only 20 test functions. The last test is `test_unmapped_glyph_skip_behavior` (lines 1255-1343), which corresponds to test 5 in the original plan.

According to the message design document (notes/bf-4kbre-messages.md), tests 7-15 would extend the file to approximately line 1646, but the current file ends at line 1345.

### Relationship to Parent Bead
This bead is part of the split-child hierarchy under parent bead bf-5dopl (Rewrite unmapped glyph assertions in test files). The parent bead's verification note (notes/bf-5dopl.md) states that all 63 assertions in encoding.rs were already updated, but this appears to be based on an inventory that included tests that were never implemented.

### Verification
```bash
$ grep -n "test_differences_overlay_with_uni_names\|test_differences_overlay_preserves_order" crates/pdftract-core/src/font/encoding.rs
# No matches found

$ wc -l crates/pdftract-core/src/font/encoding.rs
1345 crates/pdftract-core/src/font/encoding.rs
```

## Conclusion
The tests 7-15 (assertions 45-72) do not exist in the codebase and therefore cannot be updated. The current encoding.rs file ends at line 1345, while the message design document references tests that would extend to line 1646.

**Status:** Work cannot be completed - referenced tests do not exist in the codebase
**Files:** No changes needed
**Tests:** N/A - tests to be updated do not exist

## Recommendations
1. Verify whether tests 7-15 were planned but never implemented
2. If these tests are needed, they should be created as a separate task
3. Update the message design document to reflect only tests that actually exist
