# pdftract-34hxw: AcroForm Tx (text field) value extraction

## Status: PASS (implementation already present)

## Summary

The AcroForm /Tx (text field) value extraction is already fully implemented in `crates/pdftract-core/src/forms/value_text.rs`. The implementation correctly handles all requirements from the bead description.

## Implementation Verification

### Module Location
- **File:** `crates/pdftract-core/src/forms/value_text.rs` (737 lines)
- **Exports:** `TextValue` struct and `extract_text_value` function
- **Re-exports in:** `crates/pdftract-core/src/forms/mod.rs`

### TextValue Struct
```rust
pub struct TextValue {
    pub value: Option<String>,      // Current value (/V)
    pub default: Option<String>,    // Default value (/DV)
    pub multiline: bool,            // /Ff bit 12 (1<<12 = 0x1000)
    pub max_length: Option<u32>,    // /MaxLen (negative → None)
}
```

### FormFieldValue::Text Variant
The `FormFieldValue::Text` variant is properly defined in `combiner.rs`:
```rust
pub enum FormFieldValue {
    Text {
        value: Option<String>,
        default: Option<String>,
        multiline: bool,
        max_length: Option<u32>,
    },
    // ... other variants
}
```

### PDFDocEncoding/UTF-16BE Decoding
The `decode_pdf_string()` function correctly implements:
1. UTF-16BE BOM detection (`0xFE 0xFF` prefix)
2. UTF-16BE decoding (with and without BOM)
3. PDFDocEncoding fallback (full 29-character override table from PDF spec Annex D.2)
4. Heuristic UTF-16BE detection for malformed inputs

### Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Text field with /V → FormFieldValue::Text { value: Some(...), ... } | ✅ PASS | `test_extract_text_value_basic` |
| UTF-16BE BOM-prefixed /V → correct Unicode decode | ✅ PASS | `test_extract_text_value_utf16be_bom` |
| /Ff multiline bit set → multiline: true | ✅ PASS | `test_extract_text_value_multiline` |
| /MaxLen 50 → max_length: Some(50) | ✅ PASS | `test_extract_text_value_with_max_length` |
| Empty /V → value: Some("") | ✅ PASS | `test_extract_text_value_empty_value` |
| Missing /V → value: None | ✅ PASS | `test_extract_text_value_no_value` |

### Test Results
All 12 text_value tests passed:
```
PASS [   0.014s] test_extract_text_value_name_as_value
PASS [   0.016s] test_extract_text_value_with_max_length
PASS [   0.016s] test_extract_text_value_utf16be_bom
PASS [   0.017s] test_text_value_empty_constructor
PASS [   0.016s] test_extract_text_value_multiline
PASS [   0.017s] test_text_value_equality
PASS [   0.017s] test_extract_text_value_with_default
PASS [   0.018s] test_extract_text_value_basic
PASS [   0.018s] test_extract_text_value_no_value
PASS [   0.020s] test_extract_text_value_negative_max_length_ignored
PASS [   0.020s] test_extract_text_value_combined_flags
PASS [   0.021s] test_extract_text_value_empty_value
Summary [   0.028s] 12 tests run: 12 passed, 2660 skipped
```

### Additional Test Coverage
The module also includes comprehensive PDFDocEncoding tests:
- `test_decode_pdf_string_ascii`
- `test_decode_pdf_string_utf16be_bom`
- `test_decode_pdf_string_utf16be_bom_odd_length`
- `test_decode_pdf_string_pdfdocencoding_latin1`
- `test_decode_pdf_string_pdfdocencoding_lower_latin1`
- `test_decode_pdf_string_pdfdocencoding_bullet`
- `test_decode_pdf_string_pdfdocencoding_em_dash`
- `test_decode_pdf_string_pdfdocencoding_quotes`
- `test_decode_pdf_string_empty`
- `test_looks_like_utf16be`
- `test_extract_string_from_value_unrecognized_type`
- `test_decode_pdf_string_never_panics`
- `test_extract_text_value_combined_flags`

### Integration
The implementation is properly integrated:
1. Exported from `forms/mod.rs` as `pub use value_text::{extract_text_value, TextValue}`
2. Used by `acro_field_to_value()` function for Tx field conversion
3. Consumed by `combine()` function in combiner.rs
4. Part of the FormFieldValue enum for JSON serialization

## Conclusion

No code changes were required. The implementation is complete, well-tested, and properly integrated into the forms pipeline. All acceptance criteria are met.
