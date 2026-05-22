# pdftract-1zhu: Incremental Update Chain Handler Implementation

## Summary
The `/Prev` chain handler for incremental PDF updates was already fully implemented in `crates/pdftract-core/src/parser/xref.rs`. All acceptance criteria are met.

## Implementation Details

### Core Function: `load_xref_with_prev_chain`
Located at `xref.rs:2154-2269`, this function:
1. Loads the trailing xref (auto-detects traditional vs stream vs hybrid)
2. Recursively follows `/Prev` pointers in trailers
3. Merges revisions with override semantics (newest wins for each object number)
4. Returns the latest revision's trailer

### Key Features Implemented
- **Cycle detection**: `HashSet<u64>` tracks visited offsets, emits `STRUCT_CIRCULAR_REF`
- **Depth limit**: `MAX_PREV_DEPTH = 32`, emits `STRUCT_DEPTH_EXCEEDED` on exceed
- **Override semantics**: For each ObjRef, LATER revision (loaded first) wins
- **Trailer handling**: Latest revision's trailer returned (newest /Root, /Info, /ID)
- **Edge cases**:
  - `/Prev <= 0` treated as absent (no previous revision)
  - `/Prev > file_size` emits `STRUCT_INVALID_PREV_OFFSET`, ignores /Prev
  - Hybrid files: each revision calls `load_single_xref` which handles hybrid merging

### Hybrid Support
`load_single_xref` (xref.rs:2071-2107) detects hybrid files via `is_hybrid_trailer` and calls `merge_hybrid` when `/XRefStm` is present. This is invoked at each level of the `/Prev` chain.

## Test Results

### All `/Prev` Chain Tests PASS (12/12)
```
test parser::xref::tests::test_prev_chain_negative_prev_is_absent ... ok
test parser::xref::tests::test_prev_chain_object_added_only_in_latest ... ok
test parser::xref::tests::test_prev_chain_trailer_from_latest ... ok
test parser::xref::tests::test_prev_chain_three_revisions_latest_wins ... ok
test parser::xref::tests::test_prev_chain_zero_prev_is_absent ... ok
test parser::xref::tests::test_prev_chain_cycle_detection ... ok
test parser::xref::tests::test_prev_chain_depth_limit ... ok
test parser::xref::tests::test_prev_chain_invalid_offset ... ok
test parser::xref::tests::test_prev_chain_object_add_modify_free ... ok
test parser::xref::tests::test_prev_chain_hybrid_file ... ok
test parser::xref::tests::proptest_prev_chain_tests::prop_prev_chain_random_no_panic ... ok
test parser::xref::tests::proptest_prev_chain_tests::prop_prev_chain_random_offsets_no_panic ... ok
```

### Acceptance Criteria Status
| Criterion | Status | Notes |
|-----------|--------|-------|
| 3-revision chain, latest wins | ✅ PASS | test_prev_chain_three_revisions_latest_wins |
| Object 7: add/modify/free lifecycle | ✅ PASS | test_prev_chain_object_add_modify_free |
| Object added only in latest | ✅ PASS | test_prev_chain_object_added_only_in_latest |
| Trailer from latest revision | ✅ PASS | test_prev_chain_trailer_from_latest |
| /Prev cycle detection | ✅ PASS | test_prev_chain_cycle_detection |
| Depth limit (32 revisions) | ✅ PASS | test_prev_chain_depth_limit |
| proptest: random configs | ✅ PASS | prop_prev_chain_random_no_panic |
| INV-8 maintained | ✅ PASS | No changes to xref module structure |

## Pre-existing Issues
Some xref tests fail (forward_scan, multi-subsection parsing), but these are unrelated to the `/Prev` chain handler and represent pre-existing issues from earlier work.

## Files
- Implementation: `crates/pdftract-core/src/parser/xref.rs:2154-2269`
- Tests: `crates/pdftract-core/src/parser/xref.rs:3826-4338`
- Constants: `MAX_PREV_DEPTH = 32` at line 2113

## Verification
```bash
cargo test -p pdftract-core prev_chain --lib
# Result: 12 passed; 0 failed
```
