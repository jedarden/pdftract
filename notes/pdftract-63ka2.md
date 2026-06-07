# pdftract-63ka2: Phase 4.3 Column Detection (coordinator)

## Summary

Coordinator for Phase 4.3 Column Detection. All 4 child beads are closed with implementation and tests verified.

## Children Status

| Child ID | Title | Status | Verified |
|----------|-------|--------|----------|
| pdftract-56vwd | x0 histogram builder (1pt resolution) | closed | ✓ May 25 |
| pdftract-14w0w | Gap detection (>= 0.03 * page_width) | closed | ✓ May 27 |
| pdftract-2rkc1 | Column confirmation (>= 3 lines) | closed | ✓ May 27 |
| pdftract-64j83 | Column label assignment to spans/lines | closed | ✓ May 24 |

## Acceptance Criteria Verification

### Criterion 1: All 4 children closed
**Status:** PASS

All 4 children have been closed with verification notes documenting their implementation and test coverage.

### Criterion 2: Three-column academic paper detected
**Status:** PASS (verified via `test_confirm_columns_three_column_all_confirmed`)

Test creates 3-column layout with gaps at 200-219 and 400-419, 10 lines per column.
Confirmed output: 3 columns with indices 0, 1, 2 and x_ranges [0,200), [220,400), [420,600).

### Criterion 3: Full-width heading above two-column body
**Status:** PASS (verified via `test_assign_columns_to_lines_full_width_heading`)

Test verifies that when all spans on a line have `column = None` (full-width heading), the line's column is also `None`. Body spans in columns 0 and 1 are correctly assigned.

### Criterion 4: Single-column page: no false splits
**Status:** PASS (verified via `test_assign_columns_to_spans_single_column`)

Test confirms single-column page (full-width x_range [0,600)) assigns all spans to `Some(0)`.
Also verified by `test_confirm_columns_single_column_confirmed`.

## Test Coverage Summary

Total column tests: **49 tests, all PASS**

- `build_x0_histogram`: 7 tests
- `detect_column_gaps`: 13 tests
- `confirm_columns`: 14 tests
- `assign_columns_to_spans`: 5 tests
- `assign_columns_to_lines`: 5 tests
- Supporting tests: 5 tests

## Implementation Location

All code in `crates/pdftract-core/src/layout/columns.rs`:
- `build_x0_histogram()` - lines 48-82
- `detect_column_gaps()` - lines 156-201
- `confirm_columns()` - lines 252-332
- `assign_columns_to_spans()` - lines 428-437
- `assign_columns_to_lines()` - lines 464-491
- Supporting types: `ColumnGap`, `Column`, `CandidateColumn`
- Traits: `HasBBox`, `HasFirstSpan`, `HasBBoxAndColumn`, `HasSpansWithColumn`

## Critical Invariants Verified

- **3-line minimum:** Enforced in `confirm_columns` filter (line 326)
- **Column gap threshold scales with page_width:** `(page_width * 0.03).ceil()` (line 157)
- **Full-width lines get column = None:** >50% dominance check in `assign_columns_to_lines`
- **Column indices monotonic left-to-right:** Verified in tests

## Gates to Next Phase

This coordinator completion gates:
- Phase 4.4: Per-column block formation
- Phase 4.5: XY-cut reading order
