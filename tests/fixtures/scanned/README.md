# Scanned PDF Fixtures for OCR Testing

This directory contains scanned PDF fixtures with ground-truth transcripts for Word Error Rate (WER) testing.

## Purpose

These fixtures support:
- **AS-02 test scenario**: Extract a scanned receipt via OCR
- **Tier 1 OCR gate**: WER < 3% on clean 300-DPI scans
- **Performance testing**: 10-page scanned PDF extraction in < 30 seconds

## Directory Structure

```
scanned/
├── README.md                     # This file
├── receipt/                      # Single-page receipt fixtures
│   ├── receipt-300dpi.pdf       # Clean receipt at 300 DPI
│   └── receipt-300dpi.txt       # Ground truth transcript
├── documents/                    # Various document type fixtures
│   ├── invoice-300dpi.pdf
│   ├── invoice-300dpi.txt
│   ├── form-300dpi.pdf
│   └── form-300dpi.txt
└── multi-page/                   # Multi-page fixtures for performance testing
    ├── doc-10page-300dpi.pdf
    └── doc-10page-300dpi.txt
```

## Generation Instructions

Use the provided generation script to create scanned PDFs:

```bash
# Install dependencies
# Python 3 with reportlab, PIL/Pillow, img2pdf
pip3 install reportlab Pillow img2pdf

# Generate all fixtures
cd tests/fixtures/scanned
python3 generate_scanned_fixtures.py
```

For manual generation:
1. Create a PDF from the `.txt` ground truth file using a Tesseract-friendly font (Arial, Helvetica, Times New Roman)
2. Set font size to 12pt for good OCR readability
3. Use 300 DPI for the scan
4. Apply minimal preprocessing (no aggressive compression)

## WER Targets

- **Clean 300-DPI scans**: WER < 3%
- **Receipts**: WER < 3% (critical for totals, line items)
- **Multi-page documents**: Average WER < 3%, no page > 5%

## Verification

To verify WER on a fixture:

```bash
# Extract text with pdftract
pdftract extract tests/fixtures/scanned/receipt/receipt-300dpi.pdf --ocr --text > output.txt

# Compute WER (requires jiwer or similar)
python3 -c "
from jiwer import wer
with open('tests/fixtures/scanned/receipt/receipt-300dpi.txt') as f:
    ground_truth = f.read()
with open('output.txt') as f:
    hypothesis = f.read()
print(f'WER: {wer(ground_truth, hypothesis):.2%}')
"
```

## Fixtures Status

| Fixture | PDF | Ground Truth | WER Target | Status |
|---------|-----|--------------|------------|--------|
| receipt-300dpi | ❌ | ✅ | < 3% | PDF needed |
| invoice-300dpi | ❌ | ✅ | < 3% | PDF needed |
| form-300dpi | ❌ | ✅ | < 3% | PDF needed |
| doc-10page-300dpi | ❌ | ✅ | < 3% avg | PDF needed |

## Adding New Fixtures

1. Create the ground truth `.txt` file with the exact content
2. Generate the corresponding `.pdf` using the generation script or manually
3. Add the fixture to this README's table
4. Update generation script if applicable

## Notes

- All fixtures use English language with Tesseract `eng` traineddata
- Fonts should be standard: Arial, Helvetica, Times New Roman, or Courier
- Avoid decorative fonts, handwriting, or unusual layouts for baseline fixtures
- For challenging fixtures, consider creating a separate `challenging/` subdirectory
