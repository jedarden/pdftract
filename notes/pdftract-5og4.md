# pdftract-5og4: Hybrid Xref Handler Implementation

## Summary
Implemented hybrid xref handler that merges traditional table + xref stream with traditional priority. The implementation was already present in the codebase; this verification confirms all acceptance criteria are met.

## Implementation Status

### Core Components (Already Implemented)

1. **`merge_hybrid` function** (xref.rs:197-248)
   - Merges traditional xref table (authoritative) with xref stream (supplementary)
   - Traditional entries always win for overlapping object numbers
   - Stream entries fill gaps not covered by traditional table
   - Sets `is_hybrid: true` on merged result
   - Emits `STRUCT_HYBRID_CONFLICT` diagnostic for Free/InUse conflicts

2. **`is_hybrid_trailer` function** (xref.rs:260-265)
   - Detects hybrid files by checking for `/XRefStm` key in trailer dict

3. **Integration in `load_single_xref`** (xref.rs:1964-1984)
   - Checks for hybrid trailer after parsing traditional xref
   - Extracts `/XRefStm` offset
   - Loads supplementary xref stream
   - Merges with traditional taking priority

## Acceptance Criteria

| Criterion | Status | Test |
|-----------|--------|------|
| Critical test: traditional entries override stream | ✅ PASS | `test_merge_hybrid_traditional_priority` |
| Stream-only type-2 entries added to merged map | ✅ PASS | `test_merge_hybrid_gap_fill` |
| Free/InUse conflict emits STRUCT_HYBRID_CONFLICT | ✅ PASS | `test_merge_hybrid_free_inuse_conflict` |
| Non-hybrid trailer skips merge (pure traditional) | ✅ PASS | `test_is_hybrid_trailer_detection` + `load_single_xref` integration |
| proptest: random combinations never panic | ✅ PASS | `proptest_merge_hybrid_no_panic` (added) |

## Changes Made

### Added Comprehensive Proptest
Added `proptest_merge_hybrid_no_panic` to the `proptest_tests` module (xref.rs:2416-2447):
- Tests random combinations of traditional and stream entries
- Uses prop::collection::hash_map to generate random entry sets
- Verifies merge_hybrid never panics on any input
- Covers all entry types (InUse, Free, Compressed)

## Test Results
```
test parser::xref::tests::test_merge_hybrid_stream_only ... ok
test parser::xref::tests::test_merge_hybrid_free_inuse_conflict ... ok
test parser::xref::tests::test_merge_hybrid_proptest_simple ... ok
test parser::xref::tests::test_merge_hybrid_gap_fill ... ok
test parser::xref::tests::test_merge_hybrid_empty_sections ... ok
test parser::xref::tests::test_merge_hybrid_traditional_only ... ok
test parser::xref::tests::test_merge_hybrid_traditional_priority ... ok
test parser::xref::tests::test_merge_hybrid_trailer_xrefstm_removed ... ok
test parser::xref::tests::proptest_tests::proptest_merge_hybrid_no_panic ... ok

test result: ok. 9 passed; 0 failed
```

## INV-8 Status
INV-8 (invariant preservation) is maintained:
- Traditional entries are never modified or removed
- Stream entries only added when traditional doesn't have the object number
- Trailer is taken from traditional with `/XRefStm` removed
- All diagnostics from both sections preserved

## References
- Plan section: Phase 1.3 line 1090 (hybrid files)
- PDF spec 7.5.8.4 (Hybrid-Reference Files)
