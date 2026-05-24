# Verification Note: pdftract-3nwz (Borderless Table Detection)

## Summary
Implemented borderless table detection using x0-aligned span heuristic. The implementation was already present in the codebase and all tests pass.

## Changes Made
1. Added benchmark for borderless detection to verify performance
2. Verified all acceptance criteria are met

## Acceptance Criteria Status

### PASS
- **Critical test**: 3x3 borderless table detected via alignment heuristic
  - `test_detect_borderless_3x3_table_accepted` passes
- **Unit test - paragraph rejected**: Single-column text is rejected
  - `test_detect_borderless_paragraph_rejected` passes
- **Unit test - one-row pseudo-table rejected**: Single row with multiple columns rejected
  - `test_detect_borderless_one_row_pseudo_table_rejected` passes
- **Unit test - 3-row 3-column borderless table accepted**: Core table detection works
  - `test_detect_borderless_3x3_table_accepted` passes
- **Unit test - vertical-gap test**: Two separate tables with >100 pt gap detected separately
  - `test_detect_borderless_vertical_gap_test` passes
- **Public API**: `TableDetector::detect_borderless(&PageContext) -> Vec<GridCandidate>` exists
- **Performance**: 1.56 ms for 5040 text positions (well below 10 ms requirement)

## Implementation Details
The borderless detector in `crates/pdftract-core/src/table/detector.rs`:
- Collects text positions from content stream (Tm, Td, TD, T*, Tj, TJ, ', " operators)
- Groups by x0 positions within 2.0 pt tolerance using clustering
- Finds column candidates (3+ spans at same x0 on different y positions)
- Finds row candidates (y positions where >= 2 column candidates have spans)
- Validates: 3+ rows AND 3+ columns, contiguous y range, no gap > 100 pt
- Constructs GridCandidate with empty segments (no ruling lines)
- Rejects single-column paragraph reflow patterns

## Test Results
```bash
cargo test -p pdftract-core --lib table::detector::tests::test_detect_borderless
# running 6 tests
# test table::detector::tests::test_detect_borderless_empty_content ... ok
# test table::detector::tests::test_detect_borderless_no_text_block ... ok
# test table::detector::tests::test_detect_borderless_3x3_table_accepted ... ok
# test table::detector::tests::test_detect_borderless_one_row_pseudo_table_rejected ... ok
# test table::detector::tests::test_detect_borderless_paragraph_rejected ... ok
# test table::detector::tests::test_detect_borderless_vertical_gap_test ... ok
# test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured
```

## Benchmark Results
```
borderless_detection/text_positions/5040
                        time:   [1.5457 ms 1.5595 ms 1.5755 ms]
```
Performance target: < 10 ms on 5000-span page
Actual: ~1.56 ms (well within requirement)

## Files Modified
- `crates/pdftract-core/benches/table_detection.rs`: Added borderless detection benchmark

## Files Reviewed (no changes needed)
- `crates/pdftract-core/src/table/detector.rs`: Borderless detection already implemented
- `crates/pdftract-core/src/table/mod.rs`: Public API exported
- `crates/pdftract-core/src/lib.rs`: Re-exports for public API

## Integration Notes
Per task description, borderless detection should run only when line-based detection (7.2.1) returns no GridCandidate covering a region. This is a usage pattern for the caller, not enforced within the detector itself. The detector provides both methods independently:
- `TableDetector::detect_line_based()` - for bordered tables
- `TableDetector::detect_borderless()` - for borderless tables

Callers can orchestrate the fallback logic as needed.
