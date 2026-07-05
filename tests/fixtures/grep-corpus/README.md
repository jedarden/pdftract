# pdftract grep-corpus

Benchmark corpus for `pdftract-grep-1000` CI benchmark.

## Purpose

This corpus contains 1000 PDFs (~100 MB total) used to benchmark and validate the grep feature's performance and correctness.

## Structure

```
tests/fixtures/grep-corpus/
├── corpus/              # Actual PDF files
├── manifest.csv         # File metadata and expected match counts
├── regenerate.sh        # Script to rebuild the corpus
└── README.md            # This file
```

## Usage

### Running the benchmark

```bash
cargo bench --bench grep_1000
```

### Regenerating the corpus

```bash
cd tests/fixtures/grep-corpus
./regenerate.sh
```

## Corpus Requirements

The corpus must satisfy:
- **Size**: 1000 PDF files, ~100 MB total
- **Content**: Mix of vector and scanned PDFs
- **License**: Public domain or permissive (CC BY-SA, MIT, etc.)
- **Determinism**: Regenerable from source (no manual uploads)

## CI Gates

The benchmark enforces these gates on every PR:

1. **Throughput**: ≥ 50 MB/s on 4-core CI machine
2. **vs pdfgrep**: ≥ 2× faster
3. **vs pdftotext+ripgrep**: ≥ 3× faster
4. **Regression**: ≤ 10% vs historical main

## Status

TODO: Populate corpus (blocks on 7.8.1-7.8.9 grep implementation).

## Sources (TODO)

Potential corpus sources:
- arXiv API (public domain metadata)
- Wikipedia article exports (CC BY-SA)
- Synthetic PDFs via pdfjoin

## Manifest Format

```csv
filename,source_url,page_count,file_size,checksum,license
doc001.pdf,https://example.com/doc001.pdf,10,102400,abc123...,public-domain
doc002.pdf,https://arxiv.org/pdf/1234.5678.pdf,15,98304,def456...,cc-by-4.0
...
```

### Fields

- **filename**: Relative path from `corpus/` directory (e.g., `doc001.pdf`)
- **source_url**: URL where the PDF was downloaded from
- **page_count**: Number of pages in the PDF
- **file_size**: File size in bytes
- **checksum**: SHA256 hash of the file contents (for integrity verification)
- **license**: License identifier:
  - `public-domain`: Public domain
  - `cc-by-4.0`: Creative Commons Attribution 4.0
  - `cc-by-sa-4.0`: Creative Commons Attribution-ShareAlike 4.0
  - `mit`: MIT License
  - `other`: Other permissive license (document in source_url comments)
