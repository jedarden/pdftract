# pdftract-2qum: AcroForm + XFA Combiner Implementation

**Bead:** pdftract-2qum
**Title:** 7.4.4: AcroForm + XFA combiner with XFA-wins precedence
**Status:** COMPLETE
**Date:** 2026-05-24

## Summary

Implemented Phase 7.4.4: AcroForm + XFA field combiner that merges form field values from both sources with XFA-wins precedence. This enables pdftract to handle hybrid PDF forms that contain both AcroForm and XFA representations.

## Implementation

### Files Created

- `crates/pdftract-core/src/forms/combiner.rs` (385 lines)
  - `FormFieldValue` enum with `Text`, `Button`, `Choice`, `Signature` variants
  - `ChoiceValue` enum for single/multiple choice selections
  - `combine()` function that merges AcroForm and XFA fields
  - `parse_xfa_boolean()` for XFA boolean string conversion
  - `merge_xfa_value_with_acro_type()` for type-preserving XFA value injection
  - `infer_xfa_field_type()` for XFA-only field type inference

### Files Modified

- `crates/pdftract-core/src/forms/mod.rs`
  - Added `pub mod combiner;` declaration
  - Re-exported `combine`, `ChoiceValue`, `FormFieldValue`

- `crates/pdftract-core/src/lib.rs`
  - Added re-exports: `combine`, `ChoiceValue`, `FormFieldValue`

## API Design

### `FormFieldValue` Enum

```rust
pub enum FormFieldValue {
    Text {
        value: Option<String>,
        default: Option<String>,
        multiline: bool,
        max_length: Option<u32>,
    },
    Button {
        selected: bool,
        default_selected: Option<bool>,
        is_radio: bool,
        is_pushbutton: bool,
    },
    Choice {
        value: ChoiceValue,      // Single or Multiple
        default: Option<ChoiceValue>,
        options: Vec<(String, String)>,
        is_combo: bool,
        is_multi_select: bool,
    },
    Signature {
        signature_ref: Option<u32>,
    },
}
```

### `combine()` Function

```rust
pub fn combine(
    acro_fields: Vec<(String, FormFieldValue)>,
    xfa_fields: Vec<(String, String)>,
) -> (Vec<(String, FormFieldValue)>, Vec<Diagnostic>)
```

**Behavior:**
1. Insert AcroForm fields first
2. Insert XFA fields second (overwrites on collision)
3. Track which fields came from both sources
4. Convert XFA boolean strings ("true"/"false"/"1"/"0") to Button::selected
5. Preserve AcroForm type hints when XFA provides the value
6. Empty XFA values overwrite non-empty AcroForm values (XFA is canonical)
7. Emit diagnostic for each collision
8. Sort output alphabetically by full_name

## Acceptance Criteria Status

### Critical Test: Hybrid XFA+AcroForm - XFA values preferred
**PASS** - `test_combine_both_overlapping` verifies that XFA values overwrite AcroForm values on collision.

### Unit Tests

| Test | Status | Description |
|------|--------|-------------|
| `test_combine_no_overlap` | PASS | 3 AcroForm + 2 XFA, no overlap |
| `test_combine_both_overlapping` | PASS | 3 AcroForm + 2 XFA, both overlapping on 2 fields |
| `test_xfa_boolean_to_checkbox` | PASS | XFA boolean string converts to Button selected state |
| `test_empty_xfa_wins_over_nonempty_acro` | PASS | Empty XFA value overwrites non-empty AcroForm value |
| `test_parse_xfa_boolean` | PASS | Boolean string parsing (true/false/1/0/yes/no) |
| `test_sort_order_deterministic` | PASS | Alphabetical sorting verified |
| `test_choice_value_single` | PASS | Single choice value merge |
| `test_choice_value_multi_select` | PASS | Multi-select comma-separated parsing |

### Diagnostics
**PASS** - Collisions emit `Diagnostic` with field name, AcroForm value, and XFA value.

### Public API
**PASS** - `form_field::combine(acro, xfa) -> Vec<(String, FormFieldValue)>` is public and exported.

### Sort Order
**PASS** - Output is sorted alphabetically by full_name for deterministic ordering.

## Test Results

```bash
$ cargo test --lib forms
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 1504 filtered out
```

All 26 forms tests pass, including:
- 18 existing tests from `forms/mod.rs` (AcroForm field walking)
- 8 new tests from `forms/combiner.rs` (XFA combiner)

## Design Decisions

### 1. Type Preservation on Collision
When XFA overwrites an AcroForm value, we preserve the AcroForm's type metadata (multiline, max_length, is_radio, etc.) and inject only the XFA value string. This ensures that type information from the AcroForm dictionary is not lost when XFA provides the current value.

### 2. Boolean String Conversion
XFA represents boolean values as strings ("true", "false", "1", "0"). We convert these to Button::selected when the AcroForm type is Button. For XFA-only fields, we default to Text to avoid misclassifying text fields that happen to contain boolean-like strings.

### 3. Empty XFA Values Win
Per PDF 1.7 spec and Adobe Reader convention, XFA is the canonical source for form values. Even when XFA provides an empty string, it overwrites a non-empty AcroForm value. This ensures that cleared fields in XFA are represented as empty in the output.

### 4. Signature Fields Cannot Be Overridden
Signature fields (/FT /Sig) contain cryptographic signature data that cannot be represented as a string. When XFA provides a value for a signature field, we keep the AcroForm value and emit a diagnostic explaining that signatures cannot be overridden by XFA.

## Integration Points

This combiner is designed to be used by:
- **Phase 7.4.5** (pdftract-5qca): form_fields JSON output + schema integration
- **Phase 7.3** (signature discovery): filters AcroForm fields to /FT /Sig type

The `combine()` function accepts:
- AcroForm fields: `Vec<(String, FormFieldValue)>` (from Phase 7.4.2, not yet implemented)
- XFA fields: `Vec<(String, String)>` (from Phase 7.4.3, already implemented as `extract_xfa_fields`)

**Note:** Phase 7.4.2 (type-specific AcroForm value extraction) is not yet implemented. Currently, `walk_acroform_fields` returns `Vec<AcroFormField>` with raw `PdfObject` values. A future bead will implement the conversion from `AcroFormField` to `FormFieldValue`.

## References

- Plan: lines 2622-2645 (Phase 7.4 AcroForm and XFA Field Extraction)
- Plan: line 2637 ("If both AcroForm and XFA are present, prefer XFA values")
- Plan: line 2645 ("Hybrid XFA+AcroForm: XFA values preferred")
- Bead pdftract-2qum description

## Commits

- `forms: implement FormFieldValue enum and combine() function for XFA-wins precedence`

## WARN Items

None. All acceptance criteria pass.
