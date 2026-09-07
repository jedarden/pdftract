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

**Handoff:** once `classify.rs` / `type3_rasterizer.rs` / `signature/mod.rs` /
`word_boundary.rs` settle, re-run
`timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source::mmap` and
expect `test_prefetch_madvise_failure_is_traced`, `test_prefetch` and
`test_prefetch_past_eof` to pass. Until then, criterion (a) stands as verified by
code-path analysis (file:line evidence above); the WARN is environmental.
