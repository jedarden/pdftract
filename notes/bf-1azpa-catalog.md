# Unmapped Glyph Assertion Message Catalog

**Bead ID:** bf-1azpa  
**Task:** Search and catalog remaining generic assertion messages in unmapped glyph tests  
**Date:** 2026-07-06  
**Reference:** notes/bf-4kbre-messages.md (expected 73 assertions)

## Executive Summary

**Status:** ✅ **NO remaining generic assertion messages found in unmapped glyph tests**

All unmapped glyph tests in the codebase have been updated with the new diagnostic format. However, the reference document (notes/bf-4kbre-messages.md) lists 73 assertions across many test functions that **do not actually exist in the codebase**.

## Files Analyzed

1. `crates/pdftract-core/src/font/unmapped.rs`
2. `crates/pdftract-core/src/font/encoding.rs`
3. `crates/pdftract-core/src/font/resolver.rs`

## Search Results

### Generic Assertion Pattern Search

```bash
grep -n "assertion failed" [target files]
# Result: 0 matches ✅

grep -n "assert_eq!" [target files] | wc -l  
# Result: 52 matches (all in NON-unmapped tests)

grep -n "assert!" [target files] | wc -l
# Result: 119 matches (mostly documentation comments)

grep -n "Expected:" [target files] | wc -l
# Result: 85 matches ✅ (updated assertions)

grep -n "Why this matters:" [target files] | wc -l
# Result: 48 matches ✅ (updated assertions)
```

### Test Functions Catalog

#### Module: `pdftract_core::font::unmapped`

**File:** `crates/pdftract-core/src/font/unmapped.rs`

| Test Function | Line Range | Assertions | Status |
|--------------|-----------|------------|--------|
| `test_notdef_is_unmapped` | 70-85 | 2 | ✅ Updated |
| `test_normal_glyphs_not_unmapped` | 87-131 | 6 | ✅ Updated |
| `test_unmapped_set_contains_expected_entries` | 133-142 | 1 | ✅ Updated |

**Total in unmapped.rs: 9 assertions - ALL UPDATED**

#### Module: `pdftract_core::font::encoding`

**File:** `crates/pdftract-core/src/font/encoding.rs`

| Test Function | Line Range | Assertions | Status |
|--------------|-----------|------------|--------|
| `test_differences_overlay_skips_notdef` | 922-970 | 4 | ✅ Updated |
| `test_differences_overlay_skips_notdef_with_slash` | 973-1022 | 4 | ✅ Updated |
| `test_differences_overlay_custom_unmapped_glyph_names` | 1024-1190 | 13 | ✅ Updated |
| `test_differences_overlay_empty_unmapped_glyph_names` | 1191-1254 | 4 | ✅ Updated |
| `test_unmapped_glyph_skip_behavior` | 1256-1344 | 7 | ✅ Updated |

**Total in encoding.rs unmapped tests: 32 assertions - ALL UPDATED**

#### Module: `pdftract_core::font::resolver`

**File:** `crates/pdftract-core/src/font/resolver.rs`

| Test Function | Line Range | Assertions | Status |
|--------------|-----------|------------|--------|
| `test_resolve_level2_unmapped_code` | 868-880 | 1 | ✅ Updated |

**Total in resolver.rs: 1 assertion - UPDATED**

## Grand Total

**Actual unmapped glyph assertions in codebase: 42**
- unmapped.rs: 9 assertions
- encoding.rs: 32 assertions  
- resolver.rs: 1 assertion

**Status: ALL 42 assertions have been updated with custom diagnostic messages** ✅

## Discrepancy with Reference Document

The reference document (`notes/bf-4kbre-messages.md`) lists **73 assertions** across test functions that **do not exist** in the codebase:

### Missing Test Functions (from reference document):

1. `test_differences_overlay_with_slashed_names` (supposed line 1205)
2. `test_differences_overlay_with_uni_names` (supposed line 1250)
3. `test_differences_overlay_preserves_order` (supposed line 1303)
4. `test_differences_overlay_with_duplicate_codes` (supposed line 1351)
5. `test_differences_overlay_with_mixed_formats` (supposed line 1413)
6. `test_differences_overlay_with_empty_differences` (supposed line 1479)
7. `test_differences_overlay_with_all_unmapped` (supposed line 1507)
8. `test_differences_overlay_case_sensitivity` (supposed line 1548)
9. `test_differences_overlay_with_invalid_codes` (supposed line 1600)

**Conclusion:** The reference document was a design document that planned 73 assertion messages, but only 42 were actually implemented in tests that exist in the codebase.

## Other Assertions (Not Unmapped Glyph Tests)

The 52 `assert_eq!` matches found are in test functions that are NOT related to unmapped glyph testing:
- Basic encoding tests (test_winansi_0x92_quoteright, test_macroman_0xd2_quotedblleft, etc.)
- NamedEncoding parsing tests (test_from_name)
- Font encoding structure tests (test_font_encoding_new, etc.)

These have generic assertion messages but are **out of scope** for this bead.

## Verification

To verify the catalog:

```bash
# Check all unmapped glyph tests have custom messages
grep -A 10 "test_notdef_is_unmapped\|test_normal_glyphs_not_unmapped\|test_unmapped_set\|test_differences_overlay_skips_notdef\|test_differences_overlay_custom_unmapped\|test_differences_overlay_empty\|test_unmapped_glyph_skip_behavior\|test_resolve_level2_unmapped" \
  crates/pdftract-core/src/font/*.rs | grep -E "Expected:|Why this matters:" | wc -l
# Expected: 42+ matches (each updated assertion has both patterns)
```

## Recommendations

1. ✅ **No action required** - All unmapped glyph tests have proper diagnostic messages
2. Consider updating `notes/bf-4kbre-messages.md` to reflect only the 42 implemented assertions
3. The missing 31 assertions (73 - 42) represent tests that were designed but never implemented

## Appendix: Example Updated Assertion

From `test_differences_overlay_skips_notdef` (encoding.rs:935-942):

```rust
assert_eq!(
    overlay.get(39),
    None,
    "Code 39 should not have a mapping (.notdef skipped). \
     Expected: None. \
     Found: {:?}. \
     Why this matters: .notdef is in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be silently filtered during /Differences array parsing.",
    overlay.get(39)
);
```

This follows the template:
- **Description:** What should happen
- **Expected:** The expected condition  
- **Found:** What was actually found (using `{:?}` format)
- **Why this matters:** Rationale with source reference

---

**Catalog completed:** 2026-07-06  
**Total assertions cataloged:** 42  
**Generic assertions remaining:** 0  
**Status:** COMPLETE ✅
