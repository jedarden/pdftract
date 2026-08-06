# bf-309kjc: hybrid-002-vector-form-over-scan PDF fixture

## Status: VERIFIED - Already implemented

## Summary
The hybrid-002 fixture already exists in the repository (commit `4083929`). This verification confirms the fixture meets all acceptance criteria.

## Artifacts Created

### PDF File
- **Path:** `tests/fixtures/hybrid/hybrid-002-vector-form-over-scan.pdf`
- **Size:** 1,507 bytes (well under 5 MB limit)
- **Type:** Synthetic PDF generated via Python script

### Metadata Sidecar
- **Path:** `tests/fixtures/hybrid/hybrid-002-vector-form-over-scan.pdf.metadata.json`
- **All required fields present:**
  - fixture_name, fixture_id, description ✓
  - source (type, generation_method, generated_date) ✓
  - pages_with_hybrid_content ✓
  - hybrid_behavior (vector_regions, scanned_regions, overlap_type, overlap_description) ✓
  - grid_cell_coverage (total_cells, hybrid_cells_approx, hybrid_percentage, hybrid_cell_locations) ✓
  - classification_challenges ✓
  - test_focus ✓
  - expected_classification ✓

### Generator Script
- **Path:** `tests/fixtures/hybrid/hybrid-002-generator.py`
- **Function:** Creates synthetic PDF with vector form field annotations overlaid on scanned form background

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| PDF file exists < 5 MB | ✅ PASS | 1,507 bytes |
| .metadata.json exists with required fields | ✅ PASS | All fields present and valid |
| Exhibits vector form over scan | ✅ PASS | Form annotations overlaid on scanned background |
| Prefer real-world PDF | ⚠️ WARN | Synthetic (acceptable for test fixture) |

## Hybrid Behavior

**Vector regions:** Form field annotations scattered throughout page (field labels, red checkbox indicators, underline rectangles)

**Scanned regions:** Complete form background (1-bit grayscale image XObject with employee information form layout)

**Overlap type:** partial-overlay (vector annotations overlay scanned form at multiple positions)

**Grid coverage:** ~48 cells (75% of 64 total cells)

## Classification Challenges Documented

1. Scattered vector distribution makes boundary detection difficult
2. Complex merge patterns with multiple overlapping vector and scanned regions
3. Field-level extraction requires accurate OCR targeting
4. Tests cell-level OCR granularity on hybrid forms
5. Small vector elements (checkboxes) detection
6. Merge rule accuracy when vector and scanned text overlap

## Git History

- **Commit:** `4083929` - "test(bf-309kjc): add hybrid-002-vector-form-over-scan PDF fixture"
- **Date:** 2026-08-06
- **Files:** hybrid-002-generator.py, hybrid-002-vector-form-over-scan.pdf, metadata.json

## Verification Steps Performed

1. ✅ Verified PDF file exists and is under 5 MB size limit
2. ✅ Verified metadata JSON exists with all required fields
3. ✅ Validated generator script runs without errors
4. ✅ Confirmed fixture exhibits vector form fields over scanned form
5. ✅ Verified git commit history references this bead

## Conclusion

The hybrid-002 fixture is complete and meets all acceptance criteria. The synthetic generation approach is appropriate for a controlled test fixture, allowing reproducible testing of hybrid PDF classification with scattered vector annotations over scanned backgrounds.

**Bead Status:** READY TO CLOSE
