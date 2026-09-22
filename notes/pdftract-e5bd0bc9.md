# pdftract-e5bd0bc9 — /metrics + /ready end-to-end verification, prometheus-rules.yaml reconciliation

- **Date:** 2026-09-22 (attempt 2; attempt 1 died mid-run with an API error after landing the YAML fix)
- **HEAD verified:** `1e826551` (== `origin/main`, verified `git rev-list origin/main..HEAD` empty)
- **Method:** all commands run in a clean `git archive HEAD` extraction at
  `/var/tmp/pdftract-e5bd-head` (no `.git`), binary built with
  `CARGO_TARGET_DIR=/var/tmp/pdftract-e5bd-target cargo build -p pdftract-cli
  --features serve --bin pdftract` → exit 0. The shared working tree was not
  used for any verification (it carries unrelated in-flight edits).

## Criterion 1 — every metric name/label in docs/operations/prometheus-rules.yaml is emitted by the formatter: PASS

The YAML fix itself landed in attempt 1, commit `198901411f1a` (on `origin/main`):
it removed the two rules referencing never-emitted names
(`process_resident_memory_bytes`, `pdftract_max_decompress_bytes`) with a
comment explaining why, and repaired the seven runbook anchors to the stable
heading anchor `#monitoring-and-alerting` (heading verified present at
`docs/plan/plan.md:3931`). This dispatch re-verified the result at HEAD.

**Static check — names referenced by live (non-comment) YAML lines:**

```
$ grep -vE '^\s*#' docs/operations/prometheus-rules.yaml | grep -oE 'pdftract_[a-z_]+' | sort | uniq -c
      2 pdftract_cache_hits_total
      1 pdftract_cache_misses_total
      1 pdftract_cache_size_bytes
      1 pdftract_diagnostic_emitted_total
      1 pdftract_extraction_duration_seconds_bucket
      2 pdftract_http_requests_total
      1 pdftract_rayon_pool_utilization
      1 pdftract_serve            <- rule-group name, not a metric
```

(The only other `pdftract_*` strings in the file are inside the explanatory
`#` comment block at line 70 documenting the removed memory rule.)

All 7 live metric references match the shipped surface:

| YAML reference | Emitted by | Verified in live scrape |
|---|---|---|
| `pdftract_extraction_duration_seconds_bucket` | histogram buckets, `registry.rs:192` (bounds `0.05,0.1,0.25,0.5,1.0,2.5,5.0,10.0,30.0,60.0` + `+Inf`) | 11 bucket lines with `le="…"` |
| `pdftract_http_requests_total` (label `status`) | counter, labels `endpoint,status` (`registry.rs:330-334`) | `pdftract_http_requests_total{endpoint="/health",status="200"} 2` |
| `pdftract_cache_hits_total` | counter, unlabeled, zero child from process start (`registry.rs:53-59`) | `pdftract_cache_hits_total 0` |
| `pdftract_cache_misses_total` | counter, unlabeled | `pdftract_cache_misses_total 0` |
| `pdftract_rayon_pool_utilization` | gauge | `pdftract_rayon_pool_utilization 0.0` |
| `pdftract_cache_size_bytes` | gauge | `pdftract_cache_size_bytes 0` |
| `pdftract_diagnostic_emitted_total` (label `severity`) | counter, labels `code,severity` (`registry.rs:340-344`); no diagnostics fired in this run so the family exposed metadata only — rendering proven by unit test `metrics::registry::tests::diagnostic_counter_renders_code_and_severity` (passes) | `# HELP/# TYPE pdftract_diagnostic_emitted` lines present |

**Labels:** `status` and `severity` are the only label filters in the YAML and
both are declared on their metrics; `le` comes from the histogram itself.
`instance` in the `PdftractExtractionLatencyHigh` aggregation is a
Prometheus-added target label (scrape time), not an exposition obligation —
correct as written.

**Units/suffixes:** counters render `_total` samples (`registry.rs:104`); the
histogram renders `_bucket`/`_sum`/`_count` (`registry.rs:192-208`) with
`_seconds`; the cache gauge is `_bytes` — all match the YAML and the plan's
13-metric table (`plan.md:3949-3965`).

**Dynamic check:** each of the 7 names was grepped against the actual
`/metrics` response body from the e2e run below — 7/7 PRESENT (script section
1d). The scrape carries exactly 13 `# TYPE` lines and terminates `# EOF`.

**Observed, no change (thresholds, not names):** the plan's "Suggested alert
thresholds" table and the YAML differ in tunable values (p99 > 5 s vs the
YAML's > 0.3 s; hit rate < 0.30/1h vs < 0.5/10m; pool > 0.95 vs > 0.8;
severity `page` vs `critical`). The plan explicitly frames that table as
"suggested" and "operator-tunable" and the YAML as the shipped *sample* rules,
so this is within the plan's contract; criterion 1 is about names/labels/
units, which match exactly.

## Criterion 2 — end-to-end manual run: PASS

Script: `/var/tmp/pdftract-e5bd-e2e/run.sh` (phase 1) and `run2.sh` (phase 2
redo). Binary: clean-HEAD build, `--features serve` (implies `metrics`).

### Phase 1 — healthy server (writable `--cache-dir`, `--metrics 0` = OS-chosen port from the stderr banner)

```
$ pdftract serve --bind 127.0.0.1:40513 --metrics 0 --cache-dir …/cache
Metrics endpoint: http://127.0.0.1:35063        <- stderr banner, ephemeral port

$ curl /ready      -> 200 {"cache_writable":true,"pool_utilization":0.0,"status":"ready","unready":[]}
$ curl /health     -> 200 {"status":"ok","version":"0.1.0"}          (main port)
$ curl /metrics    -> 200, content-type: application/openmetrics-text; version=1.0.0; charset=utf-8
                      2582 bytes, 13 "# TYPE" lines, last line "# EOF"
```

Structural parse of the body (python, every non-`#` line matched against
`name{labels} value`):

```
PARSE OK: 22 sample lines, 13 TYPE lines, ends with # EOF
```

Real extraction driven through the server, then re-scrape — counters move:

```
$ curl -F "pdf=@tests/fixtures/valid-minimal.pdf;type=application/pdf" http://127.0.0.1:40513/extract
extract HTTP_STATUS=200   {"… "cache_status":"miss","page_count":1 …}
pdftract_extractions_total{result="success",ocr="false"} 1
pdftract_pages_extracted_total 1
pdftract_http_requests_total{endpoint="/extract",status="200"} 1
pdftract_http_requests_total{endpoint="/health",status="200"} 2
pdftract_extraction_duration_seconds_sum 0.021728124
pdftract_extraction_duration_seconds_count 1
```

Route policy on the main port (plan: `/metrics` and `/ready` only on the
metrics listener): `GET /metrics` → 404, `GET /ready` → 404.

### Phase 2 — real unwritable cache path → 503, /health still 200

First attempt used a *nonexistent* cache dir: **serve creates the missing dir
at startup**, the probe then succeeds and `/ready` stays 200 — recorded here
because it is the wrong shape for an unwritable-cache test. The real case is
an existing directory that cannot be written:

```
$ mkdir …/cache-readonly && chmod 555 …/cache-readonly      (uid 1000, not root)
$ touch …/cache-readonly/.probe -> Permission denied (setup confirmed)
$ pdftract serve --bind 127.0.0.1:35381 --metrics 0 --cache-dir …/cache-readonly

$ curl /ready  -> 503 {"cache_writable":false,"pool_utilization":0.0,
                       "status":"unready","unready":["cache_unwritable"]}
$ curl /health (main port) -> 200 {"status":"ok","version":"0.1.0"}
$ ls …/cache-readonly -> empty (the failed probe left nothing behind)
```

`pool_saturated` was **not** forced with real extraction load in the manual
run (driving the sampled gauge > 0.90 from a shell is timing-fragile); the
criterion's injectable-provider route covers it, all passing:

- `metrics::endpoint::tests::ready_is_503_naming_the_pool_when_utilization_exceeds_the_threshold`
- `metrics::endpoint::tests::ready_is_200_at_exactly_the_threshold` (0.90 itself stays ready)
- `metrics::endpoint::tests::ready_names_every_failed_condition_when_both_fail` (both conditions at once)
- `serve::tests::test_health_returns_200_when_pool_is_saturated` (liveness unaffected)

Process hygiene note: a server backgrounded from a non-interactive shell
inherits `SIG_IGN` for SIGINT, so `kill -INT` does not stop it (observed in
phase 1, fallback SIGKILL, exit 137); `kill` (SIGTERM) terminates it cleanly
(phase 2, exit 143).

## Criterion 3 — targeted tests at HEAD, timeout-wrapped: PASS

All in the clean extraction (`cd /var/tmp/pdftract-e5bd-head`,
`CARGO_TARGET_DIR=/var/tmp/pdftract-e5bd-target`), each wrapped in
`timeout --kill-after=30s 900s`, one filter per invocation:

| Command (exact) | Result |
|---|---|
| `cargo test -p pdftract-cli --features serve --test metrics-listener -- --test-threads=2` | 5 passed; 0 failed — exit 0 |
| `cargo test -p pdftract-cli --features serve --lib -- metrics::` | 45 passed; 0 failed — exit 0 |
| `cargo test -p pdftract-cli --features serve --lib -- ready` | 8 passed; 0 failed — exit 0 |
| `cargo test -p pdftract-cli --features serve --lib -- health` | 3 passed; 0 failed — exit 0 |

Integration tests (`--test metrics-listener`, real binary on the wire):
`serve_metrics_listener_serves_openmetrics_on_a_second_port`,
`mcp_metrics_listener_serves_openmetrics_on_a_second_port`,
`without_the_flag_there_is_no_metrics_listener_and_no_metrics_route`,
`an_already_bound_metrics_port_fails_startup_cleanly`,
`metrics_with_stdio_transport_is_rejected_as_a_usage_error`.

Note the `--features serve` flag is load-bearing: `metrics-listener.rs` is
`#![cfg(feature = "metrics")]`, so a run without the feature "passes"
vacuously with 0 tests.

## Criterion 4 — no orphaned processes: PASS

```
$ pgrep -af 'pdftract-e5bd[-]target/debug/pdftract'
exit=1   (no processes from this dispatch's e2e binary)
$ pgrep -af 'pdftract[ ]serve'
exit=1   (no serve processes at all)
```

(A broader first pattern also matched this dispatch's own harness process and
the pgrep shell itself, plus *other* workers' live builds in
`/build/target-workers` and `/var/tmp/pdftract-ddac33c2` — classified by argv
and excluded; none are this run's children.)

## Criterion 5 — gate-child statement: the plan's Prometheus surface is DONE

Plan section "Monitoring and Alerting" (`plan.md:3931+`) required: the
`metrics` feature registry with the 13-metric surface (closed:
`pdftract-93b388c1`), call-site instrumentation (closed: `pdftract-6a0dbb53`),
the `--metrics PORT` second listener on serve and mcp (closed:
`pdftract-c20c44c1`, wire-verified here on serve; mcp path covered by the
integration test above), `GET /ready` with 503 on pool saturation /
unwritable cache (closed: `pdftract-1a870b65`, verified live here), and
sample rules in `docs/operations/prometheus-rules.yaml` (reconciled in
`198901411f1a`, re-verified at HEAD here).

**Nothing remains of the plan-section surface.** All four child beads are
closed and this dispatch's live run confirms the whole chain works end to end
from the `1e826551` tree: `serve --metrics 0` serves parseable OpenMetrics
with the documented names, `/ready` flips 200↔503 on both failure conditions
(pool via the injectable seam, cache via a real mode-555 directory), `/health`
never leaves 200, and the shipped alert rules reference only names the
formatter actually emits. Parent umbrella `bf-9y8lb5` is clear to close once
this bead closes.

Environment footnote: `pdftract_build_info` rendered
`git_sha=""` here because the verification build came from a `git archive`
tarball with no git metadata; the label itself is present and populated by the
build script in real builds.
