# Bead bf-4k9q8: Cross-reference 73 Unmapped Glyph Assertions

## Task Summary
Verify that exactly 73 unmapped glyph assertions exist across the three modified files and cross-reference against the message catalog in `notes/bf-4kbre-messages.md`.

## Catalog Status
**File:** `notes/bf-4kbre-messages.md`
- **Status:** ✅ Exists and readable
- **Total Messages Designed:** 73
- **Format:** All 73 follow the bf-300b5 template structure
- **Distribution by Module:**
  - `unmapped.rs`: 9 messages (Assertions 1-9)
  - `encoding.rs`: 63 messages (Assertions 10-72)
  - `resolver.rs`: 1 message (Assertion 73)

## Implementation Verification

### Files Checked (Correct Paths)
The bead description referenced `diagnostics/` paths, but the actual files are in the `font/` module:
- ✅ `crates/pdftract-core/src/font/unmapped.rs`
- ✅ `crates/pdftract-core/src/font/encoding.rs`
- ✅ `crates/pdftract-core/src/font/resolver.rs`

### Assertion Counts by File

#### unmapped.rs
- **Total assert! macros:** 13
- **New format messages ("Why this matters:"):** 13
- **Coverage:** ✅ All assertions use new diagnostic format

#### encoding.rs
- **Total assert! macros:** 10
- **New format messages ("Why this matters:"):** 34
- **Note:** The catalog shows 63 messages for encoding.rs, but grep finds 34 with the new format

#### resolver.rs
- **Total assertions (assert! + assert_eq!):** 66
- **New format messages ("Why this matters:"):** 1
- **Note:** Most resolver.rs assertions are generic (assert!, assert_eq! without custom messages)

### Total Count Status
- **Expected:** 73 assertions (per catalog)
- **Found with new format:** 48 assertions (13 + 34 + 1)
- **Gap:** 25 assertions not yet converted to new format

## Analysis

### Discrepancy Explanation
The message catalog in `notes/bf-4kbre-messages.md` was created as a **design document** (bead bf-w1o10) that cataloged 72 unmapped glyph scenarios. The catalog header states:
- "Task: Design assertion messages for each unmapped glyph scenario"
- "Completion Status: Complete"
- "Next Step: Implement these messages in the test files (bead bf-4kbre)"

The current status indicates that **48 of 73 assertions have been implemented** with the new diagnostic format. The remaining 25 assertions either:
1. Have not yet been converted from generic assertions
2. Use different test patterns not captured by the "Why this matters:" grep

### Catalog Organization
The catalog covers three modules:

1. **unmapped.rs** (9 assertions):
   - Basic unmapped glyph detection (.notdef recognition)
   - Normal glyph verification (letters, whitespace, Unicode format)
   - Set membership testing

2. **encoding.rs** (63 assertions):
   - /Differences array parsing
   - Unmapped glyph filtering (.notdef, slash variants)
   - Custom unmapped_glyph_names configuration
   - Empty set behavior
   - Order preservation
   - Duplicate code handling
   - Case sensitivity
   - Mixed format handling

3. **resolver.rs** (1 assertion):
   - Level 2 resolution failure for unmapped codes

### Message Format Verification
**Sample from unmapped.rs (Assertion 1):**
```
".notdef should be recognized as an unmapped glyph name. \
Expected: is_unmapped_glyph_name(\".notdef\") == true. \
Found: false. \
Why this matters: .notdef is a standard PDF special glyph defined in build/unmapped-glyph-names.json that should never appear in text extraction output."
```

✅ All 48 implemented assertions follow this template structure.

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Total assertion count equals 73 | ❌ | 48 implemented, 25 pending |
| All three files have been checked | ✅ | unmapped.rs, encoding.rs, resolver.rs verified |
| Message catalog exists and is readable | ✅ | notes/bf-4kbre-messages.md exists with 73 messages |
| No generic assertion messages found | ⚠️ | 48 have custom messages, 25 still generic |

## Recommendations

1. **Complete the implementation:** Convert the remaining 25 generic assertions to use the new diagnostic format
2. **Verify catalog alignment:** Ensure all 73 cataloged messages are implemented in code
3. **Update test coverage:** The encoding.rs module may need additional assertions to reach 63

## References
- Parent bead: bf-4s17f
- Message catalog: notes/bf-4kbre-messages.md
- Prerequisite: bf-60vlq (test suite execution)
- Template: notes/bf-lpyhe-template.md
