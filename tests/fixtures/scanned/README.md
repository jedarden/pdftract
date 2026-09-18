# Scanned PDF Fixtures for OCR Testing

This directory contains scanned PDF fixtures with ground-truth transcripts for
Word Error Rate (WER) testing. Detailed per-fixture specifications, the
OCR-stability curation rules, and generation history live in
[GEN_MANIFEST.md](GEN_MANIFEST.md).

## Purpose

These fixtures support:

- **AS-02 test scenario**: Extract a scanned receipt via OCR
- **Tier 1 OCR gate**: WER < 3% on clean 300-DPI scans (`scripts/measure-wer.sh`)
- **Performance testing**: 10+-page scanned PDF extraction in < 30 seconds

## Directory Structure

```
scanned/
├── README.md                     # This file
├── GEN_MANIFEST.md               # Specs, curation rules, WER results
├── receipt/                      # Single-page receipt
│   ├── receipt-300dpi-scanned.pdf   # Clean receipt at 300 DPI
│   ├── receipt-300dpi.pdf           # Text twin
│   ├── receipt-300dpi.txt           # Ground truth
│   └── receipt-300dpi-ocr.txt       # Reference OCR output
├── invoice/                      # Single-page service invoice
├── letter/                       # Single-page business letter
├── form/                         # Single-page employment application form
├── multi-page/                   # report-300dpi (11 pages, canonical)
│   └── doc-10page-300dpi*           # legacy 10-page fixture
├── low-quality/                  # degraded-200dpi (intentionally degraded)
└── documents/                    # legacy invoice/form copies (compatibility)
```

Each canonical fixture directory carries the scanned PDF, a ground-truth
`.txt`, and a reference OCR output `*-ocr.txt` produced by
`pdfimages -png` + `tesseract stdout -l eng`.

## Fixtures and WER Status

Receipt/report/degraded measured 2026-09-14; invoice/letter re-verified and form
re-measured 2026-09-18 after its redesign to a single page (fresh OCR, tesseract
5.5.2, via `scripts/measure-wer.sh`; gate is exit 0, WER ≤ 3%, on clean 300 DPI
fixtures):

| Fixture | Pages | Ground truth words | WER | Gate |
|---------|-------|--------------------|-----|------|
| receipt-300dpi | 1 | 143 | 0.00% | PASS |
| invoice-300dpi | 1 | 105 | 0.00% | PASS |
| letter-300dpi | 1 | 227 | 0.00% | PASS |
| form-300dpi | 1 | 166 | 0.60% | PASS |
| report-300dpi | 11 | 1383 | 0.87% | PASS |
| degraded-200dpi | 1 | 321 | 9.97% | n/a — degraded target < 10% |

## Source and License Attribution

- **receipt, invoice, letter, form, report** (all clean 300 DPI fixtures) are
  **original synthetic compositions** written for this repository — no third-party
  document was scanned or reproduced. The ground-truth texts (fictional businesses
  and people: Acme Supplies Inc., Smith Manufacturing, the letter's addressee
  Mr. Johnson, …) are
  rendered to PDF by `tools/generate_scanned_fixtures.py` and rasterized to an
  image-only page with `pdftoppm -r 300` + `img2pdf` (US Letter, 2550×3300 px,
  300×300 ppi) — that pipeline is the documented raster provenance of every
  "scan" here; the 300 DPI metadata is embedded in the PDF images themselves.
  **License:** the ground-truth texts and the generated PDFs are dedicated to the
  public domain (CC0 1.0); they may be used, modified, and redistributed without
  restriction.
- **degraded-200dpi** (`low-quality/`) is the one fixture with a genuine
  third-party source: Abraham Lincoln's 1860 Cooper Union address, accessed via
  Project Gutenberg and public domain in the USA (attribution and legal notes in
  [`low-quality/source-document-abraham-lincoln-public-domain.txt`](low-quality/source-document-abraham-lincoln-public-domain.txt)).

### Expected OCR role of each document

| Fixture | Expected OCR role |
|---------|-------------------|
| `receipt/receipt-300dpi` | Tier 1 clean scan — dense item/price lines, totals must survive OCR (AS-02 scenario) |
| `invoice/invoice-300dpi` | Tier 1 clean scan — columnar line items, currency amounts, payment terms |
| `letter/letter-300dpi` | Tier 1 clean scan — long prose lines, the hardest line-length case in the trio |
| `form/form-300dpi` | Tier 1 clean scan — single-page fill-field form: dotted fill lines, checkboxes, section headings |
| `multi-page/report-300dpi` | Multi-page throughput/perf fixture (10+ pages < 30 s), mixed content |
| `low-quality/degraded-200dpi` | Degraded-scan robustness; WER is expected to exceed the 3% clean gate |

## Verification

```bash
# Tesseract + poppler tools come from nix (the attribute is poppler-utils)
nix-shell -p tesseract poppler-utils python3 --run '
  R=tests/fixtures/scanned
  O=$(mktemp -d)
  pdfimages -png $R/letter/letter-300dpi.pdf $O/pg
  for i in $O/pg-*.png; do tesseract "$i" stdout -l eng >> $O/ocr.txt; done
  scripts/measure-wer.sh $O/ocr.txt $R/letter/letter-300dpi-ground-truth.txt
'
```

Exit 0 = WER ≤ 3% (quality gate passed); exit 1 = WER > 3%. `VERBOSE=1`
prints the substitution/insertion/deletion breakdown.

To regenerate the scanned PDFs and reference OCR outputs, see
[GEN_MANIFEST.md](GEN_MANIFEST.md) ("Regeneration") — the generator is
`tools/generate_scanned_fixtures.py` (width-aware, DejaVu Serif; do not use
ReportLab's Type1 core fonts — their poppler rasterization splits initial
capitals and alone pushes WER above 8%). The degraded fixture is built by
`tools/create_degraded_200dpi.py`.

## OCR-stability rules

Ground-truth texts are curated so tesseract round-trips them at 300 DPI
(standalone dash rules dropped, 10-dot dotted fill lines instead of
underscores, unspaced `[]` checkboxes, no sparse two-column lines, no ASCII
box tables). The full measured rationale is in
[GEN_MANIFEST.md](GEN_MANIFEST.md) — follow these rules when adding fixtures.

## Adding New Fixtures

1. Write the ground-truth `.txt` following the OCR-stability rules above
2. Generate the scanned PDF with `tools/generate_scanned_fixtures.py`
3. Produce the reference OCR output and check it passes `scripts/measure-wer.sh`
4. Add the fixture to this table and to `GEN_MANIFEST.md`

## Notes

- All fixtures use English with tesseract `eng` traineddata
- Fonts: DejaVu Serif (real TTF resolved via `$PDFTRACT_FIXTURE_FONT` or the
  system/nix dejavu path)
- The `ocr` cargo feature (in-Rust OCR pipeline used by
  `cargo test --test ocr_integration --features ocr`) is tracked separately:
  its compile status does not affect these fixtures; see bead
  `pdftract-ecad3b80` for the tesseract 0.15 API migration
