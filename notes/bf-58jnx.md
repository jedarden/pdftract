# bf-58jnx: Add high-quality single-page OCR fixtures

## Summary
Verified and committed 300 DPI single-page OCR fixtures for invoice, letter, and form document types with ground truth text files.

## Verification

### Fixtures Created
All fixtures exist in `tests/fixtures/scanned/`:

1. **Invoice** (`invoice/invoice-300dpi.pdf` + `invoice/invoice-300dpi-ground-truth.txt`)
   - 177 words in ground truth
   - Contains realistic invoice with header, line items, totals, payment details

2. **Letter** (`letter/letter-300dpi.pdf` + `letter/letter-300dpi-ground-truth.txt`)
   - 227 words in ground truth
   - Contains business letter with header, body, signature block

3. **Form** (`form/form-300dpi.pdf` + `form/form-300dpi-ground-truth.txt`)
   - 281 words in ground truth
   - Contains employment application form with multiple fields and checkboxes

### Acceptance Criteria Verification

#### ✅ 3 fixtures exist in `tests/fixtures/scanned/`
All three fixtures (invoice, letter, form) are present with both PDF and ground truth text files.

#### ✅ Each has a PDF + ground truth .txt file
All fixtures have both components:
- Invoice: 661KB PDF + 1.8KB ground truth
- Letter: 448KB PDF + 1.4KB ground truth  
- Form: 829KB PDF + 3.0KB ground truth

#### ✅ Each PDF is ~300 DPI
Verified using `pdfimages -list`:
- All images are 2550×3300 pixels
- At standard US Letter size (8.5" × 11"), this equals exactly 300 DPI
- No embedded fonts (`pdffonts` shows no fonts), confirming true scanned content

#### ✅ Running WER script with perfect OCR gives WER = 0%
Tested `scripts/measure-wer.sh` on each ground truth file:
- Invoice: WER 0.00% (177/177 words match)
- Letter: WER 0.00% (227/227 words match)
- Form: WER 0.00% (281/281 words match)

## References
- Plan line 28 (WER <3% gate)
- Parent bead: bf-33zjo
- Prerequisite: bf-3gewx (WER script)
