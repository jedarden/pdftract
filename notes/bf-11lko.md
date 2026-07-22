# bf-11lko — Verify truncated-flate.pdf opens without panic

## Scope

Verify `PdfExtractor::open()` completes without panic on `truncated-flate.pdf`,
the handle is functional, and add assertions for `fingerprint()` and
`page_count()`. Depends on bf-2dbmo (opens the file first); parent bf-50dny.

## What was done

The smoke test `test_truncated_flate_opens_with_extractor` already existed in
`crates/pdftract-core/tests/test_truncated_flate_recovery.rs` (from bf-2dbmo).
It opened the fixture and asserted a non-empty `fingerprint()`, but only
*printed* `page_count()` without asserting it. Strengthened the test to match
this bead's acceptance criteria:

- `fingerprint()` asserted non-empty (`!is_empty()`).
- `page_count()` now asserted via `.expect(...)` to confirm it resolves to a
  valid `Ok(count)` without error/panic.

For this truncated fixture `page_count()` returns `Ok(0)` — a valid count; the
structurally-declared page is not enumerable after the FlateDecode truncation
(a fixture/parser concern owned by sibling beads, out of scope here).

## Verification run

```
cargo test --package pdftract-core --test test_truncated_flate_recovery \
  test_truncated_flate_opens_with_extractor -- --nocapture
```

Output:

```
running 1 test
✓ PdfExtractor::open() succeeded without panic
  Fingerprint: pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
  Validated page count: 0
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out
```

## Acceptance criteria

| Criterion | Status |
|---|---|
| Test runs without panic when calling `PdfExtractor::open()` | PASS |
| `extractor.fingerprint()` returns a non-empty string | PASS |
| `extractor.page_count()` returns a valid page count (`Ok(0)`) | PASS |
| Test passes via `cargo test --package pdftract-core test_truncated_flate_opens_with_extractor` | PASS |

## Artifacts

- Test file (assertions strengthened): `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
- Note: `notes/bf-11lko.md` (this file)
