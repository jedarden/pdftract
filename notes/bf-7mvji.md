# Form OCR Fixture Verification (bf-7mvji)

## Task
Source and create form OCR fixture at 300 DPI with ground truth text.

## Verification Results

### Acceptance Criteria Status

✅ **PASS**: form-300dpi.pdf exists in tests/fixtures/scanned/form/
- File: 810K PDF
- Location: `/home/coding/pdftract/tests/fixtures/scanned/form/form-300dpi.pdf`
- Created: 2026-07-05

✅ **PASS**: form-300dpi-ground-truth.txt exists with complete text content
- File: 3.0K text file
- Location: `/home/coding/pdftract/tests/fixtures/scanned/form/form-300dpi-ground-truth.txt`
- Contains complete employment application form text (79 lines)
- Includes all form fields, labels, and certification text

✅ **PASS**: PDF is actual scanned content (no text layer)
- `pdffonts` shows no embedded fonts
- `pdfimages -list` confirms 3 image pages:
  - Page 1: 2550x3300 grayscale image (335K)
  - Page 2: 2550x3300 grayscale image (323K)
  - Page 3: 2550x3300 grayscale image (147K)
- No text extraction possible (true scanned PDF)

✅ **PASS**: PDF is approximately 300 DPI
- Image dimensions: 2550 x 3300 pixels
- For US Letter (8.5" x 11"):
  - Width DPI: 2550 / 8.5 = 300.0 DPI
  - Height DPI: 3300 / 11.0 = 300.0 DPI
- **Exact match to 300 DPI target**

## Source Document

The fixtures were created as part of the scanned fixtures generation process using `convert_to_scanned.sh`, which converts text-embedded PDFs to true scanned image-based PDFs at 300 DPI. The source is a generic employment application form (public domain content).

## Files Verified

1. **tests/fixtures/scanned/form/form-300dpi.pdf**
   - 810K scanned PDF
   - 3 pages
   - True image-based content (no text layer)
   - Exactly 300 DPI

2. **tests/fixtures/scanned/form/form-300dpi-ground-truth.txt**
   - 79 lines of complete form text
   - Includes all fields: Personal Information, Availability, Education, Employment History, References, Certification
   - Formatted with line numbers for OCR evaluation

## Verification Commands Used

```bash
# Check file existence
ls -lh tests/fixtures/scanned/form/form-300dpi.pdf
ls -lh tests/fixtures/scanned/form/form-300dpi-ground-truth.txt

# Verify scanned content (no fonts)
pdffonts tests/fixtures/scanned/form/form-300dpi.pdf

# Verify image content and dimensions
pdfimages -list tests/fixtures/scanned/form/form-300dpi.pdf

# Calculate DPI
python3 -c "width_px=2550; height_px=3300; print(f'{width_px/8.5:.1f} x {height_px/11.0:.1f} DPI')"
```

## Conclusion

All acceptance criteria for bead bf-7mvji are met. The form OCR fixtures exist, contain proper scanned content at exactly 300 DPI, and include complete ground truth text for OCR evaluation.
