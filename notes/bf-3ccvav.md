# bf-3ccvav: Research and document hybrid PDF test case requirements

## Summary

All deliverables for this bead were already completed as part of parent bead `bf-1xlu7d` in commit `4d14744` (2024-08-03). The `tests/fixtures/hybrid/` directory exists with comprehensive documentation and fixture structure.

## Acceptance Criteria Status

### ✅ tests/fixtures/hybrid/ directory exists
- Created: 2024-08-03
- Location: `/home/coding/pdftract/tests/fixtures/hybrid/`
- Contains: README.md, GEN_MANIFEST.md, 10 fixture subdirectories, generation scripts

### ✅ README.md clearly defines 10 distinct hybrid case types
All 10 cases documented with full specifications:

1. **receipt-overtext** - Scanned receipt body with vector price overlay
2. **letterhead-image** - Vector letterhead header + scanned letter body
3. **form-mixed** - Vector form fields over scanned form background
4. **invoice-stamp** - Vector invoice line items + scanned approval stamp
5. **document-annotation** - Scanned document with vector highlight annotations
6. **figure-caption** - Academic paper: vector figure caption + scanned figure
7. **sidebar-image** - Newsletter: vector main text + scanned sidebar image
8. **watermark** - Vector text over scanned watermark background
9. **multi-column-scan** - Multi-column doc with vector headers + scanned columns
10. **complex-overlap** - Interleaved vector and scanned regions (checkerboard)

Each case includes:
- Challenge description
- Vector region specifications (grid cells, percentage)
- Scanned region specifications
- Hybrid cell counts (out of 64)
- Overlap type (separate/partial/complete)
- Test focus

### ✅ README.md explains the 8x8 grid-cell evaluator rule
From README.md section "8×8 Grid Cell Threshold":
- Page is **Hybrid** when ≥ 15% of cells (≥ 12 of 64 cells) are image-heavy
- Image-heavy cell = pixel coverage from images exceeds text coverage
- Cells below threshold: Vector extraction only
- Cells at/above threshold: OCR + merge with vector spans

The README also documents how each fixture exercises this rule:
- **figure-caption**: 8 cells (12.5%) - just below threshold edge case
- **invoice-stamp**: 12 cells (18.75%) - just above threshold
- **watermark**: 64 cells (100%) - maximum hybrid cell count
- Range coverage: 8-64 cells (12.5% to 100% of page)

### ✅ Sourcing strategy is documented
From README.md section "Generation Strategy":
- Synthetic generation via Python script (`generate_hybrid_fixtures.py`)
- Dependencies: reportlab, Pillow, img2pdf
- Two-step process: create vector content + create scanned content + combine
- Manual generation option documented

### ✅ Naming convention is established
Fixture naming convention: descriptive hyphenated names
- Pattern: `<primary-content>-<secondary-type>/`
- Examples: receipt-overtext, letterhead-image, form-mixed
- Metadata format documented in GEN_MANIFEST.md with fields:
  - name, description, vector_regions, scanned_regions, hybrid_cells_approx,
  - overlap_type, test_focus, generation_date, verification_status, notes

## Hybrid Extraction Pipeline Documentation

README.md documents the complete pipeline:
1. Classification (Phase 5.1): Detect PageClass::Hybrid, compute hybrid_cells
2. Render full page (Phase 5.2.4): Render at selected DPI (default 300)
3. Crop cells: For each hybrid cell, crop from rendered page
4. OCR per cell: Run Tesseract on each cell image independently
5. Merge: Combine vector spans + OCR spans using bbox overlap rule:
   - IoU(OCR, Vector) > 0.5 AND vector_confidence ≥ 0.5: keep vector
   - IoU(OCR, Vector) > 0.5 AND vector_confidence < 0.5: keep OCR
   - IoU(OCR, Vector) ≤ 0.5: keep both

## Related References

- Parent bead: bf-1xlu7d (populate tests/fixtures/hybrid/)
- PageClass::Hybrid implementation: pdftract-347, pdftract-4y9l, pdftract-2ix9u
- Plan: docs/plan/plan.md KU-2 (~line 671)
- Similar fixture patterns: tests/fixtures/scanned/, tests/fixtures/vector/

## Verification Method

Verified by reading existing documentation:
- `/home/coding/pdftract/tests/fixtures/hybrid/README.md` (11,158 bytes)
- `/home/coding/pdftract/tests/fixtures/hybrid/GEN_MANIFEST.md`
- Git commit 4d14744: "feat(bf-1xlu7d): populate tests/fixtures/hybrid/ for Phase 5.5 (KU-2)"

## Conclusion

All deliverables completed. No additional work required. The bead requirements were fully satisfied by the work done as part of bf-1xlu7d.
