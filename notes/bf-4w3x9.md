# Verification Note for bf-4w3x9: Degraded Fixture WER Measurement

## Summary
Verified the degraded PDF fixture and WER measurement script infrastructure. The fixture files are in place and the measurement script works correctly, but OCR extraction is not available in the current environment.

## Acceptance Criteria Status

### ✅ PASS: Script infrastructure is ready
- `scripts/measure-wer.sh` is executable and functional
- `tests/fixtures/scanned/calculate_wer.py` implements correct WER calculation
- Script accepts OCR output and ground truth as inputs
- Returns exit code 0 for WER ≤ 3%, exit code 1 for WER > 3%

### ✅ PASS: Fixture files are properly installed
- `tests/fixtures/scanned/low-quality/degraded-200dpi.pdf` (588KB) - verified exists
- `tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt` (1,967 chars) - verified exists
- Ground truth contains 321 words from Abraham Lincoln biography
- Source: Public domain Project Gutenberg eBook #11728

### ✅ PASS: WER calculation verified
Tested with identical files (expected 0% WER):
```
WER: 0.0000 (0.00%)
Reference words: 321
Hypothesis words: 321
Exit code: 0
```

Tested with modified file (expected small WER):
```
WER: 0.0125 (1.25%)
Reference words: 321
Hypothesis words: 325 (4 extra words added)
Exit code: 0 (passes 3% threshold)
```

### ⚠️ WARN: OCR extraction not available in current environment
Attempted to build pdftract with `--features ocr` but failed due to missing system dependencies:
```
pkg-config: Package lept was not found in the pkg-config search path
error: The system library 'lept' required by crate 'leptonica-sys' was not found
```

**Impact**: Cannot generate OCR output from the degraded PDF in this environment.

## What Would Be Needed for Full Verification

To complete the full WER measurement as intended by the bead:

1. **Install system dependencies**: `leptonica` library development headers
2. **Rebuild pdftract with OCR**: `cargo build --bin pdftract --release --features ocr`
3. **Extract text from degraded PDF**: 
   ```bash
   ./target/release/pdftract extract --text /tmp/degraded-ocr.txt \
     --ocr tests/fixtures/scanned/low-quality/degraded-200dpi.pdf
   ```
4. **Run WER measurement**:
   ```bash
   VERBOSE=1 scripts/measure-wer.sh \
     /tmp/degraded-ocr.txt \
     tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt
   ```

## Expected WER Result

Based on the intentional degradation applied (200 DPI, blur, noise, contrast reduction, compression):
- **Expected WER**: > 3% (will fail the quality gate, which is acceptable for this edge case)
- The degradation was specifically designed to test OCR quality limits
- This fixture is for validating that OCR quality gates work correctly

## Test Infrastructure Integration

The degraded fixture integrates properly with the test infrastructure:
- Located in standard fixture directory: `tests/fixtures/scanned/low-quality/`
- Follows naming convention: `{fixture-name}-ground-truth.txt`
- Documented in `tests/fixtures/PROVENANCE.md`
- Measurement script follows standard pattern for scanned fixtures

## Conclusion

The degraded fixture and WER measurement infrastructure are properly set up and verified. The only blocker is the missing OCR system library in the current environment, which prevents actual OCR extraction from the PDF. Once OCR dependencies are available, the full WER measurement can be performed as designed.

## Files Verified
- ✅ `scripts/measure-wer.sh` - executable and functional
- ✅ `tests/fixtures/scanned/calculate_wer.py` - correct WER implementation  
- ✅ `tests/fixtures/scanned/low-quality/degraded-200dpi.pdf` - 588KB fixture
- ✅ `tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt` - 1,967 chars
- ✅ `tests/fixtures/PROVENANCE.md` - fixture documented
