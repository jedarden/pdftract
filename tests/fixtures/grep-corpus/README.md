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

The corpus can be regenerated using the Makefile targets:

```bash
# Generate/regenerate the corpus (default: 1000 PDFs)
make download-grep-corpus

# Generate a specific count
make download-grep-corpus COUNT=500

# Validate corpus integrity
make validate-corpus
```

The `download-grep-corpus` target generates synthetic PDFs using ReportLab, which ensures:
- **Determinism**: Same script produces identical corpus
- **Known license**: All synthetic PDFs are public domain
- **Controlled variety**: Mix of page counts (1-20 pages per PDF)
- **Fast regeneration**: No network dependencies after initial setup

The `validate-corpus` target verifies corpus integrity by checking:
- All files listed in manifest.csv exist
- File sizes match the manifest
- SHA256 checksums are correct
- License information is present
- Total counts (files, pages, size) are accurate

### Manual regeneration (advanced)

For advanced usage, you can also run the scripts directly:

```bash
# Generate corpus
bash scripts/download-grep-corpus.sh 1000

# Regenerate manifest from existing corpus
bash scripts/grep-corpus-generate-manifest.sh

# Validate corpus
bash scripts/validate-corpus.sh tests/fixtures/grep-corpus
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
