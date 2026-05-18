# Competitive Benchmarks

This directory contains the competitive benchmark infrastructure for pdftract, comparing its performance against three popular Python PDF libraries: pdfminer.six, pypdf, and pdfplumber.

## Purpose

Speed is one of pdftract's three differentiators (per the Mission statement). These benchmarks ensure that:
1. pdftract maintains at least 10x speed advantage over pdfminer.six on vector PDFs
2. Performance regressions are caught in CI before merge
3. Competitive positioning is tracked over time

## Corpus

The benchmark corpus consists of 50 representative PDFs:
- **25 vector PDFs** (`corpus/vector/`) - Text-based PDFs where pdftract should excel
- **25 raster PDFs** (`corpus/raster/`) - Scanned documents requiring OCR

All documents are committed to the repository at ~10 MB total size.

## Tools

All competitor versions are pinned in `requirements.txt` to ensure baseline stability:
- `pdfminer.six==20231228`
- `pypdf==4.2.0`
- `pdfplumber==0.11.0`

Updates to these versions require a deliberate PR with manual baseline refresh.

## Running Benchmarks Locally

### Prerequisites

```bash
# Install hyperfine
apt-get install hyperfine

# Install competitor tools
pip install -r requirements.txt

# Ensure pdftract is in PATH
which pdftract
```

### Quick Run

```bash
cd benches/competitors
./run-benchmarks.sh
```

### Custom Baseline

```bash
BASELINE=/path/to/baseline.json OUTPUT=results.json ./run-benchmarks.sh
```

## CI Integration

The `bench-matrix` step in `.ci/argo-workflows/pdftract-ci.yaml` runs these benchmarks on every PR:
1. Installs hyperfine and competitor tools
2. Downloads the pdftract binary artifact from build-matrix
3. Runs the full benchmark suite
4. Checks regression and 10x-faster gates
5. Publishes `benchmark-results.json` as an artifact
6. Posts a formatted summary as a PR comment

## Gates

### Regression Gate

Compares pdftract's geometric mean time against the baseline (`benches/baselines/main.json`):
- **Threshold:** 10% regression
- **Baseline source:** `git show main:benches/baselines/main.json`
- **Failure:** PR is blocked if regression > 10%

### 10x-Faster Gate

Ensures pdftract maintains its speed advantage:
- **Threshold:** `pdftract_geomean / pdfminer_geomean <= 0.1`
- **Scope:** Vector PDFs only (where pdftract should excel)
- **Failure:** PR is blocked if ratio > 0.1 (less than 10x faster)

### Special Benchmark: pdftract-grep-1000

Runs `pdftract grep "the" wikipedia-1000.pdf` 5 times with warmup:
- Tests search performance on a 1000-page document
- Regression > 10% blocks the PR
- Independent of the main corpus benchmarks

## Output Schema

`benchmark-results.json` contains an array of objects:

```json
[
  {
    "tool": "pdftract",
    "doc": "misc-01.pdf",
    "mean_ms": 8.5,
    "stddev_ms": 0.3,
    "min_ms": 8.1,
    "max_ms": 9.2,
    "crash": false
  },
  {
    "tool": "pdfminer",
    "doc": "encrypted.pdf",
    "crash": true
  }
]
```

Crashes are excluded from geometric mean calculations but are recorded for visibility.

## Baseline Schema

`benches/baselines/main.json` stores the commit-sha-specific baseline:

```json
{
  "commit_sha": "abc123...",
  "timestamp": "2024-01-01T00:00:00Z",
  "pdftract_geomean": 10.0,
  "pdfminer_geomean": 100.0,
  "pypdf_geomean": 120.0,
  "pdfplumber_geomean": 150.0,
  "corpus_size": 50,
  "notes": "Baseline from main branch"
}
```

## Noise Reduction

Benchmark variance on Spot infrastructure can be high. The following strategies reduce noise:
1. **Hyperfine warmup:** 2 warmup runs discarded before timing
2. **Multiple runs:** 5 timed runs per (tool, document) pair
3. **Geometric mean:** Computed across all documents for each tool
4. **95% CI:** Reported in PR comments to show variance

## Updating Baselines

When merging to main, the baseline can be refreshed:

1. Run benchmarks locally or extract from CI artifacts
2. Update `benches/baselines/main.json` with new geomeans
3. Commit and push to main

Do NOT update baselines for PR branches - they should always compare against main.

## Troubleshooting

### Hyperfine not found

```bash
apt-get install hyperfine
```

### Python tools not found

```bash
pip install -r benches/competitors/requirements.txt
```

### Pdftract not found

Ensure the binary is built and in PATH, or use the CI artifact download.

### High variance

- Ensure CPU is not throttled (`cpufreq-info`)
- Check for background processes consuming CPU
- Run with more iterations (modify `--runs 5` in script)

## References

- Plan section: Phase 0, line 1007 (Tier 4 benchmarks)
- Quality Targets, Tier 4 (competitive bench hard gate)
- Mission (speed differentiator)
