# pdftract-0df07688 — startxref/trailer detection: recover off-keyword startxref offsets

Dispatch HEAD: `eac1e1a2` (2026-09-22). Trees named explicitly: every verification run
below executed in a clean `git archive HEAD` extraction at `/data/df07688-dev` with a
private `CARGO_TARGET_DIR` (`/data/df07688-target`). The shared checkout carried ~63
foreign dirty files and was used for no test run. The fix commit was assembled from
that extraction via a temp index (`GIT_INDEX_FILE` + `git hash-object`/`update-index` +
`git commit-tree` + compare-and-swap `update-ref`) so no other worker's in-flight edits
could be swept in.

## What the bug was

`document.rs::find_startxref` scans the last 1024 bytes for the LAST `startxref`
keyword and returns the recorded offset. `parse_traditional_xref` then required the
`xref` keyword to sit exactly at (or within leading whitespace of) that offset and, at
HEAD, only recovered BACKWARD (recorded offset landing inside the table, past its
keyword). The corpus's dominant residual class records an offset a few bytes SHORT of
the keyword — 54 files (2026-09-21 re-check) landing inside the preceding object's
`endobj` — where the backward-only scan finds nothing, the section stays trailer-less,
and document.rs surfaces `No trailer in xref section`.

## Fixture shape

`tests/fixtures/realworld/startxref-offset-edge.pdf` (from pdftract-7ec0f722): 3,213
bytes, classic xref, 2-page NDA; `startxref` records 2949, which lands inside `endobj`
(`...ica >>\nendobj\nxref\n0 8\n...`); the true keyword sits at 2955, 6 bytes forward.
PyMuPDF 1.27.2 baseline: 2 pp / 2,050 chars.

## The fix (crates/pdftract-core/src/parser/xref.rs)

- New named constant `XREF_KEYWORD_RECOVERY_WINDOW: u64 = 1024` (matches the
  pre-existing backward-scan literal and `find_startxref`'s EOF window). Whole-file
  scan deliberately out of scope — the >1 MiB chunked-scan wedge is
  pdftract-c7c43f45's bead and is not touched here.
- On a keyword miss at the recorded offset: the backward scan (pre-existing) is joined
  by a FORWARD scan over the already-read header chunk; both look for the nearest
  boundary-delimited `xref` keyword (whitespace-delimited on both sides, so the `xref`
  tail of a `startxref` keyword never matches — a `t` precedes it). Nearest candidate
  wins; forward preferred on an exact tie. A recovery diagnostic
  `xref keyword not found at startxref offset; recovered xref at N` is retained.
- When neither scan finds a keyword, the section stays trailer-less with the distinct
  `xref keyword not found` diagnostic → upstream `No trailer in xref section`.
  ea0ed51c's taxonomy (trailer-loss vs /Root-resolution) is preserved and
  contract-tested.

Provenance note: the shared working tree carried an uncommitted edit of exactly this
change, abandoned by a dead prior dispatch of this bead (xref.rs mtime 09:44Z; attempt
1 logged 09:50Z, died with an API error before committing). It was reviewed line-by-line
(full `git diff`, 272 lines, all this-bead-related), adopted, and COMPLETED: the
stranded edit had updated the harness's class-map comment to "FIXED" without flipping
the pins; the pin flip is authored in this dispatch.

## Test changes (tests/realworld_extraction_baseline.rs)

- `pin_startxref_offset_edge_current_behavior`: was `extract_failure` asserting the
  `No trailer in xref section` class error; now asserts `extract_pages == 2`.
- `desired_startxref_offset_edge_extracts_like_pymupdf`: stays `#[ignore]`d, ignore
  reason re-targeted to pdftract-5b4c3d0e — the fixture's literals carry escaped
  parentheses (`\(startxref offset edge fixture\)`), so after trailer recovery the
  text layer is still empty (verified live, below). Full PyMuPDF parity (2,050 chars)
  belongs to that bead; this matches the harness's own convention for classes blocked
  by a second open bug (cf. linearized-10 / multipage-100).
- parser/xref.rs in-module (`trailer_loss_probe`): +2 tests —
  `recovers_startxref_value_short_of_keyword` (synthetic short-by-6 doc mirroring the
  fixture; asserts entries + trailer + /Root, the recovery diagnostic naming the true
  offset, and that the `load_single_xref` / `load_xref_with_prev_chain` wrappers
  preserve it) and `unrecoverable_keyword_keeps_distinct_trailer_loss` (keyword
  corrupted → trailer-less section, specific diagnostic, `resolve_root_ref` → exactly
  `No trailer in xref section`).

## Before/after evidence

BEFORE — pristine HEAD `eac1e1a2` CLI (clean extraction):

    $ pdftract extract tests/fixtures/realworld/startxref-offset-edge.pdf
    Error: Failed to extract PDF: tests/fixtures/realworld/startxref-offset-edge.pdf

    Caused by:
        No trailer in xref section
    (exit=1)

    $ pdftract extract tests/fixtures/valid-minimal.pdf   # repo-root
    pages: 1 chars: 0        # extracts Ok; silent-zero class = pdftract-4683109d
    $ pdftract extract tests/fixtures/sample.pdf
    pages: 1 chars: 0

AFTER — fix applied, same extraction:

    $ pdftract extract tests/fixtures/realworld/startxref-offset-edge.pdf
    exit=0, pages: 2 chars: 0   # 0 chars = escaped-paren class, pdftract-5b4c3d0e
    $ pdftract extract tests/fixtures/valid-minimal.pdf
    pages: 1 chars: 0           # unchanged
    $ pdftract extract tests/fixtures/sample.pdf
    pages: 1 chars: 0           # unchanged

Test targets (every run wrapped in `timeout --kill-after=30s <600–1500>s`; counts
BEFORE → AFTER; all in the clean extraction):

| target                                        | BEFORE  | AFTER                                |
|-----------------------------------------------|---------|--------------------------------------|
| `--test realworld_extraction_baseline`        | 8p/0f/6i | 8p/0f/6i (pin now asserts resolution) |
| `--test trailer_root_contract`                | 7p/0f   | 7p/0f (taxonomy intact)               |
| `--test test_xref_debug`                      | 1p/0f   | 1p/0f                                 |
| `--test hint_stream_integration`              | 4p/9f   | 4p/9f (9 pre-existing, unchanged)     |
| `--lib parser::xref`                          | 89p/9f  | 91p/9f (+2 new; same 9 pre-existing)  |
| `--lib trailer_loss_probe`                    | —       | 8p/0f (incl. the 2 new tests)         |

The 9 pre-existing `--lib` failures at BOTH ends: `test_forward_scan_{simple,
carriage_return, false_positive_handling, multi_revision, trailer_no_space,
truncated_file, with_generations, with_trailer}` and `test_parse_multi_subsection_xref`
— pre-existing at HEAD `eac1e1a2`, unrelated to this change (remote forward_scan_xref
feature); count NOT grown. `document_model.rs` not run (fails 15/15 at pristine HEAD
via CWD-relative fixtures — pre-existing, documented in the harness header).

## Acceptance criteria

1. PASS — repo-root `tests/fixtures/valid-minimal.pdf` + `sample.pdf` still extract
   (CLI, before/after identical); the startxref-edge fixture now extracts (2 pages).
2. PASS — targeted tests pass (exact `--test` targets in the table above, all under
   `timeout --kill-after=30s`); pre-existing failure count in xref/parser tests does
   not grow.
3. PASS — error taxonomy preserved: `trailer_root_contract` 7/7 before and after;
   `unrecoverable_keyword_keeps_distinct_trailer_loss` pins the distinct message.
4. PASS — this note records the before/after signatures and every command run.
   WARN (pre-existing, out of scope): fixture text layer still empty —
   pdftract-5b4c3d0e (escaped parens zero spans).

## Environment / hygiene

- No spawned servers; library API + CLI only (repo test-hygiene rule).
- Build/test tree and private target dir live under `/data` (root mount was 99% full);
  both are removed by this dispatch after the work lands.
- Commit: `fix(pdftract-0df07688): recover startxref offsets that miss the xref
  keyword` pushed to `origin/main` (hash in the bead close reason).
