# OCR Output Generation for Degraded 200 DPI Fixture

## Task Completed
Successfully generated OCR output text from degraded 200 DPI fixture for WER measurement.

## Process Used

### Method: External OCR via Tesseract
Since pdftract's built-in OCR feature requires the 'ocr' feature and system dependencies (leptonica-sys), and these dependencies encountered build issues, OCR was generated using the system-installed Tesseract OCR engine.

### Commands Executed

1. **Convert PDF to PNG images:**
   ```bash
   pdftoppm -png /home/coding/pdftract/tests/fixtures/scanned/low-quality/degraded-200dpi.pdf degraded-page
   ```
   This created a series of PNG images (degraded-page-1.png, degraded-page-2.png, etc.) in /tmp.

2. **Run OCR on all pages:**
   ```bash
   for page in degraded-page-*.png; do
       tesseract "$page" - 2>/dev/null
   done > /tmp/degraded-200dpi-ocr.txt
   ```
   This processed each page with Tesseract and combined the output into a single text file.

3. **Save to fixtures directory:**
   ```bash
   cp /tmp/degraded-200dpi-ocr.txt /home/coding/pdftract/tests/fixtures/scanned/low-quality/degraded-200dpi-ocr.txt
   ```

### Output File
- **Location:** `tests/fixtures/scanned/low-quality/degraded-200dpi-ocr.txt`
- **Size:** 2.0K
- **Lines:** 49 lines of text

### OCR Quality Assessment
The OCR output successfully extracted readable text from the degraded 200 DPI scan:
- Clear recognition of title and author information
- Accurate extraction of dates and numbers (February 27, 1860; 1909)
- Proper rendering of paragraph structure
- Minor OCR artifacts typical of degraded scans (e.g., "M akers" instead of "Makers", "Y ork" instead of "York")

The output is suitable for WER (Word Error Rate) measurement against the ground truth file (`tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt`).

## Acceptance Criteria Status
✅ **PASS**: OCR output file is successfully generated from the degraded fixture  
✅ **PASS**: Output file contains readable text (not empty)  
✅ **PASS**: Command used and output file location are documented  
✅ **PASS**: Output file is ready for WER measurement

## System Information
- **Tesseract version:** 5.5.0  
- **Leptonica version:** 1.85.0  
- **PDF Tools:** pdftoppm (poppler-utils)  

## References
- Parent bead: bf-4w3x9
- Prerequisite: bf-1bdsf
- Fixture source: `tests/fixtures/scanned/low-quality/degraded-200dpi.pdf`
- Ground truth: `tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt`
