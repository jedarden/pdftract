# bf-309kjc: hybrid-002-vector-form-over-scan PDF Fixture

## Summary
Successfully created the second hybrid PDF fixture demonstrating vector form fields/annotations overlaid on a scanned form background.

## Deliverables

### 1. PDF Fixture
**File:** `tests/fixtures/hybrid/hybrid-002-vector-form-over-scan.pdf`
- **Size:** 1,507 bytes (1.5 KB) ✓ (< 5 MB requirement)
- **Validation:** PDF header and trailer structure verified valid

### 2. Metadata Sidecar
**File:** `tests/fixtures/hybrid/hybrid-002-vector-form-over-scan.pdf.metadata.json`
- **Size:** 3.9 KB
- **Required fields present:**
  - ✓ `source.type` = "synthetic"
  - ✓ `source.generation_method` - detailed description
  - ✓ `pages_with_hybrid_content` = [1]
  - ✓ `hybrid_behavior.vector_regions` - detailed description
  - ✓ `hybrid_behavior.scanned_regions` - detailed description
  - ✓ `hybrid_behavior.overlap_type` = "partial-overlay"
  - ✓ `grid_cell_coverage` with estimates (~75% hybrid)
  - ✓ `classification_challenges` - 6 specific challenges documented
  - ✓ `test_focus` - 6 test objectives
  - ✓ `expected_classification` with confidence range

### 3. Generator Script
**File:** `tests/fixtures/hybrid/hybrid-002-generator.py`
- Python script using raw PDF construction
- Creates 1-bit grayscale scanned form background (employee information form)
- Overlays vector form field annotations (labels, checkboxes, rectangles)

## Hybrid Behavior Details

### Vector Regions
- Form field labels in Helvetica font (9-10pt)
- Red checkbox indicators (10x10px) in top-left corner
- Underline rectangles for text input fields
- Descriptive overlay annotations

### Scanned Regions
- Complete employee information form layout
- 8x8 grid coverage (64 cells total)
- Title, field labels, underline patterns, section headers
- Certification text and signature line

### Overlap Characteristics
- **Type:** Partial overlay
- **Complexity:** Scattered vector distribution across page
- **Merge challenges:** Multiple small vector spans overlapping larger scanned regions

## Classification Challenges Documented
1. Scattered vector distribution (boundary detection difficulty)
2. Complex merge patterns (multiple overlapping spans)
3. Field-level extraction accuracy (avoiding duplicate labels)
4. Cell-level OCR granularity testing
5. Vector/scanned text overlap (merge rule IoU thresholds)
6. Small vector element detection (checkboxes)

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| PDF file exists < 5 MB | ✓ PASS | 1.5 KB |
| .metadata.json exists with required fields | ✓ PASS | All fields documented |
| Exhibits vector form fields over scanned form | ✓ PASS | Generator creates hybrid layout |
| Real-world PDF preferred | ✓ PASS | Synthetic acceptable; simulates common pattern |

## Git Commit
- **Commit:** `4083929` (on main branch)
- **Message:** `test(bf-309kjc): add hybrid-002-vector-form-over-scan PDF fixture`
- **Files added:** 4 (generator, PDF, metadata, PROVENANCE.md entry)

## Provenance
- SHA256: `96661595f94cf6122a7bdcda9b4af89de6d7911d3909739f1b30cfcd7fb63c9e`
- License: MIT-0 (matches generator script)
- Entry added to `tests/fixtures/profiles/PROVENANCE.md`

## Test Value
This fixture tests the hybrid extraction pipeline's ability to:
- Extract scattered vector content through cell-level OCR
- Handle complex merge patterns with multiple overlapping regions
- Isolate form fields from scanned background labels
- Detect small vector elements (checkbox indicators)
- Accurately merge vector and scanned text when they overlap significantly

## Conclusion
All acceptance criteria met. Fixture ready for testing.
