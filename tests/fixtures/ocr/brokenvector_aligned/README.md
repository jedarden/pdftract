# BrokenVector Aligned Fixture

This fixture tests the assisted-OCR path with a correctly-positioned invisible text layer.

## Fixture Properties

- **Page class**: BrokenVector
- **Text layer**: Invisible (Tr=3) text at correct positions
- **Ground truth**: Accurate text content from the scan
- **Expected behavior**: Assisted OCR should outperform blind OCR (WER delta < -1%)

## Generating source.pdf

This fixture is generated using the `generate_brokenvector_fixtures.py` script in the parent directory:

```bash
cd tests/fixtures/ocr
python generate_brokenvector_fixtures.py
```

The script:
1. Creates a clean text scan of Lorem Ipsum at 300 DPI
2. Embeds an invisible text layer (Tr=3) at the correct glyph positions
3. Outputs a PDF/A-1b compliant file

## Expected WER Delta

- **Blind OCR WER**: ~2-3% (baseline without position hints)
- **Assisted OCR WER**: < 1% (with position validation)
- **Delta**: Assisted should be at least 1% better than blind

## Test Coverage

This fixture validates:
- Position validation filter accepts correctly-aligned words
- Assisted OCR produces better results than blind OCR
- WER delta gate detects regression when validation filter is disabled
