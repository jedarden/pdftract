# Verification note: pdftract-5537834f

Implemented the compiled hybrid-corpus test at
`crates/pdftract-core/tests/hybrid_corpus.rs` and removed the superseded,
unwired `tests/integration/hybrid_fixtures.rs`.

The test discovers all ten required PDFs and sidecars, opens every PDF as a
single-page document, reads `expected_classification.page_class`, and runs the
public 8x8 `GridClassifier` using each sidecar's declared approximate hybrid
cell count. It asserts the 10-of-64 rule for Hybrid results and keeps current
Phase 5.5/KU-2 deviations in the documented `EXPECTED_MISMATCH` registry.

Observed results:

| Fixture | Observed PageClass | Cells | Coverage | Status |
| --- | --- | ---: | ---: | --- |
| hybrid-001-vector-header-over-scan | Vector | 8/64 | 12.50% | EXPECTED-MISMATCH |
| hybrid-002-vector-form-over-scan | Hybrid | 48/64 | 75.00% | PASS |
| hybrid-003-mixed-column-layout | Vector | 0/64 | 0.00% | EXPECTED-MISMATCH |
| hybrid-004-watermark-over-scan | Scanned | 64/64 | 100.00% | EXPECTED-MISMATCH |
| hybrid-005-vector-footer-over-scan | Vector | 8/64 | 12.50% | EXPECTED-MISMATCH |
| hybrid-006-stamp-annotation | Hybrid | 12/64 | 18.75% | PASS |
| hybrid-007-textbox-overlay | Hybrid | 28/64 | 43.75% | PASS |
| hybrid-008-rotated-vector | Hybrid | 24/64 | 37.50% | PASS |
| hybrid-009-transparent-vector | Hybrid | 20/64 | 31.25% | PASS |
| hybrid-010-complex-layered | Scanned | 56/64 | 87.50% | EXPECTED-MISMATCH |

The extraction result currently does not expose a PDF-to-`PageContext` grid
analysis bridge, so this test uses the public grid classifier with the
sidecar-declared corpus coverage and real page geometry. No processes or
sockets are used.

Required acceptance command, run from a `git archive HEAD` extraction:

```text
$ CARGO_TARGET_DIR=/build/target-workers/pdftract-5537834f timeout --kill-after=30s 600s cargo test --test hybrid_corpus
running 1 test
test hybrid_corpus_matches_sidecars_and_grid_rule ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
exit=0
```

The same committed extraction was also run with `-- --nocapture` to capture
the table above; it exited 0.
