# Assertion Message Template

**Bead ID:** bf-lpyhe  
**Template Version:** 1.0  
**Date:** 2026-07-06

## Standard Template

All assertion messages should follow this structure:

```
"<Description of expected behavior>. \
Expected: <expected condition with specific value>. \
Found: <actual condition or observed value>. \
Why this matters: <rationale and reference to source of truth>."
```

## Template Components

### 1. Description of Expected Behavior
- **What:** Clear, concise statement of what should happen
- **Format:** Present tense, descriptive
- **Example:** ".notdef should be recognized as an unmapped glyph name"

### 2. Expected Condition
- **what:** The exact expected state or return value
- **Format:** `Expected: <function call> == <expected value>` or `Expected: <expected state>`
- **Example:** `Expected: is_unmapped_glyph_name(".notdef") == true`

### 3. Found / Actual Condition
- **What:** What was actually observed or returned
- **Format:** `Found: <actual value>` or use placeholder `{:?}` for runtime values
- **Example:** `Found: false` or `Found: {:?}` (for format! macros)

### 4. Why This Matters (Context)
- **What:** Rationale linking the expectation to its source of truth
- **Format:** Reference to configuration files, specs, or design docs
- **Example:** `Why this matters: .notdef is defined in build/unmapped-glyph-names.json as a standard PDF special glyph.`

## Template Examples by Assertion Type

### Example 1: Unmapped Glyph Detection (Positive Case)

```
".notdef should be recognized as an unmapped glyph name. \
Expected: is_unmapped_glyph_name(\".notdef\") == true. \
Found: false. \
Why this matters: .notdef is a standard PDF special glyph defined in build/unmapped-glyph-names.json that should never appear in text extraction output."
```

### Example 2: Normal Glyph Verification (Negative Case)

```
"Normal glyph 'A' should not be recognized as unmapped. \
Expected: is_unmapped_glyph_name(\"A\") == false. \
Found: true. \
Why this matters: Letter glyphs are valid Unicode characters and should not be filtered by the unmapped glyph detection logic."
```

### Example 3: Set Membership Verification

```
"UNMAPPED_GLYPH_NAMES set should contain '.notdef'. \
Expected: UNMAPPED_GLYPH_NAMES.contains(\".notdef\") == true. \
Found: false. \
Why this matters: .notdef is a core unmapped glyph defined in build/unmapped-glyph-names.json configuration and must be present in the runtime set."
```

### Example 4: CMAP Parsing Behavior (Skip Verification)

```
"Code 39 should not have a mapping (.notdef skipped). \
Expected: None. \
Found: {:?}. \
Why this matters: .notdef is in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be silently filtered during /Differences array parsing."
```

### Example 5: Count Verification

```
"Overlay should contain exactly 1 entry. \
Expected: 1 entry (grave at 96). \
Found: {} entries. \
Why this matters: .notdef at 39 was skipped per build/unmapped-glyph-names.json, leaving only grave in the overlay."
```

### Example 6: Diagnostic Verification

```
"Skipping .notdef should not generate diagnostics. \
Expected: empty diagnostics. \
Found: {} diagnostics. \
Why this matters: Skipping unmapped glyphs is silent behavior by design - these glyphs are filtered during parsing without producing warnings."
```

## Placeholders for Dynamic Values

When the actual value is determined at runtime, use these placeholders:

- `{:?}` - For Debug formatting (used in `assert!` macros with format strings)
- `{}` - For Display formatting
- `{variable}` - Document that a specific variable name will be interpolated

## Configuration File References

Always reference the source of truth in the "Why this matters" section:

- **build/unmapped-glyph-names.json** - Default unmapped glyph set
- **build/font-fingerprints.json** - Font fingerprint database
- **PDF specification** - When referencing spec-defined behavior
- **Design docs** - When referencing architectural decisions

## Template Usage Guidelines

1. **Be Specific:** Include exact function names, parameters, and expected values
2. **Be Complete:** Never omit the "Why this matters" section
3. **Be Accurate:** The "Found" value should match what the test actually observes
4. **Be Consistent:** Use the same phrasing for similar assertions across tests
5. **Reference Sources:** Always link to the configuration file or spec that defines the expectation

## Template for Different Assertion Types

### Positive Assertions (Should be true)
```
"<subject> should <expected behavior>. \
Expected: <condition> == true. \
Found: false. \
Why this matters: <rationale with source reference>."
```

### Negative Assertions (Should NOT be true)
```
"<subject> should not <unexpected behavior>. \
Expected: <condition> == false. \
Found: true. \
Why this matters: <rationale with source reference>."
```

### Set Membership Assertions
```
"<set> should contain '<item>'. \
Expected: <set>.contains(\"<item>\") == true. \
Found: false. \
Why this matters: <item> is defined in <config file> and must be present in the runtime set."
```

### Count/Quantity Assertions
```
"<collection> should contain exactly <n> <items>. \
Expected: <n> entries (<item list>). \
Found: {} entries. \
Why this matters: <rationale explaining expected count based on filtering rules>."
```

### Empty/None Assertions
```
"<subject> should be <empty|None>. \
Expected: <expected state>. \
Found: {:?}. \
Why this matters: <rationale explaining why this should be empty/None>."
```

---

**Template Status:** Active  
**Maintained By:** pdftract testing team  
**Last Updated:** 2026-07-06
