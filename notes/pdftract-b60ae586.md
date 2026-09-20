# pdftract-b60ae586 — Verify serve and mcp end-to-end metrics tests and unchanged response behavior

Dispatch 2 (worker `claude-code-glm-5.3-glm-pdftract`, pluck, 2026-09-20).
Scope: parent `pdftract-6a0dbb53` acceptance criteria **1** (metrics label
surface driven end-to-end) and **3** (existing serve/mcp response behavior
unchanged, targeted tests green). Base HEAD: `af708517`.

## Outcome

The six `test_metrics_` tests existed since `d5fd6d8d` but had never had a
green run, and the reason turned out to be **three real product/test defects**
that no prior attempt reached (attempts 1–3 died before verifying; the
blocker bead's note `notes/pdftract-8e39532f.md` had already documented one
of the three as a WARN and parked it):

1. **Serve cache could never hit — `ExtractionResult::diagnostics` serde
   asymmetry** (`crates/pdftract-core/src/extract.rs:463`). The field had
   `skip_serializing_if = "Vec::is_empty"` **without** `serde(default)`: an
   empty `diagnostics` vec was omitted from the cached JSON, but
   `Deserialize` still required it. Every cache lookup therefore failed
   deserialization, was classified "corrupt", and re-counted a **miss** —
   exactly the always-`misses_total 2` / `size_bytes 932` signature the
   blocker bead's round-2 run recorded. The whole
   Writer→HMAC→zstd→Reader→`find_cached_entry` chain was verified sound by
   probe (below). Fix: add `default`, mirroring the `diagnostics_detailed`
   field two lines below (line 469) which already had the correct pair.
2. **Both production HTTP servers 500'd every request** —
   `crates/pdftract-cli/src/serve.rs:457` (serve mode) and
   `crates/pdftract-cli/src/mcp/http.rs:212` (mcp HTTP mode) handed a bare
   `Router` to `axum::serve`, which supplies no `ConnectInfo<SocketAddr>`;
   the audit/metrics middleware extracts it, so every request died with
   `Missing request extension … ConnectInfo<SocketAddr> … was not found`
   before reaching a handler (reproduced by hand: `curl /health` → 500).
   This is why the `mcp-http` integration suite failed **10/10** at HEAD
   ("Server did not start within 2 seconds" — the tests poll `/health`,
   which 500s). Fix: serve through
   `into_make_service_with_connect_info::<SocketAddr>()` at both sites.
3. **`serve::tests::test_concurrent_requests_parallel` could never pass**
   for two independent reasons: (a) it hand-built a two-route `Router`
   without production's middleware (nothing inserts the
   `Extension<RequestMetadata>` the extract handler requires → same class
   of 500), fixed by building the real `build_router(state, …)` and serving
   it through the same ConnectInfo make service as production; and (b) its
   `load_test_pdf()` read
   `crates/pdftract-libpdftract/tests/hello.pdf`, which was **deleted from
   the tree** by `4039def0` ("remove tracked debug/scratch artifacts") —
   repointed to the tracked `tests/fixtures/test-minimal.pdf`, the same
   fixture `fixture_pdf_bytes()` uses.

## Probe evidence (root-cause isolation)

A throwaway probe (run in a scratch extraction, not committed) drove
`extract_with_cache` twice against `tests/fixtures/test-minimal.pdf` and
inspected each layer:

- After fix 1: `reader ok, 813 bytes` (HMAC + decompress path sound),
  `serde OK, pages=1`, **`r2 status=hit`** — previously
  `serde FAIL: missing field diagnostics at line 1 column 714` and
  `r2 status=miss`.
- The entry file layout, `opts_hash`, filename size encoding
  (`parse_size_from_filename`), and HMAC verification all checked out
  correct — the deserialization step was the only broken link.

## Criterion-1 label surface — which test asserts which label (AC 2)

| Metric (parent AC 1) | Asserted by |
|---|---|
| `pdftract_extractions_total{result="success",ocr="false"}` | `serve.rs::test_metrics_extraction_success_through_serve_handler`; also stream + cache tests |
| `pdftract_extractions_total{result="error",ocr="false"}` (error path) | `serve.rs::test_metrics_extraction_error_records_error_result_and_422` |
| `pdftract_pages_extracted_total` | success, stream, and cache tests (1 / 1 / 2) |
| `pdftract_extraction_duration_seconds` observation (`_count 1` + `bucket{le="+Inf"}`) | success, error, and stream tests |
| `pdftract_mcp_requests_total{tool="hash"}` (tool label) | `mcp/http.rs::test_metrics_registry_counts_tool_call_and_http` |
| `pdftract_http_requests_total` endpoint/status labels | `{"/extract","200"}` success; `{"/extract","422"}` error; `{"/extract/stream","200"}` stream; `{"unmatched","404"}` `test_metrics_unmatched_path_uses_unmatched_label`; `{"/","200"}` mcp test |
| inflight gauge settles (`pdftract_inflight_extractions 0`) | success, error, stream tests |

All of parent criterion 1's required label assertions are present and green.

## Commands and results

All runs timeout-wrapped `timeout --kill-after=30s 600s`, in clean
`git archive HEAD` extractions with a private warm-seeded
`CARGO_TARGET_DIR` (shared caches untouched).

**Phase A — pristine HEAD `af708517` (ground truth):**

| Command | Result |
|---|---|
| `cargo test -p pdftract-cli --lib --features metrics test_metrics_` | 5 passed, 1 failed — `test_metrics_cache_hit_miss_and_size_counters` (misses 2, size 932) |
| `cargo test -p pdftract-cli --test mcp-http` | **0 passed, 10 failed** — server 500s every request |

**Phase B — with the three fixes applied (extraction of HEAD + patch):**

| Command | Result |
|---|---|
| `cargo test -p pdftract-cli --lib --features metrics test_metrics_` | **6 passed, 0 failed, exit 0** |
| `cargo test -p pdftract-cli --lib --features metrics -- test_413_json_format test_cache_status_conversions` | **2 passed, 0 failed, exit 0** |
| `cargo test -p pdftract-cli --test mcp-http` | **10 passed, 0 failed, exit 0** |
| `cargo test -p pdftract-core --test TH-10-cache-poison` | **10 passed, 0 failed, exit 0** (cache-poison regression guard) |
| `cargo test -p pdftract-cli --lib --features metrics serve::tests::` | **24 passed, 0 failed, exit 0** (whole serve tests module) |

Phase C (final, fresh extraction of the pushed fix commit) re-runs the four
fence commands — see the bead close reason's `verified:` block for the
recorded exit codes.

## Acceptance criteria

| # | Criterion | Result |
|---|---|---|
| 1 | All six metrics tests pass under the timeout wrapper | **PASS** — 6/6 exit 0 (was 5/6 + the known cache failure) |
| 2 | Criterion-1 label set confirmed present in the assertions | **PASS** — table above |
| 3 | Targeted pre-existing serve/mcp behavior tests pass | **PASS** — `--lib` 413/cache-status 2/2; `--test mcp-http` 10/10 (was 0/10); whole `serve::tests` module 24/24 |
| 4 | Note, bead-citing commit with explicit pathspecs, push, closure contract | **PASS** — this file + fix commits pushed; gate satisfied by the substantial commits (option 1) and a bead notes update (option 2) |

## Notes for sibling/parent beads

- **`pdftract-841093b2`** (verify cache metric counters): the defect that
  made the cache test fail is fixed here; re-derive at the fix commit —
  `test_metrics_cache_hit_miss_and_size_counters` is green.
- **`pdftract-3d0533a3`** (serve.rs E0308 in `extract_with_cache`): stale —
  HEAD compiles cleanly under `--features metrics` (all Phase B/C builds
  succeeded); re-derive at HEAD before doing anything.
- **Adjacent, not touched**: `crates/pdftract-core/src/table/grid.rs:28`
  has the same `skip_serializing_if` without `default` pattern on
  `GridCandidate::segments` (and `header_rows` skips via
  `is_zero_header_rows` without `default`). `GridCandidate` is not part of
  the cached `ExtractionResult` payload (pages carry `TableJson`), so it
  does not affect the cache — but any surface that deserializes
  `GridCandidate` JSON will hit the identical missing-field error whenever
  those fields are empty/zero. Left for the schema owners.
