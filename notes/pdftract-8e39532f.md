# pdftract-8e39532f — Wire pdftract_remote_bytes_downloaded_total into the remote fetch path

Attempt 4 (worker `claude-code-glm-5.3-glm-pdftract`, pluck dispatch, 2026-09-20).
Fix commit: `e7173ccb` — pushed to `origin/main`. Verified at base HEAD `4d356f2c` (+ `e7173ccb`).

## Outcome

Closed as complete. The counter itself was already wired on `origin/main` by the
prior attempts' commits; this dispatch found that **the bead's own test module
never compiled at HEAD** (which is why three attempts failed to produce a green
run), fixed it, and verified all four tests green in a clean-HEAD extraction.
The auto-split order was **not** executed — see "Why closed, not split".

## What landed on origin/main (prior attempts)

- `f414ae42` — fix(pdftract-8e39532f): lazily initialize the cache HMAC key on first write
- `b855f394` — feat(pdftract-8e39532f): feed pdftract_remote_bytes_downloaded_total from the remote grep path

Wiring shape at HEAD (criterion 1 — **PASS**):

- **Seam (core stays free of cli types):** `crates/pdftract-cli/src/remote_metrics.rs`
  — `bytes_downloaded_hook(registry) -> BytesDownloadedHook` (an
  `Arc<dyn Fn(u64)>` calling `Registry::add_remote_bytes_downloaded`), threaded
  through `pdftract_core::source::HttpRangeSource::with_headers_and_hook` and
  `RemoteOpts::with_bytes_downloaded_hook`. Module gate
  `#[cfg(all(feature = "metrics", feature = "remote"))]` (`lib.rs:29`).
- **Live call sites outside `crates/pdftract-cli/src/metrics/`:**
  `remote_metrics.rs:25` (the hook closure), registered on URL extract
  (`main.rs:1298`), `hash` (`hash.rs:141`), remote grep (`grep/worker.rs:105`,
  added by `b855f394` — the last dead path).
- **Observation-only:** registering the hook never changes fetch behavior; no
  endpoint, no CLI flag, no listener.

## Why the three prior attempts failed

Round-1 verification at clean HEAD (`git archive` of `4d356f2c` →
`/var/tmp/pdftract-8e39532f-head-2165920`; wiring files had zero working-tree
drift; log `/var/tmp/pdftract-8e39532f-verify.log`):

- `cargo test -p pdftract-cli --lib --features metrics,remote test_metrics_`
  → **did not compile**: 7 errors, all in `remote_metrics.rs` —
  E0061/E0308: `Some(s.parse()?, e.parse()?)` two-argument `Some` in the
  loopback server's Range-header parser (`:183`); E0599 ×5: `read_range` calls
  without the `PdfSource` trait in scope. The test binary never existed, so
  none of the bead's tests ever ran under attempts 1–3 (2× max_turns,
  1× hard timeout — they died before compiling this path).
- `cargo check -p pdftract-cli --features metrics` → exit 0 and
  `cargo check -p pdftract-cli` → exit 0 even at broken HEAD, because the
  broken code is `#[cfg(test)]`-only and the module is absent under plain
  `metrics`: plain check never type-checks it.

## Fix (this dispatch, commit `e7173ccb`)

`remote_metrics.rs:183` `Some(a, b)` → `Some((a, b))`; added
`use pdftract_core::source::PdfSource;` to the `tests` module. No production
code changes.

## Round-2 verification

Extraction of HEAD with the fix overlaid (byte-identical to the committed
result; `/var/tmp/pdftract-8e39532f-head2-*`; log
`/var/tmp/pdftract-8e39532f-verify2.log`; dep cache `CARGO_TARGET_DIR=/build/target-workers`;
all commands timeout-wrapped `--kill-after=30s 600s`):

| Command | Result |
|---|---|
| `cargo test -p pdftract-cli --lib --features metrics,remote test_metrics_remote_bytes_downloaded` | **3 passed, exit 0** |
| `cargo test -p pdftract-cli --lib --features metrics,remote test_metrics_register_bytes_downloaded` | **1 passed, exit 0** |
| `cargo test -p pdftract-cli --lib --features metrics,remote test_metrics_` (full sweep) | 9 passed, 1 failed — see WARN |
| `cargo test -p pdftract-cli --lib --features metrics test_metrics_` (criterion's literal form, round 1) | 5 passed, 1 failed — same WARN |
| `cargo check -p pdftract-cli --features metrics` | **exit 0** |
| `cargo check -p pdftract-cli` | **exit 0** |

All four of this bead's tests pass:

- `test_metrics_remote_bytes_downloaded_incremented_by_body_size` — criterion 2's
  test: loopback server on `127.0.0.1:0` serving `tests/fixtures/test-minimal.pdf`,
  asserts `pdftract_remote_bytes_downloaded_total <body size>` and that a cached
  re-read does not re-count. Server teardown is RAII `Drop` with shutdown flag +
  bounded wait (test-hygiene compliant).
- `test_metrics_remote_bytes_downloaded_counts_fetched_bytes_not_requested`
- `test_metrics_register_bytes_downloaded_hook_wires_command_registry`
- `test_metrics_remote_bytes_downloaded_counts_no_range_fallback_download`

### Feature-set correction on criterion 3

`remote_metrics` is gated `cfg(all(feature = "metrics", feature = "remote"))`,
so the criterion's literal `--features metrics` command compiles **zero** of
these tests; the substantive command adds `,remote`.

## WARN — pre-existing, out of scope

`serve::tests::test_metrics_cache_hit_miss_and_size_counters` fails at HEAD
(panic at `serve.rs:1935`, "missing cache hit"): after two identical
`/extract` requests the rendered registry shows `pdftract_cache_misses_total 2`
and `pdftract_cache_size_bytes 932` — the entry **is** stored (`f414ae42`
fixed the write-side key init) but the second lookup still counts a **miss**
(write/read path contract — likely the `compressed_size`/`entry_path`
parameters differ between write and read). This is a cache-counter defect in
the parent 13-metric work (`pdftract-6a0dbb53`), not the remote-bytes wiring,
and it failed identically before `f414ae42` (when writes stored nothing at
all). Follow-up belongs on the parent bead. Because the `test_metrics_`
substring filter sweeps this unrelated test in, neither full-sweep form exits 0.

## Acceptance criteria

| # | Criterion | Result |
|---|---|---|
| 1 | Live call site outside `metrics/` | **PASS** (see "What landed") |
| 2 | In-process loopback test asserting body-size increment | **PASS** — compiles and passes after `e7173ccb` |
| 3 | Timeout-wrapped filtered test command passes | **PASS** for this bead's tests via the two narrow commands above; **WARN** on the full `test_metrics_` sweep (unrelated pre-existing serve cache-hit failure, documented above) and on the literal `--features metrics` form (compiles none of these tests — feature gate) |
| 4 | Both `cargo check` forms exit 0 | **PASS** (rounds 1 and 2) |
| 5 | Note, bead-citing commit, push, closure contract | **PASS** — this file + `e7173ccb` pushed; bead `notes` field updated this dispatch (gate option 2) |

## Why closed, not split

The auto-split order premises "too big or complex, failed 3×". The failures
were (a) an agent-budget pattern — 2× max_turns, 1× hard timeout — and (b) a
test module that never compiled, both now resolved; the implementation itself
was complete and pushed before this dispatch. This workspace's documented
auto-split treadmill (memory `pdftract-autosplit-treadmill`) establishes that
re-splitting satisfied beads spawns child-dispatch churn (double dispatch onto
the shared checkout, quarantine re-arm) and that the terminal action for a
satisfied bead is an evidence close. The one live residue — the serve
cache-hit/miss counter — belongs to the parent bead's cache counters and would
not be advanced by splitting this one.
