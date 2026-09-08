# pdftract-b716bac5 — child 1: pdftract-core lib test target compile gate

**Bead:** pdftract-fa233a5e (auto-split child 1 of 4; umbrella pdftract-b716bac5;
origin bf-5o22rf, criterion (a) runtime evidence).
**Scope of this bead:** run the compile precondition exactly once and record the
verdict. No source changes of any kind.

## Verdict: **NO-GO**

The `pdftract-core` lib test target does **not** compile. Children 2–4 of this
split must **not** proceed with test runs until this gate clears.

## Run record

- **Command (exactly as run, single invocation, no retry):**
  `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run`
- **Start:** 2026-09-08T07:11:00Z (last pre-run clock read 2026-09-08T07:10:56Z)
- **Finished:** 2026-09-08T07:14:57Z (final write of captured output, ~4 min wall)
- **Exit code:** `101` (rustc compile failure)
- **rustc summary line:** `error: could not compile \`pdftract-core\` (lib test) due to 89 previous errors; 129 warnings emitted`
- **Harness note:** the tree carries uncommitted sibling edits, so the
  `~/.local/bin/cargo` wrapper took its documented local cgroup-limited fallback
  rather than submitting to iad-ci; the invocation itself was timeout-wrapped per
  the repo test-hygiene rule. Raw output was captured to an ephemeral
  `/tmp/b716bac5-child1-compile.log` and deliberately not committed (this note is
  the durable artifact; the tree must stay free of build logs).

## Per-file error inventory (89 errors total)

| File | Errors | Codes |
|---|---:|---|
| `crates/pdftract-core/src/classify.rs` | 54 | 52× E0609, 2× E0277 |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 | 24× E0061, 5× E0599 |
| `crates/pdftract-core/src/render/scanline.rs` | 5 | 2× E0308, 2× E0277, 1× E0599 |
| `crates/pdftract-core/src/content_stream.rs` | 1 | 1× E0061 |
| **Total** | **89** | E0609×52, E0061×25, E0599×6, E0277×4, E0308×2 |

## `source/mmap.rs` — explicitly checked: **zero errors** (expected: none ✓)

`crates/pdftract-core/src/source/mmap.rs` produced **0** error diagnostics. The
mmap observability work this chain came from (commit `08ec8db8`, plus in-flight
test additions) is not the blocker; the gate failure is entirely sibling
TEST-module edits elsewhere.

## Drift vs. the 03:10 UTC blocker set

No drift — the live inventory is byte-for-byte the same four files with the same
counts recorded in the umbrella description (classify.rs 54, type3_rasterizer.rs
29, scanline.rs 5, content_stream.rs 1). `signature/mod.rs` and
`word_boundary.rs`, which broke earlier runs, remain clear: `signature/mod.rs`
contributed only unused-`mut`/unused-variable **warnings**, no errors.

## Git state at check time

- `git rev-parse HEAD` = `e7a75b9e60e565eb393b7820d3f28605c40e4f6a`
- `origin/main` (Forgejo, fetched during this check) = `e7a75b9e60e565eb393b7820d3f28605c40e4f6a`
- Divergence: `0 0` — HEAD is exactly at the pushed remote tip.

Dirty-state split of the four error files (matters for who unblocks the gate):

| File | Working tree vs HEAD | Consequence |
|---|---|---|
| `classify.rs` | **clean** | Its 54 errors are **committed** at HEAD — a sibling committing/pushing in-flight work will not clear these |
| `render/scanline.rs` | **clean** | Its 5 errors are likewise **committed** at HEAD |
| `font/type3_rasterizer.rs` | modified (10 lines) | 29 errors sit in a locally-edited tree state |
| `content_stream.rs` | modified (305 lines) | Its 1 error sits in a locally-edited tree state |
| `source/mmap.rs` | modified (+138 lines, no errors) | The mmap work under test is clean |

This is the key finding for children 2–4: **most of the gate (59 of 89 errors,
in `classify.rs` + `render/scanline.rs`) is broken at the committed tip of
`main`**, not merely in uncommitted in-flight edits. The gate will not clear on
its own via sibling commits; it needs the test modules to actually catch up to
the changed source signatures.

## Error character (representative samples)

All 89 errors are in `#[cfg(test)]` test code lagging behind changed production
signatures — no production-source breakage:

- `E0609` at `classify.rs:2211` — `assert_eq!(result.class, PageClass::Scanned)`
  against a type that is now `Result<PageClassification, ClassificationError>`:
  the test predates the `Result`-returning signature.
- `E0061` at `content_stream.rs:3347` — `execute_with_do(...)` takes 7 arguments,
  test supplies 6.
- `E0277` at `classify.rs:2694` — `serde_json::to_string(&result)` requires
  `ClassificationError: Serialize`, which the production type does not (yet)
  implement.

## Compliance

- No file under `crates/` was touched by this bead — the working tree's
  `crates/` modifications belong to sibling workers and were left exactly as
  found.
- Single gated invocation; no overlapping retries were launched.
- Orphan check before close: `pgrep -af "cargo test|pdftract"` returned nothing
  spawned by this worker.
