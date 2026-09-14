# bf-5o22rf — child 3 synthesis: observability of prefetch madvise errors

**Bead:** pdftract-e6c4a13f (child 3 of the split chain under pdftract-de16d7aa;
original parent bf-5o22rf "Add Observability to Prefetch madvise Errors").
**Inputs:** child 1 code audit (pdftract-216e4ddc, closed) + child 2 test run
(pdftract-ff35ed32, closed, output saved to `/tmp/bf-5o22rf-mmap-test-output.txt`).
**Scope of this bead:** synthesis only — no production code changes.

## 1. Code audit (child 1 findings, re-verified against the tree)

File: `crates/pdftract-core/src/source/mmap.rs`. The trace logging itself landed in
commit `08ec8db8` ("feat(bf-4chy94): log madvise failures in MmapSource::prefetch at
trace level"); the working tree additionally carries 137 uncommitted in-flight
insertions on this file (doc comment + dedicated tests, sibling child-1 work), which
are what the line numbers below reflect.

`advise_sequential` — `mmap.rs:81-100`. Two failure paths:

| Path | Location | Behavior | Reaches the trace event? |
|---|---|---|---|
| `offset + length` overflow | `mmap.rs:85-87` — `start.checked_add(length)` → `ok_or_else` → `Err(InvalidInput, "overflow")` | Short-circuits before the EOF check | **YES** |
| Range past EOF | `mmap.rs:89-94` — `end > self.mmap.len()` → `Err(InvalidInput, "range extends beyond EOF")` | Explicit early return | **YES** |

A third, non-`Err` path: `mmap.advise_range(Advice::Sequential, …)` failure is mapped
to `io::Error` at `mmap.rs:96-98` (a real kernel madvise failure) — same `Err`
channel, so it reaches the trace event too.

`prefetch` — `mmap.rs:134-153`. It calls `advise_sequential` inside
`if let Err(e) = …` at `mmap.rs:140` and emits the `tracing::trace!` event at
`mmap.rs:144-151` with all four documented fields — `offset`, `length`,
`file_len = self.len()`, `error = %e` — plus a message naming the failed call. There
is no intermediate wrapping, swallowing, or `?` between `advise_sequential`'s return
and prefetch's match, so **every** `Err` from either validation path (and from a real
kernel madvise failure) unconditionally reaches the trace event, while `prefetch`
still returns `()` and never propagates.

Success path: when `end <= self.mmap.len()`, `advise_sequential` calls
`advise_range` and returns `Ok(())` at `mmap.rs:99`. `if let Err` does not match
`Ok`, so a successful call emits **no** trace event — confirmed silent. This is also
asserted by the in-flight test `test_prefetch_madvise_failure_is_traced`
(`mmap.rs:536-583`), which requires zero events for an in-range `prefetch(0, 10)` and
exactly one event with all four fields for a past-EOF `prefetch(0, 100)`.

The module docs at `mmap.rs:8-15` document the contract: advisory, not propagated,
but always observable at trace level.

## 2. Test run (child 2 output + one re-run by this bead)

Command (as specified by child 2, per the repo's test-hygiene rule):

```
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source::mmap 2>&1 | tee /tmp/bf-5o22rf-mmap-test-output.txt | tail -40
```

Child 2's saved output: `/tmp/bf-5o22rf-mmap-test-output.txt` (2,274 lines). Final
lines:

```
error: could not compile `pdftract-core` (lib test) due to 95 previous errors; 129 warnings emitted
```

**No tests executed.** `timeout` exited 101 (cargo compile failure — a timeout would
be 124; it never fired). The `source::mmap` filter never got a chance because the
whole `--lib` test target fails to compile. The string "mmap" appears nowhere in the
output. The 95 errors are all in *test modules* of other files, none in
`source/mmap.rs`: 58× E0609 on `Result<PageClassification, ClassificationError>`
field access in `classify.rs`, 25× E0061 argument-count mismatches
(`font/type3_rasterizer.rs`, `signature/mod.rs`, `word_boundary.rs`), 5× E0599
`CharProcType::Unknown` in `type3_rasterizer.rs`, 3× E0277, 2× E0308, plus arithmetic
mismatches — i.e. sibling workers' in-flight edits whose test-module assertions have
not caught up with signature changes.

Re-verified by this bead on 2026-09-07 23:32 UTC, same command, output saved to
`/tmp/bf-5o22rf-mmap-test-output-child3.txt`:

```
error: could not compile `pdftract-core` (lib test) due to 95 previous errors; 129 warnings emitted
EXIT: 101
```

The error-line list is byte-identical to child 2's run (`diff` of the sorted
`^error` lines is empty), so the failure is stable across three runs (child 2 run 1,
child 2 retry, this bead's re-run), not a transient mid-edit state. Fixing those
files would be a production-code change, which this bead forbids and which would
collide with the sibling workers editing them.

## 3. Verdict on parent criterion (a)

Parent bf-5o22rf criterion (a): *"Log madvise failures at trace level for debugging."*

**PASS** — with a **WARN** rider on runtime test evidence.

> **[SUPERSEDED 2026-09-14, pdftract-ec0e6526:** the WARN rider is resolved — the
> pinning test has now executed and passes. Verdict stands at **PASS with no open
> rider**; see `## Runtime evidence (source::mmap)` at the end of this note.**]**

- **PASS (substantive):** the implementation satisfies the criterion. Both validation
  failure paths (`mmap.rs:85-87` overflow, `mmap.rs:89-94` past EOF) and the kernel
  madvise-failure path (`mmap.rs:96-98`) return `Err` and are unconditionally caught
  by `prefetch` at `mmap.rs:140`, emitting the structured `tracing::trace!` event at
  `mmap.rs:144-151` with `offset`/`length`/`file_len`/`error`. Successful calls emit
  nothing. The behavior is also documented in-module (`mmap.rs:8-15`), and a
  dedicated field-level assertion test (`test_prefetch_madvise_failure_is_traced`,
  `mmap.rs:536-583`) exists to pin it.
- **WARN (infra, not a defect):** the pinning test has not executed. All three runs of
  the mmap test suite fail to compile the `pdftract-core` `--lib` test target with 95
  identical errors, zero of them in `source/mmap.rs`, so zero tests ran. This is
  blocked by unrelated in-flight sibling edits in `classify.rs`,
  `font/type3_rasterizer.rs`, `signature/mod.rs` and `word_boundary.rs` — out of scope
  for this bead and not fixable without a production-code change that would collide
  with those workers.
  **[SUPERSEDED 2026-09-14, pdftract-ec0e6526:** the `--lib` test target now compiles
  clean at HEAD and the pinning test has executed — see
  `## Runtime evidence (source::mmap)`. The compile-blocked state described here no
  longer exists at the tip.**]**

**Handoff:** once `classify.rs` / `type3_rasterizer.rs` / `signature/mod.rs` /
`word_boundary.rs` settle, re-run
`timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source::mmap` and
expect `test_prefetch_madvise_failure_is_traced`, `test_prefetch` and
`test_prefetch_past_eof` to pass. Until then, criterion (a) stands as verified by
code-path analysis (file:line evidence above); the WARN is environmental.

> **[SUPERSEDED 2026-09-14, pdftract-ec0e6526:** the re-run happened — see
> `## Runtime evidence (source::mmap)`. Expected outcome confirmed: all three tests
> pass at HEAD; criterion (a) now rests on executed-test evidence. One new finding:
> the pinning test is flaky under parallel test execution (fix bead
> pdftract-aa239a61).**]**

## 4. Follow-up bead (filed by pdftract-2efded34)

The verdict is PASS **with a WARN rider**, and the task for pdftract-2efded34
routes any FAIL/WARN outcome to a follow-up bead rather than a clean "no follow-up
needed" close. The gap is not in the implementation — every `Err` path reaches the
trace event — it is that **criterion (a) has no runtime test evidence**: the pinning
test has never executed because the `--lib` test target fails to compile on
unrelated sibling edits.

Follow-up: **pdftract-b716bac5** — "Re-run source::mmap tests for runtime evidence
of the prefetch madvise trace event (bf-5o22rf WARN handoff)". It owns the handoff
paragraph above: precondition-check the compile, re-run the mmap suite once it
passes, append a runtime-evidence section here converting the WARN to PASS (or a
recorded FAIL + fix bead), and cross-link the outcome on the umbrella note bead
pdftract-adda71a4. It explicitly forbids touching the four files breaking the
build.

## Runtime evidence (source::mmap)

**Added 2026-09-14 by pdftract-ec0e6526 — resolves the §3 WARN rider.** Criterion (a)
now has executed-test evidence; disposition at the end of this section. Inputs: the
re-run chain's child 2 (pdftract-030e8414, `notes/b716bac5-child2.md`, with committed
raw logs) plus this bead's own clean-HEAD replica re-derivation at the current tip.

### Run history

| # | UTC (2026-09-14) | HEAD | Mode | Result |
|---|---|---|---|---|
| 1 | 12:58:05→12:58:53Z | `cf554653` | parallel (default) | 25/25 PASS — pdftract-030e8414 invocation 3 |
| 2 | 12:59:41→13:00:30Z | `cf554653` | parallel + RUST_LOG=trace | 25/25 PASS — pdftract-030e8414 invocation 4 |
| 3 | 13:17:16→13:17:38Z | `c763122a` | parallel (default) | **24/25 — `test_prefetch_madvise_failure_is_traced` FAILED** |
| 4 | 13:23:31→13:23:53Z | `c763122a` | `--test-threads=1` | 25/25 PASS |
| 5 | 13:24:33→13:25:00Z | `c763122a` | parallel (default) | 25/25 PASS |

Runs 3–5 were executed by this bead (pdftract-ec0e6526) in a clean-HEAD replica
(`git archive HEAD` extraction, hardlink-cloned target dir; replica removed after
capture, raw logs at `/tmp/ec0e6526-headrun*.log` — transient). Each run recompiled
`pdftract-core` from the extracted source (`Compiling pdftract-core` present in every
log), so no run reused a binary built from another tree. Every cargo invocation ran
exactly once, timeout-wrapped, strictly sequential — no overlapping retries.

**All five runs test the same production code:** `git diff --stat cf554653..c763122a`
touches only `notes/` — zero `crates/` changes.

### Command

```
CARGO_TARGET_DIR=<replica>/target timeout --kill-after=30s 600s \
  cargo test -p pdftract-core --lib source::mmap
```

Run 4 appended `-- --test-threads=1`. Runs 1–2 used the same command shape in
pdftract-030e8414's own replica (that bead's note records its exact invocations and
committed raw logs `notes/b716bac5-child2-mmap-run-raw.log` /
`notes/b716bac5-child2-mmap-run-trace.log`).

### Verbatim per-test lines (run 1; raw log `notes/b716bac5-child2-mmap-run-raw.log`)

```
test source::mmap::tests::test_prefetch ... ok
test source::mmap::tests::test_prefetch_past_eof ... ok
test source::mmap::tests::test_prefetch_madvise_failure_is_traced ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 3562 filtered out; finished in 0.00s
```

Runs 2, 4 and 5 produced the same 25/25 summary line; run 4 (serial) explicitly
printed `test source::mmap::tests::test_prefetch_madvise_failure_is_traced ... ok`.

### The failing run (run 3) — verbatim

```
thread 'source::mmap::tests::test_prefetch_madvise_failure_is_traced' (939189) panicked at crates/pdftract-core/src/source/mmap.rs:563:9:
assertion `left == right` failed: expected exactly one trace event: []
  left: 0
 right: 1

test result: FAILED. 24 passed; 1 failed; 0 ignored; 0 measured; 3562 filtered out; finished in 0.00s
```

The test's earlier in-range assertion (`prefetch(0, 10)` must emit nothing) passed;
the past-EOF `prefetch(0, 100)` yielded a **zero-event capture** where exactly one was
expected.

### Analysis: capture race in the test harness, not a production defect

- The failure path is pure arithmetic, not kernel- or environment-dependent:
  `advise_sequential` rejects `end > self.mmap.len()` before any `madvise` syscall
  (`mmap.rs:89-94`, §1 above), and `memmap2` 0.9.10 stores the **requested** mapping
  length (10 bytes here; `MmapInner::adjust_mmap_params` coerces only zero-size maps),
  so `prefetch(0, 100)` on a 10-byte file is unconditionally past-EOF → `Err` → the
  `tracing::trace!` call is unconditionally attempted. Source-identical runs 1, 2, 4
  and 5 all captured the event, in both parallel and serial modes.
- The zero-event capture in run 3 is therefore a delivery/capture race under the
  default parallel test runner. Four other tests in the same binary install their own
  scoped dispatchers via `tracing::subscriber::with_default`
  (`font/type3_rasterizer_test.rs`, `forms/value_choice.rs`, `forms/value_text.rs`,
  `type3_test_fixtures.rs`), so the binary churns dispatchers concurrently. A race in
  `tracing`'s callsite-interest machinery is the **hypothesized** mechanism — not
  established by this bead. What is established is the empirical signature:
  intermittent (1 failure in 5 runs), parallel-mode-only in the one observed failure,
  serial clean.

### Disposition

**Criterion (a): PASS at runtime.** `test_prefetch`, `test_prefetch_past_eof` and
`test_prefetch_madvise_failure_is_traced` all pass at HEAD `cf554653` (runs 1–2) and
at HEAD `c763122a` (runs 4–5); combined with the unconditional code path above, the
criterion's runtime behavior — madvise failures logged at trace level with
`offset`/`length`/`file_len`/`error` — is confirmed by execution. The §3 WARN
("pinning test never executed") is resolved; the parent verdict is **PASS with no
open rider**.

New, distinct finding (not a criterion-(a) defect): the pinning test is **flaky under
parallel test execution** — fix bead **pdftract-aa239a61** filed, carrying the run
history above as evidence. Until it lands, a one-off red
`test_prefetch_madvise_failure_is_traced` in a parallel `cargo test` run is not, by
itself, evidence of a criterion-(a) regression; discriminate with
`-- --test-threads=1`.
