# pdftract-2e251ccd

## Before

- `tests/fixtures/hybrid/README.md` documented intended fixture patterns but
  did not record the observed PageClass, hybrid-cell coverage, or status from
  the validated primary corpus, and it did not identify the per-fixture
  Phase 5.5 routing decisions.
- `tests/fixtures/hybrid/GEN_MANIFEST.md` described the ten legacy
  subdirectory PDFs as pending reportlab-dependent placeholders without a
  consumer-facing supersession marker.

## After

- `README.md` now records the ten observed PageClass/cell-coverage/status
  results, maps every primary fixture to its Phase 5.5 tuning input, and
  distinguishes the validated root-level corpus from the legacy subdirectories.
- `GEN_MANIFEST.md` keeps every legacy fixture's `verification_status: pending`
  and adds `corpus_status: superseded-by-primary`, directing Phase 5.5 to the
  validated root-level `hybrid-001` through `hybrid-010` corpus.

The README table is consistent with all ten sidecars' `validation` fields:
observations were taken at head
`69df04ca292c5e27ef80a7f8d4484dafcf7b0f98` on 2026-09-24. No sidecar values
were changed.
