# pdftract-aa239a61 — deterministic mmap prefetch trace capture

## Root cause

The production path is deterministic: on Unix, `prefetch(0, 100)` for the
four-byte test mapping is rejected by `advise_sequential` before any madvise
syscall, and the `Err` arm unconditionally calls `tracing::trace!`. The lost
event was in the test capture.

This workspace uses tracing 0.1.44 with tracing-core 0.1.36. Constructing a
scoped subscriber registers its `Dispatch` and rebuilds callsite interest
before `tracing::subscriber::with_default` installs that dispatch as the
thread-local default. When it is the only registered dispatcher, tracing-core's
single-dispatcher fast path can therefore evaluate interest through the bare
default dispatcher and cache `Interest::never`. A concurrent first evaluation
of this callsite can then short-circuit `trace!` before the test subscriber is
consulted. This is the hypothesized mechanism from the runtime evidence; the
relevant tracing-core behavior was verified in the locked dependency source.

## Mitigation

The test-only capturing closure calls
`tracing::callsite::rebuild_interest_cache()` after entering
`with_default`. At that point the capturing dispatcher is active, so the
callsite is rebuilt with the subscriber that will receive the event. The
subscriber continues to return `Interest::sometimes()` so subsequent
dispatcher churn cannot replace the cache with a static `never` while it is
registered. No sleep or timing assumption is used, and production
`MmapSource::prefetch` is unchanged.

If a bare-thread test wins the callsite's first registration immediately
after that rebuild, the capture retries once when the first round is empty.
The retry is bounded and not timing-based: the first round has completed the
callsite registration, so constructing the second dispatcher rebuilds the
registered callsite with the capturing subscriber included.

The test retains the one-event/zero-event split and field-level assertions:
the in-range prefetch must capture zero events, while the past-EOF prefetch
must capture exactly one TRACE event containing offset, length, file_len,
error, and a message naming `madvise(MADV_SEQUENTIAL)`.

## Verification

Committed source was extracted from `HEAD` into `/tmp/tmp.fv5y4lowP3`.
`scripts/definition-of-done.sh --fast` passed with exit 0. The required
filtered test command then ran consecutively without overlap:

| Runs | Command mode | Result |
|---|---|---|
| 1–5 | `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source::mmap` | 25 passed, 0 failed; exit 0 each |
| 1–5 | `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source::mmap -- --test-threads=1` | 25 passed, 0 failed; exit 0 each |

The parallel runs satisfy the minimum five-run requirement. Targeted
`rustfmt --edition 2021 --check crates/pdftract-core/src/source/mmap.rs`
passed. `cargo clippy -p pdftract-core --lib --tests` was also attempted but
is not green at this repository baseline: it reports unrelated deny-level
lint errors in existing layout, parser, document, and Type3 files; no clippy
diagnostic was reported for the changed mmap test. The initial filtered test
run on the shared checkout also passed 25/25.
