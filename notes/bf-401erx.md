# Bead bf-401erx: Create next 4 hybrid PDF fixtures

## Completion Summary

Successfully created 4 new hybrid PDF fixtures (hybrid-004 through hybrid-007) with complete documentation.

## Deliverables

### 1. hybrid-004-watermark-over-scan.pdf
- **Pattern**: Scanned document with vector diagonal watermark overlay
- **File size**: 1,416 bytes (1.4 KB)
- **Hybrid cells**: ~64 cells (100.0% full-page overlap)
- **Generator**: `tests/fixtures/hybrid/hybrid-004-generator.py`
- **Metadata**: `tests/fixtures/hybrid/hybrid-004-watermark-over-scan.pdf.metadata.json`
- **Challenge**: Full-page overlap with diagonal watermark crossing grid cell boundaries at oblique angles

### 2. hybrid-005-vector-footer-over-scan.pdf
- **Pattern**: Scanned body with vector footer/page numbers
- **File size**: 1,460 bytes (1.5 KB)
- **Hybrid cells**: ~8 cells (12.5% bottom boundary row)
- **Generator**: `tests/fixtures/hybrid/hybrid-005-generator.py`
- **Metadata**: `tests/fixtures/hybrid/hybrid-005-vector-footer-over-scan.pdf.metadata.json`
- **Challenge**: Tests classification symmetry with hybrid-001 (vector at bottom instead of top)

### 3. hybrid-006-stamp-annotation.pdf
- **Pattern**: Scanned contract with vector circular stamp/seal overlay
- **File size**: 1,461 bytes (1.5 KB)
- **Hybrid cells**: ~12 cells (18.75% bottom right corner)
- **Generator**: `tests/fixtures/hybrid/hybrid-006-generator.py`
- **Metadata**: `tests/fixtures/hybrid/hybrid-006-stamp-annotation.pdf.metadata.json`
- **Challenge**: Localized circular stamp crossing grid cell boundaries radially, tests color-aware classification

### 4. hybrid-007-textbox-overlay.pdf
- **Pattern**: Scanned tax form with vector fillable textbox overlays
- **File size**: 1,407 bytes (1.4 KB)
- **Hybrid cells**: ~28 cells (43.75% distributed across form)
- **Generator**: `tests/fixtures/hybrid/hybrid-007-generator.py`
- **Metadata**: `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf.metadata.json`
- **Challenge**: Scattered textbox distribution similar to hybrid-002 but with tax form layout

## Documentation Updates

✅ **README.md updated**:
- Added "Additional Fixtures (Fixtures 4-7)" section with detailed descriptions
- Updated directory structure to include all 4 new fixtures
- Updated status table to mark fixtures 4-7 as Complete

✅ **Metadata files created**: Each fixture has comprehensive `.metadata.json` sidecar documenting:
- Source and generation method
- Pages with hybrid content
- Vector and scanned region descriptions
- Grid cell coverage analysis
- Classification challenges
- Test focus areas
- Expected classification results

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| 4 PDF files exist in tests/fixtures/hybrid/ | ✅ PASS | All 4 files created, under 5KB each |
| Each has .metadata.json sidecar | ✅ PASS | All 4 metadata files created with comprehensive details |
| README.md updated | ✅ PASS | Added detailed fixture descriptions and updated status table |
| At least 2 real-world PDFs if possible | ⚠️ WARN | All fixtures are synthetic (acceptable - bead description allowed synthetic generation) |
| Files < 5 MB each | ✅ PASS | All files < 1.5 KB, well under limit |
| Total fixture count now 7 | ✅ PASS | Progress toward 10-fixture target |

## Technical Notes

- All fixtures use raw PDF construction (1-bit grayscale image XObjects + vector text/graphics)
- File sizes are minimal (1.4-1.5 KB) for fast test execution
- Generator scripts follow the pattern from hybrid-001-generator.py
- Each fixture targets distinct hybrid patterns:
  - hybrid-004: Full-page diagonal watermark (oblique angle grid crossing)
  - hybrid-005: Bottom footer vector (vertical stack symmetry)
  - hybrid-006: Localized circular stamp (radial grid crossing, color)
  - hybrid-007: Distributed textboxes (form layout, scattered overlap)

## Commits

All work committed with conventional commit messages citing bead bf-401erx.

## Related Beads

- **Parent**: bf-1xlu7d (Phase 5.5 classifier tuning)
- **Dependency**: bf-49yhjc (first 3 fixtures - completed)
- **Plan reference**: docs/plan/plan.md KU-2 (~line 671)
