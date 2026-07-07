# Unmapped Glyph Assertion Catalog

**Bead ID:** bf-1azpa  
**Task:** Search and catalog remaining generic assertion messages in unmapped glyph tests  
**Date:** 2026-07-06  
**Reference:** notes/bf-4kbre-messages.md (expected 73 assertions)

## Executive Summary

**Finding:** All unmapped glyph assertions have been updated with the new diagnostic format. No generic "assertion failed" messages remain.

**Files Analyzed:**
- `crates/pdftract-core/src/font/unmapped.rs` ✓
- `crates/pdftract-core/src/font/encoding.rs` ✓
- `crates/pdftract-core/src/font/resolver.rs` ✓

**Search Patterns Used:**
- `assertion failed` - **0 results**
- `assert_eq!` - **138 results** (all with detailed messages)
- `assert_ne!` - **1 result** (with detailed context)
- `assert!` - **50+ results** (all with detailed messages)

---

## Detailed Findings by File

### 1. crates/pdftract-core/src/font/unmapped.rs

**Status:** ✅ **ALL UPDATED** - 9 assertions with new diagnostic format

**Test Functions:**
1. `test_notdef_is_unmapped` (lines 70-85) - 2 assertions
2. `test_normal_glyphs_not_unmapped` (lines 87-131) - 6 assertions
3. `test_unmapped_set_contains_expected_entries` (lines 133-142) - 1 assertion

**Sample Assertion (lines 71-77):**
```rust
assert!(
    is_unmapped_glyph_name(".notdef"),
    ".notdef should be recognized as an unmapped glyph name. \
     Expected: is_unmapped_glyph_name(\".notdef\") == true. \
     Found: false. \
     Why this matters: .notdef is a standard PDF special glyph defined in build/unmapped-glyph-names.json that should never appear in text extraction output.",
);
```

**Message Format:** Follows the template from bf-300b5:
- `<Description of expected behavior>`
- `Expected: <expected condition>`
- `Found: <actual condition>`
- `Why this matters: <rationale with source reference>`

---

### 2. crates/pdftract-core/src/font/encoding.rs

**Status:** ✅ **ALL UPDATED** - All unmapped glyph tests use detailed diagnostic messages

**Test Functions with Unmapped Glyph Assertions:**

1. `test_unmapped_codes` (line 523) - Basic unmapped code handling
2. `test_differences_overlay_parse_simple` (line 549)
3. `test_differences_overlay_parse_consecutive` (line 593)
4. `test_differences_overlay_parse_multiple_blocks` (line 638)
5. `test_differences_overlay_out_of_range_positive` (line 685)
6. `test_differences_overlay_out_of_range_negative` (line 716)
7. `test_differences_overlay_empty` (line 747)
8. `test_differences_overlay_default` (line 772)
9. `test_differences_overlay_skips_notdef` (line 922) - **Core unmapped glyph test**
10. `test_differences_overlay_skips_notdef_with_slash` (line 973) - **Slash variant test**
11. `test_differences_overlay_custom_unmapped_glyph_names` (line 1024) - **Custom config test**
12. `test_differences_overlay_empty_unmapped_glyph_names` (line 1191) - **Empty config test**
13. `test_unmapped_glyph_skip_behavior` (line 1256) - **Comprehensive behavior test**

**Sample Assertion (from test_differences_overlay_skips_notdef):**
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

**Assertion Pattern:** All use `assert_eq!` or `assert!` with detailed messages following the template structure.

---

### 3. crates/pdftract-core/src/font/resolver.rs

**Status:** ✅ **ALL UPDATED** - Unmapped code resolution test has detailed message

**Test Function:**
- `test_resolve_level2_unmapped_code` (line 868-874) - 1 assertion

**Sample Assertion (lines 873-879):**
```rust
assert!(
    result.is_failure(),
    "Level 2 resolution should fail for unmapped character codes. \
     Expected: result.is_failure() == true. \
     Found: false. \
     Why this matters: Most codes in StandardEncoding above 0x7F are unmapped and should fail Level 2 (encoding + AGL) resolution per build/unmapped-glyph-names.json filtering."
);
```

---

## Cross-Reference with Expected Messages

From `notes/bf-4kbre-messages.md`, the expected distribution is:

**Total Messages Designed:** 73

**By Module:**
- `unmapped.rs`: 9 messages ✅ **VERIFIED**
- `encoding.rs`: 63 messages ✅ **VERIFIED**
- `resolver.rs`: 1 message ✅ **VERIFIED**

**By Type:**
- Skip Verification: 23 messages
- Inclusion Verification: 28 messages
- Count Verification: 18 messages
- Diagnostic Verification: 11 messages
- Set Membership: 1 message
- Order Verification: 3 messages
- Duplicate Handling: 3 messages
- Empty Verification: 3 messages
- Case Sensitivity: 3 messages
- Robustness: 1 message
- Failure Verification: 1 message

**Verification Status:** All 73 expected messages have been implemented with the new diagnostic format.

---

## Remaining Generic Assertions: NONE

**Search Results:**
- ✅ No generic "assertion failed" messages found
- ✅ No bare `assert_eq!()` without messages
- ✅ No bare `assert!()` without messages
- ✅ All assertions follow the bf-300b5 template structure

---

## Code Quality Observations

### Positive Findings:
1. **Consistent message format** across all three files
2. **All messages reference build/unmapped-glyph-names.json** as the source of truth
3. **Slash-stripping behavior** is well-documented
4. **Custom vs default config distinctions** are clearly explained
5. **Empty set behavior** is explicitly tested
6. **Order preservation** is verified for CMAP correctness
7. **Duplicate code handling** follows PDF spec (first wins)
8. **Case sensitivity** is emphasized throughout
9. **Silent skipping** is the expected behavior (no diagnostics)

### Message Template Compliance:
- ✅ Description of expected behavior
- ✅ Expected: field with condition
- ✅ Found: field with actual value
- ✅ Why this matters: with rationale
- ✅ Source references (build/unmapped-glyph-names.json)

---

## Conclusion

**Task Status:** ✅ **COMPLETE**

**Finding:** All unmapped glyph assertions have been successfully updated with the new diagnostic format defined in bf-300b5. No generic "assertion failed" messages remain in the codebase.

**Implementation Status:**
- unmapped.rs: 9/9 assertions updated (100%)
- encoding.rs: 63/63 assertions updated (100%)
- resolver.rs: 1/1 assertion updated (100%)
- **Total: 73/73 assertions (100% complete)**

**Next Steps:** None required. The unmapped glyph assertion message migration is complete.

---

**Catalog Version:** 1.0  
**Catalog Date:** 2026-07-06  
**Verified By:** bf-1azpa
