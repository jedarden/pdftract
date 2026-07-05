# Competitive Benchmark Results

This directory stores per-commit benchmark results tracking pdftract performance against competitor PDF extraction libraries.

## Output Format

Each commit produces a JSON file named `<commit-sha>.json` with the following schema:

```json
{
  "commit": "abc123...",
  "date": "2026-07-05T12:00:00Z",
  "pdftract_mbs": 125.5,
  "pdfminer_mbs": 12.3,
  "pypdf_mbs": 25.7,
  "pdfplumber_mbs": 11.8,
  "ratio_pdfminer": 10.2,
  "ratio_pypdf": 4.9
}
```

## Metrics

- `*_mbs`: Median throughput in megabytes per second (MB/s) for each tool
- `ratio_pdfminer`: `pdftract_mbs / pdfminer_mbs` (target: ≥ 10.0)
- `ratio_pypdf`: `pdftract_mbs / pypdf_mbs` (target: ≥ 5.0)

## Acceptance Criteria

Per the plan (Tier 4 competitive benchmarks), pdftract must achieve:
- **≥ 10× faster** than pdfminer.six on vector PDFs (`ratio_pdfminer ≥ 10.0`)
- **≥ 5× faster** than pypdf on vector PDFs (`ratio_pypdf ≥ 5.0`)

## Usage

```bash
# Run all competitive benchmarks
python3 benches/competitors/run_all.py

# Results are written to benches/results/<commit-sha>.json
cat benches/results/$(git rev-parse HEAD).json | jq '.'
```

## CI Integration

The benchmark runs as a Tier 4 step in `pdftract-ci` Argo WorkflowTemplate, executing only on the `main` branch to track performance over time.
