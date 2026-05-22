# pdftract-5og4: Hybrid Xref Handler Implementation

## Summary

Implemented the hybrid xref handler that merges traditional xref tables with xref streams for hybrid PDF files. The traditional table is authoritative for objects it covers; the stream's type-2 entries fill gaps not covered by the traditional table.

## Changes Made

### 1. Added `StructHybridConflict` diagnostic code
- File: `crates/pdftract-core/src/parser/xref.rs`
- Added new variant to `XrefDiagCode` enum for hybrid conflict diagnostics

### 2. Fixed `merge_hybrid` function
- Fixed borrow checker error: was iterating by ownership then trying to borrow
- Changed to iterate by reference: `for (obj_nr, entry) in &traditional.entries`
- Updated to use new `XrefDiagCode::StructHybridConflict` diagnostic code
- Removed unused `use crate::diagnostics::DiagCode;` import

### 3. Updated test
- File: `crates/pdftract-core/src/parser/xref.rs`
- Updated `test_merge_hybrid_free_inuse_conflict` to check for `XrefDiagCode::StructHybridConflict`
- Removed unused `use crate::diagnostics::DiagCode;` import

### 4. Exported public API
- File: `crates/pdftract-core/src/parser/mod.rs`
- Added `merge_hybrid` and `is_hybrid_trailer` to public re-exports

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Critical test passes: traditional entries override stream entries | PASS | `test_merge_hybrid_traditional_priority` |
| Hybrid fixture with stream-only type-2 entries: gap fill works | PASS | `test_merge_hybrid_gap_fill` |
| Free/InUse conflict test: STRUCT_HYBRID_CONFLICT diagnostic emitted | PASS | `test_merge_hybrid_free_inuse_conflict` |
| Non-hybrid trailer (no /XRefStm): merge not invoked | PASS | `is_hybrid_trailer` returns false |
| proptest: random combinations never panic | PASS | `test_merge_hybrid_proptest_simple` |
| INV-8 maintained | PASS | All tests pass, no regressions |

## Test Results

All 9 hybrid xref tests pass:
- `test_merge_hybrid_traditional_priority` - traditional entries override stream entries
- `test_merge_hybrid_free_inuse_conflict` - Free/InUse conflict emits diagnostic
- `test_merge_hybrid_gap_fill` - stream-only type-2 entries fill gaps
- `test_merge_hybrid_trailer_xrefstm_removed` - /XRefStm key removed from merged trailer
- `test_is_hybrid_trailer_detection` - hybrid trailer detection works
- `test_merge_hybrid_empty_sections` - edge case: empty sections
- `test_merge_hybrid_stream_only` - edge case: traditional empty, stream has entries
- `test_merge_hybrid_traditional_only` - edge case: stream empty, traditional has entries
- `test_merge_hybrid_proptest_simple` - proptest verifies no panics

## Implementation Notes

The `merge_hybrid` function implements the correct priority semantics per PDF spec:
1. Start with all traditional entries
2. For each stream entry: if the same ObjRef is NOT in the traditional map, insert it
3. If an ObjRef IS in the traditional map (even as type-1 Free), traditional wins
4. Emit `STRUCT_HYBRID_CONFLICT` diagnostic when traditional Free conflicts with stream InUse
5. The merged trailer is the traditional one with `/XRefStm` key removed
6. The result has `is_hybrid: true` set

## Files Modified

- `crates/pdftract-core/src/parser/xref.rs` - Added diagnostic code, fixed merge function, updated tests
- `crates/pdftract-core/src/parser/mod.rs` - Exported public API functions

## Git Commits

- `fix(pdftract-5og4): add StructHybridConflict diagnostic code and fix merge_hybrid borrow error`
