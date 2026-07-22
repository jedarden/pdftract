# Scanned Fixtures Generation Manifest

This document tracks the generation status and specifications for all scanned fixtures.

## Fixture Specifications

### receipt-300dpi
- **Purpose**: AS-02 test scenario, basic receipt OCR
- **Ground Truth**: `receipt/receipt-300dpi.txt`
- **Target PDF**: `receipt/receipt-300dpi.pdf`
- **Specifications**:
  - Font: Helvetica 10pt
  - Page size: Letter (8.5" x 11")
  - Margins: 0.5" all sides
  - Line spacing: 14pt
  - Content: Supermarket receipt with items, prices, totals
- **WER Target**: < 3%
- **Status**: Ground truth created, PDF generation pending

### invoice-300dpi
- **Purpose**: Business document OCR testing
- **Ground Truth**: `documents/invoice-300dpi.txt`
- **Target PDF**: `documents/invoice-300dpi.pdf`
- **Specifications**:
  - Font: Helvetica 11pt
  - Page size: Letter (8.5" x 11")
  - Margins: 0.75" all sides
  - Line spacing: 16pt
  - Content: Service invoice with line items, totals, payment terms
- **WER Target**: < 3%
- **Status**: Ground truth created, PDF generation pending

### form-300dpi
- **Purpose**: Form structure OCR testing
- **Ground Truth**: `documents/form-300dpi.txt`
- **Target PDF**: `documents/form-300dpi.pdf`
- **Specifications**:
  - Font: Helvetica 11pt
  - Page size: Letter (8.5" x 11")
  - Margins: 0.75" all sides
  - Line spacing: 18pt
  - Content: Employment application form with fields and checkboxes
- **WER Target**: < 3%
- **Status**: Ground truth created, PDF generation pending

### doc-10page-300dpi
- **Purpose**: Multi-page performance testing
- **Ground Truth**: `multi-page/doc-10page-300dpi.txt`
- **Target PDF**: `multi-page/doc-10page-300dpi.pdf`
- **Specifications**:
  - Font: Times-Roman 12pt
  - Page size: Letter (8.5" x 11")
  - Margins: 1" left/right, 0.75" top/bottom
  - Line spacing: 18pt
  - Content: 10 pages with diverse content types
  - Page markers: "Page N:" format for explicit page breaks
- **WER Target**: < 3% average, no page > 5%
- **Performance Target**: < 30 seconds on 4-core CI
- **Status**: Ground truth created, PDF generation pending

## Generation Checklist

For each fixture, complete these steps:

1. [ ] Verify ground truth `.txt` file exists and is complete
2. [ ] Run generation script: `python3 tools/generate_scanned_fixtures.py <fixture-name>`
3. [ ] Verify generated PDF is readable and displays correctly
4. [ ] Test OCR extraction: `pdftract extract <pdf> --ocr --text`
5. [ ] Compute WER against ground truth using `tools/calculate_wer.py`
6. [ ] Update this manifest with WER result
7. [ ] If WER < 3%, mark as PASS; otherwise, investigate

## Low-Quality Fixtures

The `low-quality/` subdirectory contains intentionally degraded OCR fixtures for testing robustness against poor scan quality.

### degraded-200dpi
- **Purpose**: Test OCR quality on degraded 200 DPI scans
- **Ground Truth**: `low-quality/degraded-200dpi-ground-truth.txt`
- **Target PDF**: `low-quality/degraded-200dpi.pdf`
- **Generation**: `python tools/create_degraded_200dpi.py`
- **Degradation Effects**: Gaussian blur (0.3px radius), noise (±12), reduced contrast (90%), reduced sharpness (85%), JPEG compression (85%)
- **WER Target**: < 10% (degraded fixtures have higher acceptable WER)
- **Status**: Generated, OCR output available (degraded-200dpi-ocr.txt)

## WER Results

To be populated after PDF generation and testing:

| Fixture | WER | Pass/Fail | Notes |
|---------|-----|-----------|-------|
| receipt-300dpi | TBD | TBD | - |
| invoice-300dpi | TBD | TBD | - |
| form-300dpi | TBD | TBD | - |
| doc-10page-300dpi | TBD | TBD | Per-page breakdown needed |

## Dependencies

### Required for PDF Generation
- Python 3.8+
- reportlab: `pip3 install reportlab`
- (Optional) Pillow: `pip3 install Pillow`
- (Optional) img2pdf: `pip3 install img2pdf`

### Required for Scan Simulation
- poppler-utils: `apt-get install poppler-utils` (provides pdftoppm)

### Required for WER Calculation
- jiwer: `pip3 install jiwer`
- Or: Python implementation for basic WER

## Manual Generation Alternative

If the generation script fails, manual generation steps:

1. Create a new document in LibreOffice/Word
2. Copy ground truth text from `.txt` file
3. Set font to Helvetica/Arial at specified size
4. Set page size to Letter
5. Set margins as specified
6. Export to PDF
7. (Optional) Use a scanner or PDF printer to simulate scan at 300 DPI

## Related Beads

- bf-2he4t: Initial corpus assembly (this bead)
- (Future) WER gate implementation
- (Future) AS-02 test scenario implementation
