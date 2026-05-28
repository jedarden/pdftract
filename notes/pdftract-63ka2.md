# Verification Note: pdftract-63ka2

## Bead
Phase 4.3: Column Detection (coordinator)

## Current State

### Children Status
All 4 children are CLOSED:
- `pdftract-56vwd` - x0 histogram builder - CLOSED ✓
- `pdftract-14w0w` - Gap detection - CLOSED ✓
- `pdftract-2rkc1` - Column confirmation - CLOSED ✓
- `pdftract-64j83` - Column label assignment - CLOSED ✓

### Implementation Status
Column detection functions are fully implemented in `crates/pdftract-core/src/layout/columns.rs`:
- `build_x0_histogram()` - 49 unit tests pass
- `detect_column_gaps()` - Part of the 49 tests
- `confirm_columns()` - Part of the 49 tests
- `assign_columns_to_spans()` - Part of the 49 tests
- `assign_columns_to_lines()` - Part of the 49 tests

### Integration Status: BLOCKER

Column detection is NOT integrated into the main extraction pipeline:

1. **Main `Span` struct missing column field**
   - File: `crates/pdftract-core/src/span/mod.rs`
   - The `Span` struct does NOT have a `column: Option<u32>` field
   - Child bead `pdftract-64j83` added the column field to `HybridHybridSpan` (hybrid.rs) instead
   - `HybridHybridSpan` is used for hybrid pages (mixed vector/scanned content), not the main pipeline

2. **Extraction pipeline does not call column detection**
   - File: `crates/pdftract-core/src/extract.rs`
   - Column detection functions are never invoked
   - `SpanJson::column` is hardcoded to `None` (lines 1059, 1916)

3. **No end-to-end tests for column detection**
   - No fixture tests for three-column papers
   - No fixture tests for full-width headings above two-column body
   - No fixture tests for single-column pages

### Acceptance Criteria

- [PASS] All 4 children closed
- [FAIL] Three-column academic paper: three distinct columns detected - NOT VERIFIED
- [FAIL] Full-width heading above two-column body: heading spans not assigned a column - NOT VERIFIED
- [FAIL] Single-column page: no false column splits - NOT VERIFIED

## Blockers

1. **Add `column: Option<u32>` field to main `Span` struct**
   - File: `crates/pdftract-core/src/span/mod.rs`
   - Update `Span::new()` to initialize the field

2. **Integrate column detection into extraction pipeline**
   - File: `crates/pdftract-core/src/extract.rs`
   - After line formation (Phase 4.2), call column detection:
     - `build_x0_histogram(spans, page_width)`
     - `detect_column_gaps(&hist, page_width)`
     - `confirm_columns(&gaps, page_width, &lines)`
     - `assign_columns_to_spans(spans, &columns)`
     - `assign_columns_to_lines(lines)`
   - Pass the column value to `SpanJson` constructor

3. **Add end-to-end tests**
   - Create fixture for three-column academic paper
   - Create fixture for two-column page with full-width heading
   - Create fixture for single-column page
   - Verify column detection produces correct labels

## Recommendation

DO NOT CLOSE this coordinator bead. The sub-phase implementation is incomplete because:
1. The main `Span` struct lacks the column field
2. The extraction pipeline does not call column detection
3. No end-to-end verification of acceptance criteria

The child beads being closed only means the individual functions are implemented. The coordinator must ensure the sub-phase works end-to-end, which requires integration into the extraction pipeline.

## Files Requiring Changes

1. `crates/pdftract-core/src/span/mod.rs` - Add `column: Option<u32>` to `Span`
2. `crates/pdftract-core/src/extract.rs` - Integrate column detection pipeline
3. `crates/pdftract-core/tests/` or `crates/pdftract-cli/tests/` - Add fixture tests
