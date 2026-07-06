# Baseline Metrics

This directory stores baseline benchmark metrics for pdftract performance validation and regression tracking.

## Purpose

Baseline metrics serve as the reference point for:
- **Performance regression detection** - Compare current benchmark results against historical baselines
- **Competitive analysis** - Track pdftract performance relative to pdfminer.six, pypdf, and pdfplumber
- **CI/CD gates** - Block releases that introduce performance regressions beyond acceptable thresholds
- **Trend analysis** - Monitor performance improvements over time

## File Format

Each baseline file is a JSON document conforming to `schema.json`. The schema defines the following structure:

### Required Fields

| Field | Type | Unit | Description |
|-------|------|------|-------------|
| `commit_sha` | string | - | Git commit SHA (or "main" for tracking branch) |
| `timestamp` | string | ISO 8601 | When the baseline was recorded |
| `pdftract_geomean` | number | seconds | Geometric mean extraction time across all fixtures (pdftract) |
| `grep_1000_mean_ms` | number | milliseconds | Mean time for 1000-PDF corpus search |

### Optional Fields

| Field | Type | Unit | Description |
|-------|------|------|-------------|
| `pdfminer_geomean` | number | seconds | Geometric mean extraction time (pdfminer.six) |
| `pypdf_geomean` | number | seconds | Geometric mean extraction time (pypdf) |
| `pdfplumber_geomean` | number | seconds | Geometric mean extraction time (pdfplumber) |
| `throughput_mb_per_sec` | number | MB/s | Aggregate throughput for grep-corpus benchmark |
| `files_per_sec` | number | files/second | Processing rate for grep-corpus benchmark |
| `total_runtime_sec` | number | seconds | Wall-clock time for complete benchmark suite |
| `corpus_size` | integer | count | Number of PDF files in test corpus |
| `notes` | string | - | Free-form contextual information |

## Naming Convention

Baseline files are named by their Git branch or tag:

- `main.json` - Baseline for the main development branch
- `v0.1.0.json` - Baseline for release tag v0.1.0
- `v0.2.0.json` - Baseline for release tag v0.2.0

## Schema Validation

All baseline files should validate against `schema.json`:

```bash
# Validate a baseline file (requires ajv-cli or similar)
npx ajv validate --strict=false -s schema.json -d main.json

# Or use Python jsonschema
python - <<'PYTHON'
import jsonschema, json
with open('schema.json') as s, with open('main.json') as d:
    jsonschema.validate(json.load(d), json.load(s))
PYTHON
```

## Usage in CI

The baseline metrics are used in CI to detect performance regressions:

1. **Baseline comparison**: Each benchmark run compares results against the appropriate baseline file
2. **Threshold checks**: Regressions exceeding 10% for primary metrics block the PR
3. **Competitive ratios**: pdftract must maintain ≥ 10× speedup vs pdfminer.six and ≥ 5× vs pypdf

## Performance Targets

Based on the Primary Objectives in the project plan:

| Metric | Target | Measurement |
|--------|--------|-------------|
| 100-page vector PDF, 4-core | < 3 seconds | `cargo bench`, `tests/fixtures/perf/` |
| 10-page scanned PDF (OCR) | < 30 seconds | includes Tesseract |
| Single-page extraction latency | < 150 ms p99 | wrk benchmark |
| Throughput vs pdfminer.six | ≥ 10× faster | Identical hardware |
| Throughput vs pypdf | ≥ 5× faster | Same benchmark suite |
| `pdftract grep` throughput | ≥ 50 MB/s | 1000-PDF corpus, 4-core |

## Updating Baselines

When to update a baseline:

1. **After a major release** - Create a new baseline file tagged with the release version
2. **After accepted performance improvements** - Update `main.json` when improvements merge
3. **Never for regressions** - Regressions should block release, not update baselines

Update process:

```bash
# Run benchmarks to generate new baseline
cargo bench --bench grep_corpus | tee /tmp/bench-results.txt

# Extract metrics and create/update baseline file
# (This step requires a helper script to parse benchmark output)

# Validate against schema
npx ajv validate --strict=false -s schema.json -d main.json

# Commit the updated baseline
git add benches/baselines/main.json
git commit -m "bench(bf-XXX): update main baseline after performance improvements"
```

## Example Baseline

```json
{
  "commit_sha": "abc1234",
  "timestamp": "2024-07-06T10:30:45Z",
  "pdftract_geomean": 2.5,
  "pdfminer_geomean": 28.0,
  "pypdf_geomean": 15.0,
  "pdfplumber_geomean": 32.0,
  "grep_1000_mean_ms": 18.5,
  "throughput_mb_per_sec": 87.3,
  "files_per_sec": 920.0,
  "total_runtime_sec": 1.09,
  "corpus_size": 1000,
  "notes": "Baseline for v0.2.0 release - OCR improvements and grep optimization"
}
```

## Regression Detection

The benchmark harness compares current results against baselines and flags regressions:

- **PASS**: All metrics within ±5% of baseline (acceptable variance)
- **WARN**: Metrics degraded 5–10% (logged, non-blocking)
- **FAIL**: Metrics degraded >10% (blocks PR merge)
- **IMPROVEMENT**: Metrics improved >5% (logged, consider baseline update)

## Historical Context

Baseline files form a historical record of pdftract's performance evolution. The `main.json` baseline represents the current state of the main branch, while tagged baselines (e.g., `v0.1.0.json`) capture performance at specific release points.

This history enables:
- Long-term performance trend analysis
- Release-to-release comparison
- Identification of performance bottlenecks
- Validation of optimization efforts
