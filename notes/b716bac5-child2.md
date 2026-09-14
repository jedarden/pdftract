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

---

# EXECUTED RUN 2026-09-14 — source::mmap 25/25 PASS at HEAD (supersedes the compile-blocked capture above)

Re-issued after child 1's 2026-09-14 gate re-run flipped the verdict to **GO**
(`notes/b716bac5-child1.md`, RE-RUN section: lib test target compiles at `fff28e0c`,
exit 0, 2026-09-14T12:40:53Z). The compile-blocked capture above described the tree as
it stood on 2026-09-09; this section is the bead's actual deliverable: executed-test
output for `source::mmap` at the current tip, with the pinning test
`test_prefetch_madvise_failure_is_traced` (`crates/pdftract-core/src/source/mmap.rs:537`,
landed by pdftract-931e6649 as commit `454a0daf`) getting its **first runtime PASS** for
parent bf-5o22rf criterion (a). This note remains evidence, not the verdict on criterion
(a) — child 3 writes that.

## Tree state at run time

- HEAD under test: **`cf55465397493df6e8144f34a91a758894c4e248`** on `main`
  (`git rev-parse HEAD` at 12:52Z). The tip moved to `1ce0faac` at 12:59Z while this bead
  ran; `cf554653..1ce0faac` is docs-only (a sibling sweep note under `notes/`), so the
  evidence carries to the current tip unchanged.
- `crates/pdftract-core/src/source/mmap.rs` is **clean vs HEAD** (verified twice, before
  and after the runs) — the file under test carries no in-flight sibling edits.
- 84 pre-existing dirty paths from sibling in-flight work; none touched by this bead.

## Invocation 1 — shared working tree: compile-blocked by in-flight sibling edits (not a gate regression)

`set -o pipefail; timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source::mmap 2>&1 | tee /tmp/mmap-child2-20260914.log | tail -60` — 2026-09-14T12:52:32→12:52:43Z, cargo exit **101**: ``error: could not compile `pdftract-core` (lib test) due to 4 previous errors; 127 warnings emitted``. No test ran.

Fresh inventory — 4 errors, all `E0277` ("`(dyn parser::stream::PdfSource + 'static)` may
contain interior mutability … across a catch_unwind boundary"), all in **one** file's
`#[cfg(test)]` module:

```
crates/pdftract-core/src/document.rs  -> E0277 x4  (3288:47, 3323:51, 3350:47, 4022:55)
crates/pdftract-core/src/source/mmap.rs -> zero mentions in the entire log
```

Cause is **uncommitted sibling in-flight work, not committed HEAD**: `parser/xref.rs`
carries a dirty +205/−45 edit that gives `XrefResolver` a
`source: Option<Arc<dyn PdfSource>>` field (xref.rs:238), which fails the `UnwindSafe`
bound of the `catch_unwind` calls in `document.rs`'s tests; the `document.rs` dirty edit
itself is unused-import polish. No commit in `fff28e0c..HEAD` touches `document.rs` —
consistent with child 1's GO 11 minutes earlier.

## Invocation 2 — clean-HEAD replica: gate re-derived GO (the decisive check)

Per the repo's established replica methodology (the `default_rust` gate, and commit
`454a0daf`'s mutation check, both run git-archive HEAD extractions with a private target
dir — the shared checkout was never modified or stashed):

- `git archive cf554653 | tar -x` into `/data/tmp-pdftract-c2-headgate`, warm-cloned via
  `cp -al /build/target-workers <replica>/target` (hardlinks, same filesystem — the
  shared artifacts are never written; cargo replaces files by rename, leaving the
  hardlinked originals intact).
- `CARGO_TARGET_DIR=<replica>/target timeout --kill-after=30s 600s cargo test -p
  pdftract-core --lib --no-run` — 2026-09-14T12:56:46→12:57:29Z, exit **0**, **0 errors**,
  202 warnings, ``Finished `test` profile … in 43.04s``, test executable
  `pdftract_core-01467a2e479d2729`.

**Clean HEAD `cf554653` compiles the lib test target.** The 4-error set of invocation 1
exists only in the dirty shared tree and is owned by whoever lands the `xref.rs` in-flight
work; per this bead's constraints it was not touched.

## Invocation 3 — the executed run (this bead's deliverable)

Same replica, the bead's command, run exactly once: `CARGO_TARGET_DIR=<replica>/target
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source::mmap` —
2026-09-14T12:58:05→12:58:53Z, cargo exit **0**. Verbatim per-test result lines:

```
running 25 tests
test source::mmap::tests::test_advise_sequential_overflow ... ok
test source::mmap::tests::test_advise_sequential ... ok
test source::mmap::tests::test_advise_sequential_past_eof ... ok
test source::mmap::tests::test_as_slice ... ok
test source::mmap::tests::test_empty_file ... ok
test source::mmap::tests::test_is_empty ... ok
test source::mmap::tests::test_len_matches_file_size ... ok
test source::mmap::tests::test_open_nonexistent_file ... ok
test source::mmap::tests::test_large_file ... ok
test source::mmap::tests::test_open_valid_file ... ok
test source::mmap::tests::test_prefetch ... ok
test source::mmap::tests::test_prefetch_past_eof ... ok
test source::mmap::tests::test_read_mixed_with_seek ... ok
test source::mmap::tests::test_prefetch_madvise_failure_is_traced ... ok
test source::mmap::tests::test_read_range ... ok
test source::mmap::tests::test_read_range_overflow ... ok
test source::mmap::tests::test_read_range_past_eof ... ok
test source::mmap::tests::test_read_range_partial ... ok
test source::mmap::tests::test_read_trait ... ok
test source::mmap::tests::test_seek_before_start ... ok
test source::mmap::tests::test_seek_from_end ... ok
test source::mmap::tests::test_seek_trait ... ok
test source::mmap::tests::test_stream_position ... ok
test source::mmap::tests::test_send_sync ... ok
test source::mmap::tests::test_sync_multiple_threads ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 3562 filtered out; finished in 0.00s
```

The three tests this bead names, all **PASS** at HEAD `cf554653`:

| Test | Location | Result |
|---|---|---|
| `test_prefetch` | mmap.rs:430 | PASS |
| `test_prefetch_past_eof` | mmap.rs:439 | PASS |
| `test_prefetch_madvise_failure_is_traced` | mmap.rs:537 | **PASS** — first runtime execution of the pinning test for criterion (a) |

25/25 also matches the workspace run recorded in `454a0daf`'s commit message.

## Invocation 4 — optional RUST_LOG=trace re-run

`CARGO_TARGET_DIR=<replica>/target RUST_LOG=trace … cargo test -p pdftract-core --lib
source::mmap` — 2026-09-14T12:59:41→13:00:30Z, exit **0**, same 25/25. Zero trace lines on
stderr: the only `madvise` occurrence in the whole log is the test *name*. This is by
design, not a gap — the pinning test captures events with a hand-rolled
`tracing::Subscriber` installed via `tracing::subscriber::with_default` and the test
binary installs no global subscriber, so `RUST_LOG` cannot surface the event. The in-test
assertion (offset=0, length=100, file_len=10, non-empty error naming the EOF rejection,
message naming madvise; in-range prefetch emits nothing) **is** the runtime proof that the
`tracing::trace!` call fires on the failure-injection path — and `454a0daf`
mutation-verified exactly that (removing the `trace!` call or the `file_len` field fails
the test).

## No fix bead filed

No test failed (25/25 across both runs), so the "On genuine test FAIL: file a fix bead"
criterion does not trigger. The one defect-shaped finding — dirty-tree-only E0277 x4 in
`document.rs` tests from the in-flight `xref.rs` edit — belongs to the sibling worker
landing that work, not to this evidence bead; filing a bug against committed HEAD would
be false (HEAD compiles, invocation 2).

## Provenance

| Artifact | Content |
|---|---|
| `notes/b716bac5-child2-headgate-raw.log` | invocation 2 verbatim (clean-HEAD compile gate, exit 0) |
| `notes/b716bac5-child2-mmap-run-raw.log` | invocation 3 verbatim (primary executed run, build warnings + test output) |
| `notes/b716bac5-child2-mmap-run-trace.log` | invocation 4 verbatim (RUST_LOG=trace re-run) |
| `/data/tmp-pdftract-c2-headgate` | replica — **removed** after logs were captured |

## Compliance

- No file under `crates/` was created, edited, or reverted by this bead; the shared
  checkout's dirty state (84 paths) is exactly as found.
- All four invocations timeout-wrapped, strictly sequential, never overlapping; no retry
  of a hung command (none hung).
- The replica used a private target dir via hardlink clone; `/build/target-workers` was
  never written to by this bead.
- Raw evidence committed alongside this note (three logs, ~4,700 lines).
- Orphan check at close: `pgrep -af "cargo[ ]test|pdftract[ ]mcp|rustc"` shows nothing
  this bead spawned; the replica directory is removed.
- Coordination: the pinning test was landed by pdftract-931e6649 (commit `454a0daf`);
  this bead *executed* it rather than duplicating it, per the bead's reference note.
