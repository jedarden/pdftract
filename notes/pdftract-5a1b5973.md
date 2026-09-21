# pdftract-5a1b5973 — Fix /Root resolution from the parsed trailer dict

**Dispatch:** 2026-09-21, worker `claude-code-glm-5.3-glm-pdftract`, claim_epoch 1.
**Parent:** pdftract-3628e890 (auto-split child 3 of 4). Blocker pdftract-e945f7c4
(child 2, trailer-parse fix, commit `77ee8e8c`) is an ancestor of dispatch HEAD
`00fd37bc` and is closed.
**Tested trees:** all verification ran in a private `git archive HEAD` extraction
at `/var/tmp/pdftract-5a1b5973` (HEAD `00fd37bc` + this dispatch's edits), private
`CARGO_TARGET_DIR=/var/tmp/pdftract-5a1b5973-target`. The shared working tree was
used only for editing; the "before" sweep ran in the same extraction at pristine
HEAD. Every cargo run was timeout-wrapped, single, no overlapping retries.

## What the chain handed this bead, re-derived at HEAD

Characterization child pdftract-d0b22a6e found **no /Root-resolution bug on the
path**: the trailer reaches `XrefSection.trailer` intact for well-formed PDFs and
`trailer.get("Root")` works. The real defect at the sites this bead names is
**error-string conflation**: `xref_section.trailer.as_ref().and_then(|t|
t.get("Root"))` collapses *trailer absent* into "No /Root reference in trailer",
while document.rs's `parse_pdf_file`/`parse_pdf_source` (already two-stage) were
the only sites keeping the cases apart. d0b22a6e also flagged the dead
slash-prefixed `/Encrypt` lookup in `sdk.rs:310`.

## Changes (commit on origin/main, this bead ID in the message)

1. **New central helper** `pdftract_core::document::resolve_root_ref(&XrefSection)
   -> Result<ObjRef>` (`crates/pdftract-core/src/document.rs`): the two failure
   modes stay distinct — trailer absent → "No trailer in xref section"; trailer
   present without a `/Root` reference (missing key or non-reference value) →
   "No /Root reference in trailer".
2. **All resolution sites routed through it** (de-conflation, no per-site
   fallbacks):
   - `document.rs`: `parse_pdf_file`, `parse_pdf_source`, `PdfExtractor::open`
     (~:1106), `Document::from_source` (~:1600). The first two already had the
     two-stage shape; their `root_ref` derivation now calls the helper (messages
     unchanged there). The latter two previously conflated.
   - `extract.rs:659/1762/2131` → `extract_pdf`, `extract_text`,
     `extract_pdf_streaming` (previously conflated).
   - `remote.rs:102` → `open_remote` (previously conflated).
   - `pdftract-cli/src/hash.rs:97/166` → `compute_fingerprint_from_file`,
     `compute_fingerprint_from_url` (previously conflated).
3. **`sdk.rs` `get_metadata`:** `trailer.get("/Encrypt")` → `trailer.get("Encrypt")`.
   `PdfDict` keys are stored slash-less (`IndexMap<Arc<str>, PdfObject>`), so the
   slash-prefixed lookup never matched and `is_encrypted` was permanently false.
   Same-class dead lookups at `document.rs:651` and `encryption/detection.rs:121`
   are **deliberately untouched**: they feed the fingerprint's
   `CatalogFlags::is_encrypted` and the encryption feature, i.e. behavior and
   fingerprint-contract surfaces outside this bead's scope — left for an owning
   bead.
4. **Negative-contract test** `crates/pdftract-core/tests/trailer_root_contract.rs`
   (7 tests): helper-level contract (None-trailer message, missing-key exact
   message, non-ref-`/Root` exact message, successful `ObjRef` resolution) plus
   end-to-end crafted PDFs through `sdk::extract`/`sdk::extract_text` (trailer
   present, no `/Root` → exact "No /Root reference in trailer"; trailer omitted →
   "No trailer in xref section").

Sites reviewed and intentionally **not** changed (distinct behavior contracts,
outside the bead's site list): `grep/worker.rs:197` (maps to a grep skip event),
`mcp/tools/registry.rs:273` (structured MCP error flow), `document.rs:2652`
(test that skips), `examples/hash.rs` (example), `document.rs:2652`-style probes.

## AC1 — tests from child 1's failure map at this HEAD

The map's entries are the five `test_sdk_smoke` cases (failed 5/5 with these
strings on 2026-09-05). At dispatch HEAD they already passed via the
test-minimal.pdf fixture switch (66464ef7); **re-run on the fixed tree: 5 passed,
0 failed, exit 0** — no regression from routing the extract paths through the
helper.

## AC2 — negative test: PASS

`trailer_root_contract` on the fixed tree: **7 passed, 0 failed, exit 0**,
including `extract_trailer_without_root_still_exact_message` and
`extract_text_trailer_without_root_still_exact_message` (crafted PDF whose
trailer parses but lacks `/Root`; root cause asserted equal to the exact string
"No /Root reference in trailer").

## AC3 — corpus sweep spot-check (full walk, 5837 checked-in PDFs)

`sdk::extract` over every checked-in `.pdf` (repo-root `tests/`, `crates/`, and
`benches/` corpora), root error message bucketed. Throwaway probe
(`zz_sweep_probe_5a1b5973.rs`, extraction-only, never committed), same probe
before and after:

| bucket (root cause) | before (HEAD 00fd37bc) | after (fix) |
|---|---|---|
| "No /Root reference in trailer" | **54** | **0** |
| "No trailer in xref section" | 0 | 54 (same files) |
| OK | 5654 | 5654 |
| "Failed to parse catalog: Failed to resolve /Root: object N 0 R not found" | 42 | 42 |
| "Document contains no pages" | 20 | 20 |
| "Failed to create lazy page iterator: … /Pages node … not found" | 12 + 37 | 12 + 37 |
| "Failed to find startxref offset: …" | 18 | 18 |

- "No /Root reference in trailer" is **gone from the corpus**: no file whose
  parsed trailer carries a `/Root` reference can emit it any more (the helper
  returns the reference), and the 54 files that did were trailer-*absent* cases —
  byte-inspection of examples confirms: `crates/pdftract-cli/tests/fixtures/empty.pdf`
  has a `/Root 1 0 R` trailer in its bytes but its `startxref` says 295 while the
  `xref` keyword sits at 293 (lands *inside the keyword*), so the trailer never
  parses; `crates/pdftract-core/__test__.pdf` is the same shape.
- The 54 now report the honest "No trailer in xref section". **Finding for the
  parser chain (child-2 family, not this bead):** pdftract-e945f7c4's recovery
  covers a wrong startxref landing *inside the entry table*; a startxref landing
  *inside the `xref` keyword itself* (± a few bytes of it) is not recovered, so
  these 54 files' trailers still never parse. The de-conflation at least makes
  them report the accurate failure mode now.
- Every other bucket is byte-identical before/after — the change is
  message-only on failure paths; no file flipped between OK and ERR.

## Full battery (extraction of HEAD + fix, true exit codes)

| command | result |
|---|---|
| `cargo check --all-targets` (workspace, gate-equivalent) | exit 0 |
| `cargo check -p pdftract-cli --all-targets` | exit 0 |
| `cargo test -p pdftract-core --test trailer_root_contract` | exit 0, 7 passed |
| `cargo test -p pdftract-core --test test_sdk_smoke` | exit 0, 5 passed |
| `cargo test -p pdftract-core --lib trailer_loss_probe` (child-2 regressions) | exit 0, 6 passed |
| sweep probe before/after | exit 0 both, table above |
| `rustfmt --check` on the six touched files | 139 chunks, **all pre-existing** (pristine document.rs alone has 116; chunk sets map 1:1 under my line shifts; hash.rs and the new test file are clean) |

**Pre-existing failure, not mine:** `cargo test -p pdftract-core --test
document_model` fails 15/15 in 0.00s on my tree **and identically on pristine
HEAD** (0 passed; 15 failed; 1 ignored; exit 101 both). Cause: the test resolves
fixtures via the CWD-relative `PathBuf::from("tests/document_model/fixtures")`,
which under `cargo test -p` from the workspace root hits the *repo-root*
`tests/document_model/` fixture set instead of
`crates/pdftract-core/tests/document_model/fixtures/`. Both `document_model.rs`
copies are dirty in the shared tree with another worker's in-flight edit — left
alone. The 11 golden `.expected.json` files pinning "Failed to parse PDF: No
/Root reference in trailer" are unaffected by this bead either way:
`parse_pdf_file`'s messages are unchanged (they already had the two-stage shape
and now call the helper with identical messages).

## AC4 — commit

Commit on `main` pushed to `origin` (Forgejo), message cites `pdftract-5a1b5973`;
files: `document.rs`, `extract.rs`, `remote.rs`, `sdk.rs` (core), `hash.rs` (cli),
`tests/trailer_root_contract.rs` (new), this note. All six source/test files were
clean at dispatch start (verified against the 61-file dirty list) and are
committed by explicit pathspec.
