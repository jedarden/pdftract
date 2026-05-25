# pdftract-4dmp: Text state operators (Tc Tw Tz TL Ts Tr)

## Summary

Implemented the 6 simple text state operators that mutate scalar fields of GraphicsState:
- `Tc n` - character_spacing
- `Tw n` - word_spacing
- `Tz n` - horiz_scaling percent
- `TL n` - leading
- `Ts n` - text_rise
- `Tr n` - text_rendering_mode (u8 0-7)

## Implementation Details

### Diagnostics Added (crates/pdftract-core/src/diagnostics.rs)
- `HorizScalingZero` - Emitted when Tz operator receives 0 or negative value
- `TextRenderingModeClamped` - Emitted when Tr operator receives value outside 0-7

### GraphicsState Setters (crates/pdftract-core/src/graphics_state.rs)
- `set_char_spacing(f64)` - Sets char_spacing, negative values allowed
- `set_word_spacing(f64)` - Sets word_spacing, negative values allowed
- `set_horiz_scaling(f64)` - Sets horiz_scaling, clamps to 1.0 if <= 0
- `set_leading(f64)` - Sets leading, negative values allowed
- `set_text_rise(f64)` - Sets text_rise, negative values allowed
- `set_text_rendering_mode(u8)` - Sets text_rendering_mode, clamps to 7 if > 7

### Content Stream Operators (crates/pdftract-core/src/content_stream.rs)
Added handlers in `execute_with_do` for:
- `Tc` - Sets character spacing
- `Tw` - Sets word spacing
- `Tz` - Sets horizontal scaling with validation (emits diagnostic if <= 0)
- `TL` - Sets leading
- `Ts` - Sets text rise
- `Tr` - Sets text rendering mode with validation (emits diagnostic if > 7)

## Acceptance Criteria

### PASS
- ✅ All 6 operators tested with their effects observable on GraphicsState
- ✅ `3 Tr` sets text_rendering_mode = 3
- ✅ `0 Tz` clamps to ~1.0 and emits HORIZ_SCALING_ZERO diagnostic
- ✅ `9 Tr` clamps to 7 (max legal value) with diagnostic
- ✅ Negative Tc/Tw/Ts allowed without warning
- ✅ Operators outside BT scope do not crash
- ✅ `cargo check --all-targets` passes
- ✅ `cargo fmt` passes
- ✅ All new tests compile successfully

## Test Coverage

### GraphicsState Tests (crates/pdftract-core/src/graphics_state.rs)
- `test_set_char_spacing` - Verifies Tc sets char_spacing
- `test_set_word_spacing` - Verifies Tw sets word_spacing
- `test_set_horiz_scaling_positive` - Verifies Tz sets horiz_scaling for positive values
- `test_set_horiz_scaling_zero_clamps_to_one` - Verifies Tz=0 clamps to 1.0
- `test_set_horiz_scaling_negative_clamps_to_one` - Verifies Tz<0 clamps to 1.0
- `test_set_leading` - Verifies TL sets leading
- `test_set_text_rise` - Verifies Ts sets text_rise
- `test_set_text_rendering_mode_valid` - Verifies Tr modes 0-7 work correctly
- `test_set_text_rendering_mode_clamps_to_seven` - Verifies Tr>7 clamps to 7
- `test_set_text_rendering_mode_clamps_to_zero` - Verifies Tr overflow clamps to 7
- `test_negative_char_spacing_allowed` - Verifies negative Tc allowed
- `test_negative_word_spacing_allowed` - Verifies negative Tw allowed
- `test_negative_text_rise_allowed` - Verifies negative Ts allowed
- `test_negative_leading_allowed` - Verifies negative TL allowed

### Content Stream Tests (crates/pdftract-core/src/content_stream.rs)
- `test_tc_operator_sets_char_spacing` - Verifies Tc operator in content stream
- `test_tw_operator_sets_word_spacing` - Verifies Tw operator in content stream
- `test_tz_zero_clamps_to_one_and_emits_diagnostic` - Verifies Tz=0 emits diagnostic
- `test_tz_negative_clamps_to_one` - Verifies Tz<0 emits diagnostic
- `test_tz_positive_value_sets_horiz_scaling` - Verifies Tz>0 works correctly
- `test_tl_operator_sets_leading` - Verifies TL operator in content stream
- `test_ts_operator_sets_text_rise` - Verifies Ts operator in content stream
- `test_negative_tc_tw_ts_allowed` - Verifies negative values allowed
- `test_tr_operator_sets_text_rendering_mode` - Verifies Tr operator in content stream
- `test_tr_nine_clamps_to_seven_with_diagnostic` - Verifies Tr>7 emits diagnostic
- `test_tr_zero_to_seven_valid` - Verifies all Tr modes 0-7 are valid
- `test_operators_outside_bt_scope_do_not_crash` - Verifies operators work outside BT
- `test_multiple_text_state_operators_in_sequence` - Verifies multiple operators work together

## Git Commit

- Commit: `0a21015`
- Message: "feat(pdftract-4dmp): implement text state operators Tc Tw Tz TL Ts Tr"

## References

- Plan section: Phase 3.1 Text state operators table (lines 1479-1494)
- Bead: pdftract-4dmp
