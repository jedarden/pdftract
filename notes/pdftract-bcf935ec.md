# pdftract-bcf935ec — 'Document contains no pages' page-tree resolution + umbrella acceptance sweep

Dispatch 2026-09-22. Verified at **HEAD `1615dc45`** (== `origin/main`, the
umbrella tip: pdftract-0df07688, pdftract-4683109d and pdftract-4196ae99 all
landed). All runs below are from a **clean `git archive HEAD` extraction** at
`/var/tmp/pdftract-bcf935ec-head` with a private `CARGO_TARGET_DIR`
(`/var/tmp/pdftract-bcf935ec-target`) — not the shared working tree, which
carries other workers' in-flight edits.

## Part (a) — diagnosis and outcome: verification-only, fixed upstream

Byte-level inspection of `crates/pdftract-core/tests/fixtures/valid-minimal.pdf`
(583 bytes, classic xref, trailer `/Root 1 0 R`):

| object | xref records | actual offset | drift |
|---|---|---|---|
| obj 1 (catalog) | 9 | 9 | 0 |
| obj 2 (/Pages) | 58 | 58 | 0 |
| obj 3 (/Page, /Kids member) | 115 | 117 | +2 |
| obj 4 (contents) | 268 | 243 | −25 |
| obj 5 (font) | 345 | 313 | −32 |
| startxref → xref keyword | 439 | 406 | +33 |

Root cause: the page-tree walk died resolving `/Kids` member obj 3 through
its drifted xref entry (115 vs 117) — the strict parse failed and the old
resolver returned NotFound, flattening zero pages → "Document contains no
pages". This is **the drifted-xref-entry class, not an independent page-tree
bug**: fixed generically by pdftract-4683109d (bounded object-boundary rescan,
`OBJECT_OFFSET_RECOVERY_WINDOW`) + pdftract-4196ae99 (mislabelled-free
recovery + stream-object startxref), exactly the branch the bead guidance
anticipated. Per the bead instruction the fix was **not** re-done here; the
outcome is verification-only.

Pin state at HEAD (`realworld_extraction_baseline.rs`): both core
valid-minimal pins (`pin_core_valid_minimal_current_behavior`,
`desired_core_valid_minimal_page_tree_resolves`) were already flipped active
by the upstream fixes and pass — the harness header's class map
("-> pdftract-bcf935ec") is historical. Full harness at HEAD:
**12 passed / 0 failed / 2 ignored** (the two ignores are the remaining
`pdftract-5b4c3d0e` escaped-paren pins, correctly still ignored).

CLI proof: `pdftract extract crates/pdftract-core/tests/fixtures/valid-minimal.pdf
--json -` → exit 0, `page_count: 1`, "Hello World" in the text layer.

## Part (b) — umbrella end-to-end acceptance sweep (clean HEAD extraction)

`pdftract extract <fixture> --json -` (pages) and `--text -` (char count),
exit codes as recorded. Baselines: PyMuPDF 1.27.2 table pinned by
pdftract-7ec0f722 (`realworld_extraction_baseline.rs` header; linearized-10 /
multipage-100 are stubs fitz opens at 0 pages — the pdftract pin asserts the
page-object counts 10 / 100).

| fixture | exit | pages | baseline pp | chars | baseline chars | verdict |
|---|---|---|---|---|---|---|
| repo-root `tests/fixtures/valid-minimal.pdf` | 0 | 1 | 1 | 4 | 5 | PASS |
| repo-root `tests/fixtures/sample.pdf` | 0 | 1 | — | 4 | — | PASS |
| `realworld/incremental-updates-offer-letter.pdf` | 0 | 4 | 4 | 2,411 | 2,460 | PASS |
| `realworld/xref-stream-only-report.pdf` | 0 | 6 | 6 | 1,193 | 1,218 | PASS |
| `realworld/startxref-offset-edge.pdf` | 0 | 2 | 2 | 1 | 2,050 | PASS (pages); text = 5b4c3d0e class |
| `realworld/dense-one-page-agreement.pdf` | 0 | 1 | 1 | 0 | 2,050 | PASS (pages); text = 5b4c3d0e class |
| core `linearized-10.pdf` | 0 | 10 | 0 (fitz; pin 10) | 150 | — | PASS |
| core `multipage-100.pdf` | 0 | 100 | 0 (fitz; pin 100) | 715,553 | — | PASS |
| core `valid-minimal.pdf` | 0 | 1 | 1 | 11 | 12 | PASS |

9/9 fixtures extract, exit 0; every page count matches its baseline. The two
thin text layers are the known open escaped-paren class (`pdftract-5b4c3d0e`,
still open) whose current behavior is pinned as such by the harness — out of
scope here. Chars deltas on the offer-letter/report (~2%) are span-merging
whitespace differences against the fitz counter, within the harness's
existing contains-assertions.

## Part (b) — MCP `extract_text` over stdio (criterion 3)

Committed a wire-level pin in
`crates/pdftract-cli/tests/mcp-client-lifecycle.rs`
(`extract_text_returns_content_for_checked_in_valid_minimal_fixtures`), reusing
that file's existing RAII/Content-Length-framing `McpServer` pattern: spawn →
initialize → `tools/call extract_text` on **both** checked-in valid-minimal
fixtures, asserting a success result with non-empty text containing "Test"
(repo-root W3C dummy) / "Hello World" (core page-tree repro) — the -32002
extraction-failed relay is excluded by construction. Unlike the file's other
calls (which tolerate an error arm), this pin asserts extraction success.
Passes in the clean extraction: 5 passed / 0 failed.

## Before/after failure counts in the touched test areas (criterion 4)

Before = pre-fix base **`a25477c4`** (the pdftract-7ec0f722 pinning commit,
ancestor of HEAD), same clean-extraction method, its own private target dir.
After = HEAD `1615dc45`.

| test target | before (a25477c4) | after (1615dc45) | delta |
|---|---|---|---|
| pdftract-core `realworld_extraction_baseline` | 8 pass / 0 fail / 6 ign | 12 pass / 0 fail / 2 ign | 4 pins flipped to passing; 0 failures |
| pdftract-core `xref_integration_test` | 8 / **2** / 0 | 8 / **2** / 0 | hold (pre-existing: `test_forward_scan_recovery`, `test_xref_fixtures`) |
| pdftract-core `document_model` | 0 / **15** / 1 | 0 / **15** / 1 | hold (pre-existing CWD-relative fixture paths, 2026-09-21 diagnosis; not chased per bead guidance) |
| pdftract-core `remote_fetch_sequence` | 0 / 0 / 0 | 0 / 0 / 0 | hold — `#![cfg(feature = "remote")]`, vacuous without the feature |
| pdftract-core `test_truncated_flate_recovery` | 8 / **1** / 0 | 8 / **1** / 0 | hold (pre-existing: `test_truncated_flate_emits_stream_decode_error`) |
| pdftract-cli `mcp-tools-integration` | 8 / **2** / 1 | 9 / **1** / 1 | **−1 failure** (remaining: `test_nonexistent_file_returns_path_invalid`) |
| pdftract-cli `mcp-client-lifecycle` | 4 / 0 / 0 | 5 / 0 / 0 | +1 (the new extraction-content pin) |
| **total failures** | **20** | **19** | **shrunk — no growth** |

## Tree/method notes

- No production code changed in this dispatch: the bead's own guidance routes
  a "root cause = the xref gap" finding to a verification-only outcome. The
  substantial change is the committed MCP wire pin; the sweep table is this
  note.
- Shared working tree untouched except `mcp-client-lifecycle.rs` (clean vs
  HEAD before this edit); `pages.rs`/`catalog.rs`/`xref.rs` and
  `realworld_extraction_baseline.rs` carry other workers' in-flight edits and
  were deliberately not modified.

## Command record

- `cargo build -p pdftract-cli` (clean HEAD extraction) → exit 0
- sweep script over 9 fixtures (above) → 9/9 exit 0
- `cargo test -p pdftract-core --test realworld_extraction_baseline` → exit 0, 12 passed / 0 failed / 2 ignored
- `cargo test -p pdftract-cli --test mcp-client-lifecycle` → exit 0, 5 passed / 0 failed
- before/after counts (7 targets × 2 commits) → table above
