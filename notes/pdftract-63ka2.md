# Verification Note: pdftract-63ka2 (Updated 2026-06-06)

## Bead
Phase 4.3: Column Detection (coordinator)

## Current Status: BLOCKER - DO NOT CLOSE

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

### Integration Status: BLOCKER (As of 2026-05-28)

Column detection is NOT integrated into the main extraction pipeline:

1. **Main `Span` struct HAS column field but it's never used**
   - File: `crates/pdftract-core/src/span/mod.rs:179`
   - The `Span` struct DOES have `column: Option<u32>` field (updated since initial note)
   - However, the extraction pipeline never assigns column values

2. **Extraction pipeline does not call column detection**
   - File: `crates/pdftract-core/src/extract.rs`
   - Column detection functions are never invoked (grep found no matches)
   - `SpanJson::column` is hardcoded to `None` (lines 1059, 1916)
   - The extraction pipeline doesn't use `cluster_spans_into_lines` or column detection at all

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

The extraction pipeline (`extract.rs`) needs to be refactored to use the Phase 4 layout pipeline:

1. **Add Phase 4 pipeline integration**
   - File: `crates/pdftract-core/src/extract.rs`
   - Currently the pipeline doesn't use line formation or column detection
   - Need to add: glyph → span → line → column detection → block formation
   - Current pipeline goes directly from glyphs to spans to blocks without line/column phases

2. **Implement column detection call chain**
   - After line formation (Phase 4.2), call:
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

**DO NOT CLOSE this coordinator bead.** The sub-phase implementation is incomplete because:
1. The extraction pipeline doesn't use the Phase 4 layout pipeline
2. Column detection functions are never called in production
3. No end-to-end verification of acceptance criteria

The child beads being closed only means the individual functions are implemented and unit-tested. The coordinator must ensure the sub-phase works end-to-end, which requires integration into the extraction pipeline.

## Next Steps

This coordinator bead requires significant extraction pipeline refactoring:
1. Integrate Phase 4.2 line formation into extract.rs
2. Add Phase 4.3 column detection after line formation
3. Update SpanJson to use computed column values
4. Add fixture tests for acceptance criteria verification

---

## 2026-06-06 Verification

Re-verified status on 2026-06-06. All findings from 2026-05-28 remain accurate:

### Unit Tests Status
- All 49 column detection unit tests PASS
- Verified with: `cargo test -p pdftract-core --lib 'layout::columns::tests' --no-fail-fast`

### Integration Status
- Column detection functions still NOT called in extract.rs
- Verified with: `grep -rn "cluster_spans_into_lines\|build_x0_histogram" crates/pdftract-core/src/extract.rs` returned no matches
- SpanJson column field still hardcoded to None

### Recommendation: DO NOT CLOSE
The coordinator bead must remain open because:
1. The implementation exists but is not integrated into the production pipeline
2. End-to-end acceptance criteria cannot be verified without integration
3. The child beads being closed only confirms unit-level correctness, not system-level correctness

The coordinator's responsibility is to ensure the sub-phase works end-to-end in the actual extraction pipeline, not just that individual functions are implemented.
