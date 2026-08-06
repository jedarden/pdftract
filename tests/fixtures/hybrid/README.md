# Hybrid PDF Fixtures for Page Classification Testing

This directory contains hybrid PDF fixtures with mixed vector text and scanned image content for testing the PageClass::Hybrid classifier and hybrid extraction pipeline (Phase 5.2.4).

## Primary Fixtures (First 3)

### hybrid-001: Vector Header Over Scan
**File**: `hybrid-001-vector-header-over-scan.pdf` (1.2 KB)
**Pattern**: Vector letterhead header + scanned letter body (vertical stack)
- **Vector regions**: Company name "ACME Corp", report title "Annual Report 2024" (top ~15%, Helvetica 14pt)
- **Scanned regions**: Letter body with horizontal line pattern (bottom ~85%, 1-bit grayscale image)
- **Hybrid cells**: ~8 cells (12.5% of page, boundary row between header and body)
- **Classification challenge**: Clean vertical separation may cause classifier to treat as separate Vector + Scanned regions rather than unified Hybrid page
- **Test focus**: Header extraction precision, OCR on body only, non-overlapping merge rule, minimal fixture performance
- **Generation**: `hybrid-001-generator.py`

### hybrid-002: Vector Form Over Scan
**File**: `hybrid-002-vector-form-over-scan.pdf` (1.5 KB)
**Pattern**: Vector form field annotations over scanned form background (partial overlay)
- **Vector regions**: Form field annotations scattered throughout: field labels, red checkbox indicators, underline rectangles (Helvetica 9-10pt)
- **Scanned regions**: Complete employee information form background with title, labels, underlines, certification text (1-bit grayscale image)
- **Hybrid cells**: ~48 cells (75.0% of page, most grid cells contain both vector and scanned content)
- **Classification challenge**: Scattered vector distribution makes boundary detection difficult; complex merge patterns with multiple overlapping spans
- **Test focus**: Scattered vector extraction through cell-level OCR, complex merge pattern handling, form field isolation, small vector element detection
- **Generation**: `hybrid-002-generator.py`

### hybrid-003: Mixed Column Layout
**File**: `hybrid-003-mixed-column-layout.pdf` (1.6 KB)
**Pattern**: Vector text in left column + scanned content in right column (horizontal side-by-side)
- **Vector regions**: Article title and content about document processing/OCR technology (left ~45%, columns 0-3 of 8×8 grid, Helvetica 9-10pt)
- **Scanned regions**: Newspaper article content simulation with horizontal line pattern (right ~55%, columns 4-7, 1-bit grayscale image)
- **Hybrid cells**: 0 cells (0.0% - no overlapping cells due to clean horizontal separation)
- **Classification challenge**: Page-level hybrid detection required when no cells contain both content types; system may classify as separate Vector + Scanned regions
- **Test focus**: Page-level hybrid detection for horizontally separated layouts, column boundary identification, side-by-side layout handling
- **Generation**: `hybrid-003-generator.py`

## Additional Fixtures (Fixtures 4-7)

### hybrid-004: Watermark Over Scan
**File**: `hybrid-004-watermark-over-scan.pdf` (1.4 KB)
**Pattern**: Scanned document with vector diagonal watermark overlay
- **Vector regions**: Full-page diagonal watermark overlay (28pt Helvetica, gray color, rotated 45° centered) plus small page header text (10pt Helvetica, top ~10%)
- **Scanned regions**: Full-page scanned document background (1-bit grayscale image XObject with text line pattern throughout)
- **Hybrid cells**: ~64 cells (100.0% - full-page overlap)
- **Classification challenge**: Full-page overlap may trigger over-aggressive Hybrid classification; diagonal watermark creates complex cell-level hybrid patterns crossing grid boundaries at oblique angles
- **Test focus**: Full-page hybrid cell detection, diagonal text extraction precision, merge rule priority for decorative vs substantive content
- **Generation**: `hybrid-004-generator.py`

### hybrid-005: Vector Footer Over Scan
**File**: `hybrid-005-vector-footer-over-scan.pdf` (1.5 KB)
**Pattern**: Scanned document body with vector footer containing page numbers
- **Vector regions**: Page header (28pt Helvetica, top ~5%) and footer region (9pt Helvetica with page numbers and confidentiality notice, bottom ~10%) with horizontal divider line
- **Scanned regions**: Document body (1-bit grayscale image XObject, top 90%) with text line pattern throughout
- **Hybrid cells**: ~8 cells (12.5% - bottom boundary row)
- **Classification challenge**: Tests classification symmetry with hybrid-001 (vertical stack but vector at bottom instead of top); small footer elements may be missed if scan covers too much
- **Test focus**: Footer extraction precision, boundary detection accuracy, classification symmetry with top-header patterns, small vector region detection
- **Generation**: `hybrid-005-generator.py`

### hybrid-006: Stamp Annotation
**File**: `hybrid-006-stamp-annotation.pdf` (1.5 KB)
**Pattern**: Scanned contract with vector circular stamp/seal overlay
- **Vector regions**: Page header text (12pt Helvetica, top ~10%) and circular stamp seal in bottom right corner (10pt Helvetica-Bold, red color, circular border, ~60pt radius)
- **Scanned regions**: Full-page contract document background (1-bit grayscale image XObject with dense legal text pattern)
- **Hybrid cells**: ~12 cells (18.75% - bottom right corner)
- **Classification challenge**: Localized stamp in corner may be missed if classifier doesn't scan entire page; circular stamp crosses grid cell boundaries radially; tests color-aware classification (red on grayscale)
- **Test focus**: Localized hybrid region detection, circular element handling, color awareness, stamp extraction precision
- **Generation**: `hybrid-006-generator.py`

### hybrid-007: Textbox Overlay
**File**: `hybrid-007-textbox-overlay.pdf` (1.4 KB)
**Pattern**: Scanned tax form with vector fillable textbox overlays
- **Vector regions**: Form title (10pt Helvetica, top ~5%) and multiple fillable textbox overlays scattered throughout: rectangular borders with gray stroke (1pt) and placeholder labels (9pt Helvetica)
- **Scanned regions**: Full-page tax form background (1-bit grayscale image XObject with horizontal/vertical dividers and form label patterns)
- **Hybrid cells**: ~28 cells (43.75% - distributed across form)
- **Classification challenge**: Scattered textbox distribution similar to hybrid-002 but with form-specific layout; multiple small hybrid regions require accumulation; tests duplicate text elimination when vector and scanned describe same field
- **Test focus**: Scattered hybrid region detection with form-specific layout, form field extraction precision, rectangular border handling, duplicate text elimination
- **Generation**: `hybrid-007-generator.py`

## Purpose

These fixtures support:
- **KU-2 resolution**: Tesseract behaviour on Hybrid pages with overlapping vector + scan content
- **Phase 5.5 classifier tuning**: 10 known-tricky hybrid cases for testing page-classifier decision rules
- **8×8 grid cell evaluation**: Testing hybrid cell detection (≥15% image-heavy cells threshold)
- **Merge rule validation**: Testing bbox overlap rule (IoU > 0.5) for merging vector and OCR spans

## What Makes a PDF "Hybrid"?

A hybrid PDF contains BOTH:
1. **Vector text**: Text drawn via PDF content streams (selectable, in `/Text` operators)
2. **Scanned/image regions**: Raster images that require OCR

This is distinct from:
- **Pure vector PDFs** (tests/fixtures/vector/): All text is vector, no images
- **Pure scanned PDFs** (tests/fixtures/scanned/): All content is raster, OCR required
- **BrokenVector pages**: Vector text present but encoding is broken (invisible text layer over scan)

## Directory Structure

```
hybrid/
├── README.md                                    # This file
├── GEN_MANIFEST.md                             # Fixture metadata and generation records
├── generate_hybrid_fixtures.py                 # Generation script
├── hybrid-001-vector-header-over-scan.pdf      # Primary fixture 1: letterhead pattern
├── hybrid-001-vector-header-over-scan.pdf.metadata.json
├── hybrid-001-generator.py                     # Fixture 1 generator script
├── hybrid-002-vector-form-over-scan.pdf        # Primary fixture 2: form pattern
├── hybrid-002-vector-form-over-scan.pdf.metadata.json
├── hybrid-002-generator.py                     # Fixture 2 generator script
├── hybrid-003-mixed-column-layout.pdf          # Primary fixture 3: column layout pattern
├── hybrid-003-mixed-column-layout.pdf.metadata.json
├── hybrid-003-generator.py                     # Fixture 3 generator script
├── hybrid-004-watermark-over-scan.pdf          # Fixture 4: watermark overlay
├── hybrid-004-watermark-over-scan.pdf.metadata.json
├── hybrid-004-generator.py                     # Fixture 4 generator script
├── hybrid-005-vector-footer-over-scan.pdf      # Fixture 5: vector footer
├── hybrid-005-vector-footer-over-scan.pdf.metadata.json
├── hybrid-005-generator.py                     # Fixture 5 generator script
├── hybrid-006-stamp-annotation.pdf             # Fixture 6: stamp annotation
├── hybrid-006-stamp-annotation.pdf.metadata.json
├── hybrid-006-generator.py                     # Fixture 6 generator script
├── hybrid-007-textbox-overlay.pdf             # Fixture 7: textbox overlay
├── hybrid-007-textbox-overlay.pdf.metadata.json
├── hybrid-007-generator.py                     # Fixture 7 generator script
├── receipt-overtext/                           # Receipt with scanned body + vector overlay text
├── letterhead-image/                           # Letterhead: vector header + scanned body
├── form-mixed/                                 # Form: vector fields + scanned background
├── invoice-stamp/                              # Invoice: vector line items + scanned stamp/logo
├── document-annotation/                        # Document: scanned page + vector annotations
├── figure-caption/                             # Academic: vector text + scanned figure
├── sidebar-image/                              # Article: vector main text + scanned sidebar
├── watermark/                                  # Document: vector text + scanned watermark overlay
├── multi-column-scan/                          # Multi-column: vector headers + scanned columns
└── complex-overlap/                            # Complex: interleaved vector and scanned regions
```

## Generation Strategy

Each hybrid fixture is crafted to test specific aspects of the hybrid pipeline:

### 1. receipt-overtext/
**Challenge**: Scanned receipt body with vector prices overlaid
- **Vector region**: Price totals, tax calculations (bottom 25%)
- **Scanned region**: Receipt body, line items (top 75%)
- **Hybrid cells**: ~24 cells (rows 5-7, all cols)
- **Test**: Merge rule with overlapping vector/OCR on price区域

### 2. letterhead-image/
**Challenge**: Vector letterhead header + scanned letter body
- **Vector region**: Company name, address, date (top 15%)
- **Scanned region**: Letter content, signature (bottom 85%)
- **Hybrid cells**: ~40 cells (rows 1-7, cols 0-7)
- **Test**: Header extraction precision, OCR on body only

### 3. form-mixed/
**Challenge**: Vector form fields over scanned form background
- **Vector region**: Fillable form fields, checkboxes (scattered cells)
- **Scanned region**: Form labels, background, instructions (majority)
- **Hybrid cells**: ~45 cells (image-heavy with scattered vector)
- **Test**: Scattered vector extraction through cell-level OCR

### 4. invoice-stamp/
**Challenge**: Vector invoice line items + scanned approval stamp
- **Vector region**: Line items, totals, table structure (80%)
- **Scanned region**: Approval stamp, signature (20% overlap)
- **Hybrid cells**: ~12 cells (partial overlap regions)
- **Test**: Merge rule with high-confidence vector vs OCR

### 5. document-annotation/
**Challenge**: Scanned document with vector highlight annotations
- **Vector region**: Highlight boxes, margin notes (overlay)
- **Scanned region**: Original document content (background)
- **Hybrid cells**: ~36 cells (most of page)
- **Test**: OCR priority for underlying content vs vector annotations

### 6. figure-caption/
**Challenge**: Academic paper: vector figure caption + scanned figure
- **Vector region**: Figure caption, reference number (bottom 10%)
- **Scanned region**: Figure content, chart, graph (top 90%)
- **Hybrid cells**: ~8 cells (figure area only)
- **Test**: Precise caption extraction, figure OCR accuracy

### 7. sidebar-image/
**Challenge**: Newsletter: vector main text + scanned sidebar image
- **Vector region**: Main article text (70% width, left side)
- **Scanned region**: Sidebar image, photo (30% width, right side)
- **Hybrid cells**: ~24 cells (rightmost 3 columns)
- **Test**: Column-aware hybrid cell detection

### 8. watermark/
**Challenge**: Vector text over scanned watermark background
- **Vector region**: Main document text (foreground)
- **Scanned region**: Watermark, logo (background, partial overlap)
- **Hybrid cells**: ~64 cells (full page, watermark is page-wide)
- **Test**: Vector confidence vs OCR with low-contrast background

### 9. multi-column-scan/
**Challenge**: Multi-column doc with vector headers + scanned columns
- **Vector region**: Column headers, section titles (top 20%)
- **Scanned region**: Multi-column body content (bottom 80%)
- **Hybrid cells**: ~48 cells (body area only)
- **Test**: Column detection + hybrid cell grid alignment

### 10. complex-overlap/
**Challenge**: Interleaved vector and scanned regions (checkerboard pattern)
- **Vector region**: Scattered blocks (checkerboard: 32 cells)
- **Scanned region**: Complementary blocks (checkerboard: 32 cells)
- **Hybrid cells**: ~32 cells (every other cell)
- **Test**: Worst-case merge rule performance, complex bbox overlap

## 8×8 Grid Cell Threshold

Per Phase 5.5 classification rules:
- Page is **Hybrid** when ≥ 15% of cells (≥ 12 of 64 cells) are image-heavy
- Image-heavy cell = pixel coverage from images exceeds text coverage
- Cells below threshold: Vector extraction only
- Cells at/above threshold: OCR + merge with vector spans

## Generation Instructions

Use the provided generation script to create hybrid PDFs:

```bash
# Install dependencies
# Python 3 with reportlab, PIL/Pillow, img2pdf
pip3 install reportlab Pillow img2pdf

# Generate all fixtures
cd tests/fixtures/hybrid
python3 generate_hybrid_fixtures.py
```

For manual generation:
1. Create vector text regions using reportlab or similar
2. Create scanned image regions (scan or render to image)
3. Combine using reportlab's `drawImage` and `drawString`
4. Ensure image and text regions overlap strategically per fixture design
5. Export as PDF

## Hybrid Extraction Pipeline

When pdftract processes a hybrid page:

1. **Classification** (Phase 5.1): Detect PageClass::Hybrid, compute hybrid_cells (≥15% threshold)
2. **Render full page** (Phase 5.2.4): Render at selected DPI (default 300)
3. **Crop cells**: For each hybrid cell, crop from rendered page
4. **OCR per cell**: Run Tesseract on each cell image independently
5. **Merge**: Combine vector spans + OCR spans using bbox overlap rule:
   - If IoU(OCR, Vector) > 0.5 AND vector_confidence ≥ 0.5: keep vector
   - If IoU(OCR, Vector) > 0.5 AND vector_confidence < 0.5: keep OCR
   - If IoU(OCR, Vector) ≤ 0.5: keep both

## Verification

To verify hybrid extraction on a fixture:

```bash
# Extract with hybrid handling
pdftract extract tests/fixtures/hybrid/receipt-overtext/receipt-overtext.pdf --text > output.txt

# Check classification result
pdftract classify tests/fixtures/hybrid/receipt-overtext/receipt-overtext.pdf
# Should show: PageClass::Hybrid with hybrid_cells set

# Verify merge quality
# Compare output.txt with expected .txt ground truth
# Check that no duplicate text from overlapping regions
```

## Test Scenarios

Each fixture targets specific test scenarios:

| Fixture | Primary Test | Merge Rule Stress | Cell Count |
|---------|--------------|-------------------|------------|
| receipt-overtext | Price precision | High (overlap) | ~24 |
| letterhead-image | Header isolation | Low (separate) | ~40 |
| form-mixed | Scattered vector | Medium (scattered) | ~45 |
| invoice-stamp | High-conf vector | High (stamp overlap) | ~12 |
| document-annotation | Annotation vs content | Medium (overlay) | ~36 |
| figure-caption | Caption extraction | Low (separate) | ~8 |
| sidebar-image | Column alignment | Low (side-by-side) | ~24 |
| watermark | Low-contrast vector | High (page-wide) | ~64 |
| multi-column-scan | Column detection | Medium (body) | ~48 |
| complex-overlap | Worst-case merge | Very high (checkerboard) | ~32 |

## Classifier Tuning Targets

Phase 5.5 uses these fixtures to tune:

1. **Cell image-heavy threshold**: Current default 15%, may need adjustment
2. **Merge rule IoU threshold**: Current default 0.5, test for false positives/negatives
3. **Vector confidence threshold**: Current default 0.5, test OCR preference
4. **DPI selection**: Cell render DPI impacts OCR quality

## Known Unknowns (KU-2)

This fixture suite directly addresses KU-2: *Tesseract behaviour on Hybrid pages with overlapping vector + scan content*.

Resolution strategy: Use these 10 fixtures to measure and tune:
- False positive rate (Vector classified as Hybrid)
- False negative rate (Hybrid classified as Vector)
- Merge rule accuracy (duplicate elimination)
- OCR confidence on hybrid cells vs pure scanned

## Fixtures Status

| Fixture | PDF | Vector Content | Scanned Content | Hybrid Cells | Status |
|---------|-----|----------------|-----------------|--------------|--------|
| **hybrid-001** (vector-header-over-scan) | ✅ | ✅ | ✅ | ~8 (12.5%) | Complete |
| **hybrid-002** (vector-form-over-scan) | ✅ | ✅ | ✅ | ~48 (75.0%) | Complete |
| **hybrid-003** (mixed-column-layout) | ✅ | ✅ | ✅ | 0 (0.0%) | Complete |
| **hybrid-004** (watermark-over-scan) | ✅ | ✅ | ✅ | ~64 (100.0%) | Complete |
| **hybrid-005** (vector-footer-over-scan) | ✅ | ✅ | ✅ | ~8 (12.5%) | Complete |
| **hybrid-006** (stamp-annotation) | ✅ | ✅ | ✅ | ~12 (18.75%) | Complete |
| **hybrid-007** (textbox-overlay) | ✅ | ✅ | ✅ | ~28 (43.75%) | Complete |
| receipt-overtext | ❌ | ✅ | ✅ | ~24 | Pending |
| letterhead-image | ❌ | ✅ | ✅ | ~40 | Pending |
| form-mixed | ❌ | ✅ | ✅ | ~45 | Pending |
| invoice-stamp | ❌ | ✅ | ✅ | ~12 | Pending |
| document-annotation | ❌ | ✅ | ✅ | ~36 | Pending |
| figure-caption | ❌ | ✅ | ✅ | ~8 | Pending |
| sidebar-image | ❌ | ✅ | ✅ | ~24 | Pending |
| watermark | ❌ | ✅ | ✅ | ~64 | Pending |
| multi-column-scan | ❌ | ✅ | ✅ | ~48 | Pending |
| complex-overlap | ❌ | ✅ | ✅ | ~32 | Pending |

## Adding New Hybrid Fixtures

1. Design the hybrid case: Which cells are vector? Which are scanned?
2. Create vector text content (headers, forms, annotations)
3. Create scanned image content (body, figures, backgrounds)
4. Combine with strategic overlap
5. Verify ≥ 12 cells are image-heavy (15% threshold)
6. Add to this README's table and GEN_MANIFEST.md

## Notes

- All fixtures use English language with Tesseract `eng` traineddata
- Vector text uses standard fonts: Arial, Helvetica, Times New Roman
- Scanned regions rendered at 300 DPI for realistic OCR testing
- Image regions use JPEG/PNG with moderate compression (quality 85%)
- Overlap regions are designed to stress-test the merge rule
- For non-English hybrid fixtures, create a subdirectory with language-specific tests

## Related Fixtures

- `tests/fixtures/vector/` — Pure vector PDFs (no hybrid content)
- `tests/fixtures/scanned/` — Pure scanned PDFs (no vector text)
- `tests/fixtures/classifier/` — Page classification test corpus
- `tests/fixtures/page_class/` — PageClass validation fixtures
