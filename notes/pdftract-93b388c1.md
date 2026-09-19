# pdftract-93b388c1 — Metrics registry with OpenMetrics v1.0 text formatter

Bead: `pdftract-93b388c1` (split-child of parent `bf-9y8lb5`)
Date: 2026-09-19
HEAD verified: `075f2f90cba873d042109caf361dcf86c86615f2`
Implementation commit: `01fe7964cb8996044b2883af23dbeec5bf4fc3fe` (pushed to `origin/main`)

## What was done

Attempt 1 (2026-09-18, glm-5.3-flash) landed the complete implementation in
commit `01fe7964` but terminated with an agent-stream API error before
verification, so the bead could not be closed. This attempt re-derived the
full verification of that commit at HEAD in a clean extraction and confirmed
every acceptance criterion passes. No code changes were needed.

### Implementation surface (commit 01fe7964)

- `crates/pdftract-cli/src/metrics/openmetrics.rs` — hand-rolled OpenMetrics
  v1.0 text exposition: `CONTENT_TYPE`
  (`application/openmetrics-text; version=1.0.0; charset=utf-8`), `# HELP` /
  `# TYPE` metadata, counter family name without `_total` in metadata with
  `_total`-suffixed sample names (matches the OpenMetrics spec's own
  example), histogram cumulative `_bucket{le=...}` lines in ascending order
  plus mandatory `le="+Inf"` bucket, `_sum`, `_count`, label-value and HELP
  escaping (backslash, double quote, line feed), float formatting with
  mandatory decimal point / `NaN` / `+Inf` / `-Inf` tokens, `# EOF\n`
  termination. No new crates.io dependency.
- `crates/pdftract-cli/src/metrics/registry.rs` — cloneable thread-safe
  `Registry` (`Arc` + atomics / mutexes, `Send + Sync` asserted by test);
  all 13 plan metrics with exact plan names, types, labels, and HELP text;
  histogram buckets `EXTRACTION_DURATION_BUCKETS` =
  `[0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]` per the plan
  table; unlabeled counters pre-seed their zero child so the shipped
  `prometheus-rules.yaml` ratios have series from process start; saturating
  inflight gauge; NaN-clamped 0..1 rayon utilization; `build_info` from
  build.rs `GIT_SHA` / `COMPILED_FEATURES` (falls back to `"unknown"` when
  no git metadata, so clean extractions compile). 31 unit tests.
- `crates/pdftract-cli/src/metrics/mod.rs` — module root and re-exports.
- `crates/pdftract-cli/Cargo.toml` — new `metrics = []` feature;
  `serve = ["metrics"]` implication.
- `crates/pdftract-cli/build.rs` — `METRICS` added to `COMPILED_FEATURES`.
- `crates/pdftract-cli/src/lib.rs` — `#[cfg(feature = "metrics")] pub mod metrics;`

No HTTP listener, no CLI flag, no instrumentation of existing call sites —
those are the downstream split children, per the bead scope.

## Verification (clean extraction of HEAD, no .git, private target dir)

Extraction: `git archive HEAD | tar -x -C /var/tmp/verify-93b388c1/head`
(extraction of `075f2f90`; removal after close — nothing left on disk).

| Command | Exit | Outcome |
|---|---|---|
| `cargo check -p pdftract-cli --features metrics` | 0 | Finished dev profile, 51.89s |
| `cargo check -p pdftract-cli` | 0 | Finished dev profile, 16.55s (metrics module fully cfg'd out) |
| `cargo test -p pdftract-cli --features metrics --lib metrics::` | 0 | 31 passed, 0 failed, 361 filtered |
| `cargo test -p pdftract-cli --features metrics --doc metrics` | 0 | Registry doc example passes (1 passed, 11 filtered) |

Each `cargo test` invocation was wrapped in
`timeout --kill-after=30s 600s` per the repo test-hygiene rule; none was
killed.

## Acceptance criteria

1. **PASS** — Unit tests render every metric kind and assert exact lines:
   `render_labeled_counter_uses_family_name_without_total_in_metadata`
   (labeled counter with exact HELP/TYPE/label-order/# EOF),
   `render_histogram_emits_sorted_buckets_with_inf_sum_and_count` (buckets +
   +Inf + sum + count),
   `render_plain_gauge_omits_empty_label_set` (plain gauge),
   `build_info_renders_constant_one_with_build_labels` (build_info with
   version/git_sha/features at constant 1),
   `render_contains_all_thirteen_type_lines` (exactly 13 `# TYPE` lines),
   `content_type_is_openmetrics_v1`, plus escaping and float-format tests.
2. **PASS** — Registry handle is cloneable and thread-safe:
   `registry_handle_is_send_and_sync`, `clones_share_state`,
   `concurrent_increments_show_no_lost_updates` (8 threads × 2000
   increments across two label sets, asserts per-series totals AND that the
   series sum equals 16000 — no lost updates), and
   `concurrent_histogram_observations_are_all_counted` (4 × 1000
   observations, count/+Inf/le=0.5 all 4000).
3. **PASS** — `cargo check -p pdftract-cli --features metrics` exit 0;
   `cargo check -p pdftract-cli` exit 0; with the feature off the module
   does not compile at all (`#[cfg(feature = "metrics")]` gate on
   `pub mod metrics;` — nothing is emitted).
4. **PASS** — this note, committed with the bead ID in the message and
   pushed to `origin/main`.

## Notes / caveats

- `env!("GIT_SHA")` in `Registry::new()` is safe in clean no-git
  extractions because `build.rs` unconditionally emits `GIT_SHA` with an
  `"unknown"` fallback — verified by running the checks inside a
  `git archive` extraction.
- The 168 warnings in the check output are pre-existing crate-wide
  warnings, none from the metrics module files.
- The quarantine label on the bead
  (`quarantine-until:2026-09-18T21:06:47`) expired before this attempt;
  `failure-count:1` reflects attempt 1's API-error termination, not a
  verification failure.
