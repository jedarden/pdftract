# BrokenVector Misaligned Fixture

This fixture tests the assisted-OCR path with a misaligned invisible text layer.

## Fixture Properties

- **Page class**: BrokenVector
- **Text layer**: Invisible (Tr=3) text offset by (10pt, 5pt)
- **Ground truth**: Accurate text content from the scan
- **Expected behavior**: Assisted OCR should not regress significantly vs blind OCR

## Generating source.pdf

This fixture is generated using the `generate_brokenvector_fixtures.py` script in the parent directory:

```bash
cd tests/fixtures/ocr
python generate_brokenvector_fixtures.py
```

The script:
1. Creates a clean text scan of Lorem Ipsum at 300 DPI
2. Embeds an invisible text layer (Tr=3) offset by (10pt, 5pt)
3. Outputs a PDF/A-1b compliant file

The offset is intentionally outside the 5pt validation threshold to trigger the confidence cap.

## Expected WER Delta

- **Blind OCR WER**: ~2-3% (baseline without position hints)
- **Assisted OCR WER**: ~2-4% (position validation capped, but no significant regression)
- **Delta**: Assisted should be within 0.5% of blind (no significant regression)

## Test Coverage

This fixture validates:
- Position validation filter rejects misaligned words (confidence capped at 0.4)
- Assisted OCR falls back gracefully without significant regression
- WER delta gate allows small tolerance for misaligned text layers
