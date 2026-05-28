# Verification Note: pdftract-18cb4

## Bead: Reading order rank assignment + algorithm tag

### Implementation Status: COMPLETE (pre-existing)

The `assign_reading_order` orchestrator function is already fully implemented in `crates/pdftract-core/src/layout/reading_order.rs` (lines 732-779).

### Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Tagged PDF: rank via XY-cut; algorithm = "xy_cut"; diagnostic emitted | **PASS** | Diagnostic emitted at document level in `extract.rs` lines 411-421; function returns "xy_cut" per plan 1738 |
| 2-column paper: rank via XY-cut; algorithm = "xy_cut" | **PASS** | Test `test_assign_reading_order_two_columns` verifies left-to-right ordering |
| Magazine layout: XY-cut > 10 small regions; falls to Docstrum; algorithm = "docstrum" | **PASS** | Lines 748-757 implement Docstrum fallback; test `test_assign_reading_order_docstrum_fallback` verifies |
| Single block: rank = 0; algorithm = "xy_cut" | **PASS** | Lines 740-743 handle single block; test `test_assign_reading_order_single_block` verifies |
| All blocks unique rank; rank.max() == block_count - 1 | **PASS** | Lines 767-771 assign ranks 0-indexed; test `test_assign_reading_order_all_blocks_unique_rank` verifies |

### Implementation Details

**Function signature:**
```rust
pub fn assign_reading_order<B>(page_width: f32, page_height: f32, blocks: &mut [B]) -> String
where
    B: HasBBox + HasReadingOrderRank + std::clone::Clone
```

**Algorithm selection logic (lines 745-778):**
1. Run XY-cut to get initial order and region statistics
2. Calculate small_region_ratio = small_region_count / region_count
3. Trigger Docstrum if: small_region_count > 10 AND small_region_ratio > 0.5
4. Assign reading_order_rank = 0, 1, 2, ... to blocks in final order
5. Return algorithm string: "xy_cut" or "docstrum"

**Constants (lines 25-34):**
- `REGION_COUNT_THRESHOLD = 10`
- `MIN_BLOCKS_PER_REGION = 3`
- `SMALL_REGION_RATIO_THRESHOLD = 0.5`

**Integration:**
The function is called from `extract.rs` at line 1121-1125, where the returned algorithm string is set in `PageResult.reading_order_algorithm`.

### Test Results
All `assign_reading_order` tests pass:
- `test_assign_reading_order_empty`
- `test_assign_reading_order_single_block`
- `test_assign_reading_order_two_columns`
- `test_assign_reading_order_docstrum_fallback`
- `test_assign_reading_order_all_blocks_unique_rank`

### Files Modified (none - pre-existing implementation)
- `crates/pdftract-core/src/layout/reading_order.rs`: Lines 732-779 (function implementation), lines 1019-1122 (tests)

### Related Beads
This bead implements Phase 4.5 of the plan (lines 1734-1759).
