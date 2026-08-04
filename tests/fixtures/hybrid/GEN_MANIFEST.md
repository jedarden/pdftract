# Hybrid Fixture Generation Manifest

This manifest tracks the generation, modification, and verification status of all hybrid PDF fixtures in this directory.

## Fixture Metadata

Each hybrid fixture entry includes:
- `name`: Fixture directory name
- `description`: One-line summary of the hybrid case
- `vector_regions`: Description of vector text content
- `scanned_regions`: Description of scanned/image content
- `hybrid_cells_approx`: Approximate number of image-heavy cells (out of 64)
- `overlap_type`: How vector and scanned regions interact (separate/partial/complete)
- `test_focus`: What aspect of the hybrid pipeline this tests
- `generation_date`: When the fixture was generated
- `verification_status`: pending | verified | failed
- `notes`: Any additional context

## Fixtures

### receipt-overtext
- **description**: Scanned receipt body with vector price overlay text
- **vector_regions**: Price totals, tax calculations, subtotal (bottom 25% of page, rows 5-7)
- **scanned_regions**: Receipt body, line items, merchant info (top 75% of page, rows 0-4)
- **hybrid_cells_approx**: 24 cells (rows 5-7, all 8 cols)
- **overlap_type**: partial (vector prices over scanned totals area)
- **test_focus**: Merge rule with overlapping vector/OCR on price区域; vector confidence priority
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Classic e-receipt format where receipt is scanned but totals are overlaid as vector for machine readability. **Placeholder PDF created - requires reportlab for full hybrid generation.**

### letterhead-image
- **description**: Vector letterhead header + scanned letter body
- **vector_regions**: Company name, logo, address, contact info, date (top 15%, rows 0-1, cols 0-7)
- **scanned_regions**: Letter content, salutation, body paragraphs, signature (bottom 85%, rows 1-7)
- **hybrid_cells_approx**: 40 cells (rows 1-7, all cols, partial row 1)
- **overlap_type**: separate (clear boundary between header and body)
- **test_focus**: Header extraction precision; OCR on body only; non-overlapping merge
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Common business letter format; tests that vector header doesn't trigger OCR on header cells

### form-mixed
- **description**: Vector form fields over scanned form background
- **vector_regions**: Fillable text fields, checkboxes, dropdown indicators (scattered cells, ~15%)
- **scanned_regions**: Form labels, instructions, background design, field borders (most cells, ~85%)
- **hybrid_cells_approx**: 45 cells (majority of page, scattered vector overlay)
- **overlap_type**: partial (vector fields over scanned labels)
- **test_focus**: Scattered vector extraction through cell-level OCR; complex merge patterns
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Simulates PDF forms where the layout is scanned but fillable fields are vector overlays

### invoice-stamp
- **description**: Vector invoice line items + scanned approval stamp
- **vector_regions**: Invoice header, line items table, totals, calculations (80% of page)
- **scanned_regions**: Approval stamp, signature, handwritten note (20% overlap in bottom-right)
- **hybrid_cells_approx**: 12 cells (bottom-right corner where stamp overlaps)
- **overlap_type**: partial (stamp overlaps some vector totals)
- **test_focus**: High-confidence vector vs OCR merge; stamp region OCR priority
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Tests that high-confidence vector content is not replaced by OCR of overlapping stamp/signature

### document-annotation
- **description**: Scanned document with vector highlight annotations
- **vector_regions**: Highlight boxes (transparent yellow), margin notes, arrows (overlay layer)
- **scanned_regions**: Original document content, paragraphs, text (background layer)
- **hybrid_cells_approx**: 36 cells (most of page has highlights or annotations)
- **overlap_type**: complete (annotations cover entire page)
- **test_focus**: OCR priority for underlying content vs vector annotations; annotation preservation
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Simulates annotated academic papers; tests that OCR captures text under highlights while preserving annotation spans

### figure-caption
- **description**: Academic paper with vector figure caption + scanned figure
- **vector_regions**: Figure number, caption text, reference markers (bottom 10%, row 7)
- **scanned_regions**: Figure content, chart, graph, axes (top 90%, rows 0-6)
- **hybrid_cells_approx**: 8 cells (figure area only, rows 0-6)
- **overlap_type**: separate (clear boundary between figure and caption)
- **test_focus**: Precise caption extraction; figure OCR accuracy; minimal cell coverage
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Tests hybrid detection on low-hybrid-cell-count pages (8 cells = 12.5%, just below 15% threshold if miscounted)

### sidebar-image
- **description**: Newsletter with vector main text + scanned sidebar image
- **vector_regions**: Main article text, headlines, byline (70% width, left side, cols 0-4)
- **scanned_regions**: Sidebar image, photo, caption (30% width, right side, cols 5-7)
- **hybrid_cells_approx**: 24 cells (rightmost 3 columns, all rows)
- **overlap_type**: separate (vertical split, no overlap)
- **test_focus**: Column-aware hybrid cell detection; side-by-side merge without conflicts
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Tests column detection with hybrid content; verifies OCR runs only on sidebar columns

### watermark
- **description**: Vector text over scanned watermark background
- **vector_regions**: Main document text, paragraphs, headings (foreground, high contrast)
- **scanned_regions**: Watermark, logo, background pattern (background, page-wide, low opacity)
- **hybrid_cells_approx**: 64 cells (full page, watermark is page-wide)
- **overlap_type**: complete (watermark underlies all text)
- **test_focus**: Vector confidence vs OCR with low-contrast background; maximum hybrid cell count
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Worst-case for hybrid cell count (100% cells); tests that vector text is extracted despite watermark background

### multi-column-scan
- **description**: Multi-column document with vector headers + scanned columns
- **vector_regions**: Column headers, section titles, page number (top 20%, rows 0-1)
- **scanned_regions**: Multi-column body content, paragraphs (bottom 80%, rows 1-7)
- **hybrid_cells_approx**: 48 cells (body area, rows 1-7, all cols)
- **overlap_type**: partial (headers over first line of scanned content)
- **test_focus**: Column detection + hybrid cell grid alignment; multi-column OCR
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Newsletter/magazine format; tests that column detection works correctly when columns are hybrid

### complex-overlap
- **description**: Interleaved vector and scanned regions (checkerboard pattern)
- **vector_regions**: Scattered blocks in checkerboard pattern (32 alternating cells)
- **scanned_regions**: Complementary blocks in complementary checkerboard (32 other cells)
- **hybrid_cells_approx**: 32 cells (exactly half the page, every other cell)
- **overlap_type**: partial (checkerboard boundaries have mini-overlaps)
- **test_focus**: Worst-case merge rule performance; complex bbox overlap calculation
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Stress test for merge algorithm; 32 vector spans + 32 OCR regions with alternating pattern

## Generation Summary

- **Total fixtures**: 10
- **Generated (placeholder)**: 10
- **Generated (full)**: 0
- **Verified**: 0
- **Failed verification**: 0

**Note**: All fixtures have been generated as placeholder PDFs with ground truth .txt files and specification READMEs. For production-quality hybrid PDFs with proper vector+scan overlap, install `reportlab`/`Pillow`/`img2pdf` and run `generate_hybrid_fixtures.py`.

## Generation Script

Run `python3 generate_hybrid_fixtures.py` to generate all pending fixtures.

The script will:
1. Create vector content using reportlab
2. Create scanned content by rendering text to images
3. Combine vector + scanned content per fixture specifications
4. Apply strategic overlap per fixture design
5. Output PDF + .txt ground truth for each fixture

## Verification Checklist

For each fixture, verify:

- [ ] PDF generates successfully without errors
- [ ] PDF is valid (passes `pdfinfo` and `pdfextract -v`)
- [ ] Vector text is selectable (pdfextract can extract it)
- [ ] Scanned regions require OCR (not extractable as vector)
- [ ] Hybrid cell count is ≥ 12 (15% threshold)
- [ ] Hybrid cells are correctly identified by classifier
- [ ] Merge rule produces no duplicate text
- [ ] Ground truth .txt matches expected extraction

## Classification Targets

Expected classification for each fixture:

| Fixture | Expected PageClass | Expected hybrid_cells | Expected cell confidence |
|---------|-------------------|------------------------|--------------------------|
| receipt-overtext | Hybrid | ~24 | 0.70-0.85 |
| letterhead-image | Hybrid | ~40 | 0.75-0.90 |
| form-mixed | Hybrid | ~45 | 0.65-0.80 |
| invoice-stamp | Hybrid | ~12 | 0.60-0.75 |
| document-annotation | Hybrid | ~36 | 0.70-0.85 |
| figure-caption | Hybrid | ~8 | 0.55-0.70 |
| sidebar-image | Hybrid | ~24 | 0.65-0.80 |
| watermark | Hybrid | ~64 | 0.80-0.95 |
| multi-column-scan | Hybrid | ~48 | 0.70-0.85 |
| complex-overlap | Hybrid | ~32 | 0.60-0.75 |

## Test Coverage

This fixture suite covers:

- **Hybrid cell threshold**: 8 to 64 cells (12.5% to 100% of page)
- **Overlap types**: separate, partial, complete
- **Vector confidence**: low (0.2), medium (0.5), high (0.9)
- **Merge patterns**: scattered, columnar, checkerboard, full-page
- **Real-world formats**: receipts, letters, forms, invoices, academic papers, newsletters
- **Edge cases**: minimum hybrid cells (figure-caption), maximum hybrid cells (watermark)

## Related Documentation

- `README.md`: Usage and test scenario documentation
- `generate_hybrid_fixtures.py`: Generation script
- `docs/plan/plan.md` KU-2: Known Unknown this fixture suite resolves
- Phase 5.5: Page classifier tuning using these fixtures
