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
- **Ground Truth**: `invoice/invoice-300dpi.txt` (105 words)
- **Scanned PDF**: `invoice/invoice-300dpi.pdf` (1 page)
- **Text twin**: `invoice/invoice-300dpi-text-embedded.pdf`
- **Reference OCR**: `invoice/invoice-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 11pt, Letter, 0.75" margins, 16pt spacing
- **Content**: Service invoice with line items, totals, payment terms

### letter-300dpi
- **Ground Truth**: `letter/letter-300dpi.txt` (227 words)
- **Scanned PDF**: `letter/letter-300dpi.pdf` (1 page)
- **Text twin**: `letter/letter-300dpi-text-embedded.pdf`
- **Reference OCR**: `letter/letter-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 11pt, Letter, 1.0" margins, 16pt spacing
- **Content**: Business letter with letterhead, address block, salutation

### form-300dpi
- **Ground Truth**: `form/form-300dpi.txt` (166 words)
- **Scanned PDF**: `form/form-300dpi.pdf` (1 page)
- **Text twin**: `form/form-300dpi-text-embedded.pdf`
- **Reference OCR**: `form/form-300dpi-ocr.txt`
- **Specifications**: DejaVu Serif 11pt, Letter, 0.75" margins, 18pt spacing
- **Content**: Employment application form with fields and checkboxes
- **Note**: redesigned 2026-09-18 (bead `pdftract-d2201653`) from the original
  3-page/264-word layout to a single page (36 GT lines) so the invoice/letter/form
  trio is uniformly single-page; reference fields are packed onto shared lines,
  which also eliminated the sparse `Name:` fill-line drops the 3-page version had

### report-300dpi (multi-page)
- **Ground Truth**: `multi-page/report-300dpi-ground-truth.txt` (1383 words)
- **Ground Truth alias**: `multi-page/report-300dpi.txt` (byte-identical copy of
  the ground truth under the plain `<name>.txt` name, matching invoice/letter/form)
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

## Source and License Attribution

- receipt, invoice, letter, form, report are **original synthetic compositions**
  created for this repository (fictional businesses and people). No third-party
  document was scanned or reproduced. Ground-truth texts and the generated PDFs
  are dedicated to the public domain (**CC0 1.0**).
- Raster provenance (every clean fixture): ground-truth text -> reportlab PDF
  (`tools/generate_scanned_fixtures.py`, DejaVu Serif) -> `pdftoppm -r 300` PNG
  -> `img2pdf` image-only PDF. US Letter, 2550x3300 px, 300x300 ppi — the DPI is
  embedded in the PDF images (`pdfimages -list` shows x-ppi/y-ppi 300).
- degraded-200dpi derives from a genuine public-domain source: Abraham Lincoln's
  1860 Cooper Union address via Project Gutenberg — attribution and legal notes
  in `low-quality/source-document-abraham-lincoln-public-domain.txt`.

## WER Results

Receipt/report/degraded measured 2026-09-13; invoice/letter re-verified and form
re-measured 2026-09-18 after the single-page redesign, with tesseract 5.5.2 +
`scripts/measure-wer.sh` (gate: WER <= 3% on clean 300 DPI fixtures):

| Fixture | Words | WER | Gate | Notes |
|---------|-------|------|------|-------|
| receipt-300dpi | 143 | 0.00% | PASS (exit 0) | exact match |
| invoice-300dpi | 105 | 0.00% | PASS (exit 0) | exact match (re-verified 2026-09-18) |
| letter-300dpi | 227 | 0.00% | PASS (exit 0) | exact match (re-verified 2026-09-18) |
| form-300dpi | 166 | 0.60% | PASS (exit 0) | single page since 2026-09-18; 1 word substitution |
| report-300dpi | 1383 | 0.87% | PASS (exit 0) | 11 pages; residual code-block I/l and split-token noise |
| degraded-200dpi | 321 | 8.10% | n/a (degraded) | intentionally outside the gate |

To re-verify:

```sh
R=tests/fixtures/scanned
# whole corpus, live OCR, clean gate at <= 3% (tesseract 5.x + poppler from nix):
nix-shell -p tesseract poppler-utils python3 --run 'scripts/measure-wer.sh'
# single recorded pair (no OCR tools needed):
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
- **Purpose**: Test OCR quality on degraded 200 DPI scans. This fixture is
  expected to measure higher WER than the clean corpus (soft target < 10%,
  far above the clean 3% gate) while the recognized text remains usable for
  OCR regression coverage; it is manifest class `degraded` in
  `scripts/measure-wer.sh` and never contributes to the clean gate.
- **Ground Truth**: `low-quality/degraded-200dpi.txt` (canonical name;
  `low-quality/degraded-200dpi-ground-truth.txt` is a byte-identical
  duplicate kept for older references)
- **Target PDF**: `low-quality/degraded-200dpi.pdf`
- **Resolution**: 200×200 PPI — the page embeds one 1700×2200 px JPEG
  (/DCTDecode) on a 612×792 pt (8.5×11 in) US Letter page;
  1700/8.5 = 2200/11 = 200
- **Generation**: `python tools/create_degraded_200dpi.py`
- **Source**: Abraham Lincoln's 1860 Cooper Union address via Project
  Gutenberg, public domain in the USA — attribution and legal notes in
  `low-quality/source-document-abraham-lincoln-public-domain.txt`
- **Degradation Effects**: Gaussian blur (0.3px radius), noise (±12), reduced contrast (90%), reduced sharpness (85%), JPEG compression (85%)
- **WER Target**: < 10% (degraded fixtures have higher acceptable WER)
- **Status**: Generated, OCR output available (degraded-200dpi-ocr.txt;
  8.10% recorded, 9.97% live with tesseract 5.5.2 — re-verified 2026-09-20)

## Dependencies

- Python 3.8+: reportlab, Pillow, img2pdf
- poppler-utils (pdftoppm, pdfimages)
- A DejaVu Serif TTF (nix: `dejavu_fonts`)
- tesseract 5.x (for reference OCR outputs and the WER gate)

## Related Beads

- bf-2he4t: Initial corpus assembly
- bf-33zjo: Regeneration (width-aware generator, DejaVu Serif, OCR-stable
  GT curation), WER gate verification across all clean fixtures
- pdftract-d2201653: form-300dpi redesigned to a single page (invoice/letter/form
  trio uniformly single-page), source/license attribution recorded
