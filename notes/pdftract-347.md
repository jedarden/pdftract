# Verification Note: pdftract-347

## Task
5.1.3: Hybrid grid-cell evaluator (8x8 decomposition + >=15% rule)

## Summary
Implemented the per-region Hybrid evaluator that detects mixed-content pages by 8x8 grid decomposition. The implementation is in `crates/pdftract-core/src/classify.rs` and includes all required types and tests.

## Acceptance Criteria

### PASS: Critical test - hybrid page with text header (top 2 rows) + scanned body (bottom 6 rows)
- Test: `test_critical_hybrid_page_text_header_scanned_body`
- Result: PASS
- Verifies:
  - Classification is `PageClass::Hybrid`
  - `hybrid_cells` contains exactly 48 cells (6 rows × 8 cols)
  - All scanned cells are from rows 2-7 only (no vector header cells included)

### PASS: Unit test - below threshold (9 vector + 9 scanned cells)
- Test: `test_grid_classifier_below_threshold`
- Result: PASS
- Verifies:
  - Page is NOT classified as Hybrid (below 10-cell threshold)
  - `hybrid_cells` is None for non-Hybrid pages

### PASS: Determinism - classify twice produces byte-identical serialization
- Test: `test_determinism_classify_twice`
- Result: PASS
- Uses `BTreeSet` (not `HashSet`) for deterministic ordering
- Verifies JSON serialization is byte-identical across runs

### PASS: Cells exposed for 5.2 OCR routing
- `PageClassification.hybrid_cells: Option<BTreeSet<usize>>`
- Contains flat cell indices (0-63) for scanned cells
- Ready for downstream OCR-only-on-cells routing in Phase 5.2

## Implementation Details

### Grid Decomposition
- 8 rows × 8 cols = 64 cells
- Cell index: `row * 8 + col` (0-63)
- Row 0 = top of page (after rotation applied)
- Col 0 = left of page

### Cell Classification Rules
- **Vector**: `text_op_count > 0 AND char_validity > 0.6`
- **Scanned**: `image_coverage > 0.80 AND text_op_count == 0`
- **Mixed**: neither condition met (empty or ambiguous)

### Hybrid Detection Rule
- Hybrid when: `vector_cell_count >= 10 AND scanned_cell_count >= 10`
- Confidence: `min(vector_ratio, scanned_ratio)` where `ratio = count / 64`
- Returns `hybrid_cells` set containing scanned cell indexes

### Rotation Handling
- `GridClassifier` stores rotation (0, 90, 180, 270)
- Width/height are expected to be post-rotation values
- Coordinates should be transformed by rotation matrix before `point_to_cell()`

## Test Results
```
running 32 tests
test classify::tests::test_critical_hybrid_page_text_header_scanned_body ... ok
test classify::tests::test_grid_classifier_below_threshold ... ok
test classify::tests::test_determinism_classify_twice ... ok
test classify::tests::test_grid_classifier_hybrid_detection ... ok
test classify::tests::test_exactly_10_cells_threshold ... ok
... (28 more classify tests) ...
test result: ok. 32 passed; 0 failed
```

## Files Modified/Created
- `crates/pdftract-core/src/classify.rs` (new file, 705 lines)
- `crates/pdftract-core/src/lib.rs` (already exports `classify` module)

## No WARN Items
All acceptance criteria met without environmental blockers.
