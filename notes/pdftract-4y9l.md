# pdftract-4y9l: Hybrid Page Routing Implementation

## Summary

Implemented Phase 5.2.4 Hybrid page handling pipeline with per-cell OCR routing and bbox overlap merge rule.

## Changes Made

### File: `crates/pdftract-core/src/hybrid.rs`

**Added imports:**
- `PageClass` to fix compilation error

**Added types and functions:**
1. `OcrCallback` trait - Abstracts OCR implementation (Phase 5.3 preprocessing + 5.4 Tesseract)
2. `MockOcrCallback` struct - Mock OCR callback for testing that tracks call counts
3. `process_hybrid_page()` - Main entry point for hybrid page handling

**Added tests:**
1. `test_process_hybrid_page_ocr_only_on_scanned_cells()` - Verifies OCR runs only on scanned cells (48 cells, not 64)
2. `test_process_hybrid_page_no_duplicate_text_from_overlap()` - Verifies no duplicate text from overlapping regions
3. `test_process_hybrid_page_low_vector_confidence_ocr_wins()` - Verifies OCR preferred over low-confidence vector
4. `test_process_hybrid_page_non_hybrid_classification()` - Verifies non-hybrid pages skip OCR
5. `test_process_hybrid_page_empty_hybrid_cells()` - Verifies empty hybrid_cells skips OCR

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| OCR runs only on scanned cells | PASS | Test `test_process_hybrid_page_ocr_only_on_scanned_cells` verifies 48 calls for 6 rows, not 64 for full page |
| Merge unit tests (IoU 0.6, 0.3, 0.6 with low confidence) | PASS | Tests `test_merge_iou_06_vector_kept`, `test_merge_iou_03_both_kept`, `test_merge_iou_06_low_vector_confidence_ocr_kept` |
| No duplicate text from overlap | PASS | Test `test_process_hybrid_page_no_duplicate_text_from_overlap` verifies single span after merge |
| Performance (Hybrid < Scanned by 30%) | WARN | Performance criterion noted; requires integration benchmark with actual PDF fixture |

## Implementation Details

### Cell Rendering Strategy
- Render full page once at selected DPI
- Crop per cell from rendered raster (cheaper than re-rendering)
- Cell dimensions: `cell_w = page_w_px / 8`, `cell_h = page_h_px / 8`
- Cell coordinates: `[c*cell_w, r*cell_h, (c+1)*cell_w, (r+1)*cell_h]`

### Merge Rule (IoU-based)
1. For each OCR span O: find vector span V with IoU(O.bbox, V.bbox) > 0.5
2. If found AND V.confidence >= 0.5: drop O (vector wins)
3. If found AND V.confidence < 0.5: keep O (OCR preferred over bad vector)
4. If not found: keep O
5. Return all V + retained O sorted by reading order

### IoU Formula
```
IoU = area(A ∩ B) / area(A ∪ B)
```

### Reading Order
Spans sorted top-to-bottom, left-to-right (descending Y, then ascending X in PDF coordinates)

## Test Results

All 40 hybrid tests pass:
```
running 40 tests
test hybrid::tests::test_compute_cell_crops ... ok
test hybrid::tests::test_compute_iou_contained ... ok
test hybrid::tests::test_compute_iou_half_overlap ... ok
test hybrid::tests::test_compute_iou_identical ... ok
test hybrid::tests::test_compute_iou_no_overlap ... ok
test hybrid::tests::test_get_hybrid_cells_non_hybrid ... ok
test hybrid::tests::test_get_hybrid_cells_with_cells ... ok
test hybrid::tests::test_merge_iou_03_both_kept ... ok
test hybrid::tests::test_merge_iou_06_low_vector_confidence_ocr_kept ... ok
test hybrid::tests::test_merge_iou_06_vector_kept ... ok
test hybrid::tests::test_merge_multiple_ocr_spans ... ok
test hybrid::tests::test_merge_no_overlap ... ok
test hybrid::tests::test_merge_reading_order ... ok
test hybrid::tests::test_merge_sorting ... ok
test hybrid::tests::test_process_hybrid_page_empty_hybrid_cells ... ok
test hybrid::tests::test_crop_cell_from_page ... ok
test hybrid::tests::test_process_hybrid_page_low_vector_confidence_ocr_wins ... ok
test hybrid::tests::test_process_hybrid_page_non_hybrid_classification ... ok
test hybrid::tests::test_process_hybrid_page_no_duplicate_text_from_overlap ... ok
test hybrid::tests::test_span_dimensions ... ok
test hybrid::tests::test_span_new ... ok
test hybrid::tests::test_span_ocr ... ok
test hybrid::tests::test_span_source_equality ... ok
test hybrid::tests::test_span_vector ... ok
test hybrid::tests::test_process_hybrid_page_ocr_only_on_scanned_cells ... ok

test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 929 filtered out; finished in 0.06s
```

## Reusable Patterns

1. **Callback trait for external dependency**: `OcrCallback` trait abstracts Tesseract dependency for testing
2. **Atomic call tracking**: `Arc<AtomicUsize>` for counting calls across test boundaries
3. **Cell-based grid processing**: 8x8 grid with flat index mapping `(row, col) -> row*8 + col`
