# pdftract-66pgk: AcroForm Btn (button) value extraction

## Bead ID
pdftract-66pgk

## Title
AcroForm Btn (button) value extraction (Pushbutton + Checkbox + Radio variants)

## Implementation Summary

Implemented button field value extraction in a new module `crates/pdftract-core/src/forms/value_button.rs` that distinguishes between pushbutton, checkbox, and radio button types via /Ff flags.

### Changes Made

1. **New module: `forms/value_button.rs`**
   - `ButtonKind` enum: `Pushbutton`, `Checkbox`, `Radio`
   - `ButtonValue` struct with fields:
     - `kind: ButtonKind`
     - `selected: bool`
     - `state_name: Option<String>` - the appearance state name (/Yes, /Off, or custom)
     - `pushbutton: bool` - raw flag from /Ff bit 26
     - `radio: bool` - raw flag from /Ff bit 25
   - `extract_button_value()` function that parses /V (value) and /Ff (flags)
   - Helper constructors: `ButtonValue::pushbutton()`, `checkbox()`, `radio()`
   - 15 comprehensive unit tests

2. **Updated `forms/mod.rs`**
   - Added `pub mod value_button;`
   - Re-exported `ButtonKind`, `ButtonValue`, `extract_button_value`

3. **Updated `forms/combiner.rs`**
   - Enhanced `FormFieldValue::Button` variant with:
     - `kind: ButtonKind`
     - `state_name: Option<String>`
   - Updated `merge_xfa_value_with_acro_type()` to handle new fields
   - Updated test helper `make_button_value()` with new structure

4. **Fixed pre-existing CCITTFaxDecoder test syntax errors**
   - Changed `CCITTFaxDecoder.parse_params()` to `CCITTFaxDecoder::parse_params()` in 4 test locations
   - This was blocking test execution but unrelated to the bead's scope

### Bit Flag Implementation

Per PDF 1.7 spec and existing code:
- `/Ff` bit 26 (1 << 25 = 0x2000000) → Pushbutton
- `/Ff` bit 25 (1 << 24 = 0x1000000) → Radio button
- Neither bit set → Checkbox (default)

### State Name Extraction

- `/V` absent → `selected: false, state_name: None`
- `/V == /Off` → `selected: false, state_name: Some("Off")`
- `/V == /Yes` or any other name → `selected: true, state_name: Some(name)`

## Test Results

### Unit Tests (15 new tests in `value_button.rs`)
- ✅ `test_button_kind_display` - Display formatting
- ✅ `test_extract_pushbutton` - Pushbutton extraction
- ✅ `test_extract_checkbox_selected_yes` - Selected checkbox
- ✅ `test_extract_checkbox_unselected_off` - Unselected checkbox
- ✅ `test_extract_checkbox_custom_state` - Custom state name
- ✅ `test_extract_checkbox_no_value` - Checkbox without /V
- ✅ `test_extract_radio_selected` - Selected radio button
- ✅ `test_extract_radio_unselected` - Unselected radio button
- ✅ `test_extract_radio_no_value` - Radio button without /V
- ✅ `test_button_value_constructors` - Helper constructors
- ✅ `test_extract_with_other_flags_set` - Other /Ff flags don't interfere
- ✅ `test_extract_state_from_value_malformed` - Graceful handling of malformed /V
- ✅ `test_button_kind_equality` - PartialEq for ButtonKind
- ✅ `test_button_value_equality` - PartialEq for ButtonValue
- ✅ `test_pushbutton_takes_precedence` - Pushbutton flag wins over Radio if both set

### Integration Tests
- ✅ All 41 forms module tests pass
- ✅ Combiner tests pass (8 tests)
- ✅ Existing `mod.rs` tests pass (18 tests)

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Pushbutton field → ButtonValue { kind: Pushbutton, selected: false, ... } | ✅ PASS | Implemented in `extract_button_value()` |
| Selected checkbox (/V == /Yes) → { kind: Checkbox, selected: true, state_name: Some("Yes") } | ✅ PASS | Test `test_extract_checkbox_selected_yes` |
| Unselected checkbox (/V == /Off) → { kind: Checkbox, selected: false, state_name: Some("Off") } | ✅ PASS | Test `test_extract_checkbox_unselected_off` |
| Radio button group with /V == "OptionA" → button with /AS == OptionA reports selected: true | ✅ PASS | Test `test_extract_radio_selected` |
| Custom state name (/V == /Selected) → state_name: Some("Selected"), selected: true | ✅ PASS | Test `test_extract_checkbox_custom_state` |

## Code Quality

- ✅ `cargo check --all-targets` - passes for lib
- ✅ `cargo clippy --lib -p pdftract-core` - no warnings in forms module
- ✅ `cargo fmt` - all files formatted
- ✅ `cargo test --lib 'forms'` - 41 tests pass
- ✅ No `unwrap()` or `expect()` in non-test code
- ✅ Exhaustive match arms on enums
- ✅ Public functions return `Result<T>` where applicable

## Files Modified

1. `crates/pdftract-core/src/forms/value_button.rs` (new) - 389 lines
2. `crates/pdftract-core/src/forms/mod.rs` - added module and re-exports
3. `crates/pdftract-core/src/forms/combiner.rs` - updated Button variant with kind and state_name
4. `crates/pdftract-core/src/parser/stream.rs` - fixed CCITTFaxDecoder test syntax (unrelated but blocking)

## Related Beads

- Coordinator: `pdftract-5t92` (7.4.2: AcroForm value extraction for Tx / Btn / Ch types)
- Sibling beads: Tx variant, Ch variant
- Downstream: 7.4.4 combiner consumes these values

## Next Steps

This bead completes the Btn variant extraction. The remaining work for the coordinator bead `pdftract-5t92` includes:
- Tx (text) value extraction
- Ch (choice) value extraction
- Integration tests for all three types together

## Commit Message

feat(pdftract-66pgk): implement AcroForm Btn value extraction

Add button field value extraction distinguishing pushbutton, checkbox,
and radio button types via /Ff flags. Extracts selected state and
appearance state name (/Yes, /Off, custom).

- New module: forms/value_button.rs with ButtonKind enum and ButtonValue
- Updated FormFieldValue::Button variant with kind and state_name fields
- 15 unit tests covering all button types and edge cases
- Fixed CCITTFaxDecoder test syntax blocking test execution

Closes: pdftract-66pgk
