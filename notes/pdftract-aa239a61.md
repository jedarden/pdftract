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

The test retains the one-event/zero-event split and field-level assertions:
the in-range prefetch must capture zero events, while the past-EOF prefetch
must capture exactly one TRACE event containing offset, length, file_len,
error, and a message naming `madvise(MADV_SEQUENTIAL)`.

## Verification

Verification results will be appended after the committed clean-extraction
run set.
