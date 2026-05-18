# Competitive Benchmark Corpus

This directory contains the PDF corpus used for competitive benchmarking against pdfminer.six, pypdf, and pdfplumber.

## Structure

```
corpus/
├── vector/           # 25 vector PDFs (text-based)
├── raster/           # 25 raster PDFs (OCR-required, image-based)
└── README.md         # This file
```

## Corpus Composition

The corpus consists of 50 representative PDF documents:

- **Vector PDFs (25)**: Synthetic test documents from the classifier corpus (misc category). These are pure text-based PDFs that test text extraction performance without OCR.
- **Raster PDFs (25)**: Synthetic test documents from the classifier corpus (invoice category). These test performance on documents that would require OCR for full text extraction.

## Usage

The corpus is used by the CI `bench-matrix` step to run competitive benchmarks:

```bash
hyperfine --warmup 2 --runs 5 --export-json result.json \
  "./run-pdftract.sh corpus/vector/misc-01.pdf"
```

## Baseline

The baseline performance is stored in `benches/baselines/main.json`. Any PR that causes a regression > 10% on the geomean across the corpus will be blocked.

## 10x-Faster Gate

Per the Phase 0 quality targets, pdftract must be >= 10x faster than pdfminer.six on vector PDFs. This gate is enforced in CI as:

```
pdftract_geomean / pdfminer_geomean <= 0.1
```

## Corpus Maintenance

- The corpus is checked into the repo for reproducibility
- Total size: ~100 KB (synthetic test data)
- All documents are licensed under MIT-0 (no attribution required)
- To update the corpus: modify files, then run `bf batch` to refresh the baseline

## Notes

- This is a placeholder corpus for Phase 0 CI infrastructure
- The full 500-PDF regression corpus will be assembled in Phase 0.5
- Vector vs raster classification is approximate; true classification requires runtime analysis
