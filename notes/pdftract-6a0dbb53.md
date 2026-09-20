# pdftract-6a0dbb53 — Consolidated metrics-wiring evidence: all 13 metrics audited against live call sites

Audit dispatch (child `pdftract-a2eeb7ee`, worker `claude-code-glm-5.3-glm-pdftract`,
pluck, 2026-09-20). Scope: reconcile every one of the 13 metrics against a live
call site on a real request path, close remaining gaps, and consolidate the
evidence trail the parent umbrella needs to close. Base HEAD: `41960916`;
this dispatch's commit: `9f035d42`.

## 1. Audit table — registry method → live call sites (outside `crates/pdftract-cli/src/metrics/`)

Grep surface: all 13 `Registry` methods, `*.rs` under `crates/`, excluding the
`metrics` module itself. Line numbers at HEAD `9f035d42`.

| # | Metric (`pdftract_`) | Registry method | Live call site(s) | Status |
|---|---|---|---|---|
| 1 | `extractions_total` | `inc_extraction` | `serve.rs:602` (`record_extraction`, POST /extract + /extract/text), `serve.rs:991` / `serve.rs:994` (POST /extract/stream success/error) | wired |
| 2 | `extraction_duration_seconds` | `observe_extraction_duration` | `serve.rs:601` (`record_extraction`), `serve.rs:988` (stream) | wired |
| 3 | `pages_extracted_total` | `add_pages_extracted` | `serve.rs:603` (`record_extraction`), `serve.rs:992` (stream) | wired |
| 4 | `cache_hits_total` | `inc_cache_hit` | `serve.rs:605` (`record_extraction`, cache status `hit`) | wired |
| 5 | `cache_misses_total` | `inc_cache_miss` | `serve.rs:606` (`record_extraction`, cache status `miss`) | wired |
| 6 | `cache_size_bytes` | `set_cache_size_bytes` | `serve.rs:693` (POST /extract, post-extraction index sample), `serve.rs:826` (POST /extract/text) | wired |
| 7 | `mcp_requests_total` | `inc_mcp_request` | `mcp/http.rs:385` (`handle_post_request` tools/call loop, by requested tool name) | wired |
| 8 | `http_requests_total` | `inc_http_request` | `serve.rs:585` (serve router middleware), `mcp/http.rs:284` (mcp router middleware) | wired |
| 9 | `remote_bytes_downloaded_total` | `add_remote_bytes_downloaded` | `remote_metrics.rs:25` (hook factory) consumed at `main.rs:1305` (URL extract path), `hash.rs:146` (`compute_fingerprint_from_url`), `grep/worker.rs:113` (remote grep) | wired |
| 10 | `diagnostic_emitted_total` | `inc_diagnostic` | `serve.rs:610` (`record_extraction`) **and `mcp/http.rs:331` (new, this dispatch — see §2)** | wired |
| 11 | `inflight_extractions` | `inc_inflight_extractions` / `dec_inflight_extractions` | `serve.rs:672` / `720` (POST /extract), `serve.rs:806` / `851` (POST /extract/text), `serve.rs:945` / `986` (stream) | wired |
| 12 | `rayon_pool_utilization` | `set_rayon_pool_utilization` | `serve.rs:428` (serve sampler task), `mcp/http.rs:187` (mcp sampler task), both fed by `metrics/sampler.rs:38` `sample_utilization()` | wired |
| 13 | `build_info` | `with_build_info` | called inside the metrics module by `Registry::new()` (`registry.rs:293-299`), which is the constructor both servers use: `ServeState::new` (`serve.rs`, registry created at startup) and `McpServerState::new` (`mcp/http.rs:107`) | wired by design (see §3) |

All 13 metrics have live call sites. One gap was found and closed (metric 10,
mcp transport); everything else was already live from the chain commits below.

## 2. Gap found and closed: `inc_diagnostic` on the mcp tools/call path

**Finding.** Diagnostics were counted only on the serve HTTP transport
(`serve.rs:610`). mcp-served extractions do **not** flow through the serve
recorder: `handle_request` (`mcp/http.rs`) dispatches to `Tool::execute`
(`mcp/tools/registry.rs`), which calls `extract_pdf` directly and has no
registry access — so every diagnostic an mcp extraction emitted was invisible
to `pdftract_diagnostic_emitted_total`.

**Wiring (observation-only).** `handle_post_request` now reads the response a
tool just produced (`mcp/http.rs:390` → `record_response_diagnostics`,
`mcp/http.rs:315`) and feeds each entry of the result's
`metadata.diagnostics_detailed` array to `inc_diagnostic` with the same
code/severity labels the serve path uses. Guarantees, per the parent's
constraints: no endpoint, no CLI flag, no listener; error responses, results
without the array, and unparseable entries count nothing; response bodies and
status codes unchanged (the helper only reads `Response::get_result()`).

**Judgments recorded (out of scope, with reasons):**

- **stdio transport** (`mcp/stdio.rs`): not wired. `run_stdio` constructs no
  registry at all and stdio has no exposition surface; wiring it would create
  a registry nothing can observe. Owned by the planned
  `pdftract mcp --bind/--stdio … --metrics PORT` work, which must introduce
  the registry and its exposition together.
- **`extract_text` / `extract_markdown` tools**: their result JSON carries no
  diagnostics array (plain `{"text"}` / `{"markdown"}`), and adding one would
  change response bodies — forbidden here. The generic
  `metadata.diagnostics_detailed` sniff covers every tool that reports
  diagnostics, today (`extract`) and tomorrow.
- **mcp-served extraction/duration/pages/inflight counters**: mcp tool
  invocations have their own metric (`pdftract_mcp_requests_total`, wired);
  the extraction-shaped counters were scoped to the serve HTTP handlers by
  the chain children and remain so. Extending them to the mcp transport is
  the same future work as the mcp metrics exposition — today the mcp
  registry has no scrape route (the plan's `--metrics PORT` on `mcp` is not
  implemented as a CLI flag yet), so anything counted there is unobservable
  until that bead lands. Counters `mcp_requests_total`, `http_requests_total`,
  `rayon_pool_utilization`, `build_info` and now `diagnostic_emitted_total`
  accumulate correctly on the mcp registry meanwhile.
- **`with_build_info` has no call site outside `crates/pdftract-cli/src/metrics/` — by design.** It is the named constructor; `Registry::new()` delegates to it with `env!("CARGO_PKG_VERSION")`, `env!("GIT_SHA")`, `env!("COMPILED_FEATURES")` (`registry.rs:293-299`). Those are real, not placeholders: `build.rs` sets `GIT_SHA` from `git rev-parse HEAD` (falling back to `"unknown"` only outside a git repo) and `COMPILED_FEATURES` from the actual cargo feature set (`build.rs:22-53`). Both server states construct via `Registry::new()` at startup, so `pdftract_build_info` is populated for the process's whole lifetime; the rendered-label assertions live in `serve.rs` (test at `serve.rs:1921`) and `mcp/http.rs:1141`.

## 3. Chain evidence — what each earlier child established

| Commit | Bead | Evidence |
|---|---|---|
| `01fe7964` | pdftract-93b388c1 | metrics registry + OpenMetrics v1.0 text formatter behind the `metrics` feature (foundation) |
| `d5fd6d8d` | pdftract-6a0dbb53 | registry wired into serve (POST /extract, /extract/text, /extract/stream) and mcp request paths — 12 of 13 metrics live |
| `db783e40` | pdftract-6a0dbb53 | nested-`Result` and bin-crate build fixes for the wiring |
| `cc57f946` | pdftract-3d0533a3 | restored the second `?` on nested `extract_with_cache` results |
| `f948d7c6` | pdftract-8e39532f | mocked `ConnectInfo` for in-process metrics router tests |
| `c3a03431`, `0a749ea7`, `b855f394`, `e7173ccb` | pdftract-8e39532f | `pdftract_remote_bytes_downloaded_total` fed from the remote fetch path (extract/hash/grep) + test fixes |
| `3686b539`, `2566480f` | pdftract-b60ae586 | serde default on `ExtractionResult::diagnostics` (cache entries desync fixed); serve/mcp routers via `into_make_service_with_connect_info` (parent criteria 1, 3) |
| `d4a15302` | pdftract-841093b2 | cache hit/miss/size asserted at registry state level (parent criterion 2); feature-gated compiles (parent criterion 4) |
| `9f035d42` | pdftract-a2eeb7ee | this dispatch: mcp tool-result diagnostics counted; audit table above |

## 4. Verification (clean `git archive HEAD` extraction `/var/tmp/pdftract-a2eeb7ee-4HN1`, timeout-wrapped `timeout --kill-after=30s 600s …`, exit codes as shown)

| Command | Result |
|---|---|
| `cargo test -p pdftract-cli --lib --features metrics mcp::http::tests::test_metrics` | **3 passed; 0 failed** (398 filtered) — exit 0 |
| `cargo test -p pdftract-cli --lib --features metrics serve::tests::test_metrics` | **5 passed; 0 failed** (396 filtered) — exit 0 |
| `cargo test -p pdftract-cli --lib --features metrics metrics::registry::tests` | **20 passed; 0 failed** (381 filtered) — exit 0 |
| `cargo check -p pdftract-cli --features metrics` | exit 0 |
| `cargo check -p pdftract-cli` | exit 0 |

The three mcp tests are: the pre-existing router test (`…counts_tool_call_and_http`:
tool + http + build_info counters), the new `record_response_diagnostics` unit
test (labeled counting; error responses and array-less results count nothing),
and the new live-path test (`…counted_from_extract_result`: a real `tools/call`
to `extract` over the router; the rendered diagnostic counters must mirror what
the response body reports and count nothing phantom).

**Head-state caveat, verified empirically:** at HEAD `9f035d42` no fixture
emits `diagnostics_detailed` through extraction — the engine's diagnostics
pipeline is in a known degraded window (probe over 12 fixtures: the malformed
set hard-errors with mcp error `-32002`, the rest extract with zero
diagnostics; `tests/fixtures/encoding/unmapped-glyphs.pdf` returns raw byte
spans with no GLYPH_UNMAPPED diagnostics). The live-path test therefore
asserts the zero-mirror + no-phantom-counting invariant today and the
exact-mirror invariant the moment the engine emits diagnostics again; the
non-zero label rendering is pinned by the unit test. The serve path has the
identical limitation (no fixture-based non-zero diagnostics e2e exists there
either). Fixing the extraction pipeline is separate work, not this audit.

Verification ran from the committed state, never the shared working tree
(the tree carries other workers' stranded edits and does not compile as a
union; HEAD does). All runs are in-process `oneshot` routers — no spawned
servers, no orphan risk; extraction directory removed after the runs.

## 5. Parent acceptance-criteria map (pdftract-6a0dbb53)

| # | Criterion | Evidence |
|---|---|---|
| 1 | in-process serve extraction + mcp tool call assert rendered counters | serve tests (5, incl. `…cache_hit_miss_and_size_counters` and error/422) + mcp router test; `serve.rs` metric assertions at snapshot and render level |
| 2 | cache hit/miss/size move, direct registry assertion | `d4a15302` (`notes/pdftract-841093b2.md`) |
| 3 | existing serve/mcp response behavior unchanged | serve tests + mcp http metrics tests green at HEAD (§4); this dispatch's wiring only reads responses |
| 4 | `cargo check` with and without `metrics` | both exit 0 (§4) |
| 5 | notes + bead-citing commit pushed | this note + `9f035d42` |
