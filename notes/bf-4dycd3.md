# Verification: hybrid-007-textbox-overlay Fixture

## Bead
bf-4dycd3 - Create hybrid-007-textbox-overlay fixture

## Status
✅ COMPLETE - Fixture already existed and meets all requirements

## Acceptance Criteria Verification

### 1. File exists in tests/fixtures/hybrid/
- ✅ File: `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf`
- ✅ Size: 1,407 bytes (well under 5 MB limit)
- ✅ Valid PDF header: `%PDF-1.4`

### 2. Has .metadata.json sidecar with all required fields
- ✅ File: `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf.metadata.json`
- ✅ Size: 3,512 bytes
- ✅ Contains all required fields:
  - `source.type`: "synthetic"
  - `source.generation_method`: Full description via hybrid-007-generator.py
  - `source.generated_date`: "2026-08-06"
  - `pages_with_hybrid_content`: [1]
  - `grid_cell_coverage.total_cells`: 64
  - `grid_cell_coverage.hybrid_cells_approx`: 28
  - `grid_cell_coverage.hybrid_percentage`: 43.75%
  - `classification_challenges`: 5 detailed challenges listed

### 3. File is < 5 MB
- ✅ Actual size: 1.4 KB (far under 5 MB limit)

### 4. Textbox overlays are clearly vector on scan background
- ✅ Generator creates hybrid-007-generator.py creates:
  - **Scanned background**: 1-bit grayscale image XObject (612×792) simulating tax form with horizontal dividers every 60pts, vertical dividers, and form label patterns
  - **Vector textbox overlays**: Multiple rectangular borders with gray stroke (1pt) and placeholder labels in 9-10pt Helvetica:
    - "Form 1040 - U.S. Individual Income Tax Return" (title, 10pt)
    - "First name" textbox (210, 680, 150×20)
    - "Last name" textbox (210, 650, 150×20)
    - "SSN" textbox (420, 680, 150×20)
    - "Home address" textbox (210, 560, 360×60)
    - "City, State, ZIP" (under Home address)
    - "Total income" textbox (210, 480, 150×20)
    - "Tax withheld" textbox (420, 480, 150×20)

## Why This Case Is Tricky for Classification

From metadata, the fixture tests these classification challenges:

1. **Scattered textbox distribution** - Similar to hybrid-002 but with form-specific layout (tax form pattern)
2. **Rectangular textbox borders** - Additional vector content beyond just text labels
3. **Multiple small hybrid regions** - May be missed if classifier doesn't accumulate distributed hybrid cells
4. **Duplicate text** - Form field labels in both vector (textboxes) and scanned (background)
5. **Merge rule accuracy** - Tests merge rules when vector and scanned content describe the same form field

## Expected Classification Results

- **Page class**: "Hybrid"
- **Hybrid cells**: ~28 cells (distributed across form)
- **Confidence range**: 0.80-0.92
- **Hybrid percentage**: 43.75% (28 of 64 grid cells)

## Real-World Pattern Representation

This fixture represents a common real-world scenario: **fillable forms with vector field annotations overlaid on scanned form backgrounds**. Examples include:
- Tax forms with fillable PDF textboxes
- Government forms with electronic field overlays
- Business applications with vector form fields on scanned originals

## Files Generated
- `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf` (1,407 bytes)
- `tests/fixtures/hybrid/hybrid-007-textbox-overlay.pdf.metadata.json` (3,512 bytes)
- `tests/fixtures/hybrid/hybrid-007-generator.py` (generator script, executable)

## Test Coverage

This fixture complements:
- **hybrid-002** (vector form over scan) - General form fields
- **hybrid-007** (textbox overlay) - Tax form with fillable textboxes including borders

Both test form-specific layouts but hybrid-007 adds:
- Rectangular borders as vector graphics (not just text)
- Tax form pattern with characteristic horizontal/vertical dividers
- Distributed hybrid regions across multiple form sections

## PASS Summary
All acceptance criteria PASS:
- ✅ File exists in correct location
- ✅ Complete metadata with all required fields
- ✅ File size well under 5 MB limit
- ✅ Clear vector-on-scan hybrid pattern with textbox overlays
