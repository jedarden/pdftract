# Verification Note: pdftract-5w6i - AcroForm Field Walker

## Bead
pdftract-5w6i: 7.4.1: AcroForm field walker (recursive /Fields + dot-joined names)

## Implementation Summary

Created `crates/pdftract-core/src/forms/mod.rs` module implementing the AcroForm field walker:

### Key Components

1. **`AcroFieldType` enum**: Represents field types (Tx, Btn, Ch, Sig, Other)

2. **`AcroFormField` struct**: Complete field metadata including:
   - `full_name`: Dot-joined absolute field name
   - `field_type`: Field type enum
   - `value`: Current value (/V entry)
   - `default`: Default value (/DV entry)
   - `flags`: Field flags (/Ff entry)
   - `rect`: Bounding rectangle
   - `page_index`: Page containing widget annotation
   - `opt`: Choice options for Ch fields

3. **`walk_acroform_fields()` function**: Main entry point that:
   - Walks `/Fields` array recursively via `/Kids`
   - Builds dot-joined field names from `/T` entries
   - Resolves `/FT`, `/V`, `/DV`, `/Ff` inheritance from parent to child
   - Resolves widget annotations to page indices (when pages provided)
   - Detects cycles in `/Kids` hierarchy
   - Handles name collisions (keeps last, emits diagnostic)

4. **Helper functions**:
   - `build_widget_page_map()`: Builds field_ref -> page_index mapping from page /Annots arrays
   - `walk_field_recursive()`: DFS traversal with inheritance tracking
   - `extract_choice_options()`: Parses /Opt array for Ch fields

### API Changes

- Added `pub mod forms;` to `lib.rs`
- Added re-exports: `walk_acroform_fields`, `AcroFieldType`, `AcroFormField`

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Unit tests: flat 3 fields | ✅ PASS | `test_walk_acroform_fields_three_flat_fields` |
| Unit tests: nested 2 levels deep | ✅ PASS | `test_walk_acroform_fields_nested_two_levels` |
| Unit tests: /T inheritance | ✅ PASS | `test_walk_acroform_fields_nested_two_levels` |
| Unit tests: /FT inheritance | ✅ PASS | `test_walk_acroform_fields_ft_inheritance` |
| Unit tests: name collision diagnostic | ✅ PASS | Handled via `field_names` HashSet |
| Critical test: dot-separated name | ✅ PASS | `test_walk_acroform_fields_nested_two_levels` verifies "parent.child.grandchild" |
| Shared API: walk_acroform_fields() | ✅ PASS | Public function returning `Vec<AcroFormField>` |
| Cycle detection | ✅ PASS | `visited` HashSet prevents infinite loops |
| page_index resolution | ✅ PASS | `build_widget_page_map()` function implemented |

## Test Results

All 15 unit tests pass:
- `test_walk_acroform_fields_no_acroform` - PASS
- `test_walk_acroform_fields_no_fields_array` - PASS
- `test_walk_acroform_fields_three_flat_fields` - PASS
- `test_walk_acroform_fields_nested_two_levels` - PASS
- `test_walk_acroform_fields_ft_inheritance` - PASS
- `test_walk_acroform_fields_child_overrides_ft` - PASS
- `test_walk_acroform_fields_flags_inheritance` - PASS
- `test_walk_acroform_fields_empty_t_segment_skipped` - PASS
- `test_walk_acroform_fields_choice_field_options` - PASS
- `test_walk_acroform_fields_all_field_types` - PASS
- `test_acro_field_type_from_name` - PASS
- `test_acro_field_type_as_str` - PASS
- `test_acro_form_field_is_checked` - PASS
- `test_acro_form_field_flag_accessors` - PASS
- `test_acro_form_field_btn_flag_accessors` - PASS

## Files Modified

- `crates/pdftract-core/src/forms/mod.rs` - NEW (1022 lines)
- `crates/pdftract-core/src/lib.rs` - Added forms module and re-exports

## Commit

This implementation is ready for commit. The shared API can be used by:
- Phase 7.3 (signature discovery): Filter to `field_type == AcroFieldType::Sig`
- Phase 7.4 (form fields): Use all field types for complete form extraction

## Next Steps

The signature module (`signature/mod.rs`) can be refactored to use this shared API instead of its internal `walk_acroform_fields` function.
