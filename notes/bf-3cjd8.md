# Verification: unmapped.rs Uses New Diagnostic Message Format

**Bead ID:** bf-3cjd8  
**Date:** 2026-07-06  
**File:** crates/pdftract-core/src/font/unmapped.rs

## Task Completed

Verified that all unmapped glyph assertions in unmapped.rs use the new diagnostic message format instead of generic 'assertion failed' messages.

## Analysis

### Test Assertions (9 total)

| Test Function | Line Range | Assertions | Format |
|--------------|-----------|------------|--------|
| `test_notdef_is_unmapped` | 70-85 | 2 | ✅ New format |
| `test_normal_glyphs_not_unmapped` | 87-131 | 6 | ✅ New format |
| `test_unmapped_set_contains_expected_entries` | 133-142 | 1 | ✅ New format |

**Total: 9 test assertions**

### Documentation Examples (4 additional)

- Lines 26-32, 33-39, 40-46, 47-53 (doc examples also use new format)

## New Format Pattern

All assertions follow the template:
```rust
assert!(
    <condition>,
    "<description>. \
     Expected: <expected condition>. \
     Found: <actual value>. \
     Why this matters: <rationale with source reference>.",
);
```

## Verification Against Catalog

✅ **Count matches bf-1azpa catalog:** 9 assertions in unmapped.rs

## Acceptance Criteria

- ✅ All assertions in unmapped.rs use new message format
- ✅ No generic 'assertion failed' messages remain
- ✅ Count matches expected from inventory (bf-1azpa)

## Conclusion

All 9 unmapped glyph assertions in `unmapped.rs` use the new diagnostic message format with full context (description, Expected, Found, Why this matters). No generic assertion failures remain in this file.
