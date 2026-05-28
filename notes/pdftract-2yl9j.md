# pdftract-2yl9j: Heading Detection Implementation Verification

## Task
Implement `classify_heading(block, page_body_median_font_size) -> bool` for Phase 4.4 heading detection.

## Status: ALREADY IMPLEMENTED ✓

The heading detection functionality is already fully implemented in:
- **File:** `crates/pdftract-core/src/layout/line.rs`
- **Functions:** `classify_heading` (lines 666-703) and `classify_page_headings` (lines 705-722)

## Implementation Details

### `classify_heading` Function
```rust
pub fn classify_heading<L>(block: &mut BlockInput<L>, page_body_median_font_size: f32) -> bool
where
    L: LineMetadata + Clone,
{
    // INV: threshold is strictly > 1.2
    let ratio = block.median_font_size / page_body_median_font_size;
    let size_criterion = ratio > 1.2;

    // Single-line criterion
    let line_count_criterion = block.lines.len() <= 1;

    if size_criterion && line_count_criterion {
        block.kind = "heading".to_string();
        true
    } else {
        false
    }
}
```

### Implementation Correctness

**All requirements met:**
1. ✓ `classify_heading(block, page_body_median_font_size) -> bool` signature
2. ✓ Returns true when `block.median_font_size > 1.2 * page_body_median`
3. ✓ AND `block.lines.len() <= 1`
4. ✓ Sets `block.kind = "heading"` on positive

**INV compliance:**
- Threshold is strictly `> 1.2`, not `>= 1.2` (line 692)

## Acceptance Criteria Verification

All acceptance criteria tests exist and pass:

| Test Case | Expected | Status |
|-----------|----------|--------|
| 18pt block, body 12pt, 1 line | Heading (1.5 > 1.2) | ✓ PASS |
| 14pt block, body 12pt, 1 line | NOT (1.17 < 1.2) | ✓ PASS |
| 18pt block, 3 lines | NOT (too many lines) | ✓ PASS |
| 12pt block, body 12pt | NOT | ✓ PASS |

### Additional Tests
- Threshold exactly 1.2: NOT heading (strict inequality) ✓
- Empty block (0 lines): NOT heading ✓
- 2-line block: NOT heading ✓

## Test Results

Tests were run successfully with `cargo nextest run`:
```
PASS [   0.006s] ( 1/10) test_classify_heading_18pt_block_12pt_body_one_line_heading
PASS [   0.006s] ( 2/10) test_classify_heading_empty_lines_not_heading
PASS [   0.006s] ( 3/10) test_classify_heading_small_page_body_median
PASS [   0.006s] ( 4/10) test_classify_heading_18pt_block_three_lines_not_heading
PASS [   0.006s] ( 5/10) test_classify_heading_large_page_body_median
PASS [   0.006s] ( 6/10) test_classify_heading_14pt_block_12pt_body_one_line_not_heading
PASS [   0.006s] ( 7/10) test_classify_heading_threshold_exactly_1_2_not_heading
PASS [   0.006s] ( 8/10) test_classify_heading_12pt_block_12pt_body_not_heading
PASS [   0.006s] ( 9/10) test_classify_heading_two_lines_not_heading
PASS [   0.006s] (10/10) test_classify_heading_threshold_just_above_1_2_is_heading
```

## Notes

- The implementation uses string `kind` field (set to "heading") following the same pattern as `caption.rs`
- There are pre-existing compilation errors in other files (`header_footer.rs`, `table/output.rs`) that prevent the full test suite from running, but these are unrelated to the heading detection implementation
- The heading detection code itself compiles correctly and all tests pass when run in isolation

## Conclusion

The task is **already complete**. The `classify_heading` function is fully implemented, well-tested, and meets all acceptance criteria specified in the bead description and the plan (Phase 4.4, line 1702).
