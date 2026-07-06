# Standard Assertion Message Format Template

**Bead:** bf-300b5  
**Parent:** bf-lpyhe (enhance assertion messages with diagnostic context)  
**Date:** 2026-07-06  
**Status:** ✅ **COMPLETE**

---

## Purpose

This template defines the standard format for all assertion messages in the pdftract codebase. A consistent format ensures that test failures provide clear, actionable diagnostic information that includes:

1. **What should happen** - A clear description of the expectation
2. **Expected** - The expected value or state
3. **Found** - The actual value or state (when assertion fails)
4. **Why** - The business logic significance and/or configuration source of truth

---

## Standard Template Structure

### For `assert_eq!` and `assert_ne!`:

```rust
assert_eq!(
    actual_value,
    expected_value,
    "Clear description of what should happen. \
     Expected: {expected_description}. \
     Found: {:?}. \
     Why: {business_logic_significance}.",
    actual_value  // For formatting in the error message
);
```

### For `assert!` (boolean predicates):

```rust
assert!(
    condition,
    "Clear description of what should happen. \
     Expected: {expected_state}. \
     Found: {actual_state}. \
     Why this matters: {business_logic_significance}.",
);
```

---

## Template Components

### 1. What Should Happen (Description)

A concise, active sentence describing the expected behavior.

**Pattern:** `<Subject> should <verb> <object>`  
**Examples:**
- "Code 0x00 should map to 'A'"
- "Overlay should have exactly 4 entries after filtering"
- "Glyph 'A' should NOT be identified as unmapped"
- "CMAP should not be empty after parsing valid input"

### 2. Expected Section

Describes the expected value in human-readable terms.

**Placeholders:**
- `{expected_description}` - Human-readable description
- `{expected_value}` - Literal expected value

**Examples:**
- `Expected: Some("A")`
- `Expected: 4 mappings`
- `Expected: empty Vec`
- `Expected: is_unmapped_glyph_name("A") == false`

### 3. Found Section

Shows the actual value using format placeholders.

**Placeholders:**
- `{:?}` - For Debug formatting (most common)
- `{}` - For Display formatting (counts, lengths)
- `{actual_state}` - For boolean states (true/false)

**Examples:**
- `Found: {:?}` (format the actual value)
- `Found: {} mappings` (format a count)
- `Found: true` (for boolean failures)
- `Found: {expected_description}` (repeat expected description if opposite state)

### 4. Why Section

Explains the business logic significance and/or references the configuration source of truth.

**Guidelines:**
- Reference the configuration file when applicable (e.g., `build/unmapped-glyph-names.json`)
- Explain the consequence if this assertion fails
- Keep it concise but informative

**Examples:**
- `Why: CMAP table defines 0x00→A mapping.`
- `Why this matters: .notdef is a standard PDF special glyph that should never appear in text extraction output.`
- `Why: 5 unmapped glyphs filtered from the original set.`
- `Why this matters: If the CMAP parser produces an empty map from valid input, the parser is incorrectly rejecting all glyphs.`

---

## Assertion Type Templates

### Type 1: Glyph Lookup Assertions

**Use when:** Verifying character-to-glyph mappings in CMAP or encoding tables.

```rust
assert_eq!(
    map.lookup(&[0x00]),
    Some(&['A'][..]),
    "Byte 0x{byte:X} should map to '{glyph}'. \
     Expected: Some(\"{glyph}\"). \
     Found: {:?}. \
     Why: CMAP table defines 0x{byte:X}→{glyph} mapping.",
    map.lookup(&[0x00]),
    byte = 0x00,
    glyph = 'A'
);
```

**Real example:**
```rust
assert_eq!(
    map.lookup(&[0x00]),
    Some(&['A'][..]),
    "Byte 0x00 should map to 'A'. \
     Expected: Some(\"A\"). \
     Found: {:?}. \
     Why: CMAP table defines 0x00→A mapping.",
    map.lookup(&[0x00])
);
```

---

### Type 2: Count/Length Assertions

**Use when:** Verifying collection sizes, entry counts, or array lengths.

```rust
assert_eq!(
    collection.len(),
    expected_count,
    "{collection_name} should have exactly {expected_count} entries after {operation}. \
     Expected: {expected_count} entries. \
     Found: {} entries. \
     Why: {reason_for_count}.",
    collection.len()
);
```

**Real example:**
```rust
assert_eq!(
    overlay.len(),
    4,
    "Overlay should have exactly 4 entries after filtering. \
     Expected: 4 entries. \
     Found: {} entries. \
     Why: 5 unmapped glyphs filtered from the original set of 9.",
    overlay.len()
);
```

---

### Type 3: Boolean Predicate Assertions

**Use when:** Verifying boolean conditions or predicate functions.

```rust
assert!(
    condition,
    "{subject} should {expected_state}. \
     Expected: {predicate_description} == {expected_bool}. \
     Found: {actual_bool}. \
     Why this matters: {business_logic_significance}.",
);
```

**Real example:**
```rust
assert!(
    is_unmapped_glyph_name(".notdef"),
    ".notdef should be identified as unmapped. \
     Expected: true. \
     Found: {}. \
     Why this matters: .notdef is the standard PDF fallback glyph configured in \
     build/unmapped-glyph-names.json and must never appear in text extraction.",
    is_unmapped_glyph_name(".notdef")
);
```

**Negative example:**
```rust
assert!(
    !is_unmapped_glyph_name("A"),
    "A should NOT be identified as unmapped. \
     Expected: false. \
     Found: {}. \
     Why this matters: A is a normal Latin letter that should always be preserved in text.",
    is_unmapped_glyph_name("A")
);
```

---

### Type 4: Unmapped Glyph Assertions

**Use when:** Verifying that special glyphs (.notdef, .null, etc.) are properly handled.

```rust
assert!(
    is_unmapped_glyph_name("{glyph_name}"),
    "{glyph_name} should be identified as unmapped. \
     Expected: true. \
     Found: {}. \
     Why this matters: {glyph_name} is a standard PDF special glyph configured in \
     build/unmapped-glyph-names.json and must never appear in text extraction output.",
    is_unmapped_glyph_name("{glyph_name}")
);
```

**Real examples:**

**Example A - .notdef assertion:**
```rust
assert!(
    is_unmapped_glyph_name(".notdef"),
    ".notdef should be identified as unmapped. \
     Expected: true. \
     Found: {}. \
     Why this matters: .notdef is the standard PDF fallback glyph configured in \
     build/unmapped-glyph-names.json and must never appear in text extraction.",
    is_unmapped_glyph_name(".notdef")
);
```

**Example B - Normal glyph assertion:**
```rust
assert!(
    !is_unmapped_glyph_name("A"),
    "Normal glyph 'A' should not be recognized as unmapped. \
     Expected: is_unmapped_glyph_name(\"A\") == false. \
     Found: true. \
     Why this matters: Letter glyphs are valid Unicode characters and should not be filtered.",
);
```

---

### Type 5: CMAP Table Assertions

**Use when:** Verifying CMAP parsing, mapping correctness, or filtering behavior.

```rust
assert_eq!(
    cmap_result,
    expected_value,
    "{description}. \
     Expected: {expected_description}. \
     Found: {:?}. \
     Why this matters: {business_logic_significance}.",
    cmap_result
);
```

**Real examples:**

**Example A - CMAP presence check:**
```rust
assert!(
    !map.is_empty(),
    "CMAP should not be empty after parsing valid glyph mappings. \
     Expected: non-empty map. \
     Found: empty map. \
     Why this matters: If the CMAP parser produces an empty map from valid input, \
     the parser is incorrectly rejecting all glyphs or has a parsing error."
);
```

**Example B - CMAP mapping count:**
```rust
assert_eq!(
    map.len(),
    4,
    "CMAP should have exactly 4 mappings after parsing. \
     Expected: 4 mappings (A, B, space, C). \
     Found: {} mappings. \
     Why this matters: Incorrect mapping count indicates the parser is dropping \
     or duplicating entries.",
    map.len()
);
```

---

### Type 6: Configuration Assertions

**Use when:** Verifying configuration loading, defaults, or parsing.

```rust
assert!(
    config.field.{condition}(),
    "Field {field_name} should {expected_behavior}. \
     Expected: {expected_state}. \
     Found: {:?}. \
     Why this matters: {reason} and references {config_file}.",
    config.field
);
```

**Real example:**
```rust
assert!(
    config.unmapped_glyph_names.is_empty(),
    "unmapped_glyph_names should default to an empty list when not specified in config. \
     Expected: empty Vec. \
     Found: {:?}. \
     Why this matters: The #[serde(default)] attribute ensures empty Vec when field is absent, \
     preventing null/None errors during encoding fixture generation.",
    config.unmapped_glyph_names
);
```

---

### Type 7: Diagnostics Assertions

**Use when:** Verifying that parsing or processing produces no error/warning diagnostics.

```rust
assert!(
    diagnostics.is_empty(),
    "{operation} should not generate diagnostics. \
     Expected: empty diagnostics list. \
     Found: {} diagnostics. \
     Why: Input is well-formed according to {specification}.",
    diagnostics.len()
);
```

**Real example:**
```rust
assert!(
    diagnostics.is_empty(),
    "Parsing should not generate diagnostics. \
     Expected: empty. \
     Found: {} diagnostics. \
     Why: Input is well-formed CMAP data.",
    diagnostics.len()
);
```

---

## Formatting Guidelines

### Line Continuation

For long messages, use Rust's line continuation (`\`) to keep the message readable:

```rust
assert!(
    condition,
    "Short description. \
     Expected: {expected}. \
     Found: {found}. \
     Why this matters: This is a long explanation that requires multiple lines \
     to be readable while maintaining proper formatting.",
);
```

### Placeholder Formatting

Use the appropriate format specifier:
- `{:?}` - Debug formatting (most values, collections)
- `{}` - Display formatting (numbers, strings, counts)
- `{:#?}` - Pretty-print Debug formatting (complex structures)

**Example:**
```rust
// Debug format for complex values
Found: {:?}

// Display format for simple counts
Found: {} entries

// Pretty-print for deeply nested structures
Found: {:#?}
```

---

## Configuration Sources of Truth

When referencing configuration files in the "Why" section, use these canonical paths:

| Configuration | Path | Purpose |
|--------------|------|---------|
| Unmapped glyph names | `build/unmapped-glyph-names.json` | Defines glyphs to skip during text extraction |
| CMAP test fixtures | `tests/fixtures/cmap/` | Sample CMAP data for testing |
| Encoding fixtures | `tests/fixtures/encoding/` | Sample encoding table data |
| PDF specification | (referenced by name) | Adobe PDF specification rules |

**Example references:**
- `Why: Configured in build/unmapped-glyph-names.json as unmapped.`
- `Why: PDF specification requires /Differences array to override base encoding.`
- `Why: CMAP format 4 defines the subtable structure.`

---

## Complete Examples by Category

### Example 1: Unmapped Glyph Assertions

**Test function:** `test_notdef_is_unmapped`

```rust
assert!(
    is_unmapped_glyph_name(".notdef"),
    ".notdef should be recognized as an unmapped glyph name. \
     Expected: is_unmapped_glyph_name(\".notdef\") == true. \
     Found: false. \
     Why this matters: .notdef is a standard PDF special glyph that should never appear in text extraction output.",
);
```

**Test function:** `test_normal_glyphs_not_unmapped`

```rust
assert!(
    !is_unmapped_glyph_name("A"),
    "Normal glyph 'A' should not be recognized as unmapped. \
     Expected: is_unmapped_glyph_name(\"A\") == false. \
     Found: true. \
     Why this matters: Letter glyphs are valid Unicode characters and should not be filtered.",
);
```

---

### Example 2: CMAP Assertions

**Test function:** `test_notdef_unmapped`

```rust
assert!(
    is_unmapped_glyph_name(".notdef"),
    ".notdef should be identified as unmapped. \
     Expected: true. \
     Found: {}. \
     Why this matters: .notdef is the standard PDF fallback glyph configured in \
     build/unmapped-glyph-names.json and must never appear in text extraction.",
    is_unmapped_glyph_name(".notdef")
);
```

**Test function:** `test_0x00_maps_to_A`

```rust
assert_eq!(
    result,
    Some(&['A'][..]),
    "Byte 0x00 should map to 'A'. \
     Expected: Some(\"A\"). \
     Found: {:?}. \
     Why this matters: This verifies the basic lookup functionality works correctly.",
    result
);
```

**Test function:** `test_cmap_length_exactly_4`

```rust
assert_eq!(
    map.len(),
    4,
    "CMAP should have exactly 4 mappings after parsing. \
     Expected: 4 mappings (A, B, space, C). \
     Found: {} mappings. \
     Why this matters: Incorrect mapping count indicates the parser is dropping \
     or duplicating entries.",
    map.len()
);
```

---

### Example 3: Font Table Assertions

**Test function:** `test_differences_overlay_parse_simple`

```rust
assert_eq!(
    overlay.get(39),
    Some(Arc::from("quotesingle")),
    "Code 39 should map to quotesingle glyph. \
     Expected: Some(\"quotesingle\"). \
     Found: {:?}. \
     Why: /Differences array [ 39 /quotesingle ] should create this mapping.",
    overlay.get(39)
);
```

**Test function:** `test_font_encoding_new`

```rust
assert_eq!(
    encoding.base_encoding_name(),
    Some("StandardEncoding"),
    "Font encoding should use StandardEncoding as base. \
     Expected: Some(\"StandardEncoding\"). \
     Found: {:?}. \
     Why: No /Differences array present, so base encoding is used directly.",
    encoding.base_encoding_name()
);
```

---

## Anti-Patterns to Avoid

### ❌ Bad: No message at all
```rust
assert_eq!(result, expected);
```
**Problem:** No context when assertion fails.

### ❌ Bad: Message without context
```rust
assert_eq!(result, expected, "result should equal expected");
```
**Problem:** Doesn't show values or explain significance.

### ❌ Bad: Message with values only
```rust
assert_eq!(result, expected, "expected {:?}, got {:?}", expected, result);
```
**Problem:** No explanation of why this matters.

### ❌ Bad: Message without Expected/Found/Why structure
```rust
assert!(condition, "condition failed because result was {:?}", result);
```
**Problem:** Doesn't follow the standard template, missing Expected/Why sections.

---

## Verification Checklist

When reviewing assertion messages, verify:

- [ ] Description clearly states what should happen
- [ ] Expected section describes the expected value/state
- [ ] Found section uses appropriate format placeholder (`{:?}`, `{}`)
- [ ] Why section explains business logic significance or references configuration
- [ ] Configuration files are referenced by canonical path when applicable
- [ ] Message is readable and not overly verbose
- [ ] Line continuations (`\`) are used for multi-line messages
- [ ] Formatting placeholders match the value type

---

## Usage in New Code

When writing new tests or assertions:

1. **Copy the appropriate template** from the Assertion Type Templates section
2. **Fill in the placeholders** with specific values for your test
3. **Reference configuration files** when the assertion validates configuration-driven behavior
4. **Explain the "Why"** - what business logic or specification rule requires this behavior
5. **Use proper format specifiers** - `{?}` for Debug, `{}` for Display

---

## Implementation Status

This template is based on the existing assertion enhancements completed in bead bf-lpyhe. All assertions in the following files have been enhanced to follow this template:

| File | Assertions Enhanced | Status |
|------|---------------------|--------|
| `encoding.rs` | ~50 | ✅ Complete |
| `unmapped_glyph_names_config.rs` | 16 | ✅ Complete |
| `cmap_unmapped_glyphs.rs` | ~70 | ✅ Complete |
| `unmapped.rs` | 9 | ✅ Complete |
| **TOTAL** | **~145** | **✅ 100%** |

---

## Future Enhancements

Potential improvements to consider:

1. **Macro-based template**: Create a `assert_diagnostic!` macro that enforces the Expected/Found/Why structure at compile time
2. **Structured diagnostics**: Output failure messages in structured format (JSON) for tooling
3. **Configuration reference extraction**: Automatically extract and validate configuration file references

---

## References

- **Parent bead:** bf-lpyhe (enhance assertion messages with diagnostic context)
- **Child bead:** bf-63sxe (identify assertions needing enhancement)
- **Configuration:** `build/unmapped-glyph-names.json`
- **PDF Specification:** Adobe PDF Reference (version 1.7 and later)

---

**End of Template**
