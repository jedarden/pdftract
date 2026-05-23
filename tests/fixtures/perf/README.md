# Performance Test Fixtures

This directory contains PDF files used for performance and memory ceiling testing.

## Memory Budgets

Per the plan (Phase 0 Quality Targets), the following memory budgets apply:

| Category | Target | Measurement |
|----------|--------|-------------|
| Peak RSS, 100-page vector PDF (buffered mode) | < 512 MB | RSS sampled at 10 ms intervals |
| Peak RSS, streaming/NDJSON mode (any page count) | < 256 MB, constant in page count | Must stay flat as page count grows |
| Peak RSS, adversarial fixtures | < 1 GB hard ceiling | Must not scale with payload size |

## Fixtures

### 100-page vector PDF
- `100-page-vector.pdf` - Synthetic 100-page PDF for buffered mode testing
- Target: < 512 MB peak RSS

### 10k-page stress test
- `10k-page.pdf` - Synthetic 10,000-page PDF for streaming mode validation
- Target: < 256 MB peak RSS in streaming mode
- Must remain constant regardless of page count

## Generating Fixtures

Fixtures can be generated using the `generate_stress_pdf.py` script in the tools directory.

```bash
python tools/generate_stress_pdf.py --pages 100 -o tests/fixtures/perf/100-page-vector.pdf
python tools/generate_stress_pdf.py --pages 10000 -o tests/fixtures/perf/10k-page.pdf
```
