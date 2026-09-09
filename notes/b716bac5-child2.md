# pdftract-b716bac5 — child 2 (pdftract-030e8414): source::mmap executed-test capture — COMPILE-BLOCKED

**Bead:** pdftract-030e8414 (auto-split child 2 of 4; umbrella pdftract-b716bac5)
**Scope given:** execute the `source::mmap` test suite and capture verbatim executed-test
output, so `test_prefetch_madvise_failure_is_traced`
(`crates/pdftract-core/src/source/mmap.rs:537`, introduced by commit `08ec8db8`) finally
gets a runtime verdict for parent bf-5o22rf criterion (a).
**This note is evidence, not a verdict on criterion (a)** — child 3 writes that verdict.

## Outcome: NO TESTS RAN — the lib test target does not compile

Child 1 (`notes/b716bac5-child1.md`) recorded **NO-GO**. Per this bead's step 1, the
precondition was re-run **once** at the current tip. It still exits non-zero, so the
documented-compile-blocked branch applies: nothing under `crates/` was edited, no test
executable was produced, and **no test executed**. Any "PASS" for
`test_prefetch_madvise_failure_is_traced` would be fabricated; this bead claims none.

## Gate re-derivation (the one invocation this bead ran)

| Field | Value |
|---|---|
| Command | `set -o pipefail; timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run 2>&1 \| tee /tmp/gate-b716bac5-child2.log \| tail -40` |
| UTC completed | 2026-09-09T00:01:50Z |
| cargo exit code | **101** (rustc compile failure, read from `${PIPESTATUS[0]}` — no timeout kill; 124/137 does not apply) |
| Invocations | exactly **one**; no retries, no overlapping runs |
| Raw output | committed verbatim as `notes/b716bac5-child2-gate-raw.log` (2,209 lines, stdout+stderr combined) |
| Tree under test | HEAD `589f8c5587af610e4343e9a05ce757cd2b51fb0d` on `main`, dirty working tree carrying 49 uncommitted in-flight paths under `crates/` from other workers (none touched by this bead); `crates/pdftract-core/src/source/mmap.rs` is one of the dirty paths |
| rustc summary | ``error: could not compile `pdftract-core` (lib test) due to 89 previous errors; 129 warnings emitted`` |

## Fresh per-file error inventory — 89 errors, 4 files (this bead's own log)

Parsed from the **primary** `-->` span of each of the 89 diagnostics in
`notes/b716bac5-child2-gate-raw.log`:

```
crates/pdftract-core/src/classify.rs             -> E0609 x52, E0277 x2                  -> 54
crates/pdftract-core/src/font/type3_rasterizer.rs -> E0061 x24, E0599 x5                  -> 29
crates/pdftract-core/src/render/scanline.rs      -> E0277 x2, E0308 x2, E0599 x1          -> 5
crates/pdftract-core/src/content_stream.rs       -> E0061 x1                              -> 1
--------------------------------------------------------------------------------------------
TOTAL                                                                                     -> 89
```

Code distribution across all 89: `E0609 x52, E0061 x25, E0599 x6, E0277 x4, E0308 x2`.

### Comparison to child 1 — byte-identical, fourth consecutive derivation

Identical file set, identical per-file × per-code counts, identical total (89), identical
warning count (129) as child 1's committed log (`notes/b716bac5-child1-gate-raw.log`,
verdict `notes/b716bac5-child1.md`). This is the **fourth** consecutive identical result:
child 1's two captures (09:20:54Z superseded, 12:28:41Z verdict), the pdftract-fa233a5e
re-derivation at HEAD `bd12b867` (commit `589f8c55`), and this one at HEAD `589f8c55`.
All 89 errors are in `#[cfg(test)]` test code of four sibling files; every diagnostic's
character matches child 1's analysis (tests predate changed production signatures —
`.class` off a now-`Result`-returning `classify` API, stale arities for
`execute_with_do`/`detect_char_proc_type`, removed `CharProcType::Unknown`, integer-vs-float
`scanline` edge fields).

### `source/mmap.rs` — zero diagnostics, as child 1 found

`grep -c 'source/mmap'` over the entire 2,209-line log: **0**. No error and no warning
points into `source/mmap.rs`. The mmap observability work this chain exists to verify
(commit `08ec8db8`) is **not** implicated in the gate failure; the three tests it adds —
`test_prefetch` (mmap.rs:430), `test_prefetch_past_eof` (mmap.rs:439),
`test_prefetch_madvise_failure_is_traced` (mmap.rs:537) — are present in the working tree
but cannot be linked into a test binary until the four sibling test modules are repaired.
59 of the 89 errors (`classify.rs` + `render/scanline.rs`) are broken at the committed,
pushed tip of `main`, so the gate will not clear from local edits alone (child 1's finding,
unchanged).

## What this means for the umbrella

Criterion (a) ("log madvise failures at trace level for debugging") still has **no runtime
evidence** — it has only ever been PASSed on code-path analysis (pdftract-e6c4a13f). The
blocking work is: bring the four sibling test modules up to the changed production
signatures. Until then, children 3 and 4 have no executed-test output to adjudicate either.

## No fix bead filed

The acceptance criterion "On genuine test FAIL: file a fix bead" does not trigger — no test
ran, so there is no FAIL. The blocker is compile-time breakage in four sibling files
already inventoried and owned by the gate chain (child 1 + pdftract-68091d06 + the
pdftract-3c39e12c split); filing another duplicate fix bead would feed the auto-split
treadmill rather than clear the gate.

## Compliance

- No file under `crates/` was created, edited, or reverted by this bead.
- Exactly one cargo invocation, timeout-wrapped, no overlapping retries.
- No test was claimed to run; no PASS/FAIL is asserted for any `source::mmap` test.
- Raw evidence committed: `notes/b716bac5-child2-gate-raw.log`.
- Orphan check at close: `pgrep -af "cargo test|pdftract"` shows no process this bead
  spawned. The hits at that moment were sibling needle dispatchers only — including one
  sibling worker's 17-attempt `cargo check --lib` poll loop (not started by, and not
  killed by, this bead).
