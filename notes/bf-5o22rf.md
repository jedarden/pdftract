# bf-5o22rf umbrella verification

**Parent:** `bf-5o22rf` — Add Observability to Prefetch madvise Errors
**Consolidation child:** `pdftract-adda71a4`
**Date:** 2026-09-24

This note consolidates the four child evidence chains. The parent acceptance
criteria are reproduced below exactly as the alternatives described by the
parent bead:

| Criterion | Result | Evidence |
|---|---|---|
| **(a) Log madvise failures at trace level for debugging.** | **PASS** | `MmapSource::prefetch` catches every `advise_sequential` error and emits structured `tracing::trace!` fields `offset`, `length`, `file_len`, and `error`. The current clean-HEAD `source::mmap` run passed 25/25, including the trace-event regression. |
| **(b) Document why silencing is intentional with a code comment.** | **PASS** | The advisory/no-correctness-impact contract and deliberate default no-op are documented at `crates/pdftract-core/src/source/mod.rs:107-118`; the implementation comments explain non-propagation at `mmap.rs:143-147` and `http_range.rs:517-524`. Criterion (a) is the selected parent alternative, so the parent OR is satisfied even without relying on (b). |

## Consolidated code evidence

The required parent-scope anchors are:

- `crates/pdftract-core/src/source/mmap.rs:134-153`: the parent’s original
  `MmapSource::prefetch` scope. At the current tip the same implementation is
  shifted to `mmap.rs:142-159`; `advise_sequential` rejects overflow at
  `mmap.rs:89-92`, rejects past-EOF ranges at `mmap.rs:94-99`, maps kernel
  advice errors at `mmap.rs:101-103`, and the `Err` arm traces all four fields
  at `mmap.rs:148-159`. The success path returns `Ok(())` and is silent.
- `crates/pdftract-core/src/source/http_range.rs:446-482`: the parent’s
  source-range scope. At the current tip `HttpRangeSource::prefetch` begins at
  `http_range.rs:483`; its batched fetch loop logs failed runs at
  `http_range.rs:515-533` with `run_start`, `run_end`, and `error` at debug
  level, while retaining `()` best-effort behavior and allowing later reads to
  fetch synchronously.
- `crates/pdftract-core/src/source/mod.rs:107`: the trait contract anchor.
  The current rustdoc at `mod.rs:107-118` says prefetch cannot fail the caller
  or change read correctness, failures only reduce readahead performance,
  structured tracing is expected (`trace` for local madvise and `debug` for
  network fetches), and the default is a deliberate no-op.

## Per-child evidence

1. **Child 1 — `pdftract-de16d7aa` (mmap audit and runtime evidence).**
   `notes/bf-5o22rf-child1.md` records that all Unix failure paths reach the
   structured trace event and that success remains silent. The original
   implementation is `08ec8db8`; the runtime verdict is recorded by
   `pdftract-ec0e6526` in `761218a3`, and the current clean-HEAD acceptance
   refresh is `pdftract-0fd3417e` in `a3db4397`. The five historical runtime
   runs included 25/25 passes on both tested heads; one parallel run exposed a
   test-harness capture race, while serial and subsequent parallel runs passed.
   The separate flake follow-up is `pdftract-aa239a61`; it is not a production
   trace-path failure.

2. **Child 2 — `pdftract-a8f096bc` (mmap regression assertions).** Commit
   `98413454b6954c7ba28f73ce881b85854f3670e2` adds assertions for silent
   in-range prefetch, past-EOF trace fields/message, and the overflow path.
   Its recorded focused run passed 25 tests under the required timeout.

3. **Child 3 — `pdftract-9009aa36` (HTTP-range observability).** Commit
   `8ad420ebd7e94e190739742f318345d16d5aa172` changes the discarded fetch
   error to a structured `tracing::debug!` event while retaining the
   non-propagating contract, and its regression test confirms a later
   `read_range` succeeds after the origin recovers. Its recorded source-module
   run passed under the required timeout.

4. **Child 4 — `pdftract-76df861c` (trait contract documentation).** Commit
   `afd957986fa60b52e067bb8801e257bfb2750442` documents advisory semantics,
   correctness preservation, expected structured observability, and the
   deliberate no-op default. Its recorded `cargo doc` and source-module test
   commands passed; the note also records unrelated full-suite CLI failures.

## Verification disposition

The earlier child-1 compile-blocked WARN is superseded by the executed
clean-HEAD evidence: `source::mmap` ran 25 tests with zero failures, including
`test_prefetch`, `test_prefetch_past_eof`, and
`test_prefetch_madvise_failure_is_traced`. No failure was caused by the
implementation. The known parallel capture race is retained as a WARN in the
history and tracked by `pdftract-aa239a61`; it does not change criterion (a)’s
PASS verdict.

## Final committed-HEAD checks

The mandatory clean extraction was made from verification commit `c1872f3d` at
`/home/coding/scratch/pdftract-adda71a4-final-dod.6kfPdM`; the subsequent
note-only commit `340d9b95` changes no source or tests. The repository has no
`scripts/definition-of-done.sh`, so the default definition was used:
`cargo build --all-targets && cargo test`. The build passed (exit 0), but the
full test command exited 101 after 354 passed and 7 unrelated
`pdftract-cli` unit tests failed (inspect SVG/MCID, pages, and URL behavior).
The extraction was retained for diagnosis as required; this is a repository
baseline WARN/FAIL for the broad definition-of-done gate, not a failure in the
source prefetch suite.

The required final command was run in that same committed extraction:

```text
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source 2>&1 | tail -40
```

It exited 0 (pipeline exit with `pipefail`), was not timeout-killed, and ended
with `114 passed; 0 failed; 0 ignored; 0 measured; 3570 filtered out`.
The trace regression `test_prefetch_madvise_failure_is_traced` and the other
source tests all passed.

The literal required process scan, `pgrep -af 'pdftract'`, matched only the
active Codex/Needle supervisor command lines and the scan shell itself. A
follow-up inspection found no cargo, rustc, test, or pdftract worker binary
spawned by this bead left behind; unrelated workers from other repositories
were visible on the shared host. Therefore **no orphan task processes remain**
for this verification, and the scan was not evidence of a timeout or leaked
worker.

The umbrella `bf-5o22rf` is ready to close once this terminal child and its
dependency chain are closed; this child does not close the umbrella.
