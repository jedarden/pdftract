# bf-32zce9: hybrid-006-stamp-annotation fixture verification

## Summary
Fixture `hybrid-006-stamp-annotation.pdf` already exists and meets all acceptance criteria.

## Artifacts produced

### Fixture file
- **Path:** `tests/fixtures/hybrid/hybrid-006-stamp-annotation.pdf`
- **Size:** 1461 bytes (1.4 KB) — well under 5 MB limit ✅
- **Generator:** `tests/fixtures/hybrid/hybrid-006-generator.py`
- **Verification:** PDF is valid (1 page, 612x792 letter size, PDF 1.4)

### Metadata sidecar
- **Path:** `tests/fixtures/hybrid/hybrid-006-stamp-annotation.pdf.metadata.json`
- **All required fields present:** ✅
  - `fixture_name`, `fixture_id`, `description`
  - `source` (synthetic, with generation method)
  - `pages_with_hybrid_content`: [1]
  - `hybrid_behavior`: vector_regions, scanned_regions, overlap_type, overlap_description
  - `grid_cell_coverage`: 64 total cells, ~12 hybrid cells (18.75%)
  - `classification_challenges`: 5 challenges documented
  - `test_focus`: 6 focus areas documented
  - `expected_classification`: Hybrid class with confidence range
  - `file_size_bytes`: 1461
  - `validation_status`: pending
  - `notes`: Comprehensive description

## Hybrid behavior verified

**Fixture structure:**
- Scanned contract document background (1-bit grayscale image XObject)
- Vector circular stamp/seal overlay in bottom right corner
- Stamp: red color, circular border, "APPROVED" + "Official Seal" text

**Page 1 hybrid coverage:** ~18.75% (12 of 64 grid cells in bottom right corner)

## Acceptance criteria status

| Criterion | Status | Details |
|-----------|--------|---------|
| File exists in tests/fixtures/hybrid/ | ✅ PASS | `hybrid-006-stamp-annotation.pdf` present |
| Has .metadata.json sidecar with all required fields | ✅ PASS | All 13 required fields present and populated |
| File is < 5 MB | ✅ PASS | 1461 bytes (0.0014 MB) |
| Stamp/seal is clearly vector overlay on scanned content | ✅ PASS | Generator confirms: vector circular stamp (red color, circular border) over full-page scanned contract image |

## Generator verification

Ran `tests/fixtures/hybrid/hybrid-006-generator.py`:
- Output: `Created tests/fixtures/hybrid/hybrid-006-stamp-annotation.pdf (1461 bytes)`
- Generator produces valid PDF structure with:
  - Full-page scanned contract (1-bit grayscale image XObject)
  - Vector stamp overlay (red circular border + text in bottom right)
  - Vector header text ("CONTRACT AGREEMENT")

## Test value

This fixture tests:
1. **Localized hybrid region detection** — small corner stamp on predominantly scanned page
2. **Circular element handling** — stamp crosses grid cell boundaries radially (not axis-aligned)
3. **Color awareness** — red stamp on grayscale background tests color-aware classification
4. **Classification threshold** — how much localized hybrid content triggers Hybrid classification

## Commits

Fixture was created on 2026-08-06 and already exists in the repository. No new commits needed for the fixture itself (already tracked). This verification note documents the fixture's compliance with acceptance criteria.

## Status

**All acceptance criteria PASS.** Fixture is ready for use in hybrid classification testing.
