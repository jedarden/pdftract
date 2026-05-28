# pdftract-2rkc1: Column confirmation verification

## Work completed

The `confirm_columns` function has been implemented in `/home/coding/pdftract/crates/pdftract-core/src/layout/columns.rs` (lines 252-332).

## Implementation details

The implementation follows the algorithm specified in the plan:

1. **No gaps case** (lines 257-274): Entire page is one candidate column. Counts lines whose first span's x0 falls within page bounds. Returns single column if >= 3 lines.

2. **Candidate column construction** (lines 276-308):
   - Before-first gap: `(0, gap_0.lo)`
   - Between consecutive gaps: `(gap_i.hi + 1, gap_i+1.lo)`
   - After-last gap: `(gap_last.hi + 1, page_width)`

3. **Line counting** (lines 310-321): For each line, gets first span's bbox via `HasFirstSpan` trait, checks if x0 falls within candidate column range, counts unique lines.

4. **Column promotion** (lines 324-329): Filters candidates with `line_count >= 3` to confirmed columns, reassigns indices left-to-right.

## Supporting code added

- `ColumnGap` struct (lines 89-116): Represents a gap in the x0 histogram with lo/hi bounds, width(), and midpoint() methods.
- `detect_column_gaps` function (lines 156-201): Finds zero-coverage regions >= 3% of page width.
- `HasFirstSpan` trait (lines 334-343): Trait for accessing first span's bbox.
- `CandidateColumn` struct (lines 203-213): Internal tracking of x_range and line_count.
- `Column` struct (lines 372-396): Confirmed column with index and x_range.

## Test results

All 49 column tests pass, including all acceptance criteria:

| Acceptance criteria | Test | Result |
|-------------------|------|--------|
| 2-column page with 30 lines each: both confirmed | `test_confirm_columns_two_column_both_confirmed` | PASS |
| 2-column page with 30 lines + 2 lines: only 30-line column confirmed | `test_confirm_columns_two_column_one_confirmed` | PASS |
| Single column: 1 candidate -> confirmed | `test_confirm_columns_single_column_confirmed` | PASS |
| Empty page: 0 confirmed | `test_confirm_columns_empty_page` | PASS |

Additional edge cases tested:
- Exactly 3 lines (boundary case): PASS
- Leading/trailing gaps: PASS
- Lines in gap unassigned: PASS
- Lines with no spans: PASS
- Three-column layouts: PASS

## Invariants verified

- **INV: 3-line minimum**: The filter condition `c.line_count >= 3` is enforced at line 326.
- **Lines in gaps remain unassigned**: Lines whose first span's x0 falls in a gap region are not counted for any candidate column.
- **"First span" = leftmost post-sort**: The `HasFirstSpan` trait provides the first (leftmost) span's bbox; within-line sorting is assumed to be done before calling `confirm_columns`.

## Command used

```bash
cargo nextest run -p pdftract-core 'columns::'
```

Result: 49 passed, 2382 skipped (0 failed)
