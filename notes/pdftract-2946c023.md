# pdftract-2946c023 — hybrid corpus validation

Date: 2026-09-24
Validated HEAD: `69df04ca292c5e27ef80a7f8d4484dafcf7b0f98`

## Observed results

The compiled child-1 harness passed for all ten primary fixtures. `verified`
means the observed `PageClass` was `Hybrid`; `failed` records a mismatch
against the sidecar's expected `Hybrid` classification and is a Phase 5.5
tuning input, not a test-run failure. The observed cell count is the 8x8
coverage value supplied to the harness from the sidecar; the harness does not
derive an independent PDF-to-cell count or expose a confidence value.

| Fixture | Expected | Observed PageClass | Observed hybrid cells | Percentage | File size (bytes) | Status |
|---|---|---|---:|---:|---:|---|
| hybrid-001-vector-header-over-scan.pdf | Hybrid | Vector | 8/64 | 12.50% | 1191 | failed |
| hybrid-002-vector-form-over-scan.pdf | Hybrid | Hybrid | 48/64 | 75.00% | 1507 | verified |
| hybrid-003-mixed-column-layout.pdf | Hybrid | Vector | 0/64 | 0.00% | 1647 | failed |
| hybrid-004-watermark-over-scan.pdf | Hybrid | Scanned | 64/64 | 100.00% | 1416 | failed |
| hybrid-005-vector-footer-over-scan.pdf | Hybrid | Vector | 8/64 | 12.50% | 1460 | failed |
| hybrid-006-stamp-annotation.pdf | Hybrid | Hybrid | 12/64 | 18.75% | 1461 | verified |
| hybrid-007-textbox-overlay.pdf | Hybrid | Hybrid | 28/64 | 43.75% | 1407 | verified |
| hybrid-008-rotated-vector.pdf | Hybrid | Hybrid | 24/64 | 37.50% | 1435 | verified |
| hybrid-009-transparent-vector.pdf | Hybrid | Hybrid | 20/64 | 31.25% | 1653 | verified |
| hybrid-010-complex-layered.pdf | Hybrid | Scanned | 56/64 | 87.50% | 2027 | failed |

The failed fixtures match the compiled test's `EXPECTED_MISMATCH` registry:

- 001, 003, and 005 have fewer than ten scanned-side cells, so the grid rule
  returns `Vector`.
- 004 and 010 have fewer than ten vector-side cells, so the grid rule returns
  `Scanned`.

The sidecars and manifest preserve their expected `Hybrid` declarations while
recording these outcomes as failed tuning baselines. No PDF was modified.

## Commands

All classifier verification was run from a fresh `git archive HEAD` extraction,
not from the shared dirty worktree:

```text
git rev-parse HEAD
date -u +%F
timeout --kill-after=30s 600s cargo test -p pdftract-core --test hybrid_corpus -- --nocapture
stat -c '%n %s' tests/fixtures/hybrid/hybrid-0??-*.pdf
```

The test command exited 0 and reported `1 passed; 0 failed`; its output table
is reproduced above. The exact tool string is recorded in every sidecar's
`validation` block.
