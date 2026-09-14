# pdftract-7747e33e: Audit STREAM_DECOMPRESS_ERROR vs STREAM_DECODE_ERROR naming and committed assertion state

Date: 2026-09-14. Audit bead for parent bf-hyhjnl ("Verify and validate
STREAM_DECOMPRESS_ERROR test assertion"), prerequisite bf-4bww6c (commit
`da96555a`, 2026-08-01, 1229 commits behind HEAD `f8dd98a1`).

## 1. Canonical diagnostic string: `STREAM_DECODE_ERROR`

`STREAM_DECOMPRESS_ERROR` **does not exist anywhere in the codebase or its
history**:

- `grep -rn "STREAM_DECOMPRESS_ERROR" --include="*.rs" crates/` → 0 hits.
- `git grep STREAM_DECOMPRESS` over the last 200 commits of history → 0 hits.
- The only occurrences in the repo are in `notes/*.md` (six prior audit notes,
  every one already concluding the string does not exist: `bf-60qj2`,
  `bf-4g6dj`, `bf-4bx00-step1-research`, `bf-junlj`, `bf-348zd`, `bf-3erm0`,
  `bf-4bx00`, `bf-39gau9`, `bf-4fb3b`) and in bead text (19 hits in
  `.beads/checkpoint/forensic.jsonl`).

The canonical string is bound at the `code_str()` table:

- `crates/pdftract-core/src/diagnostics.rs:1278` —
  `DiagCode::StreamDecodeError => "STREAM_DECODE_ERROR"` (variant declared at
  `diagnostics.rs:465`; `DiagInfo` registry entry at `diagnostics.rs:1847-1853`,
  category STREAM, severity Warning, recoverable, phase "1.5").
- Do not confuse with the sibling `diagnostics.rs:1314` —
  `DiagCode::StreamTruncated => "STREAM_TRUNCATED"`. A bead title that says
  "DECOMPRESS" matches neither; there is no decompress-specific code.

**Conclusion: the assertion committed at `da96555a` already checks the correct
canonical string.** The naming defect lives only in the bead-chain titles
(bf-hyhjnl / bf-4bww6c say STREAM_DECOMPRESS_ERROR), not in the test.

## 2. Emitter ground truth: nothing emits the code for this fixture

Fixture `tests/fixtures/malformed/truncated-flate.pdf` is 588 bytes: classic
`xref` table (no xref stream), no ObjStm, one FlateDecode content stream
truncated mid-data.

The complete set of `DiagCode::StreamDecodeError` constructors in `src/`:

| Site | Trigger | Fires for this fixture? |
|---|---|---|
| `crates/pdftract-core/src/parser/objstm.rs:97` | `ObjStmError::DecompressionFailed` | No — fixture has no ObjStm |
| `crates/pdftract-core/src/parser/xref.rs:1725` | "Xref stream decompression produced empty output" | No — fixture uses a classic xref table |

The flate truncation itself is **deliberately silent** (INV-8 soft recovery):

- `crates/pdftract-core/src/parser/stream.rs:540-543` — EOF mid-stream →
  `break` → partial bytes, no diagnostic.
- `crates/pdftract-core/src/parser/stream.rs:3928-3941` — filter pipeline
  `Ok(decoded)` arm → `current_bytes = decoded`, no diagnostic pushed.
- `crates/pdftract-core/src/parser/stream.rs:3552-3557` — `DecodeResult` has a
  `diagnostics: Vec<Diagnostic>` field, but nothing ever puts a truncation
  diagnostic in it.
- `crates/pdftract-core/src/parser/stream.rs:3670-3677` — `decode_stream`
  wrapper keeps `.bytes` only, dropping `DecodeResult.diagnostics` even where
  other codes (STREAM_BOMB, STREAM_UNKNOWN_FILTER) *were* built.
- Extraction-path callers all use the dropping wrapper:
  `crates/pdftract-core/src/extract.rs:121`,
  `crates/pdftract-core/src/content_stream.rs:1845`.

So `STREAM_DECODE_ERROR` surfaces **neither in `metadata.diagnostics` nor in
events** for this fixture, on every channel, at both commits examined. The
comment at `stream_decoder_fixtures.rs:367-375` (bf-60qj2 EC1) claims the code
is "only emitted via `emit!()` on the full-extraction path" — no such `emit!()`
site exists; that comment is aspirational. The full-extraction path emits
nothing because both constructors above are unreachable for this fixture.

Consumer side (for the eventual emitter bead to target):
`crates/pdftract-core/src/extract.rs:454` — `pub diagnostics: Vec<String>` on
`ExtractionMetadata`, formatted strings (so a diagnostic rendered via
`code_str()` as `STREAM_DECODE_ERROR …` satisfies the `.contains()` check at
`test_truncated_flate_recovery.rs:385`).

## 3. Runtime ground truth: committed vs HEAD

Ran the committed test from pristine `git archive` extractions against a
private `CARGO_TARGET_DIR` (no working-tree edits of mine involved):

**At `da96555a` (the assertion's own commit):**

```
✓ extract_pdf() succeeded
  Page count: 0
  Total diagnostics: 0
panicked at test_truncated_flate_recovery.rs:389:5:
Expected STREAM_DECODE_ERROR diagnostic not found. Got 0 diagnostics: []
```

The extraction succeeded (soft-recovered, 0 pages) but the diagnostics array
was **empty** — the assertion fails because emission was never implemented.
Matches the run recorded in `notes/bf-39gau9.md` and honestly recorded as
FAIL in `notes/bf-4bww6c.md` ("Test Execution: ✓ FAIL (expected — diagnostics
not yet emitted)").

**At HEAD `f8dd98a1`:**

```
panicked at test_truncated_flate_recovery.rs:366:10:
Should extract truncated-flate.pdf: Document contains no pages
```

The failure moved **earlier**: `extract_pdf` now hard-errors (error string from
`crates/pdftract-core/src/page_extraction_error.rs:169` /
`page_helper.rs:116`) and the assertion at :389 is never reached. Consistent
with the fleet-wide "extract broken on all fixtures" state observed
2026-09-14. So two independent blockers exist for the parent chain:

1. **Emission gap** (present since da96555a): no code emits
   `DiagCode::StreamDecodeError` for a truncated flate content stream, and the
   extraction path cannot even carry stream-level diagnostics (dropped by the
   `decode_stream` wrapper).
2. **Extraction hard-failure** (new between da96555a and HEAD, ≤1229 commits):
   this fixture no longer soft-recovers through `extract_pdf` at all.

## 4. Working-tree attribution

`git diff HEAD -- crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
contains **only** the `parse_pdf_file` 5-tuple adaptation: `_objects` →
`_trailer` at lines 64, 81, 98 (matching the in-flight
`document.rs:390` signature change that now returns `PdfDict` as the fifth
element). That is **another worker's change — not committed by this bead**.
The assertion block (:347-395) is byte-identical across `da96555a`, HEAD, and
the working tree.

Other drift checked for contamination of the evidence: `stream.rs` working
diff is a test-module import tweak only; the 206-line `xref.rs` working diff
does not touch the `:1725` emission (0 hits); `diagnostics.rs` has no drift.

## 5. zz_probe scratch file disposition

`zz_probe_hyhjnl.rs`, `zz_probe_hyhjnl2.rs`, `zz_debug_probe.rs`,
`zz_tmp_diag.rs`: **none exist** — tracked or untracked, at repo root or
anywhere within the workspace (sweep for `zz_*`, `*probe*`, `*_diag*.rs`
outside `target/` and `.beads/` found nothing). A prior attempt already
removed them (or they were never created on this checkout). Their observations
survive in the durable notes: `bf-39gau9` (0-diagnostics run), `bf-4bww6c`
(fail-by-design record), `bf-4bx00`/`bf-348zd` (naming settlement). Nothing to
clean up; do not re-create them.

## 6. Disposition for the parent chain

- **Naming:** assertion string is already canonical (`STREAM_DECODE_ERROR`).
  The parent bead's title is the only place the phantom string lives. No test
  change needed for naming.
- **The test cannot pass** until the chain implements (a) emission of
  `DiagCode::StreamDecodeError` for truncated flate streams routed into
  `ExtractionMetadata.diagnostics` — which requires threading
  `DecodeResult.diagnostics` through the `decode_stream` wrapper or its call
  sites — and (b) restoration of soft recovery for this fixture through
  `extract_pdf` (currently hard-Errs before the assertion).
- This audit made **no source changes**; deliverable is this note plus the bead
  record.
