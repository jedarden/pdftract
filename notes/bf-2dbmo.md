# bf-2dbmo — Open truncated-flate.pdf with PdfExtractor

## Scope

Open the `truncated-flate.pdf` fixture via `PdfExtractor::open()`, handling the
`Result` with `.expect()` (descriptive message). Depends on bf-1739m
(PdfExtractor instance creation), parent bf-50dny.

## What was verified

The `PdfExtractor::open()` call this bead requires already exists in the
committed test file `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
(originally added in commit `5f356195`, exercised further under bf-1739m in
`ad85f144`). Two tests in that file call `PdfExtractor::open()` with the fixture
path and unwrap via `.expect()`:

- `test_truncated_flate_opens_with_extractor` (line ~192):
  ```rust
  let extractor = PdfExtractor::open(&path)
      .expect("Should open truncated-flate.pdf with PdfExtractor");
  ```
- `test_truncated_flate_extraction_result_structure` (line ~125):
  ```rust
  let extractor = PdfExtractor::open(&path)
      .expect("Should open truncated_mid_stream.pdf with PdfExtractor");
  ```

API confirmed against `crates/pdftract-core/src/document.rs:432`:
```rust
pub fn open<P: AsRef<Path>>(pdf_path: P) -> Result<Self>
```

Fixture confirmed present:
`tests/fixtures/malformed/truncated-flate.pdf` (588 bytes), resolved by the
`fixture_path()` helper (`CARGO_MANIFEST_DIR/../../tests/fixtures/malformed/truncated-flate.pdf`).

## Verification run

```
cargo nextest run -p pdftract-core --test test_truncated_flate_recovery
```

Result (relevant lines):

```
PASS [ 0.011s] test_truncated_flate_extraction_result_structure
PASS [ 0.012s] test_truncated_flate_opens_with_extractor
Summary [ 0.012s] 6 tests run: 4 passed, 2 failed, 0 skipped
```

Both `PdfExtractor::open()`-based tests **PASS** and the test binary compiles
with no errors related to the `open()` call.

## Acceptance criteria

| Criterion | Status |
|---|---|
| `PdfExtractor::open()` called with the fixture path | PASS |
| Result unwrapped with `.expect()` + descriptive message | PASS |
| Test compiles successfully | PASS |
| No compile-time errors related to the `open()` call | PASS |

## WARN (out of scope — sibling beads)

Two tests in the same file FAIL, but they do **not** use `PdfExtractor::open()`;
they use the separate `parse_pdf_file` API and assert `pages.len() > 0`:

- `test_truncated_flate_parses_as_pdf` (FAIL: "Document should have at least one page")
- `test_truncated_flate_partial_content_accessible` (FAIL: "Should have at least one page")

Root cause (per bf-1739m close reason): the malformed fixture structurally
declares a page (`/Count 1` + `/Type /Page`) yet both parser entry points
enumerate 0 pages. This is a fixture/parser page-enumeration concern scoped to
sibling beads, not bf-2dbmo's `PdfExtractor::open()` scope.

## Artifacts

- Test file (committed, no changes this bead): `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
- Note: `notes/bf-2dbmo.md` (this file)
