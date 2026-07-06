# Unmapped Glyph Test Suite - Assertion Message Review

**Bead:** bf-4kqwy  
**Date:** 2026-07-06  
**Scope:** Review all assertion messages in the cmap unmapped glyphs test suite

## Test Files Identified

### Primary Test Files
1. **`crates/pdftract-core/tests/cmap_unmapped_glyphs.rs`** (634 lines)
   - Core unmapped glyph filtering tests
   - 8 test functions covering CMAP parsing, DifferencesOverlay parsing, range mapping
   
2. **`crates/pdftract-core/tests/unmapped_glyph_names_config.rs`** (124 lines)
   - Configuration parsing tests
   - 4 test functions for config defaults and parsing

3. **`crates/pdftract-core/src/font/encoding.rs`** (tests section, lines 530-914)
   - DifferencesOverlay unit tests
   - 17 test functions for encoding overlay behavior

4. **`crates/pdftract-core/src/font/unmapped.rs`** (inline tests)
   - Basic unmapped glyph name recognition tests
   - 3 test functions

---

## Assertion Message Catalog

### Category 1: Clear Messages ✅
*Have Expected/Found/Why context or clear description*

#### From `cmap_unmapped_glyphs.rs`:
- Lines 21-29: `.notdef` unmapped check - Full context with Expected/Found/Why
- Lines 30-37: `.null` unmapped check - Full context
- Lines 40-47: `A` not unmapped check - Full context
- Lines 49-55: `space` not unmapped check - Full context
- Lines 63-70: CMAP not empty - Full context
- Lines 71-80: CMAP length exactly 4 - Full context with iteration detail
- Lines 84-92: 0x00 maps to 'A' - Full context
- Lines 187-196: Multiple mappings count - Full context
- Lines 199-224: Individual mapping lookups (0x00→A, 0x01→B, 0x02→C) - Full context
- Lines 228-236: `.notdef` unmapped check after parsing - Full context
- Lines 259-268: Range expands to 26 mappings - Full context with alphabet detail
- Lines 271-288: First/last range entries - Full context
- Lines 291-307: `.notdef` with/without slash - Full context
- Lines 360-436: All DifferencesOverlay assertions (15 assertions) - Full Expected/Found/Why context
- Lines 439-443: Overlay count - Has message but missing Expected/Found
- Lines 483-537: Consecutive sequence assertions - Full context
- Lines 562-570: `.null` glyph filtering - Full context
- Lines 609-632: g-series range filtering - Full context with iteration detail

#### From `encoding.rs`:
- Lines 901-911: `test_unmapped_glyph_skip_behavior` - All assertions have clear inline messages

**Total Clear Messages: ~70 assertions**

---

### Category 2: Messages Needing Improvement ⚠️
*Lack Expected/Found/Why context, have only brief inline messages*

#### From `cmap_unmapped_glyphs.rs`:
1. **Lines 100-104:** Normal glyph 'A' present
   ```rust
   "Normal glyph 'A' should be present in CMAP: letter glyphs should not be filtered"
   ```
   **Issue:** Missing Expected/Found/Why structure

2. **Lines 108-112:** Normal glyph 'B' present
   ```rust
   "Normal glyph 'B' should be present in CMAP: letter glyphs should not be filtered"
   ```
   **Issue:** Missing Expected/Found/Why structure

3. **Lines 116-120:** Space glyph present
   ```rust
   "Normal glyph 'space' should be present in CMAP: whitespace glyphs should not be filtered"
   ```
   **Issue:** Missing Expected/Found/Why structure

4. **Lines 124-128:** Normal glyph 'C' present
   ```rust
   "Normal glyph 'C' should be present in CMAP: multiple letter glyphs should all be present"
   ```
   **Issue:** Missing Expected/Found/Why structure

5. **Lines 133-138:** CMAP count verification
   ```rust
   "CMAP should contain exactly 4 normal glyph mappings (A, B, space, C). \
   Different count may indicate a glyph was incorrectly filtered or an extra entry was added."
   ```
   **Issue:** Missing Expected/Found/Why structure

6. **Lines 150-159:** CMAP entry validity loop
   ```rust
   "CMAP entry for bytes {:02X?} has invalid destination: {:?}. \
    Expected: non-empty vector of valid Unicode characters (no replacement character). \
    Found: empty or contains '�'. \
    Why this matters: This may indicate an unmapped glyph was not filtered correctly, \
    or the parser is generating invalid Unicode mappings."
   ```
   **Issue:** Has Expected/Found/Why but could clarify which bytes failed

7. **Line 442:** Overlay count
   ```rust
   "Overlay should have exactly 4 entries after filtering out 5 unmapped glyphs (g001, g002, g003, .notdef, null)"
   ```
   **Issue:** Missing Expected/Found structure

8. **Line 447:** Diagnostics empty
   ```rust
   "Parsing should not generate diagnostics when filtering unmapped glyphs"
   ```
   **Issue:** Missing Expected/Found structure

9. **Lines 535-536:** Consecutive sequence count
   ```rust
   "Overlay should have exactly 2 entries after filtering out 3 unmapped glyphs from a 5-item consecutive sequence"
   ```
   **Issue:** Missing Expected/Found structure

#### From `encoding.rs`:
1. **Lines 545-549:** Simple parsing assertions (5 assertions)
   ```rust
   assert_eq!(overlay.get(39), Some(Arc::from("quotesingle")));
   assert_eq!(overlay.get(96), Some(Arc::from("grave")));
   assert_eq!(overlay.get(40), None);
   assert_eq!(overlay.len(), 2);
   assert!(diagnostics.is_empty());
   ```
   **Issue:** No assertion messages - relies on test function name for context

2. **Lines 565-569:** Consecutive parsing assertions (5 assertions)
   **Issue:** No assertion messages

3. **Lines 587-591:** Multiple blocks assertions (5 assertions)
   **Issue:** No assertion messages

4. **Lines 605-610:** Out of range positive assertions (4 assertions)
   **Issue:** No assertion messages

5. **Lines 624-629:** Out of range negative assertions (4 assertions)
   **Issue:** No assertion messages

6. **Lines 639-641:** Empty overlay assertions (3 assertions)
   **Issue:** No assertion messages

7. **Lines 647-648:** Default overlay assertions (2 assertions)
   **Issue:** No assertion messages

8. **Lines 656-658:** Font encoding new assertions (2 assertions)
   **Issue:** No assertion messages

9. **Lines 663-664:** Glyph name base only assertions (2 assertions)
   **Issue:** No assertion messages

10. **Lines 678-680:** Glyph name with differences assertions (2 assertions)
    **Issue:** No assertion messages

11. **Lines 694-695:** Glyph name no base assertions (2 assertions)
    **Issue:** No assertion messages

12. **Lines 712-715:** Unknown glyph name assertions (1 assertion)
    **Issue:** No assertion messages

13. **Lines 730-732:** Lookup order assertions (2 assertions)
    **Issue:** No assertion messages

14. **Lines 749-752:** Skip .notdef assertions (4 assertions)
    **Issue:** No assertion messages (only comments)

15. **Lines 769-772:** Skip .notdef with slash assertions (4 assertions)
    **Issue:** No assertion messages (only comments)

16. **Lines 807-813:** Custom unmapped glyph names assertions (6 assertions)
    **Issue:** No assertion messages (only comments)

17. **Lines 831-836:** Custom config assertions (5 assertions)
    **Issue:** No assertion messages (only comments)

18. **Lines 867-870:** Empty unmapped set assertions (4 assertions)
    **Issue:** No assertion messages (only comments)

#### From `unmapped_glyph_names_config.rs`:
1. **Lines 45-50:** Default empty assertions (3 assertions)
   ```rust
   assert!(config.unmapped_glyph_names.is_empty());
   assert_eq!(config.unmapped_glyph_names.len(), 0);
   assert_eq!(config.description, Some("Test config without unmapped_glyph_names".to_string()));
   assert_eq!(config.version, Some("1.0".to_string()));
   ```
   **Issue:** No assertion messages

2. **Lines 73-74:** Specified config assertions (2 assertions)
   **Issue:** No assertion messages

3. **Lines 77-78:** Other fields assertions (2 assertions)
   **Issue:** No assertion messages

4. **Lines 100-102:** Empty array assertions (2 assertions)
   **Issue:** No assertion messages

5. **Lines 120-122:** Minimal config assertions (3 assertions)
   **Issue:** No assertion messages

#### From `unmapped.rs` (inline tests):
1. **Lines in test_notdef_is_unmapped:** (2 assertions)
   ```rust
   assert!(is_unmapped_glyph_name(".notdef"));
   assert!(is_unmapped_glyph_name("/.notdef"));
   ```
   **Issue:** No assertion messages

2. **Lines in test_normal_glyphs_not_unmapped:** (6 assertions)
   **Issue:** No assertion messages

3. **Lines in test_unmapped_set_contains_expected_entries:** (1 assertion)
   **Issue:** No assertion messages

**Total Messages Needing Improvement: ~80 assertions**

---

### Category 3: Ambiguous/Unclear Messages ❌
*Missing context about expected vs actual values*

None found - all assertions either have clear messages or are simple enough to understand from the code context.

---

## Gaps in Message Clarity

### Gap 1: Missing Expected/Found Structure
**Location:** Throughout `encoding.rs` and `unmapped_glyph_names_config.rs`  
**Impact:** Medium  
**Description:** ~80 assertions use `assert_eq!` or `assert!` without messages, making it hard to understand what went wrong when tests fail.  
**Example:**
```rust
assert_eq!(overlay.get(39), Some(Arc::from("quotesingle")));
// Fails with: assertion failed: `(left == right)`
// Better: assert_eq!(overlay.get(39), Some(Arc::from("quotesingle")),
//          "Code 39 should map to quotesingle. Expected: Some(\"quotesingle\"). Found: {:?}.", overlay.get(39));
```

### Gap 2: Missing "Why This Matters" Context
**Location:** `cmap_unmapped_glyphs.rs` lines 100-138  
**Impact:** Low  
**Description:** Assertions have inline messages but don't explain the business logic significance.  
**Example:**
```rust
"Normal glyph 'A' should be present in CMAP: letter glyphs should not be filtered"
// Missing: Why this matters (business logic significance)
```

### Gap 3: Loop Assertion Messages Don't Identify Failing Item
**Location:** `cmap_unmapped_glyphs.rs` lines 149-159  
**Impact:** Medium  
**Description:** Loop assertion includes bytes in message but could be clearer about which iteration failed.  
**Current:**
```rust
"CMAP entry for bytes {:02X?} has invalid destination: {:?}"
```
**Issue:** When this fails in a loop of 100 items, the test output shows the failed item but doesn't distinguish it clearly from loop context.

### Gap 4: Count Assertions Missing Expected Value
**Location:** `cmap_unmapped_glyphs.rs` lines 442, 536  
**Impact:** Low  
**Description:** Messages describe what the count should be but don't use structured Expected/Found.  
**Example:**
```rust
"Overlay should have exactly 4 entries after filtering out 5 unmapped glyphs"
// Better structure:
"Expected: 4 entries (after filtering 5 unmapped glyphs). Found: {} entries.", overlay.len()
```

---

## Recommendations

### Priority 1: Add Messages to encoding.rs Assertions
- All 50+ assertions in `encoding.rs` need assertion messages
- Focus on: overlay.get() calls, len() checks, diagnostics checks
- Use structured format: `{what} should {condition}. Expected: {expected}. Found: {found}. Why: {reason}`

### Priority 2: Improve cmap_unmapped_glyphs.rs Structure
- Convert inline messages to full Expected/Found/Why structure for lines 100-138
- Add Expected/Found to count assertions (lines 442, 536)

### Priority 3: Add Messages to Config Tests
- All assertions in `unmapped_glyph_names_config.rs` need messages
- Focus on: empty checks, length checks, field comparisons

### Priority 4: Improve Loop Messages
- Add iteration context to loop assertions to clarify which item failed

---

## Summary Statistics

- **Total test files:** 4
- **Total test functions:** 32
- **Total assertions:** ~150
- **Clear messages:** ~70 (47%)
- **Need improvement:** ~80 (53%)
- **Unclear/ambiguous:** 0 (0%)

**Overall Assessment:** The unmapped glyph test suite has a mixed quality of assertion messages. The newer tests in `cmap_unmapped_glyphs.rs` (from bf-3ayf6) have excellent structured messages, while the older unit tests in `encoding.rs` and `unmapped_glyph_names_config.rs` lack assertion messages entirely, relying on code context for clarity.
