# pdftract-1jkme: sort_spans_in_line implementation

## Work Summary

The `sort_spans_in_line` function was already implemented in `crates/pdftract-core/src/layout/line.rs` (lines 723-770). The implementation correctly handles:

- **LTR**: Sorts by `bbox[0]` ascending (left edge first)
- **RTL**: Sorts by `bbox[2]` descending (right edge first)
- **Mixed**: Falls back to `bbox[0]` ascending (v0.1.0 limitation documented)
- **NaN bbox**: Treated as `Ordering::Equal` to prevent panic
- **Stable sort**: Uses `sort_by` (not `sort_unstable_by`) to preserve insertion order on ties

## Changes Made

Fixed a compilation bug in `crates/pdftract-core/src/layout/correction.rs`:
- Added `use std::sync::Arc;` to the test module (line 993)
- The test module was using `Arc::from("Helvetica")` but `Arc` was not in scope

## Acceptance Criteria - PASS

| Criterion | Status | Notes |
|-----------|--------|-------|
| LTR 3 spans x0=100,50,150: sorted 50,100,150 | PASS | test_sort_spans_in_line_ltr_ascending_x0 |
| RTL 3 spans x1=100,50,150: sorted 150,100,50 | PASS | test_sort_spans_in_line_rtl_descending_x1 |
| Mixed: spans in logical x0 ASC | PASS | test_sort_spans_in_line_mixed_fallback_x0_ascending |
| NaN bbox: no panic | PASS | test_sort_spans_in_line_nan_bbox_no_panic |
| Stable: ties preserve order | PASS | test_sort_spans_in_line_stable_sort_preserves_insertion_order |

## Test Results

```
PASS [   0.006s] (1/7) pdftract-core layout::line::tests::test_sort_spans_in_line_single_span_unchanged
PASS [   0.006s] (2/7) pdftract-core layout::line::tests::test_sort_spans_in_line_rtl_descending_x1
PASS [   0.006s] (3/7) pdftract-core layout::line::tests::test_sort_spans_in_line_empty_line_no_panic
PASS [   0.006s] (4/7) pdftract-core layout::line::tests::test_sort_spans_in_line_stable_sort_preserves_insertion_order
PASS [   0.006s] (5/7) pdftract-core layout::line::tests::test_sort_spans_in_line_ltr_ascending_x0
PASS [   0.007s] (6/7) pdftract-core layout::line::tests::test_sort_spans_in_line_mixed_fallback_x0_ascending
PASS [   0.007s] (7/7) pdftract-core layout::line::tests::test_sort_spans_in_line_nan_bbox_no_panic
```

All 7 tests passed in 0.010s.

## Commit

- Commit: `a0ea5f4` - "fix(pdftract-1jkme): add missing Arc import to correction.rs test module"
- Pushed to: `forgejo main` at `8cfbe70`
