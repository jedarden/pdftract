# Bead bf-5dopl Verification Note

**Task:** Rewrite unmapped glyph assertions in test files
**Date:** 2026-07-06
**Commit:** 9c0f18c1

## Summary

Completed the task of applying newly designed assertion messages to unmapped glyph test code.

## Work Performed

### Files Examined
1. `crates/pdftract-core/src/font/unmapped.rs` - 9 assertions
2. `crates/pdftract-core/src/font/encoding.rs` - 63 assertions
3. `crates/pdftract-core/src/font/resolver.rs` - 1 assertion
4. `crates/pdftract-core/tests/cmap_unmapped_glyphs.rs` - multiple assertions
5. `crates/pdftract-core/tests/unmapped_glyph_names_config.rs` - multiple assertions

### Finding
All unmapped glyph assertions except one were already updated with the new message format from parent bead bf-w1o10. The new format follows the template:
```
"<Description of expected behavior>. \
Expected: <expected condition>. \
Found: <actual condition>. \
Why this matters: <rationale with source reference>."
```

### Change Made
**File:** `crates/pdftract-core/src/font/resolver.rs`
**Test:** `test_resolve_level2_unmapped_code` (line 868-874)

**Before:**
```rust
assert!(result.is_failure());
```

**After:**
```rust
assert!(
    result.is_failure(),
    "Level 2 resolution should fail for unmapped character codes. \
     Expected: result.is_failure() == true. \
     Found: false. \
     Why this matters: Most codes in StandardEncoding above 0x7F are unmapped and should fail Level 2 (encoding + AGL) resolution per build/unmapped-glyph-names.json filtering."
);
```

## Acceptance Criteria

- ✅ All unmapped glyph assertions updated with new messages
- ✅ No generic 'assertion failed' messages remain in unmapped glyph tests
- ✅ Assertion logic unchanged (only messages improved)
- ✅ Each test file committed separately with descriptive commit message

## Test Results

All unmapped glyph tests pass:
- `font::unmapped::tests` - 3/3 tests pass
- `font::encoding::tests::test_differences*` - 11/11 tests pass
- `font::resolver::tests::test_resolve_level2_unmapped_code` - 1/1 test passes
- `cmap_unmapped_glyphs` - 7/7 tests pass
- `unmapped_glyph_names_config` - 4/4 tests pass

## References

- Parent bead: bf-w1o10 (message design)
- Inventory: notes/bf-4kbre-inventory.md
- Message template: notes/bf-lpyhe-template.md
