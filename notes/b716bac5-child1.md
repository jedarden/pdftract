# pdftract-b716bac5 — child 1: pdftract-core lib test target compile gate

**Bead:** pdftract-fa233a5e (auto-split child 1 of 4; umbrella pdftract-b716bac5;
origin bf-5o22rf, criterion (a) runtime evidence).
**Scope of this bead:** run the compile precondition exactly once and record the
verdict. No source changes of any kind.

> This capture **supersedes** the 2026-09-08 07:11 UTC NO-GO capture of the same
> bead (preserved in git history at commit `20189f01`). Two sibling commits
> landed in between (`e7a75b9e` → `f785930f`); the gate was re-run against the
> updated tip as the bead's re-inventory requirement directs.

## Verdict: **NO-GO**

The `pdftract-core` lib test target does **not** compile. Children 2–4 of this
split must **not** proceed with test runs until this gate clears.

## Run record

- **Command (exactly as run, single invocation, no retry):**
  `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run 2>&1 | tee <ephemeral log> | tail -120`
  (run under `set -o pipefail`; the recorded exit code is the pipeline status,
  i.e. cargo's own exit code, not `tail`'s)
- **Start:** 2026-09-08T09:20:54Z — **End:** 2026-09-08T09:21:03Z (~9 s wall;
  incremental build, dependencies already cached by sibling runs)
- **Exit code:** `101` (rustc compile failure)
- **rustc summary line:** `error: could not compile \`pdftract-core\` (lib test) due to 89 previous errors; 129 warnings emitted`
- **Harness note:** the tree carries uncommitted sibling edits, so the
  `~/.local/bin/cargo` wrapper took its documented local cgroup-limited fallback
  rather than submitting to iad-ci; the invocation itself was timeout-wrapped per
  the repo test-hygiene rule. Raw output was captured to an ephemeral
  `/tmp/fa233a5e-gate-0915.log` and deliberately not committed (this note is the
  durable artifact; the tree must stay free of build logs).

## Per-file error inventory (89 errors total)

| File | Errors | Codes |
|---|---:|---|
| `crates/pdftract-core/src/classify.rs` | 54 | 52× E0609, 2× E0277 |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 | 24× E0061, 5× E0599 |
| `crates/pdftract-core/src/render/scanline.rs` | 5 | 2× E0277, 2× E0308, 1× E0599 |
| `crates/pdftract-core/src/content_stream.rs` | 1 | 1× E0061 |
| **Total** | **89** | E0609×52, E0061×25, E0599×6, E0277×4, E0308×2 |

## `source/mmap.rs` — explicitly checked: **zero errors** (expected: none ✓)

`crates/pdftract-core/src/source/mmap.rs` produced **0** error diagnostics
(parsed across every `-->` span in the full 89-error log). The mmap
observability work this chain came from (commit `08ec8db8`, plus in-flight test
additions) is not the blocker; the gate failure is entirely sibling TEST-module
edits elsewhere.

## Drift vs. the 03:10 UTC blocker set and the 07:11 UTC capture

**No drift — third consecutive identical inventory.** The live run reproduces
byte-for-byte the same four files with the same counts and the same code
distribution recorded at 03:10 UTC (umbrella description) and at 07:11 UTC
(prior capture of this bead). `signature/mod.rs` and `word_boundary.rs`, which
broke earlier runs, remain clear: `signature/mod.rs` contributed only
unused-`mut`/unused-variable **warnings**, no errors.

## Git state at check time

- `git rev-parse HEAD` = `f785930ff77d76efbe2732b0b5300481248d26de`
- `origin/main` (Forgejo, fetched immediately before the run) = `f785930ff77d76efbe2732b0b5300481248d26de`
- Divergence: `0 0` — HEAD is exactly at the pushed remote tip.

Dirty-state split of the four error files (matters for who unblocks the gate):

| File | Working tree vs HEAD | Consequence |
|---|---|---|
| `classify.rs` | **clean** | Its 54 errors are **committed** at HEAD — a sibling committing/pushing in-flight work will not clear these |
| `render/scanline.rs` | **clean** | Its 5 errors are likewise **committed** at HEAD |
| `font/type3_rasterizer.rs` | modified (local sibling edits) | 29 errors sit in a locally-edited tree state |
| `content_stream.rs` | modified (local sibling edits) | Its 1 error sits in a locally-edited tree state |
| `source/mmap.rs` | modified (+local sibling edits, no errors) | The mmap work under test is clean |

Same key finding for children 2–4 as the 07:11 capture: **59 of 89 errors (in
`classify.rs` + `render/scanline.rs`) are broken at the committed tip of
`main`**, not merely in uncommitted in-flight edits, and the two sibling
commits that landed between the two captures did not change that. The gate will
not clear on its own via sibling commits; the test modules have to catch up to
the changed production signatures.

## Error character (representative samples, this run)

All 89 errors are in `#[cfg(test)]` test code lagging behind changed production
signatures — no production-source breakage:

- `E0609` at `classify.rs:2211` — `result.class` read off a type that is now
  `Result<PageClassification, ClassificationError>`: the test predates the
  `Result`-returning signature.
- `E0061` at `content_stream.rs:3347` — `execute_with_do(...)` takes 7 arguments,
  test supplies 6.
- `E0277` at `classify.rs:2694` — `ClassificationError: serde::Serialize` is
  not satisfied, so `serde_json::to_string(&result)` in the test cannot compile.

## Compliance

- No file under `crates/` was touched by this bead — the working tree's
  `crates/` modifications belong to sibling workers and were left exactly as
  found.
- Single gated invocation; no overlapping retries were launched (pre-run check
  showed no `cargo test`/`cargo-remote` process running).
- Orphan check before close: `pgrep -af "cargo test|pdftract"` — see close
  record on the bead.
