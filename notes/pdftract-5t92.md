# pdftract-5t92: AcroForm Value Extraction for Tx/Btn/Ch Types

## Summary

Completed Phase 7.4.2: AcroForm value extraction for Tx / Btn / Ch field types. The implementation was already present in the codebase - this task involved fixing two test failures and verifying complete functionality.

## Work Done

### Bug Fixes

1. **Fixed `test_extract_combo_with_multi_select_flag`** (`value_choice.rs:473-491`)
   - **Problem**: When both Combo and MultiSelect flags were set (malformed but possible), the code returned `ChoiceValue::Multiple` instead of `ChoiceValue::Single(Some(_))`.
   - **Root Cause**: `extract_selected_value` was called with `is_multi_select=true` for all fields, but combo boxes are always single-select regardless of the multi-select flag.
   - **Fix**: Modified `extract_choice_value` to pass `is_multi_select && !is_combo` to `extract_selected_value` calls (line 199-205).

2. **Fixed `test_extract_default_none_becomes_none`** (`value_choice.rs:626-637`)
   - **Problem**: Empty string defaults (`Single(Some(""))`) were being filtered out because `is_empty()` returns `true` for empty strings.
   - **Root Cause**: The filter `default_val.filter(|v| !v.is_empty())` treated `Single(Some(""))` as empty and removed it.
   - **Semantics**: An explicit empty string default is different from no default at all. `/DV ""` means "default to empty" vs no `/DV` meaning "no default specified".
   - **Fix**: Added new `is_truly_empty()` method that only returns `true` for `Single(None)` and empty `Multiple`, not for `Single(Some(""))`. Changed filter to use `is_truly_empty()` instead of `is_empty()` (line 210).

### Verification

All acceptance criteria from the plan are met:

| Criterion | Status | Notes |
|-----------|--------|-------|
| Critical test (text, checkbox, dropdown) | **PASS** | `test_extract_values_tx_btn_ch_critical` passes |
| Unit test: unselected checkbox | **PASS** | `test_extract_values_unselected_checkbox` passes |
| Unit test: selected radio | **PASS** | `test_extract_values_selected_radio` passes |
| Unit test: multi-select list | **PASS** | `test_extract_values_multi_select_list` passes |
| Unit test: combo with /Opt 2-tuple entries | **PASS** | `test_extract_values_combo_with_opt_tuples` passes |
| Unit test: multi-line text | **PASS** | `test_extract_values_multiline_text` passes |
| Public API `extract_values` function | **PASS** | `pub fn extract_values(fields: &[AcroFormField]) -> Vec<(String, FormFieldValue)>` exists |
| Sig fields are skipped | **PASS** | `test_extract_values_skips_sig_fields` passes |
| All /Ff bits preserved | **PASS** | `FormFieldValue` variants preserve all flags via `multiline`, `pushbutton`, `radio`, `is_combo`, `is_multi_select` fields |

### Implementation Details

The implementation consists of:

1. **`forms/mod.rs`**: Main entry point `extract_values()` and `acro_field_to_value()` - converts AcroFormField to FormFieldValue.
2. **`forms/value_text.rs`**: Text field extraction with PDFDocEncoding/UTF-16BE BOM decoding via `decode_pdf_string()`.
3. **`forms/value_button.rs`**: Button field extraction distinguishing pushbutton, checkbox, and radio button types via /Ff flags.
4. **`forms/value_choice.rs`**: Choice field extraction for combo/list boxes with single/multi-select support.
5. **`forms/combiner.rs`**: FormFieldValue enum definition for type-safe values.

## Files Modified

- `crates/pdftract-core/src/forms/value_choice.rs`: Fixed multi-select flag handling for combo boxes and empty string default filtering.

## Test Results

```
test result: ok. 96 passed; 0 failed
```

All forms module tests pass:
- 16 tests in `forms::tests` (main module)
- 27 tests in `forms::value_text::tests`
- 31 tests in `forms::value_button::tests`
- 22 tests in `forms::value_choice::tests`

## References

- Plan section 7.4 lines 2610-2613 (Tx/Btn/Ch)
- PDF 1.7 spec 12.7.4.2 (Tx), 12.7.4.3 (Btn), 12.7.4.4 (Ch)
- Phase 1 PdfString decoder (reused for text decoding)
- Phase 7.4.1 (input walker - provides AcroFormField)
- Phase 7.4.4 (combiner consumer - uses FormFieldValue)
