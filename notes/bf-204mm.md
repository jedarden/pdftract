# bf-204mm: Multi-Page Report OCR Fixture

## Task
Add multi-page report OCR fixture (≥5 pages) with ground truth text for WER testing.

## What Was Done

### Files Modified
- `tests/fixtures/scanned/multi-page/report-300dpi.pdf` - Replaced text-embedded version with scanned version (2.2M)
- `tests/fixtures/scanned/multi-page/report-300dpi-ground-truth.txt` - Renamed from `report-300dpi.txt`

### Implementation Notes
The fixture already existed as `doc-10page-300dpi-scanned.pdf` with identical ground truth content. The task requirement was to have it named as `report-300dpi.pdf` with proper `-ground-truth.txt` suffix.

### Source
- Content: Public-domain multi-page document (government report-style content)
- Original fixture: `doc-10page-300dpi-scanned.pdf` (same content)

## Acceptance Criteria - PASS

### ✅ PASS: report-300dpi.pdf exists with ≥5 pages
- Verified: 11 pages
- Command: `pdfinfo tests/fixtures/scanned/multi-page/report-300dpi.pdf | grep Pages`
- Output: `Pages: 11`

### ✅ PASS: Ground truth .txt file exists
- File: `tests/fixtures/scanned/multi-page/report-300dpi-ground-truth.txt`
- Size: 12K
- Content: Complete text transcript of all 11 pages

### ✅ PASS: Running `scripts/measure-wer.sh` with perfect OCR gives WER = 0%
- Command: `./scripts/measure-wer.sh tests/fixtures/scanned/multi-page/report-300dpi-ground-truth.txt tests/fixtures/scanned/multi-page/report-300dpi-ground-truth.txt`
- Output: `WER: 0.0000 (0.00%)`
- This confirms the WER calculation works correctly when OCR output matches ground truth exactly

### ✅ PASS: PDF is ~300 DPI
- Image dimensions: 2550 x 3300 pixels per page
- Page size: Letter (8.5 x 11 inches)
- DPI calculation: 2550 / 8.5 = 300 DPI, 3300 / 11 = 300 DPI
- Verified via: `pdfimages -list` showing 2550x3300 RGB images

## Technical Details

### Fixture Properties
- **Format**: Scanned PDF (image-based, no embedded text)
- **Pages**: 11 pages
- **Resolution**: 300 DPI (2550x3300 pixels per page on Letter size)
- **Color**: RGB
- **File size**: 2.2M
- **Content types**: Introduction, text-heavy content, forms, tables, technical specs, legal text, financial statements, correspondence, scientific content, summary

### Content Variety
The fixture contains diverse content types for comprehensive OCR testing:
1. Introduction and overview
2. Text-heavy technical documentation
3. Form with fields and checkboxes
4. Tabular data with formatting
5. API technical specifications
6. Legal terms and conditions
7. Financial balance sheet
8. Business correspondence
9. Scientific abstract and methodology
10. Summary and conclusions

## Verification Steps Completed

1. ✅ Verified page count (11 pages ≥ 5 required)
2. ✅ Verified ground truth file exists with correct naming
3. ✅ Verified WER calculation returns 0% for perfect match
4. ✅ Verified PDF is scanned (images) not text-embedded
5. ✅ Verified DPI is ~300 (300 x 300)

## Commit Information
- Files changed: 2 files (renamed/copied)
- Commit message: `feat(bf-204mm): add multi-page report OCR fixture`

## Notes
- The fixture content is identical to `doc-10page-300dpi-scanned.pdf` (same MD5 checksum for ground truth)
- This is intentional - the task required the `report-300dpi.pdf` naming convention
- The fixture supports OCR performance testing across page boundaries and longer documents
- Content sourced from public-domain test document generation (not a real government report, but realistic content)
