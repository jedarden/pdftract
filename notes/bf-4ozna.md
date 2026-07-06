# Bead bf-4ozna: Create degraded 200 DPI PDF from source document

## Summary

Successfully created a degraded 200 DPI PDF from the Abraham Lincoln public domain source document (Project Gutenberg eBook #11728).

## Files Created/Modified

1. **tests/fixtures/scanned/low-quality/degraded-200dpi.pdf** (588KB)
   - Created from source-document-abraham-lincoln-public-domain.txt (first 2500 chars)
   - Deliberately degraded at 200 DPI with multiple effects:
     - Gaussian blur (radius 0.3) - simulates poor focus
     - Random noise (amount 12) - simulates scan artifacts
     - Reduced contrast (0.9) - simulates poor scan quality
     - Reduced sharpness (0.85) - simulates compression artifacts
   - Image-only PDF suitable for OCR testing

2. **tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt** (2.3KB)
   - Updated with correct ground truth from Abraham Lincoln text
   - Contains the text that was embedded in the degraded PDF

3. **tests/fixtures/scanned/low-quality/create_degraded_200dpi.py**
   - Python script to create degraded PDFs with configurable effects
   - Uses reportlab for PDF creation, Pillow for image processing
   - Requires nix-shell with python3Packages.reportlab and python3Packages.pillow

## Acceptance Criteria Status

- ✅ degraded-200dpi.pdf exists in tests/fixtures/scanned/low-quality/
- ✅ PDF is visibly degraded when viewed (200 DPI, artifacts, or noise present)
- ✅ PDF is readable enough for OCR but clearly poor quality
- ✅ File size reasonable for a test fixture (588KB)

## Verification

The PDF was created using the following process:
1. Extract first 2500 characters from Abraham Lincoln public domain source
2. Create clean PDF from text at 200 DPI using reportlab
3. Convert PDF to images at 200 DPI using pdftoppm
4. Apply degradation effects (blur, noise, contrast, sharpness reduction)
5. Convert degraded images back to PDF using Pillow

The resulting PDF is an image-only document with visible degradation effects that simulate poor scan quality while remaining readable enough for OCR testing.

## Commands Used

```bash
# Create the degraded PDF
cd tests/fixtures/scanned/low-quality
nix-shell -p python3Packages.reportlab python3Packages.pillow --run 'python3 create_degraded_200dpi.py'

# Verify the output
pdfinfo degraded-200dpi.pdf
ls -lh degraded-200dpi.pdf degraded-200dpi-ground-truth.txt
```

## Git Commit

All changes committed with message: "feat(bf-4ozna): create degraded 200 DPI PDF from public domain source"