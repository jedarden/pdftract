# bf-4bx00 step 4 — STREAM_DECODE_ERROR assertion verification (bf-hyhjnl closeout)

- **Bead:** pdftract-9356414f (split-child of umbrella bf-hyhjnl, grandparent bf-4bx00)
- **Date:** 2026-09-24 · **HEAD:** `40d23bd3` · **Worker:** claude-code-glm-5.3-glm-pdftract
- **Prerequisite assertion commit:** `da96555a` (bf-4bww6c, 2026-08-01, landed in
  `crates/pdftract-core/tests/test_truncated_flate_recovery.rs:347-395`)

## Executive summary

The committed assertion is correct, non-tautological, and compiles — but it **fails at
HEAD as a genuine product gap, not as an environmental artifact**: `extract_pdf()` on
`tests/fixtures/malformed/truncated-flate.pdf` succeeds (page count 1, fingerprint
`pdftract-v1:ab24a95f…`) and returns **0 diagnostics**, so the `.any(|d| d.contains("STREAM_DECODE_ERROR"))`
assert at `test_truncated_flate_recovery.rs:387` fails with `Got 0 diagnostics: []`.
Root cause (established by the audit child, re-confirmed here): `decode_stream` returns
`decode_stream_impl(...).bytes` and discards `DecodeResult.diagnostics`, so the low-level
INV-8 soft-recovery diagnostic never reaches `metadata.diagnostics` (see
`notes/bf-hyhjnl.md` "The Gap"). The only `DiagCode::StreamDecodeError` emitters are
`parser/objstm.rs:97` and `parser/xref.rs:1885` — neither on this classic-xref fixture's path.

All failure shapes reproduce **byte-identically in a pristine `git archive HEAD`
extraction**, so none of what follows is shared-checkout pollution.

## Commands and results (all timeout-wrapped: `timeout --kill-after=30s 600s cargo test …`)

Run twice each: (A) in the shared working tree at `/home/coding/pdftract`, (B) in the
pristine extraction `/var/tmp/bf4bx00-step4-head` (= HEAD `40d23bd3`, `git archive HEAD | tar -x`).
Outcomes and failure text were **identical in A and B** for every command.

| # | Command | Result (A = B) | Exit |
|---|---------|----------------|------|
| 1 | `cargo test -p pdftract-core --test stream_decoder_fixtures` | `test_each_filter_exercised` ok; `test_all_stream_decoder_fixtures` FAILED — **13/17 fixtures**, failures: `flate_png_pred15_all_six` (expected 48 B, got 48 B), `lzw_early_change_0` (expected 10 B, got 0 B), `lzw_early_change_1` (expected 10 B, got 0 B), `filter_array_a85_then_flate` (expected 46 B, got 0 B). **`flate_truncated` passed** (INV-8 soft-recovery contract holds on the low-level `StreamDecoder::decode` path). | 101 |
| 2 | `cargo test -p pdftract-core --test test_truncated_flate_recovery` | **8 passed / 1 failed.** Failure: `test_truncated_flate_emits_stream_decode_error` panicked at `:387` — `Expected STREAM_DECODE_ERROR diagnostic not found. Got 0 diagnostics: []` after `✓ extract_pdf() succeeded`, `Page count: 1`, `Total diagnostics: 0`. | 101 |
| 3 | `cargo test -p pdftract-core --lib parser::stream` | **171 passed / 0 failed** (3506 filtered) | 0 |
| 4 | `cargo test -p pdftract-core --lib decoder::` | **23 passed / 0 failed** (jbig2 + jpx unit tests) | 0 |

### Definition of done (pristine extraction B only)

- `cargo build --all-targets` → **exit 0** (2m07s cold; `Finished dev profile`).
- `cargo test` (workspace default) → **exit 101, pre-existing and unrelated**: halts at
  `pdftract-cli --lib` with 352 passed / 9 failed —
  `inspect::api::tests::test_render_page_svg_basic`, `test_render_page_svg_empty_page`,
  `inspect::render::mcid::tests::test_render_mcid_labels_multiple`,
  `inspect::render::tests::test_extract_columns_from_spans`,
  `pages::tests::test_parse_and_filter_out_of_range`, `pages::tests::test_parse_comma_separated`,
  `url::tests::test_parse_url_invalid`, `test_parse_url_urlencoded_credentials`,
  `test_parse_url_with_empty_path`. The workspace run stops there (default fail-fast), so
  `pdftract-core` binaries are not reached by the bare DoD command; the focused per-target
  runs in the table above cover them. Same shape as the sibling's 2026-09-23 record
  (then 353/8); this bead touches no code, so none of these are attributable to it.

### Pollution attribution (environmental analysis)

- `crates/pdftract-core/src/parser/stream.rs` and `crates/pdftract-core/tests/stream_decoder_fixtures.rs`
  are **byte-identical to HEAD** in the shared tree (`git diff HEAD --stat` empty on both).
  The only stream-chain file with working-tree drift is `test_truncated_flate_recovery.rs`
  (6 lines: another worker's `parse_pdf_file` 5-tuple adaptation), and the test still
  compiled and ran in the shared tree with the same outcome as pristine HEAD.
- **Conclusion: zero shared-checkout pollution effect on these suites.** Both failure
  shapes are pre-existing conditions of HEAD `40d23bd3` itself.

## Verdicts on every parent (bf-hyhjnl) acceptance criterion

| Parent criterion | Verdict | Evidence |
|---|---|---|
| Test passes with new assertion | **FAIL** (product gap at HEAD; *not* environmental, *not* introduced by the chain) | Command #2: 8/9 pass; the assertion test fails with `Total diagnostics: 0` because `decode_stream` drops `DecodeResult.diagnostics` (`notes/bf-hyhjnl.md` "The Gap"; `parser/stream.rs` `decode_stream` → `.bytes`). Identical in pristine extraction. The chain's assertion commit `da96555a` changed test code only (2 files: the test + its note) and cannot affect emission. |
| No regressions in other tests | **PASS** for this chain's changes — with a flagged pre-existing condition | Commands #3 (171/0) and #4 (23/0) green; the other 8 tests of the truncated-flate suite all pass. The 4 `stream_decoder_fixtures` mismatches (command #1) reproduce identically at pristine HEAD and are independent of the chain: `da96555a` touched no `src/` file and the failing suite does not read the changed test file. **Flagged for follow-up:** `lzw_early_change_0/1`, `filter_array_a85_then_flate`, `flate_png_pred15_all_six` fail at HEAD; a closed legacy bead (`pdftract-1xwks`) once claimed all corpus fixtures passing, but its cited commit `f775286` is not reachable in this history, so regression timing is undetermined. |
| Test compiles cleanly | **PASS** | `cargo build --all-targets` exit 0 in pristine extraction; the test target builds and runs. The workspace carries pre-existing warnings tree-wide (74 from `pdftract-core` lib alone at build time); none originate from the assertion block. |
| Assertion correctly validates STREAM_DECOMPRESS_ERROR presence | **PASS** (with naming correction) | Canonical code is **`STREAM_DECODE_ERROR`** (`diagnostics.rs:1315`); `STREAM_DECOMPRESS_ERROR` has **zero** `.rs` occurrences anywhere in tree or history — the name exists only in the parent bead titles (audit `pdftract-7747e33e`, commit `a29a9cb1`). The committed assertion checks the canonical string. Negative control by `pdftract-62d62af5` (2026-09-23): swapping the fixture for valid-minimal.pdf made the same assert fail (exit 101) — the check is not a tautology. The positive path is currently unexercisable at HEAD because of criterion 1's emitter gap. |
| Verification documented in `notes/bf-4bx00-step4-verification.md` | **PASS** | This file, committed with a pathspec-limited commit citing pdftract-9356414f. |

## Child-bead ledger

| Bead | Commit | Contribution |
|---|---|---|
| pdftract-7747e33e (audit) | `a29a9cb1` (`notes/pdftract-7747e33e.md`) | Canonical-string determination (STREAM_DECODE_ERROR), committed-vs-worktree state, zz-probe dispositions, two-blocker root-cause analysis (no emitter on this path + extraction regression, the latter since healed — extract now succeeds). |
| pdftract-7fba1720 (fix-and-commit) | none needed — assertion already correct at HEAD via `da96555a` | Verified assertion byte-identical at HEAD; deleted the 4 chain-owned scratch probes (`zz_probe_hyhjnl.rs`, `zz_probe_hyhjnl2.rs`, `zz_debug_probe.rs`, `zz_tmp_diag.rs`); deliberately left `zz_probe_core_extract.rs` (owned by the b716bac5 chain). |
| pdftract-62d62af5 (isolation run) | none (verification-only; closed `verified_success` 2026-09-23, receipt `ao-28a4187e23a40867217e369815c6c546`) | Focused run 8 passed / 1 failed (`Total diagnostics: 0`, assert reached); negative control via valid-minimal.pdf fixture swap (failed at the same assert, exit 101) with fixture restored and verified by `cmp` + SHA-256 `5b866a7b…`; no orphaned processes. Today's runs reproduce its result exactly at `40d23bd3`. |

## Follow-ups (out of scope here)

1. Wire `DecodeResult.diagnostics` from `decode_stream_impl` through to
   `ExtractionResult.metadata.diagnostics` (options 1–3 in `notes/bf-hyhjnl.md`) —
   this is what converts criterion 1's FAIL into PASS.
2. Investigate the 4 pre-existing `stream_decoder_fixtures` mismatches at HEAD
   (`lzw_early_change_*`, `filter_array_a85_then_flate`, `flate_png_pred15_all_six`).
