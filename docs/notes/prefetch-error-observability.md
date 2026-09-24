# Prefetch error observability

## Decision

`PdfSource::prefetch` is intentionally infallible: it returns `()` and is an
advisory hint, not part of reading the PDF. Local sources use `madvise` to
improve readahead; the HTTP source speculatively warms its range cache. A
failure loses a performance optimization only. The later `read_range` still
performs the real read (including a synchronous HTTP fetch), so propagating a
prefetch error would turn a performance condition into a correctness failure.

Parent `bf-3a1p3b` offered two choices: (a) trace-level logging of `madvise`
failures, or (b) a comment justifying silent failure. Option (a) was chosen
and implemented in commit `08ec8db8` (bead `bf-4chy94`). Option (b) was
rejected as insufficient: a comment explains the contract but gives no way to
see bad-range calculations or environmental failures while debugging. The
comments remain alongside the behavior; the trace event is the durable
observability mechanism.

## Mmap event

`MmapSource::prefetch` catches `advise_sequential` errors and emits a
`tracing::trace!` event before returning. Its structured fields are:

- `offset` and `length`: the requested byte range, identifying the bad input.
- `file_len`: the mapped file length, making a past-EOF calculation visible at
  a glance when compared with `offset + length`.
- `error`: the underlying I/O error, such as the `InvalidInput` produced for an
  overflow or a range beyond EOF.

The exact Rust module path is `pdftract_core::source::mmap`. With the CLI's
`tracing_subscriber` environment filter, enable this event with:

```text
RUST_LOG=pdftract_core::source::mmap=trace pdftract extract FILE.pdf
```

`RUST_LOG=pdftract_core::source=trace` is the broader filter for all source
implementations. It also includes the HTTP implementation's debug-level
prefetch events.

## Implementation inventory

- `PdfSource` default, `crates/pdftract-core/src/source/mod.rs:100-109`, is a
  documented no-op. Implementations without a prefetch strategy inherit it.
- `MmapSource::prefetch`, `crates/pdftract-core/src/source/mmap.rs:142-160`,
  logs failed `madvise(MADV_SEQUENTIAL)` calls at trace level and swallows the
  error.
- `TempMmapSource::prefetch`,
  `crates/pdftract-core/src/source/mod.rs:471-473`, delegates to
  `MmapSource::prefetch`, so it inherits the same event and infallible behavior.
- `HttpRangeSource::prefetch`,
  `crates/pdftract-core/src/source/http_range.rs:483-533`, swallows failed
  range fetches and logs each missing-block run with `run_start`, `run_end`,
  and `error` at debug level. A trace filter includes these events.
