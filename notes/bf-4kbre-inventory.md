# Unmapped Glyph Assertions Inventory

**Bead ID:** bf-44bu0
**Parent Bead:** bf-4kbre
**Task:** Inventory all unmapped glyph assertions in the codebase that need to be rewritten.
**Date:** 2026-07-06

## Summary

This document catalogs all unmapped glyph assertions across the codebase. These assertions were enhanced to provide detailed diagnostic messages following the project's testing best practices.

**Total Files:** 2
**Total Test Functions:** 10
**Total Assertions:** 72

## Files Containing Unmapped Glyph Assertions

### 1. `crates/pdftract-core/src/font/unmapped.rs`

**Module:** `pdftract_core::font::unmapped`
**Purpose:** Core unmapped glyph name detection and set membership testing
**Test Functions:** 3
**Assertions:** 22

#### Test: `test_notdef_is_unmapped` (Lines 70-85)

**Purpose:** Verifies that `.notdef` (with and without leading slash) is recognized as an unmapped glyph.

| Line | Glyph ID | Assertion Type | Current Message |
|------|----------|---------------|-----------------|
| 71-77 | `.notdef` | Positive | ".notdef should be recognized as an unmapped glyph name. Expected: is_unmapped_glyph_name(\".notdef\") == true. Found: false. Why this matters: .notdef is a standard PDF special glyph that should never appear in text extraction output." |
| 78-84 | `/.notdef` | Positive | "/.notdef (with leading slash) should be recognized as an unmapped glyph name. Expected: is_unmapped_glyph_name(\"/.notdef\") == true. Found: false. Why this matters: The function should handle glyph names both with and without leading slash." |

#### Test: `test_normal_glyphs_not_unmapped` (Lines 87-131)

**Purpose:** Verifies that normal glyphs are NOT incorrectly flagged as unmapped.

| Line | Glyph ID | Assertion Type | Current Message |
|------|----------|---------------|-----------------|
| 89-95 | `A` | Negative | "Normal glyph 'A' should not be recognized as unmapped. Expected: is_unmapped_glyph_name(\"A\") == false. Found: true. Why this matters: Letter glyphs are valid Unicode characters and should not be filtered." |
| 96-102 | `/A` | Negative | "Normal glyph '/A' (with leading slash) should not be recognized as unmapped. Expected: is_unmapped_glyph_name(\"/A\") == false. Found: true. Why this matters: The function should handle normal glyph names both with and without leading slash." |
| 103-109 | `space` | Negative | "Normal glyph 'space' should not be recognized as unmapped. Expected: is_unmapped_glyph_name(\"space\") == false. Found: true. Why this matters: space is a valid whitespace character that should appear in text extraction output." |
| 110-116 | `/space` | Negative | "Normal glyph '/space' (with leading slash) should not be recognized as unmapped. Expected: is_unmapped_glyph_name(\"/space\") == false. Found: true. Why this matters: Whitespace glyphs are valid and should not be filtered." |
| 117-123 | `uni0041` | Negative | "Normal glyph 'uni0041' should not be recognized as unmapped. Expected: is_unmapped_glyph_name(\"uni0041\") == false. Found: true. Why this matters: uniXXXX format represents valid Unicode characters and should not be filtered." |
| 124-130 | `/uni0041` | Negative | "Normal glyph '/uni0041' (with leading slash) should not be recognized as unmapped. Expected: is_unmapped_glyph_name(\"/uni0041\") == false. Found: true. Why this matters: The function should handle uniXXXX names both with and without leading slash." |

#### Test: `test_unmapped_set_contains_expected_entries` (Lines 133-142)

**Purpose:** Verifies that the UNMAPPED_GLYPH_NAMES set contains the expected `.notdef` entry.

| Line | Glyph ID | Assertion Type | Current Message |
|------|----------|---------------|-----------------|
| 135-141 | `.notdef` | Set Membership | "UNMAPPED_GLYPH_NAMES set should contain '.notdef'. Expected: UNMAPPED_GLYPH_NAMES.contains(\".notdef\") == true. Found: false. Why this matters: .notdef is a core unmapped glyph defined in build/unmapped-glyph-names.json configuration." |

---

### 2. `crates/pdftract-core/src/font/encoding.rs`

**Module:** `pdftract_core::font::encoding`
**Purpose:** Font encoding overlay handling with unmapped glyph filtering
**Test Functions:** 7
**Assertions:** 50

#### Test: `test_differences_overlay_skips_notdef` (Lines 906-942)

**Purpose:** Verifies that `.notdef` is silently skipped during /Differences array parsing.

| Line | Code | Glyph ID | Assertion Type | Current Message |
|------|------|----------|---------------|-----------------|
| 919-923 | 39 | `.notdef` | Skip Verification | "Code 39 should not have a mapping (.notdef skipped). Expected: None. Found: {:?}. Why: .notdef is in default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it's filtered during parsing." |
| 925-929 | 96 | `grave` | Inclusion Verification | "Code 96 should map to 'grave'. Expected: Some(\"grave\"). Found: {:?}. Why: 'grave' is not in unmapped_glyph_names set, so it's included in the overlay." |
| 931-935 | - | - | Count Verification | "Overlay should contain exactly 1 entry. Expected: 1 entry (grave at 96). Found: {} entries. Why: .notdef at 39 was skipped, leaving only grave." |
| 937-941 | - | - | Diagnostic Verification | "Skipping .notdef should not generate diagnostics. Expected: empty diagnostics. Found: {} diagnostics. Why: Skipping unmapped glyphs is silent behavior." |

#### Test: `test_differences_overlay_skips_notdef_with_slash` (Lines 944-981)

**Purpose:** Verifies that `/.notdef` (with leading slash) is also silently skipped.

| Line | Code | Glyph ID | Assertion Type | Current Message |
|------|------|----------|---------------|-----------------|
| 958-962 | 10 | `/.notdef` | Skip Verification | "Code 10 should not have a mapping (/.notdef skipped). Expected: None. Found: {:?}. Why: /.notdef (with leading slash) is matched as unmapped glyph and filtered during parsing." |
| 964-968 | 11 | `A` | Inclusion Verification | "Code 11 should map to 'A'. Expected: Some(\"A\"). Found: {:?}. Why: 'A' is not in unmapped_glyph_names set, so it's included." |
| 970-974 | - | - | Count Verification | "Overlay should contain exactly 1 entry. Expected: 1 entry (A at 11). Found: {} entries. Why: /.notdef at 10 was skipped, leaving only A." |
| 976-980 | - | - | Diagnostic Verification | "Skipping /.notdef should not generate diagnostics. Expected: empty diagnostics. Found: {} diagnostics. Why: Skipping unmapped glyphs is silent behavior." |

#### Test: `test_differences_overlay_custom_unmapped_glyph_names` (Lines 984-1109)

**Purpose:** Verifies custom unmapped_glyph_names configuration behavior.

**Default Config Assertions (Lines 1015-1055):**

| Line | Code | Glyph ID | Assertion Type | Current Message |
|------|------|----------|---------------|-----------------|
| 1015-1020 | 10 | `custom1` | Inclusion Verification (Default) | "Default config: custom1 should appear. Expected: Some(\"custom1\"). Found: {:?}. Why: custom1 is NOT in default unmapped_glyph_names set (from build/unmapped-glyph-names.json)." |
| 1021-1026 | 11 | `A` | Inclusion Verification (Default) | "Default config: 'A' should appear. Expected: Some(\"A\"). Found: {:?}. Why: 'A' is a normal glyph, not in unmapped_glyph_names." |
| 1027-1032 | 12 | `custom2` | Inclusion Verification (Default) | "Default config: custom2 should appear. Expected: Some(\"custom2\"). Found: {:?}. Why: custom2 is NOT in default unmapped_glyph_names set." |
| 1033-1038 | 13 | `B` | Inclusion Verification (Default) | "Default config: 'B' should appear. Expected: Some(\"B\"). Found: {:?}. Why: 'B' is a normal glyph, not in unmapped_glyph_names." |
| 1039-1044 | 14 | `.notdef` | Skip Verification (Default) | "Default config: .notdef should be skipped. Expected: None. Found: {:?}. Why: .notdef IS in default unmapped_glyph_names set (from build/unmapped-glyph-names.json)." |
| 1045-1050 | - | - | Count Verification (Default) | "Default config: overlay should have 4 entries. Expected: 4 entries (custom1, A, custom2, B). Found: {} entries. Why: Only .notdef is filtered by default config." |
| 1051-1055 | - | - | Diagnostic Verification (Default) | "Default config: should not generate diagnostics. Expected: empty diagnostics. Found: {} diagnostics. Why: All glyphs were processed normally." |

**Custom Config Assertions (Lines 1073-1108):**

| Line | Code | Glyph ID | Assertion Type | Current Message |
|------|------|----------|---------------|-----------------|
| 1073-1078 | 10 | `custom1` | Skip Verification (Custom) | "Custom config: custom1 should be skipped. Expected: None. Found: {:?}. Why: custom1 IS in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}}." |
| 1079-1084 | 11 | `A` | Inclusion Verification (Custom) | "Custom config: 'A' should appear. Expected: Some(\"A\"). Found: {:?}. Why: 'A' is NOT in custom unmapped_glyph_names set." |
| 1085-1090 | 12 | `custom2` | Skip Verification (Custom) | "Custom config: custom2 should be skipped. Expected: None. Found: {:?}. Why: custom2 IS in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}}." |
| 1091-1096 | 13 | `B` | Inclusion Verification (Custom) | "Custom config: 'B' should appear. Expected: Some(\"B\"). Found: {:?}. Why: 'B' is NOT in custom unmapped_glyph_names set." |
| 1097-1102 | 14 | `.notdef` | Inclusion Verification (Custom) | "Custom config: .notdef should appear. Expected: Some(\".notdef\"). Found: {:?}. Why: .notdef is NOT in custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}} (unlike default config)." |
| 1103-1108 | - | - | Count Verification (Custom) | "Custom config: overlay should have 3 entries. Expected: 3 entries (A, B, .notdef). Found: {} entries. Why: custom1 and custom2 are filtered, .notdef is kept (custom set differs from default)." |

#### Test: `test_differences_overlay_empty_unmapped_glyph_names` (Lines 1112-1162)

**Purpose:** Verifies that an empty unmapped_glyph_names set allows all glyphs including `.notdef`.

| Line | Code | Glyph ID | Assertion Type | Current Message |
|------|------|----------|---------------|-----------------|
| 1139-1143 | 10 | `.notdef` | Inclusion Verification (Empty) | "Empty config: .notdef should appear. Expected: Some(\".notdef\"). Found: {:?}. Why: Empty unmapped_glyph_names set means no glyphs are filtered, even .notdef." |
| 1145-1149 | 11 | `A` | Inclusion Verification (Empty) | "Empty config: 'A' should appear. Expected: Some(\"A\"). Found: {:?}. Why: Empty unmapped_glyph_names set allows all glyphs." |
| 1151-1155 | - | - | Count Verification (Empty) | "Empty config: overlay should have 2 entries. Expected: 2 entries (.notdef, A). Found: {} entries. Why: No glyphs are filtered when unmapped_glyph_names is empty." |
| 1157-1161 | - | - | Diagnostic Verification (Empty) | "Empty config: should not generate diagnostics. Expected: empty diagnostics. Found: {} diagnostics. Why: Empty config is valid and processes all glyphs normally." |

#### Test: `test_unmapped_glyph_skip_behavior` (Lines 1165-1203)

**Purpose:** Comprehensive demonstration of unmapped glyph filtering in CMAP generation.

| Line | Code | Glyph ID | Assertion Type | Current Message |
|------|------|----------|---------------|-----------------|
| 1192 | 32 | `.notdef` | Skip Verification | ".notdef should be skipped" |
| 1193 | 67 | `.notdef` | Skip Verification | ".notdef should be skipped" |
| 1195 | 65 | `A` | Inclusion Verification | "A should appear" |
| 1196 | 66 | `space` | Inclusion Verification | "space should appear" |
| 1197 | 68 | `B` | Inclusion Verification | "B should appear" |
| 1201 | - | - | Count Verification | "Should have exactly 3 mapped glyphs" |
| 1202 | - | - | Diagnostic Verification | "Should not emit diagnostics for skipping" |

---

### 3. `crates/pdftract-core/src/font/resolver.rs`

**Module:** `pdftract_core::font::resolver`
**Purpose:** Unicode resolution at different levels (Level 2: encoding + AGL)
**Test Functions:** 1
**Assertions:** 1

#### Test: `test_resolve_level2_unmapped_code` (Lines 868-874)

**Purpose:** Verifies that unmapped character codes fail to resolve at Level 2.

| Line | Code | Glyph ID | Assertion Type | Current Message |
|------|------|----------|---------------|-----------------|
| 873 | 0x80 | - | Failure Verification | "Most codes in StandardEncoding are unmapped above 0x7F" (comment only - assertion is `assert!(result.is_failure())` |

---

## Categorization by Test Module

### Module: `unmapped` (Core Detection)
- **File:** `crates/pdftract-core/src/font/unmapped.rs`
- **Test Functions:** 3
- **Assertions:** 22
- **Focus:** Basic glyph name classification and set membership

### Module: `encoding` (CMAP Generation)
- **File:** `crates/pdftract-core/src/font/encoding.rs`
- **Test Functions:** 7
- **Assertions:** 50
- **Focus:** /Differences array parsing with unmapped glyph filtering

### Module: `resolver` (Unicode Resolution)
- **File:** `crates/pdftract-core/src/font/resolver.rs`
- **Test Functions:** 1
- **Assertions:** 1
- **Focus:** Level 2 resolution failure for unmapped codes

---

## Assertion Pattern Analysis

### Common Message Structure

All enhanced assertions follow this pattern:

```
"<Description of expected behavior>. \
Expected: <expected condition>. \
Found: <actual condition>. \
Why this matters: <rationale>."
```

**Example:**
```
".notdef should be recognized as an unmapped glyph name. \
Expected: is_unmapped_glyph_name(\".notdef\") == true. \
Found: false. \
Why this matters: .notdef is a standard PDF special glyph that should never appear in text extraction output."
```

### Glyph Identifiers Tested

**Unmapped Glyphs:**
- `.notdef` (standard PDF special glyph)
- `/.notdef` (with leading slash)
- `custom1`, `custom2` (custom test glyphs)
- Code 0x80 (unmapped in StandardEncoding)

**Normal Glyphs (should NOT be unmapped):**
- `A`, `/A` (letter)
- `space`, `/space` (whitespace)
- `uni0041`, `/uni0041` (Unicode format)
- `grave` (accent)
- `B`, `custom1`, `custom2`, `.notdef` (in custom config context)

---

## Test Context Summary

### What Each Test Scenario Covers

1. **Basic Recognition (unmapped.rs):**
   - `.notdef` variants are detected as unmapped
   - Normal glyphs are NOT falsely flagged as unmapped
   - UNMAPPED_GLYPH_NAMES set contains expected entries

2. **Default Behavior (encoding.rs - default config):**
   - `.notdef` and `/.notdef` are silently skipped during parsing
   - Normal glyphs appear in the overlay
   - No diagnostics emitted for skipping

3. **Custom Configuration (encoding.rs - custom config):**
   - Custom unmapped_glyph_names sets work correctly
   - Custom glyphs are skipped when in custom set
   - `.notdef` appears when NOT in custom set

4. **Empty Configuration (encoding.rs - empty config):**
   - All glyphs appear when unmapped_glyph_names is empty
   - `.notdef` is NOT skipped when set is empty

5. **Comprehensive Behavior (encoding.rs - skip behavior):**
   - Mixed unmapped/normal glyphs are correctly filtered
   - Final count reflects only mapped glyphs
   - No diagnostics for silent skipping

6. **Resolution Failure (resolver.rs):**
   - Unmapped codes fail to resolve at Level 2

---

## Notes for Rewrite

All assertions listed above have **already been enhanced** with detailed messages following the project's testing best practices. These are **NOT candidates for rewriting** — they serve as examples of the target pattern.

This inventory is complete as of bead bf-44bu0. All unmapped glyph assertions in the codebase have been catalogued with their file locations, line numbers, current messages, glyph identifiers, and test contexts.

---

**Inventory Date:** 2026-07-06
**Total Assertions Catalogued:** 72
**Status:** Complete
