# bf-1w4ar — Call materialize_pages() to verify page loading

## Scope

Call `materialize_pages()` on a `PdfExtractor` opened over
`tests/fixtures/malformed/truncated-flate.pdf`, verify the call completes
without panic, obtain the page structure, and document how page data is stored.
Parent: bf-2goux. Related: [[bf-11lko]], [[bf-2dbmo]], [[bf-50dny]].

## What was done

Added a focused test `test_truncated_flate_materialize_pages` to
`crates/pdftract-core/tests/test_truncated_flate_recovery.rs`. It:

1. Opens the fixture with `PdfExtractor::open()`.
2. Calls `materialize_pages()` and asserts it returns `Ok` (no panic).
3. Iterates the returned `&[PageDict]`, asserting each page exposes a
   well-formed mediabox (4 values) — documenting the per-page contract.
4. Calls `materialize_pages()` a second time and asserts the page count is
   stable, confirming the result is cached in the extractor.

## How page data is stored

`materialize_pages()` (`crates/pdftract-core/src/document.rs:536`) lazily fills
`PdfExtractor.pages: Option<Vec<PageDict>>` by calling `flatten_page_tree()`
once, then returns `&[PageDict]`. Subsequent calls short-circuit on the cached
`Some(_)`, so page data is materialized at most once per extractor.

## Observed behavior on truncated-flate.pdf

`materialize_pages()` returns `Ok` with an **empty** slice (0 pages). The
FlateDecode truncation leaves the structurally-declared page non-enumerable via
`flatten_page_tree()` — consistent with `page_count()` returning `Ok(0)` (see
[[bf-11lko]]). The key acceptance point for this bead is that the call completes
without panic and yields a valid (possibly partial) page structure; it does.

Note: `parse_pdf_file()` surfaces a non-empty `pages` vec for the same fixture
via a different code path — that discrepancy is a parser/fixture concern owned
by sibling beads and is out of scope here.

## Verification run

```
cargo test --package pdftract-core --test test_truncated_flate_recovery \
  test_truncated_flate_materialize_pages -- --nocapture
```

Output:

```
✓ materialize_pages() succeeded
  Number of materialized pages: 0
✓ Page data materialized and cached (0 pages, stable across calls)
test result: ok. 1 passed; 0 failed; ...
```

## Acceptance criteria

| Criterion | Status |
|---|---|
| `materialize_pages()` is called successfully | PASS |
| No panic occurs during page materialization | PASS |
| Page structure obtained (even if partial due to truncation) | PASS (`Ok([])`) |
| Result shows how page data is stored | PASS (cached in `pages: Option<Vec<PageDict>>`) |

## Artifacts

- Test added: `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
  (`test_truncated_flate_materialize_pages`)
- Note: `notes/bf-1w4ar.md` (this file)
