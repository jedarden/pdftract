# pdftract-48ea: BrokenVector fixtures + WER delta CI gate

## Summary

Created two PDF/A fixtures for testing the assisted-OCR (BrokenVector) path and extended the WER gate CI script to include WER validation for these fixtures.

## Changes Made

### 1. Fixture Generation (xtask)

Added `generate-brokenvector-fixtures` command to xtask:
- `generate_brokenvector_fixtures()`: Main function that orchestrates fixture generation
- `create_brokenvector_pdf()`: Creates PDFs with invisible text layer (Tr=3) at controllable positions
- `escape_pdf_string()`: Helper to escape special characters for PDF text literals

### 2. Fixtures Created

**Aligned fixture**: `tests/fixtures/ocr/brokenvector_aligned/`
- `source.pdf`: PDF with invisible text layer at correct positions
- `ground_truth.txt`: Lorem Ipsum text content
- `README.md`: Documentation for the fixture
- Size: 1.5 KB (well under 200 KB requirement)

**Misaligned fixture**: `tests/fixtures/ocr/brokenvector_misaligned/`
- `source.pdf`: PDF with invisible text layer offset by (10pt, 5pt)
- `ground_truth.txt`: Same Lorem Ipsum text content
- `README.md`: Documentation for the fixture
- Size: 1.5 KB (well under 200 KB requirement)

### 3. WER Gate Extension (ci/wer-gate.sh)

Extended the WER gate script with:
- New threshold constants for BrokenVector fixtures
- `test_brokenvector_aligned_fixture()`: Tests aligned fixture (expects WER < 2%)
- `test_brokenvector_misaligned_fixture()`: Tests misaligned fixture (expects WER < 5%)
- Updated help text to include new fixture thresholds
- Integrated new tests into main test flow

## Acceptance Criteria Status

- ✅ Two BrokenVector fixtures committed
  - Aligned: `tests/fixtures/ocr/brokenvector_aligned/source.pdf` (1.5 KB)
  - Misaligned: `tests/fixtures/ocr/brokenvector_misaligned/source.pdf` (1.5 KB)
- ✅ Fixture sizes < 200 KB each (both are 1.5 KB)
- ✅ ci/wer-gate.sh extended with new fixture invocations
- ⚠️ WER delta test passes on both fixtures (requires OCR environment)
  - Tests will be skipped gracefully when Tesseract is not available
  - In environment with OCR: assisted OCR should outperform blind OCR on aligned
- ⚠️ Regression test (disabling validation filter) requires OCR environment

## Verification

Generated fixtures using:
```bash
cd xtask && cargo run --bin xtask -- generate-brokenvector-fixtures
```

Verified:
- Fixtures are valid PDFs with different hashes (confirming offset works)
- Ground truth files are identical between fixtures
- File sizes are well under 200 KB requirement
- WER gate script syntax is valid and includes new tests
- Tests will skip gracefully when OCR dependencies are unavailable

## Notes

The WER delta comparison between assisted and blind OCR requires:
1. pdftract built with `--features ocr`
2. System Tesseract installation with language packs
3. Ability to force different OCR modes (not yet exposed in CLI)

The current implementation tests that assisted OCR produces reasonable WER values:
- Aligned: < 2% (assisted OCR should work very well)
- Misaligned: < 5% (should not regress significantly)

Full WER delta testing (assisted vs blind comparison) would require CLI flags to force specific extraction modes, which is not currently implemented. The fixtures and infrastructure are in place for future enhancement.

## Files Modified

- `xtask/src/main.rs`: Added fixture generation code
- `ci/wer-gate.sh`: Extended with BrokenVector test functions
- `tests/fixtures/ocr/brokenvector_aligned/`: New fixture directory
- `tests/fixtures/ocr/brokenvector_misaligned/`: New fixture directory
- `tests/fixtures/ocr/generate_brokenvector_fixtures.py`: Python generation script (alternative method)

## References

- Plan section: Phase 5.5 critical tests (lines 1940-1941)
- Bead: pdftract-48ea
