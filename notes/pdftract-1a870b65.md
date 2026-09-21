# pdftract-1a870b65 — GET /ready readiness probe (pool saturation / unwritable cache)

Bead: `pdftract-1a870b65` (split child of parent `bf-9y8lb5`, blocked by `pdftract-c20c44c1` — the
`--metrics PORT` second listener). Plan reference: `docs/plan/plan.md` "Monitoring and Alerting" →
"Health and readiness endpoints" (lines ~3971-3973; the bead description's "~lines 3865-3909" points
at the Rollout section — the actual /ready spec is at 3971-3973, quoted below):

> - `GET /health` returns `200 OK` ... Always returns 200 as long as the process is up; intended for
>   liveness probes.
> - `GET /ready` returns `200 OK` only when the rayon pool utilization is below 90% AND the cache
>   (if enabled) is writable. Returns 503 otherwise. Intended for readiness probes; routing layers
>   SHOULD pull a node out of rotation when `/ready` reports 503.

## What was implemented

Commit `17c0904a` (amends the first cut `9856b21c`, which accidentally dropped `listener_addr` in the
endpoint.rs rewrite — caught immediately by the clean-extraction compile, restored verbatim).

- `crates/pdftract-cli/src/metrics/endpoint.rs` — `GET /ready` added to the metrics listener router:
  - 200 `{"status":"ready","unready":[],"pool_utilization":<f64>,"cache_writable":<bool>}` while
    accepting work.
  - 503 `{"status":"unready","unready":[<conditions>],...}` otherwise; the `unready` array names
    every failed condition — `pool_saturated` (utilization strictly `> 0.90`,
    `SATURATION_THRESHOLD`) and/or `cache_unwritable` — so an operator can tell them apart without
    re-testing. Exactly 0.90 is still ready (the bead pins the 503 to *exceeding* 0.90).
  - Both readiness inputs injectable via `metrics::Readiness`:
    - `Readiness::production(registry, cache_dir)` — reads the registry's sampled
      `pdftract_rayon_pool_utilization` gauge (new public getter
      `Registry::rayon_pool_utilization()` in `registry.rs`) and probes `cache_dir`;
      `None` (cache disabled/absent — always the case for `pdftract mcp`) makes the cache condition
      trivially pass.
    - `Readiness::with_inputs(utilization_fn, writable_fn)` — tests force each failure condition
      without real load or read-only mounts.
  - `probe_cache_writable(dir)`: creates, writes, and removes a uniquely named probe file
    (`.pdftract-ready-probe-<pid>-<nanos>`) in the cache dir. The file is removed on every path
    where it was created (including when the write fails); a create-but-cannot-remove dir reports
    unwritable. Plain filesystem only — the local extraction cache (plan Phase 6.9); no OpenBao/KV
    path is consulted or required.
  - `bind_and_spawn` now takes `(addr, registry, readiness)`; router state is a private
    `MetricsState { registry, readiness }`.
- `crates/pdftract-cli/src/metrics/registry.rs` — added the public gauge getter
  `rayon_pool_utilization()`.
- `crates/pdftract-cli/src/metrics/mod.rs` — re-exports (`Readiness`, `ReadinessReport`,
  `SATURATION_THRESHOLD`) and layout doc updated.
- `crates/pdftract-cli/src/serve.rs` — `ServeState` gains a `readiness` field built by
  `ServeState::new` from the startup cache configuration (disabled/absent cache → `None` → cache
  condition passes); `with_readiness(...)` replaces the inputs for tests. `run()` passes
  `state.readiness.clone()` to `bind_and_spawn`. Module-doc endpoint enumeration updated (second
  listener serves exactly two routes; `/health` documented as the liveness probe that stays 200).
- `crates/pdftract-cli/src/mcp/http.rs` — `run_server` builds `Readiness::production(registry, None)`
  (MCP mode has no extraction cache); module + param docs updated.
- `crates/pdftract-cli/src/cli.rs` + `crates/pdftract-cli/src/main.rs` — both `Commands` enums:
  endpoint enumeration gains `GET /ready`, and both `--metrics` flag docs (serve + mcp) now say the
  second listener serves `GET /metrics` and `GET /ready`, neither on the main port.
- `/health` itself is untouched — it remains a static always-200 handler on the main serve/MCP
  routers, deliberately blind to both failure states.

## Verification (clean extraction of commit `17c0904a` via `git archive`, not the shared tree)

The shared working tree does not compile (pre-existing stranded edits in
`crates/pdftract-core/src/parser/pages.rs` from other workers — E0308 ObjRef mismatches, documented
in memory). All verification below ran in `git archive 17c0904a` extracted to `/data/pdftract-verify-1a870b65`
(removed after verification), building into the global `/build/target-workers` target dir.

`cargo test --features serve -p pdftract-cli --lib metrics::endpoint::` → **exit 0, 13 passed, 0 failed**:

- `ready_is_200_on_a_healthy_server` — criterion 1 (injected seam), 200 + `status:"ready"`.
- `ready_is_503_naming_the_pool_when_utilization_exceeds_the_threshold` — criterion 2 (injected
  0.95), 503 + `unready == ["pool_saturated"]` and nothing else.
- `ready_is_200_at_exactly_the_threshold` — boundary: 0.90 is ready.
- `ready_is_503_naming_the_cache_when_the_cache_probe_fails` — criterion 3 via the forced seam,
  503 + `unready == ["cache_unwritable"]` and nothing else.
- `ready_names_every_failed_condition_when_both_fail` — 503 body names BOTH conditions at once.
- `production_readiness_reads_the_registry_gauge` — the production wiring consumes the registry
  gauge (0.0 ready → 0.91 `pool_saturated`).
- `probe_accepts_a_writable_directory_and_leaves_nothing_behind` — criterion 5 (directory empty
  after the probe).
- `probe_rejects_a_readonly_directory_and_leaves_nothing_behind` — criterion 5 on the failure path
  (0o555 dir; self-skips under uid 0 where permission bits are advisory; dir empty after).
- `probe_rejects_a_nonexistent_directory` — criterion 3's nonexistent-parent case at the probe level.
- plus the three pre-existing listener tests (`listener_addr_*`, `bind_failure_is_a_clean_error`,
  `served_route_returns_openmetrics_with_the_exact_content_type`) — still green.

`cargo test --features serve -p pdftract-cli --lib -- serve::tests::test_health serve::tests::test_ready`
→ **exit 0, 5 passed, 0 failed**:

- `test_health_returns_200_when_pool_is_saturated` — criterion 4, saturation state.
- `test_health_returns_200_when_cache_is_unwritable` — criterion 4, cache state.
- `test_ready_is_not_routed_on_the_main_port` — plan endpoint policy: 404 on the main router.
- `test_ready_is_200_when_cache_location_is_writable` — criterion 1 through PRODUCTION wiring (real
  prober, real tempdir cache, real listener).
- `test_ready_is_503_when_cache_points_at_an_unwritable_path` — criterion 3 through PRODUCTION
  wiring (cache enabled at `tempdir/does-not-exist`, default prober, no injection):
  503 + `unready == ["cache_unwritable"]`.

`cargo check --all-targets -p pdftract-cli` (default features — the repo gate's shape) → **exit 0**:
the new feature-gated code and tests do not disturb the default-feature build.

## Acceptance criteria

1. **PASS** — healthy server → 200 from `GET /ready` (`ready_is_200_on_a_healthy_server`,
   `test_ready_is_200_when_cache_location_is_writable`).
2. **PASS** — injected utilization 0.95 → 503, body names pool saturation
   (`ready_is_503_naming_the_pool_when_utilization_exceeds_the_threshold`).
3. **PASS** — cache at an unwritable path → 503, body names the cache, exercised end-to-end through
   the default prober with a nonexistent directory
   (`test_ready_is_503_when_cache_points_at_an_unwritable_path`, `probe_rejects_a_nonexistent_directory`,
   `probe_rejects_a_readonly_directory_and_leaves_nothing_behind`).
4. **PASS** — `GET /health` returns 200 in both failure states
   (`test_health_returns_200_when_pool_is_saturated`, `test_health_returns_200_when_cache_is_unwritable`).
5. **PASS** — the probe removes what it creates (verified empty-dir assertions on both the success
   and read-only failure paths); it is plain std::fs on the cache dir — no OpenBao/KV path.
6. **PASS** — this note; commit `17c0904a` cites the bead; pushed to `origin main`.

## Notes / decisions

- The bead's plan citation ("~lines 3865-3909") lands on the Rollout/Rollback section; the actual
  `/ready` spec is plan lines 3971-3973 (same "Monitoring and Alerting" section the second listener
  came from). Implemented from 3971-3973 + the bead text, which agree.
- 503 uses `SERVICE_UNAVAILABLE` with `application/json` (serde_json via axum `Json`), matching the
  serve router's JSON error convention.
- `pdftract_http_requests_total` does not count metrics-listener traffic (unchanged policy — the
  counting middleware lives on the main router only).
- The metrics listener's readiness inputs are evaluated per request (two cheap closure calls), so
  probes observe current state, not a startup snapshot.
