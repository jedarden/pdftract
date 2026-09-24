# bf-3a1p3b verification note

Verified 2026-09-24 against `main` at `5ad253ec` (before this note was
added). The parent offers two alternative ways to make prefetch failures
observable. The implementation chooses (a), and the parent is satisfied by
that choice; (b) is recorded as the unselected alternative rather than as a
second required implementation.

## Parent acceptance criteria

| Criterion | Verdict | Evidence |
|---|---|---|
| (a) Log `madvise` failures at trace level with enough context to diagnose them | **PASS — chosen** | `08ec8db8` (`crates/pdftract-core/src/source/mmap.rs:142-160`) catches `advise_sequential` errors and emits `tracing::trace!` fields `offset`, `length`, `file_len`, and `error`. The regression test landed in `454a0daf` and is present at `crates/pdftract-core/src/source/mmap.rs` in the `source::mmap` tests. |
| (b) Add a comment explaining why silencing is intentional and why observability is unnecessary | **WARN — not chosen** | The code has the best-effort rationale comment, but this comment-only alternative was rejected because it cannot expose bad-range calculations or environmental failures. `docs/notes/prefetch-error-observability.md` (committed in `5ad253ec`) records the choice and rejection explicitly. The parent passes through criterion (a). |

The related source implementations were also checked while consolidating the
children: `PdfSource::prefetch` remains the documented no-op at
`crates/pdftract-core/src/source/mod.rs:100-109`; `TempMmapSource::prefetch`
delegates to the mmap implementation at `:471-473`; and
`HttpRangeSource::prefetch` keeps failures infallible while recording
`run_start`, `run_end`, and `error` at debug level at
`crates/pdftract-core/src/source/http_range.rs:483-535`. The HTTP work was
implemented in `d691c5e2`, its compile fix is `4bdc3e9d`, and the later log
level/test naming adjustment is `8ad420eb`.

## Earlier child beads

- `pdftract-931e6649` — **CLOSED**: added and verified the mmap trace-event regression test in `454a0daf` (with the later assertion refinement in `98413454`).
- `pdftract-e1793e09` — **CLOSED**: added HTTP prefetch failure observability and its failure-injection test in `d691c5e2`; compile correction in `4bdc3e9d`.
- `pdftract-259b0053` — **CLOSED**: documented the option decision and implementation inventory in `docs/notes/prefetch-error-observability.md` in `5ad253ec`.

## Commit and path checks

All commits cited above (`08ec8db8`, `454a0daf`, `98413454`, `d691c5e2`,
`4bdc3e9d`, `8ad420eb`, and `5ad253ec`) are ancestors of `main`. Every cited
source or documentation path exists in the tree. The verification note itself
is this checked-in path: `notes/bf-3a1p3b.md`.

## Required test

The required timeout-wrapped fallback was run during this child. The pipeline
used `PIPESTATUS[0]` so the recorded status is the cargo/timeout status rather
than `tail`'s status:

```text
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib source:: 2>&1 | tail -80
exit=0
```

Actual output tail:

```text
warning: `pdftract-core` (lib test) generated 188 warnings (run `cargo fix --lib -p pdftract-core --tests` to apply 132 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 20.87s
     Running unittests src/lib.rs (/build/pdftract/debug/deps/pdftract_core-3d4addd5601841fa)

running 46 tests
test source::file_source::tests::test_empty_file ... ok
test source::file_source::tests::test_large_file ... ok
test source::file_source::tests::test_concurrent_read_range ... ok
test source::file_source::tests::test_open_nonexistent_file ... ok
test source::file_source::tests::test_open_valid_file ... ok
test source::file_source::tests::test_read_mixed_with_seek ... ok
test source::file_source::tests::test_read_range ... ok
test source::file_source::tests::test_read_range_bounds ... ok
test source::file_source::tests::test_read_range_past_eof_returns_err ... ok
test source::file_source::tests::test_read_seek ... ok
test source::file_source::tests::test_send_sync ... ok
test source::memory::tests::test_empty ... ok
test source::file_source::tests::test_sync_multiple_threads ... ok
test source::memory::tests::test_from_slice ... ok
test source::memory::tests::test_new ... ok
test source::memory::tests::test_read_range ... ok
test source::memory::tests::test_read_range_offset_past_end ... ok
test source::memory::tests::test_read_range_past_end ... ok
test source::memory::tests::test_read_trait ... ok
test source::memory::tests::test_seek_from_end ... ok
test source::memory::tests::test_seek_trait ... ok
test source::mmap::tests::test_advise_sequential ... ok
test source::mmap::tests::test_advise_sequential_overflow ... ok
test source::mmap::tests::test_advise_sequential_past_eof ... ok
test source::mmap::tests::test_as_slice ... ok
test source::mmap::tests::test_empty_file ... ok
test source::mmap::tests::test_is_empty ... ok
test source::mmap::tests::test_len_matches_file_size ... ok
test source::mmap::tests::test_large_file ... ok
test source::mmap::tests::test_open_nonexistent_file ... ok
test source::mmap::tests::test_open_valid_file ... ok
test source::mmap::tests::test_prefetch ... ok
test source::mmap::tests::test_prefetch_past_eof ... ok
test source::mmap::tests::test_prefetch_madvise_failure_is_traced ... ok
test source::mmap::tests::test_read_mixed_with_seek ... ok
test source::mmap::tests::test_read_range ... ok
test source::mmap::tests::test_read_range_overflow ... ok
test source::mmap::tests::test_read_range_past_eof ... ok
test source::mmap::tests::test_read_range_partial ... ok
test source::mmap::tests::test_read_trait ... ok
test source::mmap::tests::test_seek_before_start ... ok
test source::mmap::tests::test_seek_from_end ... ok
test source::mmap::tests::test_stream_position ... ok
test source::mmap::tests::test_send_sync ... ok
test source::mmap::tests::test_sync_multiple_threads ... ok

test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 3631 filtered out; finished in 0.00s

TEST_EXIT_CODE=0
```

## Clean-archive definition-of-done gate

The committed `HEAD` was extracted to `/tmp/tmp.dp3TLxVKmK` and verified
outside the shared worktree. `cargo build --all-targets` passed with exit 0.
The repository-default `cargo test` then exited 101, so the extraction was
retained for diagnosis as required. This failure is outside the scoped source
test: 353 tests passed and these 8 unrelated `pdftract-cli` tests failed:

```text
---- inspect::api::tests::test_render_page_svg_empty_page stdout ----
assertion failed: svg.contains("<g class=\"selection\"")
---- inspect::api::tests::test_render_page_svg_basic stdout ----
assertion failed: svg.contains("<g class=\"layer-columns\"")
---- inspect::render::tests::test_extract_columns_from_spans stdout ----
assertion `left == right` failed
---- pages::tests::test_parse_and_filter_out_of_range stdout ----
assertion `left == right` failed
---- pages::tests::test_parse_comma_separated stdout ----
assertion `left == right` failed
---- url::tests::test_parse_url_invalid stdout ----
assertion failed: matches!(result, Err(UrlError::MissingHost(_)))
---- url::tests::test_parse_url_urlencoded_credentials stdout ----
assertion `left == right` failed
---- url::tests::test_parse_url_with_empty_path stdout ----
assertion `left == right` failed

test result: FAILED. 353 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

error: test failed, to rerun pass `-p pdftract-cli --lib`
CARGO_TEST_EXIT_CODE=101
ARCHIVE_VERIFICATION=FAIL
RETAINED=/tmp/tmp.dp3TLxVKmK
```

Because this mandatory full-suite gate failed, the child remains open even
though this bead's scoped source test and all parent acceptance evidence pass.

## Orphan-process check

The requested check was run using the shell-safe equivalent pattern so the
checking shell cannot match its own command line:

```text
pgrep -af 'pdftract[ ]mcp|TH[_]0|TH[-]0'
ORPHAN_CHECK_EXIT_CODE=1
```

There was no output, so no matching `pdftract mcp`, `TH_0`, or `TH-0` process
was present. The parent `bf-3a1p3b` remains open; this note supplies evidence
only and does not close the umbrella.
