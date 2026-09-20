# pdftract-841093b2 — Verify cache metric counters and feature-gated compilation of the metrics wiring

Dispatch 1 (worker `claude-code-glm-5.3-glm-pdftract`, pluck, 2026-09-20).
Scope: parent `pdftract-6a0dbb53` acceptance criteria **2** (cache counters
move, direct registry assertion) and **4** (compiles with and without the
`metrics` feature). Base HEAD: `0c56c857`; this dispatch's commit:
`d4a15302`.

## What was found

`serve::tests::test_metrics_cache_hit_miss_and_size_counters` existed since
`d5fd6d8d` and was green at HEAD (the blocking defect — the
`ExtractionResult::diagnostics` serde asymmetry that turned every cache hit
into a "corrupt" miss — was fixed by `3686b539`, per
`notes/pdftract-b60ae586.md`). But it asserted **only rendered OpenMetrics
text** (`metrics.render()` + `contains`), not the Registry state, which is
what parent criterion 2 asks for ("direct registry assertion").

## Change (commit `d4a15302`, `crates/pdftract-cli/src/serve.rs` only)

- New test helper `snapshot_int(&[MetricFamily], &str) -> u64` reads a
  plain sample's integer value straight from `Registry::snapshot()`,
  panicking on absence, stray labels, or a non-integer sample.
- The cache test now asserts at the state level, in addition to the kept
  rendered-text assertions (which must agree):
  - after request 1 (miss): `pdftract_cache_misses_total == 1`,
    `pdftract_cache_hits_total == 0`, `pdftract_cache_size_bytes > 0`;
  - after request 2 (identical doc, hit): `pdftract_cache_hits_total == 1`,
    `pdftract_cache_misses_total` still `1`, and
    `pdftract_cache_size_bytes` **byte-identical** to the post-miss value —
    an invariant rendered-text matching could not express.

Mechanics note: the zero-valued `pdftract_cache_hits_total` sample exists
from process start because `CounterVec::new` pre-materializes the single
zero child of every unlabeled family (`registry.rs:52-59`), so the direct
`== 0` assertion is meaningful, not vacuous.

## Verification (clean `git archive HEAD` extraction, `/var/tmp/pdftract-841093b2-4ftD`, timeout-wrapped, exit codes as shown)

| Command | Result |
|---|---|
| `cargo test -p pdftract-cli --lib --features metrics test_metrics_cache_hit_miss_and_size_counters` | **1 passed; 0 failed** (398 filtered), exit 0 |
| `cargo check -p pdftract-cli --features metrics` | exit 0 |
| `cargo check -p pdftract-cli` | exit 0 |

All runs used the box's global target dir (`/build/target-workers`); judged
by cargo exit codes, not `target/` paths (global `CARGO_TARGET_DIR`
override). No test spawned a lingering process (in-process `oneshot`
routers only); extraction directory removed after the runs.

## Acceptance criteria

| # | Criterion | Result |
|---|---|---|
| 1 | Cache metrics test passes under the timeout wrapper, with direct registry assertions for hits, misses and size | **PASS** — snapshot-level asserts added and green, exit 0 |
| 2 | `cargo check -p pdftract-cli --features metrics` exits 0 | **PASS** — exit 0 |
| 3 | `cargo check -p pdftract-cli` exits 0 | **PASS** — exit 0 |
| 4 | Note written; bead-citing commit with explicit pathspecs pushed; closure contract satisfied | **PASS** — `d4a15302` (substantial, option 1) + this note; both pushed to `origin main` |

## Notes for sibling/parent beads

- Parent `pdftract-6a0dbb53` criteria 2 and 4 are now evidence-backed;
  remaining criteria were covered by `pdftract-b60ae586` (1 and 3).
- rustfmt default-config drift in `serve.rs` (4 hunks, e.g. lines 984,
  1900, 2057) is pre-existing at HEAD — none of it falls inside this
  dispatch's edited regions; left untouched.
