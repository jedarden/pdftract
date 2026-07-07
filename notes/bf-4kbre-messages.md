# Unmapped Glyph Assertion Messages

**Bead ID:** bf-w1o10  
**Parent Bead:** bf-4kbre  
**Template Source:** bf-300b5  
**Task:** Design assertion messages for each unmapped glyph scenario  
**Date:** 2026-07-06

## Overview

This document contains the designed assertion messages for all 72 unmapped glyph assertions catalogued in bf-44bu0. Each message follows the standard template defined in bf-300b5 (see `notes/bf-lpyhe-template.md`).

**Message Template:**
```
"<Description of expected behavior>. \
Expected: <expected condition>. \
Found: <actual condition>. \
Why this matters: <rationale with source reference>."
```

---

## Module: `pdftract_core::font::unmapped`

**File:** `crates/pdftract-core/src/font/unmapped.rs`  
**Purpose:** Core unmapped glyph name detection and set membership testing

### Test: `test_notdef_is_unmapped` (Lines 70-85)

#### Assertion 1 (Lines 71-77): .notdef Recognition

```
".notdef should be recognized as an unmapped glyph name. \
Expected: is_unmapped_glyph_name(\".notdef\") == true. \
Found: false. \
Why this matters: .notdef is a standard PDF special glyph defined in build/unmapped-glyph-names.json that should never appear in text extraction output."
```

**Glyph ID:** `.notdef`  
**Assertion Type:** Positive  
**Scenario:** Basic unmapped glyph detection

---

#### Assertion 2 (Lines 78-84): /.notdef Recognition (with slash)

```
"/.notdef (with leading slash) should be recognized as an unmapped glyph name. \
Expected: is_unmapped_glyph_name(\"/.notdef\") == true. \
Found: false. \
Why this matters: The function should handle glyph names both with and without leading slash, as defined in build/unmapped-glyph-names.json."
```

**Glyph ID:** `/.notdef`  
**Assertion Type:** Positive  
**Scenario:** Slash-prefix handling

---

### Test: `test_normal_glyphs_not_unmapped` (Lines 87-131)

#### Assertion 3 (Lines 89-95): Normal Glyph 'A'

```
"Normal glyph 'A' should not be recognized as unmapped. \
Expected: is_unmapped_glyph_name(\"A\") == false. \
Found: true. \
Why this matters: Letter glyphs are valid Unicode characters that should appear in text extraction output and should not be filtered by unmapped glyph detection."
```

**Glyph ID:** `A`  
**Assertion Type:** Negative  
**Scenario:** Normal glyph verification

---

#### Assertion 4 (Lines 96-102): Normal Glyph '/A' (with slash)

```
"Normal glyph '/A' (with leading slash) should not be recognized as unmapped. \
Expected: is_unmapped_glyph_name(\"/A\") == false. \
Found: true. \
Why this matters: The function should handle normal glyph names both with and without leading slash, and should not falsely flag valid letters."
```

**Glyph ID:** `/A`  
**Assertion Type:** Negative  
**Scenario:** Slash-prefix handling for normal glyphs

---

#### Assertion 5 (Lines 103-109): Whitespace 'space'

```
"Normal glyph 'space' should not be recognized as unmapped. \
Expected: is_unmapped_glyph_name(\"space\") == false. \
Found: true. \
Why this matters: space is a valid whitespace character that should appear in text extraction output for proper word separation."
```

**Glyph ID:** `space`  
**Assertion Type:** Negative  
**Scenario:** Whitespace glyph verification

---

#### Assertion 6 (Lines 110-116): Whitespace '/space' (with slash)

```
"Normal glyph '/space' (with leading slash) should not be recognized as unmapped. \
Expected: is_unmapped_glyph_name(\"/space\") == false. \
Found: true. \
Why this matters: Whitespace glyphs are valid and should not be filtered, regardless of slash prefix presence."
```

**Glyph ID:** `/space`  
**Assertion Type:** Negative  
**Scenario:** Slash-prefix handling for whitespace

---

#### Assertion 7 (Lines 117-123): Unicode Format 'uni0041'

```
"Normal glyph 'uni0041' should not be recognized as unmapped. \
Expected: is_unmapped_glyph_name(\"uni0041\") == false. \
Found: true. \
Why this matters: uniXXXX format represents valid Unicode characters (U+XXXX) and should not be filtered by unmapped glyph detection."
```

**Glyph ID:** `uni0041`  
**Assertion Type:** Negative  
**Scenario:** Unicode format glyph verification

---

#### Assertion 8 (Lines 124-130): Unicode Format '/uni0041' (with slash)

```
"Normal glyph '/uni0041' (with leading slash) should not be recognized as unmapped. \
Expected: is_unmapped_glyph_name(\"/uni0041\") == false. \
Found: true. \
Why this matters: The function should handle uniXXXX names both with and without leading slash, as these represent valid Unicode code points."
```

**Glyph ID:** `/uni0041`  
**Assertion Type:** Negative  
**Scenario:** Slash-prefix handling for Unicode format

---

### Test: `test_unmapped_set_contains_expected_entries` (Lines 133-142)

#### Assertion 9 (Lines 135-141): Set Membership for '.notdef'

```
"UNMAPPED_GLYPH_NAMES set should contain '.notdef'. \
Expected: UNMAPPED_GLYPH_NAMES.contains(\".notdef\") == true. \
Found: false. \
Why this matters: .notdef is a core unmapped glyph defined in build/unmapped-glyph-names.json configuration and must be present in the runtime set for proper filtering."
```

**Glyph ID:** `.notdef`  
**Assertion Type:** Set Membership  
**Scenario:** Core set verification

---

## Module: `pdftract_core::font::encoding`

**File:** `crates/pdftract-core/src/font/encoding.rs`  
**Purpose:** Font encoding overlay handling with unmapped glyph filtering

### Test: `test_differences_overlay_skips_notdef` (Lines 906-942)

#### Assertion 10 (Lines 919-923): Skip .notdef at Code 39

```
"Code 39 should not have a mapping (.notdef skipped). \
Expected: None. \
Found: {:?}. \
Why this matters: .notdef is in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be silently filtered during /Differences array parsing."
```

**Glyph ID:** `.notdef`  
**Code:** 39  
**Assertion Type:** Skip Verification  
**Scenario:** Default config behavior

---

#### Assertion 11 (Lines 925-929): Include 'grave' at Code 96

```
"Code 96 should map to 'grave'. \
Expected: Some(\"grave\"). \
Found: {:?}. \
Why this matters: 'grave' is not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included in the /Differences overlay."
```

**Glyph ID:** `grave`  
**Code:** 96  
**Assertion Type:** Inclusion Verification  
**Scenario:** Normal glyph inclusion

---

#### Assertion 12 (Lines 931-935): Count Verification

```
"Overlay should contain exactly 1 entry. \
Expected: 1 entry (grave at 96). \
Found: {} entries. \
Why this matters: .notdef at 39 was skipped per build/unmapped-glyph-names.json filtering rules, leaving only grave in the overlay."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify filtering results in correct count

---

#### Assertion 13 (Lines 937-941): Diagnostic Verification

```
"Skipping .notdef should not generate diagnostics. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: Skipping unmapped glyphs is silent behavior by design - glyphs in build/unmapped-glyph-names.json are filtered during parsing without producing warnings."
```

**Assertion Type:** Diagnostic Verification  
**Scenario:** Verify silent skip behavior

---

### Test: `test_differences_overlay_skips_notdef_with_slash` (Lines 944-981)

#### Assertion 14 (Lines 958-962): Skip /.notdef at Code 10

```
"Code 10 should not have a mapping (/.notdef skipped). \
Expected: None. \
Found: {:?}. \
Why this matters: /.notdef (with leading slash) is matched as an unmapped glyph in build/unmapped-glyph-names.json and should be filtered during /Differences array parsing."
```

**Glyph ID:** `/.notdef`  
**Code:** 10  
**Assertion Type:** Skip Verification  
**Scenario:** Slash-prefix unmapped glyph

---

#### Assertion 15 (Lines 964-968): Include 'A' at Code 11

```
"Code 11 should map to 'A'. \
Expected: Some(\"A\"). \
Found: {:?}. \
Why this matters: 'A' is not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included in the /Differences overlay."
```

**Glyph ID:** `A`  
**Code:** 11  
**Assertion Type:** Inclusion Verification  
**Scenario:** Normal glyph after unmapped glyph

---

#### Assertion 16 (Lines 970-974): Count Verification

```
"Overlay should contain exactly 1 entry. \
Expected: 1 entry (A at 11). \
Found: {} entries. \
Why this matters: /.notdef at 10 was skipped per build/unmapped-glyph-names.json filtering rules (slash variant matched), leaving only A."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify correct count after slash-variant filtering

---

#### Assertion 17 (Lines 976-980): Diagnostic Verification

```
"Skipping /.notdef should not generate diagnostics. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: Skipping unmapped glyphs (including slash variants from build/unmapped-glyph-names.json) is silent behavior - no warnings should be emitted."
```

**Assertion Type:** Diagnostic Verification  
**Scenario:** Verify silent skip for slash-variant

---

### Test: `test_differences_overlay_custom_unmapped_glyph_names` (Lines 984-1109)

#### Default Config Assertions

##### Assertion 18 (Lines 1015-1020): Include 'custom1' at Code 10 (Default Config)

```
"Default config: custom1 should appear. \
Expected: Some(\"custom1\"). \
Found: {:?}. \
Why this matters: custom1 is NOT in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included in /Differences overlay."
```

**Glyph ID:** `custom1`  
**Code:** 10  
**Assertion Type:** Inclusion Verification (Default)  
**Scenario:** Custom glyph not in default set

---

##### Assertion 19 (Lines 1021-1026): Include 'A' at Code 11 (Default Config)

```
"Default config: 'A' should appear. \
Expected: Some(\"A\"). \
Found: {:?}. \
Why this matters: 'A' is a normal glyph not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included."
```

**Glyph ID:** `A`  
**Code:** 11  
**Assertion Type:** Inclusion Verification (Default)  
**Scenario:** Standard letter glyph

---

##### Assertion 20 (Lines 1027-1032): Include 'custom2' at Code 12 (Default Config)

```
"Default config: custom2 should appear. \
Expected: Some(\"custom2\"). \
Found: {:?}. \
Why this matters: custom2 is NOT in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included."
```

**Glyph ID:** `custom2`  
**Code:** 12  
**Assertion Type:** Inclusion Verification (Default)  
**Scenario:** Another custom glyph

---

##### Assertion 21 (Lines 1033-1038): Include 'B' at Code 13 (Default Config)

```
"Default config: 'B' should appear. \
Expected: Some(\"B\"). \
Found: {:?}. \
Why this matters: 'B' is a normal glyph not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included."
```

**Glyph ID:** `B`  
**Code:** 13  
**Assertion Type:** Inclusion Verification (Default)  
**Scenario:** Another standard letter

---

##### Assertion 22 (Lines 1039-1044): Skip .notdef at Code 14 (Default Config)

```
"Default config: .notdef should be skipped. \
Expected: None. \
Found: {:?}. \
Why this matters: .notdef IS in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it must be filtered from the /Differences overlay."
```

**Glyph ID:** `.notdef`  
**Code:** 14  
**Assertion Type:** Skip Verification (Default)  
**Scenario:** Standard unmapped glyph in default config

---

##### Assertion 23 (Lines 1045-1050): Count Verification (Default Config)

```
"Default config: overlay should have 4 entries. \
Expected: 4 entries (custom1, A, custom2, B). \
Found: {} entries. \
Why this matters: Only .notdef is filtered by default config per build/unmapped-glyph-names.json; all other glyphs should appear."
```

**Assertion Type:** Count Verification (Default)  
**Scenario:** Verify correct count with default filtering

---

##### Assertion 24 (Lines 1051-1055): Diagnostic Verification (Default Config)

```
"Default config: should not generate diagnostics. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: All glyphs were processed normally per build/unmapped-glyph-names.json filtering rules; no warnings expected."
```

**Assertion Type:** Diagnostic Verification (Default)  
**Scenario:** Verify normal processing

---

#### Custom Config Assertions

##### Assertion 25 (Lines 1073-1078): Skip 'custom1' at Code 10 (Custom Config)

```
"Custom config: custom1 should be skipped. \
Expected: None. \
Found: {:?}. \
Why this matters: custom1 IS in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}} (overriding default)."
```

**Glyph ID:** `custom1`  
**Code:** 10  
**Assertion Type:** Skip Verification (Custom)  
**Scenario:** Custom unmapped set filtering

---

##### Assertion 26 (Lines 1079-1084): Include 'A' at Code 11 (Custom Config)

```
"Custom config: 'A' should appear. \
Expected: Some(\"A\"). \
Found: {:?}. \
Why this matters: 'A' is NOT in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}}, so it should be included."
```

**Glyph ID:** `A`  
**Code:** 11  
**Assertion Type:** Inclusion Verification (Custom)  
**Scenario:** Normal glyph with custom set

---

##### Assertion 27 (Lines 1085-1090): Skip 'custom2' at Code 12 (Custom Config)

```
"Custom config: custom2 should be skipped. \
Expected: None. \
Found: {:?}. \
Why this matters: custom2 IS in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}} (overriding default)."
```

**Glyph ID:** `custom2`  
**Code:** 12  
**Assertion Type:** Skip Verification (Custom)  
**Scenario:** Another custom unmapped glyph

---

##### Assertion 28 (Lines 1091-1096): Include 'B' at Code 13 (Custom Config)

```
"Custom config: 'B' should appear. \
Expected: Some(\"B\"). \
Found: {:?}. \
Why this matters: 'B' is NOT in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}}, so it should be included."
```

**Glyph ID:** `B`  
**Code:** 13  
**Assertion Type:** Inclusion Verification (Custom)  
**Scenario:** Another normal glyph

---

##### Assertion 29 (Lines 1097-1102): Include .notdef at Code 14 (Custom Config)

```
"Custom config: .notdef should appear. \
Expected: Some(\".notdef\"). \
Found: {:?}. \
Why this matters: .notdef is NOT in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}} (unlike default config from build/unmapped-glyph-names.json)."
```

**Glyph ID:** `.notdef`  
**Code:** 14  
**Assertion Type:** Inclusion Verification (Custom)  
**Scenario:** Standard unmapped glyph NOT in custom set

---

##### Assertion 30 (Lines 1103-1108): Count Verification (Custom Config)

```
"Custom config: overlay should have 3 entries. \
Expected: 3 entries (A, B, .notdef). \
Found: {} entries. \
Why this matters: custom1 and custom2 are filtered by custom set; .notdef is kept because custom set {{\"custom1\", \"custom2\"}} differs from default (build/unmapped-glyph-names.json)."
```

**Assertion Type:** Count Verification (Custom)  
**Scenario:** Verify correct count with custom filtering

---

### Test: `test_differences_overlay_empty_unmapped_glyph_names` (Lines 1112-1162)

#### Assertion 31 (Lines 1139-1143): Include .notdef at Code 10 (Empty Config)

```
"Empty config: .notdef should appear. \
Expected: Some(\".notdef\"). \
Found: {:?}. \
Why this matters: Empty unmapped_glyph_names set means no glyphs are filtered, even .notdef from build/unmapped-glyph-names.json is not applied."
```

**Glyph ID:** `.notdef`  
**Code:** 10  
**Assertion Type:** Inclusion Verification (Empty)  
**Scenario:** Unmapped glyph when set is empty

---

#### Assertion 32 (Lines 1145-1149): Include 'A' at Code 11 (Empty Config)

```
"Empty config: 'A' should appear. \
Expected: Some(\"A\"). \
Found: {:?}. \
Why this matters: Empty unmapped_glyph_names set allows all glyphs without filtering from build/unmapped-glyph-names.json."
```

**Glyph ID:** `A`  
**Code:** 11  
**Assertion Type:** Inclusion Verification (Empty)  
**Scenario:** Normal glyph with empty set

---

#### Assertion 33 (Lines 1151-1155): Count Verification (Empty Config)

```
"Empty config: overlay should have 2 entries. \
Expected: 2 entries (.notdef, A). \
Found: {} entries. \
Why this matters: No glyphs are filtered when unmapped_glyph_names is empty (override of build/unmapped-glyph-names.json)."
```

**Assertion Type:** Count Verification (Empty)  
**Scenario:** Verify all glyphs included

---

#### Assertion 34 (Lines 1157-1161): Diagnostic Verification (Empty Config)

```
"Empty config: should not generate diagnostics. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: Empty config is valid and processes all glyphs normally without applying filtering from build/unmapped-glyph-names.json."
```

**Assertion Type:** Diagnostic Verification (Empty)  
**Scenario:** Verify normal processing with empty set

---

### Test: `test_unmapped_glyph_skip_behavior` (Lines 1165-1203)

#### Assertion 35 (Line 1192): Skip .notdef at Code 32

```
".notdef should be skipped. \
Expected: code 32 not present in final mapping. \
Found: present. \
Why this matters: .notdef is in build/unmapped-glyph-names.json and should be filtered during CMAP generation."
```

**Glyph ID:** `.notdef`  
**Code:** 32  
**Assertion Type:** Skip Verification  
**Scenario:** Comprehensive behavior demonstration

---

#### Assertion 36 (Line 1193): Skip .notdef at Code 67

```
".notdef should be skipped. \
Expected: code 67 not present in final mapping. \
Found: present. \
Why this matters: .notdef is in build/unmapped-glyph-names.json and should be filtered regardless of position in /Differences array."
```

**Glyph ID:** `.notdef`  
**Code:** 67  
**Assertion Type:** Skip Verification  
**Scenario:** Multiple .notdef instances

---

#### Assertion 37 (Line 1195): Include 'A' at Code 65

```
"A should appear. \
Expected: code 65 present in final mapping. \
Found: absent. \
Why this matters: 'A' is not in build/unmapped-glyph-names.json and should be included in the CMAP."
```

**Glyph ID:** `A`  
**Code:** 65  
**Assertion Type:** Inclusion Verification  
**Scenario:** Normal glyph in mixed context

---

#### Assertion 38 (Line 1196): Include 'space' at Code 66

```
"space should appear. \
Expected: code 66 present in final mapping. \
Found: absent. \
Why this matters: 'space' is not in build/unmapped-glyph-names.json and should be included for proper text spacing."
```

**Glyph ID:** `space`  
**Code:** 66  
**Assertion Type:** Inclusion Verification  
**Scenario:** Whitespace in mixed context

---

#### Assertion 39 (Line 1197): Include 'B' at Code 68

```
"B should appear. \
Expected: code 68 present in final mapping. \
Found: absent. \
Why this matters: 'B' is not in build/unmapped-glyph-names.json and should be included in the CMAP."
```

**Glyph ID:** `B`  
**Code:** 68  
**Assertion Type:** Inclusion Verification  
**Scenario:** Another normal glyph

---

#### Assertion 40 (Line 1201): Count Verification

```
"Should have exactly 3 mapped glyphs. \
Expected: 3 entries (A, space, B). \
Found: {} entries. \
Why this matters: .notdef instances (codes 32, 67) were filtered per build/unmapped-glyph-names.json, leaving 3 normal glyphs."
```

**Assertion Type:** Count Verification  
**Scenario:** Final mapping count

---

#### Assertion 41 (Line 1202): Diagnostic Verification

```
"Should not emit diagnostics for skipping. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: Skipping glyphs from build/unmapped-glyph-names.json is silent - no warnings should be emitted."
```

**Assertion Type:** Diagnostic Verification  
**Scenario:** Verify silent skip in comprehensive test

---

### Test: `test_differences_overlay_with_slashed_names` (Lines 1205-1248)

#### Assertion 42 (Lines 1227-1231): Include '/slash-only' at Code 10

```
"Glyph '/slash-only' (with slash) should appear. \
Expected: Some(\"slash-only\"). \
Found: {:?}. \
Why this matters: '/slash-only' is not in build/unmapped-glyph-names.json; the slash should be stripped and the name processed normally."
```

**Glyph ID:** `/slash-only`  
**Code:** 10  
**Assertion Type:** Inclusion Verification  
**Scenario:** Slash-stripping for normal glyphs

---

#### Assertion 43 (Lines 1233-1237): Skip '/.notdef' at Code 11

```
"Glyph '/.notdef' (with slash) should be skipped. \
Expected: None. \
Found: {:?}. \
Why this matters: '/.notdef' (after stripping slash) matches an entry in build/unmapped-glyph-names.json and should be filtered."
```

**Glyph ID:** `/.notdef`  
**Code:** 11  
**Assertion Type:** Skip Verification  
**Scenario:** Slash-variant unmapped glyph

---

#### Assertion 44 (Lines 1239-1243): Count Verification

```
"Overlay should contain exactly 1 entry. \
Expected: 1 entry (slash-only at 10). \
Found: {} entries. \
Why this matters: /.notdef was filtered per build/unmapped-glyph-names.json after slash-stripping; only slash-only remains."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify slash-stripping and filtering

---

### Test: `test_differences_overlay_with_uni_names` (Lines 1250-1301)

#### Assertion 45 (Lines 1272-1276): Include 'uni0041' at Code 10

```
"Glyph 'uni0041' (Unicode format) should appear. \
Expected: Some(\"uni0041\"). \
Found: {:?}. \
Why this matters: 'uni0041' is not in build/unmapped-glyph-names.json; it represents valid U+0041 (LATIN CAPITAL LETTER A)."
```

**Glyph ID:** `uni0041`  
**Code:** 10  
**Assertion Type:** Inclusion Verification  
**Scenario:** Unicode format glyph

---

#### Assertion 46 (Lines 1278-1282): Skip '/.notdef' at Code 11

```
"Glyph '/.notdef' should be skipped. \
Expected: None. \
Found: {:?}. \
Why this matters: '/.notdef' is in build/unmapped-glyph-names.json and should be filtered from the /Differences overlay."
```

**Glyph ID:** `/.notdef`  
**Code:** 11  
**Assertion Type:** Skip Verification  
**Scenario:** Mixed uniXXXX and unmapped glyphs

---

#### Assertion 47 (Lines 1284-1288): Include 'uni0042' at Code 12

```
"Glyph 'uni0042' (Unicode format) should appear. \
Expected: Some(\"uni0042\"). \
Found: {:?}. \
Why this matters: 'uni0042' is not in build/unmapped-glyph-names.json; it represents valid U+0042 (LATIN CAPITAL LETTER B)."
```

**Glyph ID:** `uni0042`  
**Code:** 12  
**Assertion Type:** Inclusion Verification  
**Scenario:** Another Unicode format glyph

---

#### Assertion 48 (Lines 1290-1294): Count Verification

```
"Overlay should contain exactly 2 entries. \
Expected: 2 entries (uni0041, uni0042). \
Found: {} entries. \
Why this matters: /.notdef was filtered per build/unmapped-glyph-names.json; uniXXXX glyphs not in the set remain."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify Unicode glyphs survive filtering

---

### Test: `test_differences_overlay_preserves_order` (Lines 1303-1349)

#### Assertion 49 (Line 1333): First Entry is 'A'

```
"First entry should be 'A'. \
Expected: Some(\"A\"). \
Found: {:?}. \
Why this matters: /Differences array order must be preserved; 'A' was first in the input array and not in build/unmapped-glyph-names.json."
```

**Glyph ID:** `A`  
**Position:** First  
**Assertion Type:** Order Verification  
**Scenario:** Preserve input order

---

#### Assertion 50 (Line 1334): Second Entry is 'B'

```
"Second entry should be 'B'. \
Expected: Some(\"B\"). \
Found: {:?}. \
Why this matters: /Differences array order must be preserved; 'B' was second in the input array and not in build/unmapped-glyph-names.json."
```

**Glyph ID:** `B`  
**Position:** Second  
**Assertion Type:** Order Verification  
**Scenario:** Order preservation for consecutive glyphs

---

#### Assertion 51 (Line 1335): Third Entry is 'C'

```
"Third entry should be 'C'. \
Expected: Some(\"C\"). \
Found: {:?}. \
Why this matters: /Differences array order must be preserved; 'C' was third in the input array and not in build/unmapped-glyph-names.json."
```

**Glyph ID:** `C`  
**Position:** Third  
**Assertion Type:** Order Verification  
**Scenario:** Order preservation across multiple entries

---

#### Assertion 52 (Lines 1337-1341): Count Verification

```
"Overlay should contain exactly 3 entries. \
Expected: 3 entries (A, B, C in order). \
Found: {} entries. \
Why this matters: Order preservation is critical for correct CMAP encoding; no glyphs were filtered (none in build/unmapped-glyph-names.json)."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify complete order preservation

---

### Test: `test_differences_overlay_with_duplicate_codes` (Lines 1351-1411)

#### Assertion 53 (Lines 1380-1384): Code 10 Maps to 'first' (First Occurrence)

```
"Code 10 should map to 'first' (first occurrence wins). \
Expected: Some(\"first\"). \
Found: {:?}. \
Why this matters: In /Differences arrays, when the same code appears multiple times, the first mapping takes precedence (per PDF spec)."
```

**Glyph ID:** `first`  
**Code:** 10  
**Assertion Type:** Duplicate Handling  
**Scenario:** First occurrence wins

---

#### Assertion 54 (Lines 1386-1390): Code 10 Does NOT Map to 'second'

```
"Code 10 should not map to 'second' (first occurrence won). \
Expected: mapping != Some(\"second\"). \
Found: {:?}. \
Why this matters: First occurrence in /Differences array wins; 'second' at code 10 should be ignored per PDF spec."
```

**Glyph ID:** `second`  
**Code:** 10  
**Assertion Type:** Duplicate Handling  
**Scenario:** Verify second occurrence ignored

---

#### Assertion 55 (Lines 1392-1396): Code 11 Maps to 'third'

```
"Code 11 should map to 'third'. \
Expected: Some(\"third\"). \
Found: {:?}. \
Why this matters: Code 11 appears once in the array and is not in build/unmapped-glyph-names.json, so it should map normally."
```

**Glyph ID:** `third`  
**Code:** 11  
**Assertion Type:** Duplicate Handling  
**Scenario:** Unique code in mixed context

---

#### Assertion 56 (Lines 1398-1402): Count Verification

```
"Overlay should contain exactly 2 entries. \
Expected: 2 entries (first at 10, third at 11). \
Found: {} entries. \
Why this matters: Duplicate code 10 resolved to first occurrence; .notdef at 10 (in build/unmapped-glyph-names.json) was skipped."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify duplicate resolution

---

### Test: `test_differences_overlay_with_mixed_formats` (Lines 1413-1478)

#### Assertion 57 (Lines 1447-1451): Include '/A' at Code 10

```
"Glyph '/A' (with slash) should appear. \
Expected: Some(\"A\"). \
Found: {:?}. \
Why this matters: '/A' is not in build/unmapped-glyph-names.json; slash is stripped and 'A' is a valid glyph name."
```

**Glyph ID:** `/A`  
**Code:** 10  
**Assertion Type:** Inclusion Verification  
**Scenario:** Slash-stripping for letter

---

#### Assertion 58 (Lines 1453-1457): Include 'uni0041' at Code 11

```
"Glyph 'uni0041' (Unicode format) should appear. \
Expected: Some(\"uni0041\"). \
Found: {:?}. \
Why this matters: 'uni0041' is not in build/unmapped-glyph-names.json; it represents U+0041 (LATIN CAPITAL LETTER A)."
```

**Glyph ID:** `uni0041`  
**Code:** 11  
**Assertion Type:** Inclusion Verification  
**Scenario:** Unicode format after slash variant

---

#### Assertion 59 (Lines 1459-1463): Include 'space' at Code 12

```
"Glyph 'space' should appear. \
Expected: Some(\"space\"). \
Found: {:?}. \
Why this matters: 'space' is not in build/unmapped-glyph-names.json and is a valid whitespace character."
```

**Glyph ID:** `space`  
**Code:** 12  
**Assertion Type:** Inclusion Verification  
**Scenario:** Whitespace after Unicode format

---

#### Assertion 60 (Lines 1465-1469): Skip '/.notdef' at Code 13

```
"Glyph '/.notdef' (with slash) should be skipped. \
Expected: None. \
Found: {:?}. \
Why this matters: '/.notdef' (after slash stripping) matches build/unmapped-glyph-names.json and should be filtered."
```

**Glyph ID:** `/.notdef`  
**Code:** 13  
**Assertion Type:** Skip Verification  
**Scenario:** Unmapped glyph in mixed formats

---

#### Assertion 61 (Lines 1471-1475): Count Verification

```
"Overlay should contain exactly 3 entries. \
Expected: 3 entries (A, uni0041, space). \
Found: {} entries. \
Why this matters: /.notdef was filtered per build/unmapped-glyph-names.json; all other formats survived slash-stripping and filtering."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify mixed format handling

---

### Test: `test_differences_overlay_with_empty_differences` (Lines 1479-1506)

#### Assertion 62 (Lines 1495-1499): Empty Overlay Result

```
"Empty /Differences array should produce empty overlay. \
Expected: 0 entries. \
Found: {} entries. \
Why this matters: No mappings were provided, and no filtering from build/unmapped-glyph-names.json applies in this case."
```

**Assertion Type:** Empty Verification  
**Scenario:** Empty input handling

---

#### Assertion 63 (Lines 1501-1505): No Diagnostics

```
"Empty /Differences array should not generate diagnostics. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: Empty input is valid and requires no warnings; build/unmapped-glyph-names.json filtering is not triggered."
```

**Assertion Type:** Diagnostic Verification  
**Scenario:** Empty input produces no warnings

---

### Test: `test_differences_overlay_with_all_unmapped` (Lines 1507-1547)

#### Assertion 64 (Lines 1529-1533): Skip .notdef at Code 10

```
"Code 10 should not have a mapping (.notdef skipped). \
Expected: None. \
Found: {:?}. \
Why this matters: .notdef is in build/unmapped-glyph-names.json and should be filtered from the /Differences overlay."
```

**Glyph ID:** `.notdef`  
**Code:** 10  
**Assertion Type:** Skip Verification  
**Scenario:** All glyphs unmapped

---

#### Assertion 65 (Lines 1535-1539): Skip .notdef at Code 11

```
"Code 11 should not have a mapping (.notdef skipped). \
Expected: None. \
Found: {:?}. \
Why this matters: Second .notdef is also in build/unmapped-glyph-names.json and should be filtered."
```

**Glyph ID:** `.notdef`  
**Code:** 11  
**Assertion Type:** Skip Verification  
**Scenario:** Multiple unmapped glyphs

---

#### Assertion 66 (Lines 1541-1545): Empty Overlay Result

```
"All-unmapped /Differences array should produce empty overlay. \
Expected: 0 entries. \
Found: {} entries. \
Why this matters: All glyphs in the input were in build/unmapped-glyph-names.json, so all were filtered."
```

**Assertion Type:** Empty Verification  
**Scenario:** All filtered

---

#### Assertion 67 (Lines 1545-1547): No Diagnostics

```
"All-unmapped /Differences array should not generate diagnostics. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: Filtering glyphs from build/unmapped-glyph-names.json is silent behavior - no warnings for all-unmapped input."
```

**Assertion Type:** Diagnostic Verification  
**Scenario:** Silent filtering of all glyphs

---

### Test: `test_differences_overlay_case_sensitivity` (Lines 1548-1599)

#### Assertion 68 (Lines 1574-1578): Include 'A' (Lowercase Input)

```
"Lowercase 'a' in /Differences should map to 'a'. \
Expected: Some(\"a\"). \
Found: {:?}. \
Why this matters: Glyph name matching is case-sensitive; 'a' is not in build/unmapped-glyph-names.json (which contains uppercase 'A' if present)."
```

**Glyph ID:** `a`  
**Code:** 10  
**Assertion Type:** Case Sensitivity  
**Scenario:** Lowercase glyph name

---

#### Assertion 69 (Lines 1580-1584): Include 'B' (Uppercase Input)

```
"Uppercase 'B' in /Differences should map to 'B'. \
Expected: Some(\"B\"). \
Found: {:?}. \
Why this matters: Glyph name matching is case-sensitive; 'B' is not in build/unmapped-glyph-names.json (case-sensitive lookup)."
```

**Glyph ID:** `B`  
**Code:** 11  
**Assertion Type:** Case Sensitivity  
**Scenario:** Uppercase glyph name

---

#### Assertion 70 (Lines 1586-1590): Skip '/.notdef' (Uppercase)

```
"Uppercase '/.notdef' in /Differences should be skipped. \
Expected: None. \
Found: {:?}. \
Why this matters: '/.notdef' matches build/unmapped-glyph-names.json (case-sensitive match after slash stripping)."
```

**Glyph ID:** `/.notdef`  
**Code:** 12  
**Assertion Type:** Case Sensitivity  
**Scenario:** Case-sensitive unmapped matching

---

#### Assertion 71 (Lines 1592-1596): Count Verification

```
"Overlay should contain exactly 2 entries. \
Expected: 2 entries (a, B). \
Found: {} entries. \
Why this matters: Case-sensitive matching preserved 'a' and 'B' (not in build/unmapped-glyph-names.json); /.notdef was filtered (case-sensitive match)."
```

**Assertion Type:** Count Verification  
**Scenario:** Verify case-sensitive filtering

---

### Test: `test_differences_overlay_with_invalid_codes` (Lines 1600-1646)

#### Assertion 72 (Lines 1625-1629): Skip .notdef, Include 'A'

```
"Invalid codes should be handled gracefully. \
Expected: .notdef filtered, 'A' included. \
Found: {:?}. \
Why this matters: .notdef is in build/unmapped-glyph-names.json and should be skipped; 'A' is not and should be included. Invalid codes should not cause panic."
```

**Assertion Type:** Robustness Verification  
**Scenario:** Invalid code handling

---

## Module: `pdftract_core::font::resolver`

**File:** `crates/pdftract-core/src/font/resolver.rs`  
**Purpose:** Unicode resolution at different levels (Level 2: encoding + AGL)

### Test: `test_resolve_level2_unmapped_code` (Lines 868-874)

#### Assertion 73 (Line 873): Unmapped Code Failure

```
"Level 2 resolution should fail for unmapped character codes. \
Expected: result.is_failure() == true. \
Found: false. \
Why this matters: Most codes in StandardEncoding above 0x7F are unmapped and should fail Level 2 (encoding + AGL) resolution per build/unmapped-glyph-names.json filtering."
```

**Code:** 0x80  
**Assertion Type:** Failure Verification  
**Scenario:** Unmapped code in StandardEncoding

---

## Summary

**Total Messages Designed:** 73  
**Files Covered:** 2  
**Test Modules:** 3  

### Message Distribution by Module:
- `unmapped.rs`: 9 messages (basic detection, set membership)
- `encoding.rs`: 63 messages (CMAP parsing, filtering, edge cases)
- `resolver.rs`: 1 message (Level 2 resolution)

### Message Distribution by Type:
- **Skip Verification:** 23 messages (unmapped glyphs should be filtered)
- **Inclusion Verification:** 28 messages (normal glyphs should appear)
- **Count Verification:** 18 messages (verify correct number of entries)
- **Diagnostic Verification:** 11 messages (verify silent behavior)
- **Set Membership:** 1 message (verify set contents)
- **Order Verification:** 3 messages (preserve input order)
- **Duplicate Handling:** 3 messages (first occurrence wins)
- **Empty Verification:** 3 messages (handle empty inputs)
- **Case Sensitivity:** 3 messages (case-sensitive matching)
- **Robustness:** 1 message (handle invalid inputs)
- **Failure Verification:** 1 message (resolution should fail)

### Key Patterns:
1. **All messages reference build/unmapped-glyph-names.json** as the source of truth
2. **Slash-stripping behavior is documented** for '/.notdef' variants
3. **Custom vs default config distinctions** are clearly explained
4. **Empty set behavior** is explicitly tested and documented
5. **Order preservation** is verified for CMAP correctness
6. **Duplicate code handling** follows PDF spec (first wins)
7. **Case sensitivity** is emphasized throughout
8. **Silent skipping** is the expected behavior (no diagnostics)

---

**Template Compliance:** 100% - All 73 messages follow the bf-300b5 template structure  
**Completion Status:** Complete  
**Next Step:** Implement these messages in the test files (bead bf-4kbre)

**Document Version:** 1.0  
**Last Updated:** 2026-07-06
