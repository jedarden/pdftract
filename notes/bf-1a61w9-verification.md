# bf-1a61w9 verification

Date: 2026-09-23  
Verification child: `pdftract-a61be287`  
Parent: `bf-1a61w9` (Resolve marked content property references)

The implementation is represented by the real symbols
`ResourceDict::lookup_properties()` and
`extract_mcid_from_props()`/`extract_ocg_ref_from_props()`; there is no
`extract_properties_from_dict()` symbol.

## Parent acceptance criteria

| # | Criterion | Result | Evidence |
|---|---|---|---|
| 1 | Full marked-content/resources/content-stream surface passes through the required full gate, with no timeout/retry/orphan processes | **FAIL** | In a clean `git archive HEAD` extraction of `22e05f27414f064b1f3423032b8fab66904bb744` at `/home/coding/scratch/pdftract-a61be287-head-clean.M9pyKD`, `timeout --kill-after=30s 600s cargo test --all-targets` exited `101`: 352 passed, 9 unrelated `pdftract-cli` failures. The focused `cargo test -p pdftract-core --lib content_stream` also exited `101` (125 passed, 10 failures), including the committed stale one-glyph assertions in `crates/pdftract-core/src/content_stream.rs` after `66464ef7` changed normal text extraction to per-character glyphs. Marked-content operators (`50/50`) and resources (`9/9`) passed. No `TIMEOUT`/`TERMINATED` lines appeared. The exact `pgrep -af 'pdftract|TH-0'` pattern sees the Needle/Codex supervisor command itself; the filtered binary-process check found no running pdftract or TH-0 test process. These are repository test failures, not infra-only warnings, so this criterion cannot be marked PASS. |
| 2 | Both child-1 fixtures exercise indirect-reference and direct-dict `/Properties` through CLI extraction, with retained MCID/ActualText observable | **PASS** | Fixtures: `tests/fixtures/tagged/mc_properties_indirect.pdf` and `tests/fixtures/tagged/mc_properties_direct.pdf`. From the same clean extraction, `pdftract extract tests/fixtures/tagged/mc_properties_indirect.pdf --json -`, `--md -`, and the corresponding two direct-fixture commands all exited `0`; captured output contains `Tagged content`. `cargo test -p pdftract-core --test marked_content_properties_fixtures` exited `0` with `4 passed`, including `bdc_name_props_indirect_reference_recovers_mcid`, `bdc_name_props_direct_inline_dict_recovers_mcid`, and `public_extract_pdf_indirect_properties_recovers_mcid_end_to_end`. The test assertions verify `Some(0)` for both paths and successful public extraction. The CLI JSON/Markdown schema does not serialize a numeric `mcid` field, so the numeric MCID evidence is from the integration assertions and the retained ActualText is the CLI-visible `Tagged content`. |
| 3 | A durable note maps all four parent criteria to PASS/WARN/FAIL with commands, paths, and commits | **PASS** | This file: `notes/bf-1a61w9-verification.md`. No WARN is being used to conceal the non-infrastructure failures above. Relevant implementation/test commits are `ebc7f922` (`pdftract-966fddc5`), `355860ee` (`pdftract-ff642b7b`), `e9355cfd` and `8fefd95d` (`pdftract-0f063151`), and `22e05f27` (`pdftract-9b3f77fd`). |
| 4 | Commits cite the child and parent IDs; the note path is available for the eventual parent close | **PASS** | The verification commit(s) for this note use both `pdftract-a61be287` and `bf-1a61w9` in their messages. The eventual parent close must cite `notes/bf-1a61w9-verification.md`; this child is not closed while criterion 1 remains FAIL. |

## Focused command results

All commands below were run from the clean archive extraction named above with
`CARGO_TARGET_DIR=/build/target-workers/pdftract-a61be287-head`:

```text
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib parser::marked_content_operators
  exit=0; 50 passed
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib parser::resources
  exit=0; 9 passed
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib content_stream
  exit=101; 125 passed, 10 failed
timeout --kill-after=30s 600s cargo test -p pdftract-core --test marked_content_properties_fixtures
  exit=0; 4 passed
/build/target-workers/pdftract-a61be287-head/debug/pdftract extract tests/fixtures/tagged/mc_properties_indirect.pdf --json -
  exit=0; output contains Tagged content
/build/target-workers/pdftract-a61be287-head/debug/pdftract extract tests/fixtures/tagged/mc_properties_indirect.pdf --md -
  exit=0; output contains Tagged content
/build/target-workers/pdftract-a61be287-head/debug/pdftract extract tests/fixtures/tagged/mc_properties_direct.pdf --json -
  exit=0; output contains Tagged content
/build/target-workers/pdftract-a61be287-head/debug/pdftract extract tests/fixtures/tagged/mc_properties_direct.pdf --md -
  exit=0; output contains Tagged content
```

The child remains open because the mandatory full-gate command is red on
committed baseline failures outside this marked-content change. No parent close
was attempted.
