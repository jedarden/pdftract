# pdftract-c20c44c1 — `--metrics PORT` listener on `serve` and `mcp --bind`

**Date:** 2026-09-21 · **Dispatch:** claude-code-glm-5.3-glm-pdftract (attempt 3; attempts 1–2 died on
upstream API 499 before starting any work)
**Commit:** `937a3b33` — `feat(pdftract-c20c44c1): add --metrics PORT listener to serve and mcp --bind`
(8 files, 767 insertions)

## Scope

Per plan "Monitoring and Alerting" (plan.md ~3931+; the bead's cited line range 3865–3909 has
drifted — the section now sits at 3931): a `--metrics PORT` flag on BOTH `pdftract serve` and
`pdftract mcp --bind`; when set, a SECOND listener bound on `--bind`'s interface serves
`GET /metrics` with the OpenMetrics v1.0 exposition from the foundation registry
(pdftract-93b388c1 / pdftract-6a0dbb53 at HEAD). Flag absent → byte-identical behavior: no extra
listener, no metrics route anywhere.

## What was done

The workspace already held stranded in-flight work for exactly this scope (untracked
`metrics/endpoint.rs`, plus `pub mod endpoint` registration in `metrics/mod.rs` and the Serve-side
flag in `cli.rs` — none of it compiled, because the shared tree's union does not build). That work
was adopted, fixed, and completed:

- **`crates/pdftract-cli/src/metrics/endpoint.rs`** (new): the shared listener —
  `listener_addr()` (host of `--bind`, port replaced; IPv4/IPv6/hostname forms) and
  `bind_and_spawn()` (binds, prints the `Metrics endpoint:` stderr banner, serves `GET /metrics`
  from the registry with `openmetrics::CONTENT_TYPE`). Fixed a stranded bug: the `#[cfg(test)]`
  module referenced `super::CONTENT_TYPE`, which from inside `mod tests` resolves to
  `endpoint`, not `metrics` → E0425; now `crate::metrics::CONTENT_TYPE`.
- **`serve.rs` / `mcp/http.rs`**: `run()` / `run_server()` take `metrics_port: Option<u16>`; with
  the `metrics` feature the listener is bound BEFORE the main listener (an unbindable
  `--metrics` port fails startup cleanly, before anything is served); without the feature, a set
  flag is a clean `anyhow` error (nonzero exit). Module doc endpoint list updated (bead
  requirement).
- **`mcp/server.rs`**: `run()` threads `metrics_port` to `run_server`.
- **`cli.rs` and `main.rs`**: this repo has TWO `Commands` enums — the lib's `cli.rs` (public API)
  and the bin's own duplicate in `main.rs` (no `mod cli` in the bin). Both must carry the flag;
  only `cli.rs` had the stranded field, which is why the first committed tree failed E0026 under
  the bin. Both enums now have `metrics: Option<u16>` on `Serve` and `Mcp` with matching docs.
  `main.rs` wiring: Serve passes through `cmd_serve` → `serve::run`; Mcp rejects `--metrics` +
  stdio with a usage error (exit 2) and otherwise passes to `mcp::run`.
- **`crates/pdftract-cli/tests/metrics-listener.rs`** (new): five process-level tests (see below).

## Acceptance criteria

1. **PASS — exposition test.** `serve --metrics 0` and `mcp --bind … --metrics 0` each spawn the
   real binary; the metrics port is OS-chosen and parsed from the stderr banner. `GET /metrics`
   asserts: 200; content type exactly
   `application/openmetrics-text; version=1.0.0; charset=utf-8`; exactly 13 `# TYPE` lines; all
   13 documented `pdftract_*` names present; body ends `# EOF\n`.
   *Note on the 13-name check:* OpenMetrics renders counter SAMPLES with `_total` but counter
   family METADATA without it, and a labeled counter with no observations yet
   (`extractions_total`, `mcp_requests_total`, `http_requests_total`, `diagnostic_emitted_total`)
   exposes metadata only at boot (registry design: only unlabeled families seed zero children).
   The test therefore asserts each documented name as "sample line OR family `# TYPE` line" —
   every family is present. To prove the labeled families move to sample form, the serve test
   then POSTs a `%PDF-`-prefixed garbage document (passes `validate_pdf_magic_bytes`, fails
   extraction) and re-asserts `pdftract_extractions_total{result="error"}` and
   `pdftract_http_requests_total{` appear as samples.
2. **PASS — flag absent.** Serve and Mcp spawned without `--metrics`: `/metrics` on the main port
   → 404, `/health` → 200, and no `Metrics endpoint:` banner in stderr (no second listener).
3. **PASS — unbindable port.** Squatter listener held in-test; `serve --bind <free> --metrics
   <held>` exits nonzero within the bounded wait; stderr names `Failed to bind metrics listener`,
   carries the top-level `Error:` line, and never contains `panicked at`.
4. **PASS — test hygiene.** Ephemeral ports only (`--metrics 0` + banner parse; main port chosen
   by bind-`:0`-and-release); RAII `Server` guard kills on `Drop` and reaps with a bounded
   `try_wait` loop plus a backstop `wait()`; stdin/stdout are null and stderr is drained on a
   background thread (line-appended, so banners are readable while the child runs); every
   readiness/bail wait is wall-clock bounded. Post-run orphan check:
   `pgrep -af 'pdftract[ ]serve|pdftract[ ]mcp|metrics[-]listener'` → no matches (exit 1).
5. **PASS — note + commit + push.** This note; commit `937a3b33` cites the bead; pushed to
   `origin main`.

Also evidenced: a default-features (no `metrics` cargo feature) binary rejects the flag cleanly —
`pdftract serve --bind 127.0.0.1:0 --metrics 9999` →
`Error: --metrics requires a build with the `metrics` cargo feature …`, exit 1, no panic.
`mcp --stdio --metrics 0` → usage error, exit 2 (test-asserted).

## Verification (clean extraction of `937a3b33`, `/var/tmp/pdftract-c20c44c1-final-*`)

All commands run from a fresh `git archive 937a3b33` extraction (no `.git`; `build.rs` falls back
to `GIT_SHA=""`), `set -o pipefail` so cargo's exit code is what is recorded:

| Command | Result |
|---|---|
| `cargo check --all-targets` (workspace, default features) | exit 0 |
| `cargo check -p pdftract-cli --all-targets` (default) | exit 0 |
| `cargo check -p pdftract-cli --features serve --all-targets` | exit 0 |
| `cargo test -p pdftract-cli --features serve --test metrics-listener` | exit 0 — **5 passed** |
| `cargo test -p pdftract-cli --features serve --lib metrics` | exit 0 — **44 passed** (incl. endpoint/openmetrics/registry/serve metrics unit tests) |
| `cargo test -p pdftract-cli --features serve --test mcp-http` | exit 0 — **10 passed** (existing MCP HTTP suite unaffected) |
| `cargo build -p pdftract-cli` (default) | exit 0 |

No WARN/FAIL items. The shared working tree itself does NOT compile as a union (pre-existing
stranded edits in `pdftract-core`, ≥2026-09-17) — all verification above is from the committed
tree, which is what the gate sees.

## Notes for the next worker on this surface

- The `--metrics` runtime error in non-metrics builds is deliberate (feature `serve` implies
  `metrics`; `mcp` does not).
- The bin/lib duplicate `Commands` enum is a standing trap: CLI field additions must be made in
  BOTH `cli.rs` and the `enum Commands` inside `main.rs`, and the compile check must run the
  `serve` feature — the default-feature check alone will not catch drift in server-only fields
  quickly, and the shared tree cannot compile at all, so extraction checks are the only real
  signal.
