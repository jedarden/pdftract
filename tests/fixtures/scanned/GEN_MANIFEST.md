# Scanned Fixtures Generation Manifest

This document tracks the generation status and specifications for all scanned fixtures.

Regenerated 2026-09-13 (bead `bf-33zjo`) with a width-aware generator and an
OCR-stable corpus; every clean 300 DPI fixture now passes the
`scripts/measure-wer.sh` WER <= 3% gate (see [WER Results](#wer-results)).

## Fixture Specifications

All clean fixtures render in **DejaVu Serif** (real TTF, resolved from
`$PDFTRACT_FIXTURE_FONT` or a system/nix dejavu path by the generator).
Reportlab's built-in Type1 core fonts (Helvetica/Times-Roman) must NOT be
used: poppler's rasterization of them mis-advances initial capitals and
tesseract splits them off ("Johnson" -> "J ohnson"), which alone pushes WER
above 8%. DejaVu Serif also has a serifed capital I, avoiding the sans-serif
`I` -> `|` confusion.

### receipt-300dpi
- **Ground Truth**: `receipt/receipt-300dpi.txt` (143 words)
- **Scanned PDF**: `receipt/receipt-300dpi-scanned.pdf` (1 page)
- **Text twin**: `receipt/receipt-300dpi.pdf`
- **Reference OCR**: `receipt/receipt-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 10pt, Letter, 0.5" margins, 14pt spacing
- **Content**: Supermarket receipt with items, prices, totals

### invoice-300dpi
- **Ground Truth**: `invoice/invoice-300dpi-ground-truth.txt` (105 words)
- **Scanned PDF**: `invoice/invoice-300dpi.pdf` (1 page)
- **Text twin**: `invoice/invoice-300dpi-text-embedded.pdf`
- **Reference OCR**: `invoice/invoice-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 11pt, Letter, 0.75" margins, 16pt spacing
- **Content**: Service invoice with line items, totals, payment terms

### letter-300dpi
- **Ground Truth**: `letter/letter-300dpi-ground-truth.txt` (227 words)
- **Scanned PDF**: `letter/letter-300dpi.pdf` (1 page)
- **Text twin**: `letter/letter-300dpi-text-embedded.pdf`
- **Reference OCR**: `letter/letter-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 11pt, Letter, 1.0" margins, 16pt spacing
- **Content**: Business letter with letterhead, address block, salutation

### form-300dpi
- **Ground Truth**: `form/form-300dpi-ground-truth.txt` (264 words)
- **Scanned PDF**: `form/form-300dpi.pdf` (3 pages)
- **Text twin**: `form/form-300dpi-text-embedded.pdf`
- **Reference OCR**: `form/form-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 11pt, Letter, 0.75" margins, 18pt spacing
- **Content**: Employment application form with fields and checkboxes

### report-300dpi (multi-page)
- **Ground Truth**: `multi-page/report-300dpi-ground-truth.txt` (1383 words)
- **Scanned PDF**: `multi-page/report-300dpi.pdf` (11 pages)
- **Reference OCR**: `multi-page/report-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 12pt, Letter, 1.0" L/R + 0.75" T/B margins,
  18pt spacing, explicit page breaks at `Page N:` markers
- **Content**: 10-section report (prose, form, table, API docs, legal,
  financials, correspondence, scientific abstract, summary)
- **Performance Target**: < 30 seconds on 4-core CI

### doc-10page-300dpi (legacy)
- `multi-page/doc-10page-300dpi.txt` + `-scanned.pdf`: earlier 10-page
  fixture kept for compatibility; `report-300dpi` is the canonical multi-page
  fixture.

## OCR-Stability Curation Rules

The ground-truth texts avoid constructs that tesseract cannot round-trip at
300 DPI (measured with tesseract 5.5.2, poppler rasterization):

- **Standalone dash rules are dropped** (`----` alone on a line is removed as
  a ruling line at any length) — receipt/invoice separators were removed from
  the GT entirely. Mid-line dash runs <= 16 chars survive.
- **Underscore runs never survive** (any length, always treated as ruling
  lines) — form fill fields use 10-dot dotted lines (`..........`), the
  classic printed-form convention, which round-trips verbatim. Dotted runs
  longer than ~12 chars start hallucinating letters, so fills are capped.
- **`[ ]` spaced checkboxes sometimes merge** to `[]` — use unspaced `[]`.
- **Sparse two-column lines are fragile** (whole lines can be dropped from
  layout analysis) — the REFERENCES block in the form GT is single-column.
- **ASCII box tables (`+----+`, `|` cells) do not round-trip** — the report's
  quarterly sales table uses aligned columns without box borders.

## WER Results

Measured 2026-09-13 with tesseract 5.5.2 + `scripts/measure-wer.sh`
(gate: WER <= 3% on clean 300 DPI fixtures):

| Fixture | Words | WER | Gate | Notes |
|---------|-------|------|------|-------|
| receipt-300dpi | 143 | 0.00% | PASS (exit 0) | exact match |
| invoice-300dpi | 105 | 0.00% | PASS (exit 0) | exact match |
| letter-300dpi | 227 | 0.00% | PASS (exit 0) | exact match |
| form-300dpi | 264 | 2.27% | PASS (exit 0) | 3 sparse `Name:` fill lines dropped |
| report-300dpi | 1383 | 0.87% | PASS (exit 0) | 11 pages; residual code-block I/l and split-token noise |
| degraded-200dpi | 321 | 8.10% | n/a (degraded) | intentionally outside the gate |

To re-verify:

```sh
R=tests/fixtures/scanned
scripts/measure-wer.sh $R/invoice/invoice-300dpi-ocr.txt $R/invoice/invoice-300dpi-ground-truth.txt
# ... same for receipt, letter, form, multi-page/report
```

## Regeneration

```sh
nix-shell -p 'python3.withPackages(p: [p.reportlab p.pillow p.img2pdf])' \
           -p poppler-utils -p dejavu_fonts \
           --run 'python3 tools/generate_scanned_fixtures.py'
```

The generator renders each GT line width-aware (shrink to fit down to 7pt,
then word-wrap), so no line is ever clipped at the right margin — the
original fixed-size drawing truncated every long letter line.

Reference OCR outputs are produced by `pdfimages -png` + `tesseract <img>
stdout -l eng` per page, concatenated in page order.

## Low-Quality Fixtures

The `low-quality/` subdirectory contains intentionally degraded OCR fixtures for testing robustness against poor scan quality.

### degraded-200dpi
- **Purpose**: Test OCR quality on degraded 200 DPI scans
- **Ground Truth**: `low-quality/degraded-200dpi-ground-truth.txt`
- **Target PDF**: `low-quality/degraded-200dpi.pdf`
- **Generation**: `python tools/create_degraded_200dpi.py`
- **Degradation Effects**: Gaussian blur (0.3px radius), noise (±12), reduced contrast (90%), reduced sharpness (85%), JPEG compression (85%)
- **WER Target**: < 10% (degraded fixtures have higher acceptable WER)
- **Status**: Generated, OCR output available (degraded-200dpi-ocr.txt, 8.10%)

## Dependencies

- Python 3.8+: reportlab, Pillow, img2pdf
- poppler-utils (pdftoppm, pdfimages)
- A DejaVu Serif TTF (nix: `dejavu_fonts`)
- tesseract 5.x (for reference OCR outputs and the WER gate)

## Related Beads

- bf-2he4t: Initial corpus assembly
- bf-33zjo: Regeneration (width-aware generator, DejaVu Serif, OCR-stable
  GT curation), WER gate verification across all clean fixtures
