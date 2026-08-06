# Verification: hybrid-007-textbox-overlay fixture

## Status: ✅ COMPLETE (fixture already exists)

## Summary
The hybrid-007-textbox-overlay fixture was created in commit f2bc02e as part of bead bf-401erx. All acceptance criteria are met.

## Files Present
- `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf` (1,407 bytes)
- `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf.metadata.json` (3,512 bytes)
- `tests/fixtures/hybrid/hybrid-007-generator.py` (4,585 bytes, executable)

## Acceptance Criteria Verification

### ✅ File exists in tests/fixtures/hybrid/
Confirmed: `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf` exists.

### ✅ Has .metadata.json sidecar with all required fields
Confirmed: `hybrid-007-textbox-overlay.pdf.metadata.json` contains:
- **source**: type="synthetic", generation_method documented
- **pages_with_hybrid_content**: [1]
- **grid_cell_coverage**: 
  - total_cells: 64
  - hybrid_cells_approx: 28
  - hybrid_percentage: 43.75%
  - hybrid_cell_locations: distributed across form
- **classification_challenges**: 5 documented challenges
- **test_focus**: 5 test focus areas
- **expected_classification**: page_class="Hybrid", confidence_range="0.80-0.92"

### ✅ File is < 5 MB
Confirmed: File size is 1,407 bytes (~1.4 KB), well under 5 MB limit.

### ✅ Textbox overlays are clearly vector on scan background
Verified via generator script (hybrid-007-generator.py):
- **Scanned background**: Full-page 1-bit grayscale image XObject simulating tax form with horizontal dividers every 60pts, vertical dividers, and form label patterns
- **Vector overlays**: Multiple fillable textboxes with:
  - Rectangular borders (gray stroke, 1pt)
  - Placeholder labels (9pt Helvetica): "First name", "Last name", "SSN", "Home address", "Total income", "Tax withheld"
  - Form title (10pt Helvetica): "Form 1040 - U.S. Individual Income Tax Return"

## Fixture Details

### PDF Structure
- **Pages**: 1
- **Page size**: 612 x 792 pts (letter)
- **PDF version**: 1.4
- **Content stream**: Image XObject (scanned form) + vector graphics (textbox borders) + vector text (labels)

### Hybrid Content Pattern
- **Overlap type**: scattered-overlay (multiple textboxes distributed across form)
- **Vector regions**: Form title at top, fillable textboxes at various positions
- **Scanned regions**: Full-page tax form background with horizontal/vertical dividers
- **Hybrid cells**: ~28 of 64 (43.75%) - distributed across rows where textboxes overlap background

### Why This Case is Tricky for Classification
1. **Scattered distribution**: Multiple small hybrid regions may be missed if classifier doesn't accumulate distributed hybrid cells
2. **Form field duplicates**: Labels appear in both vector (textboxes) and scanned (background), creating potential duplicate text
3. **Multiple content types**: Vector includes both text (labels) and graphics (rectangular borders)
4. **Form-specific layout**: Tax form pattern with characteristic dividers may confuse classifiers expecting different layouts

## Conclusion
The fixture meets all acceptance criteria and is ready for use in testing hybrid PDF classification.

## References
- Generator: `tests/fixtures/hybrid/hybrid-007-generator.py`
- Metadata: `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf.metadata.json`
- Original commit: f2bc02e (bf-401erx)
- Bead: bf-4dycd3
