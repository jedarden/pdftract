# pdftract-2a6rk: OCG /OCProperties parsing verification

## Summary

The OCG (Optional Content Groups) implementation is complete in `/home/coding/pdftract/crates/pdftract-core/src/parser/ocg.rs`. The implementation was already present in the codebase when this task was picked up.

## Implementation details

### Public API
- `parse_oc_properties(resolver: &XrefResolver, oc_props_ref: Option<ObjRef>) -> OcProperties`
- Located in `crates/pdftract-core/src/parser/ocg.rs:273-422`

### Data structures

1. **OcProperties** (lines 192-271):
   - `present: bool` — true if /OCProperties was present
   - `groups: HashMap<ObjRef, OcGroup>` — all OCGs in the document
   - `default_visibility: HashMap<ObjRef, bool>` — default visibility per OCG
   - `base_state: BaseState` — overall default (ON/OFF/Unchanged)
   - `ocmds: HashMap<ObjRef, Ocmd>` — optional content membership dicts
   - `diagnostics: Vec<Diagnostic>` — parsing diagnostics

2. **OcGroup** (lines 122-186):
   - `name: Option<String>` — /Name entry
   - `intent: Vec<String>` — /Intent (e.g., "View", "Design")
   - `usage: Option<PdfDict>` — /Usage dictionary

3. **BaseState** enum (lines 15-49):
   - `On`, `Off`, `Unchanged`
   - `Unchanged` treated as `On` for default config

4. **Ocmd** and **OcmdPolicy** (lines 51-120):
   - OCMD support for boolean combinations (AllOn, AnyOn, AllOff, AnyOff)
   - Policy evaluation deferred to Phase 3

### Catalog integration

The catalog.rs already integrates OCG parsing:
- Lines 486-491: Extract /OCProperties from catalog
- Stores result in `Catalog.oc_properties: Option<OcProperties>`

## Acceptance criteria verification

| Criteria | Status | Test |
|----------|--------|------|
| EC-16: OCG with default OFF state | ✅ PASS | `test_parse_oc_properties_base_state_off` |
| /BaseState OFF + /ON [ocg1 ocg2] | ✅ PASS | `test_parse_oc_properties_with_on_array` |
| No /OCProperties: present = false | ✅ PASS | `test_oc_properties_not_present` |
| /OCMD with /AllOn policy preserved | ✅ PASS | `test_ocmd_parse`, `test_ocmd_evaluation_all_on` |
| proptest: never panics on random input | ✅ PASS | `fuzz_parse_oc_properties_no_panics`, `fuzz_ocg_group_parse_no_panics`, `fuzz_ocmd_parse_no_panics` |
| INV-8: no panics on arbitrary input | ✅ PASS | All proptests verify INV-8 compliance |

## Test results

```
running 20 tests
test parser::ocg::tests::test_base_state_from_name ... ok
test parser::ocg::tests::test_ocmd_evaluation_any_on ... ok
test parser::ocg::tests::test_ocmd_policy_from_name ... ok
test parser::ocg::tests::test_ocg_group_parse ... ok
test parser::ocg::tests::test_ocg_name_none ... ok
test parser::ocg::tests::test_ocmd_parse ... ok
test parser::ocg::tests::test_ocmd_parse_single_ref ... ok
test parser::ocg::tests::test_parse_oc_properties_base_state_off ... ok
test parser::ocg::tests::test_parse_oc_properties_off_overrides_on ... ok
test parser::ocg::tests::test_parse_oc_properties_simple ... ok
test parser::ocg::tests::test_ocg_name_retrieval ... ok
test parser::ocg::tests::test_ocmd_evaluation_all_on ... ok
test parser::ocg::tests::test_parse_oc_properties_with_off_array ... ok
test parser::ocg::tests::test_parse_oc_properties_with_on_array ... ok
test parser::ocg::tests::test_unknown_ocg_treated_as_visible ... ok
test parser::ocg::tests::test_oc_properties_not_present ... ok
test parser::ocg::tests::test_base_state_as_bool ... ok
test parser::ocg::tests::proptests::fuzz_ocmd_parse_no_panics ... ok
test parser::ocg::tests::proptests::fuzz_parse_oc_properties_no_panics ... ok
test parser::ocg::tests::proptests::fuzz_ocg_group_parse_no_panics ... ok

test result: ok. 20 passed; 0 failed; 0 ignored
```

## Phase 3 integration

The `OcProperties` struct provides:
- `is_visible(ocg_ref: ObjRef) -> bool` — check OCG default visibility
- `is_ocmd_visible(ocmd_ref: ObjRef) -> bool` — evaluate OCMD policy
- `ocg_name(ocg_ref: ObjRef) -> Option<&str>` — get OCG name

Phase 3 content stream processing will use these methods to suppress glyphs inside `/OC /OCGRef` BDC blocks when the referenced OCG is OFF.

## Files

- Implementation: `crates/pdftract-core/src/parser/ocg.rs` (909 lines)
- Catalog integration: `crates/pdftract-core/src/parser/catalog.rs` (lines 10, 326, 486-491)
- Tests: inline in `ocg.rs` (lines 424-908)

## Changes made

Fixed compilation error in `crates/pdftract-core/src/parser/xref.rs:3460`:
- Issue: u64 literal `0x5DEECE66D` used with u32 state caused overflow
- Fix: Changed `state` to u64 for proper Java Random algorithm behavior
- This was blocking the test suite from running

## Retrospective

- **What worked:** The implementation was already complete and well-tested
- **What didn't:** N/A (implementation was already present)
- **Surprise:** The bead description matched an existing implementation exactly
- **Reusable pattern:** Use `OcProperties::not_present()` for documents without /OCProperties
