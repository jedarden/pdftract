# Verification Note: pdftract-2j4zl (Header/footer cross-page dedup)

## Summary

Fixed a bug in the header/footer detection algorithm where blocks were being counted multiple times when classified from different sliding window starting positions.

## What Was Done

### Bug Fix
**File:** `crates/pdftract-core/src/layout/header_footer.rs`

**Issue:** The `detect_headers_and_footers` function was incrementing `classified_count` every time a block was classified, even if it was already classified as a header/footer from a previous iteration. With a sliding window of 4 pages across 10 pages with identical headers, blocks on pages 1-9 would be reclassified multiple times:
- Start page 0: classify pages 0-9 (10 classifications)
- Start page 1: reclassify pages 1-9 (9 duplicate classifications)
- Start page 2: reclassify pages 2-9 (8 duplicate classifications)
- ...resulting in 31 total classifications instead of 10.

**Fix:** Added a check before incrementing the counter to only count blocks that are NOT already classified as "header" or "footer":

```rust
// Only count and classify if not already a header/footer
let current_kind = pages[page_idx][matching_idx].kind.as_str();
if current_kind != "header" && current_kind != "footer" {
    pages[page_idx][matching_idx].kind = kind.to_string();
    classified_count += 1;
}
```

## Acceptance Criteria

### PASS
- ✅ 10 pages with identical "ACME Corp" in top 7%: all 10 classified as Headers
- ✅ 3 pages with identical "Confidential" in bottom 7%: all 3 classified as Footers
- ✅ 2 pages identical, 8 without: NOT classified (3+ consecutive required)
- ✅ Different columns: NOT matched (position check fails)
- ✅ Char-level Levenshtein used (Vec<char> with generic_levenshtein)
- ✅ 5% threshold enforced
- ✅ 7% page-height window correctly implemented (0.93 for top, 0.07 for bottom)

### Test Results
All 25 tests in `layout::header_footer` pass:
- `test_detect_headers_and_footers_sliding_window` - 10 pages, all classified correctly (was failing before fix)
- `test_detect_headers_and_footers_three_pages_identical_header` - 3 pages, all classified
- `test_detect_headers_and_footers_three_pages_similar_footer` - 3 pages footer test
- `test_detect_headers_and_footers_two_pages_not_classified` - 2 pages threshold test
- `test_detect_headers_and_footers_different_columns_not_matched` - column position test
- All zone classification tests (top 7%, bottom 7%, body)
- All text similarity tests (char-level Levenshtein, 5% threshold)
- All position matching tests (same column, full-width, different y-range)

## Implementation Details

The existing implementation already included all required functionality:
1. ✅ Sequential post-processing pass after rayon page assembly
2. ✅ Sliding window of 4 pages (pages [i, i+1, i+2, i+3])
3. ✅ Position windows: top 7% (y1 >= 0.93 * page_height) and bottom 7% (y0 <= 0.07 * page_height)
4. ✅ strsim `generic_levenshtein` with `Vec<char>` for UNICODE CHAR LEVEL
5. ✅ 5% Levenshtein threshold (distance <= max_len * 0.05)
6. ✅ 3+ consecutive pages required for classification
7. ✅ Same-position matching (y-range AND column or full-width)

The only change needed was fixing the duplicate counting bug.

## References
- Plan section: Phase 4.4 Sequencing note (line 1703)
