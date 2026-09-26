# bf-4bx00 step 4 — STREAM_DECODE_ERROR assertion verification (bf-hyhjnl closeout)

- **Bead:** pdftract-9356414f (split-child of umbrella bf-hyhjnl, grandparent bf-4bx00)
- **Date:** 2026-09-24 (Round 1) · **HEAD:** `40d23bd3` · **Worker:** claude-code-glm-5.3-glm-pdftract
- **Round 2 re-verification:** 2026-09-25 at HEAD `48ac7db0` (pdftract-82ef5e9d) — see the
  Round 2 section below; every Round 1 outcome reproduced identically
- **Prerequisite assertion commit:** `da96555a` (bf-4bww6c, 2026-08-01, landed in
  `crates/pdftract-core/tests/test_truncated_flate_recovery.rs:347-395`)

## Round 2 — 2026-09-25 re-verification and consolidation (pdftract-82ef5e9d)

Dispatch chain this round (all at HEAD `48ac7db0` = origin/main; `git rev-list
origin/main..HEAD` empty): **pdftract-c47fe3a3** (inspect) → **pdftract-e22f24e9**
(focused test) → **pdftract-a7ad64ba** (regression + warnings) → **pdftract-82ef5e9d**
(this consolidation). Every outcome below reproduces Round 1 byte-identically —
nothing on this chain's surface drifted between `40d23bd3` and `48ac7db0`.

### Consolidated sibling results

- **Inspection (pdftract-c47fe3a3).** Integration target
  `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`, cargo target
  `test_truncated_flate_recovery`, focused test `test_truncated_flate_emits_stream_decode_error`;
  exact form: `cargo test -p pdftract-core --test test_truncated_flate_recovery
  test_truncated_flate_emits_stream_decode_error -- --exact`. Fixture
  `tests/fixtures/malformed/truncated-flate.pdf` (588 B, sha256
  `5b866a7b53a65583354490367968b187c5f1a98334c0317b6e3909b188c4179b` — re-verified
  this round). The sidecar `tests/error_recovery/fixtures/truncated_mid_stream.expected_diagnostics.json`
  (min_count 1, partial output, no panic) documents the same contract but is **not
  loaded** by this integration test. Assertion semantics: `extract_pdf` with default
  options → `ExtractionResult.metadata.diagnostics` (`Vec<String>`); the test passes
  iff any entry `.contains("STREAM_DECODE_ERROR")`; it fails via `assert!` ("Expected
  STREAM_DECODE_ERROR diagnostic not found. Got N diagnostics: […]") or earlier at the
  `.expect` if extraction errors. Neighbor `test_truncated_flate_emits_diagnostics`
  is a scaffold parse/no-error check and asserts no diagnostics.
- **Focused test (pdftract-e22f24e9).** `cargo test -p pdftract-core --test
  test_truncated_flate_recovery test_truncated_flate_emits_stream_decode_error --
  --nocapture` → **exit 101**; 1 ran / 8 filtered out; output `✓ extract_pdf()
  succeeded`, `Page count: 1`, `Total diagnostics: 0`, panic at
  `test_truncated_flate_recovery.rs:387`.
- **Regression + warnings (pdftract-a7ad64ba).** Working-tree full-target run:
  9 ran, 8 passed, 1 failed (the assertion test), exit 101. `cargo check -p
  pdftract-core --tests` → exit 0 with pre-existing warnings only (74 from the
  pdftract-core lib at build time plus the build-script `glyph-shapes.json`
  missing/empty-database note); none from the affected test file. Clean extraction
  `/tmp/tmp.8bb0pnEg8l` (retained): `cargo build --all-targets` exit 0; repository-default
  `cargo test` exit 101 (353 passed / 9 pre-existing pdftract-cli unit failures);
  affected target exit 101 with the same 8-pass/1-fail shape; `git rev-list
  origin/main..HEAD` empty; orphan-process check empty.

### Independent re-derivation at HEAD `48ac7db0` (this dispatch)

Clean `git archive HEAD` extraction `/var/tmp/82ef5e9d-head-EpIw` (retained — the
bare DoD `cargo test` exits 101 on pre-existing failures). All runs
`timeout --kill-after=30s 600s`-wrapped.

| # | Command | Result | Exit |
|---|---------|--------|------|
| R1 | `cargo build --all-targets` | Finished dev profile, 1m20s | 0 |
| R2 | `cargo test` (repository-default DoD) | Halts at `pdftract-cli --lib`: **354 passed / 8 failed** — `inspect::api` SVG ×2, `inspect::render::test_extract_columns_from_spans`, `pages` ×2, `url` ×3 — same pre-existing set Round 1 recorded (then 352/9); fail-fast never reaches `pdftract-core`, covered per-target below | 101 |
| R3 | focused `test_truncated_flate_emits_stream_decode_error -- --nocapture` | `✓ extract_pdf() succeeded`, `Page count: 1`, `Total diagnostics: 0`, panic at `:387` — identical to both siblings' runs | 101 |
| R4 | full `--test test_truncated_flate_recovery` | 8 passed / 1 failed (assertion test only) | 101 |
| R5 | `cargo check -p pdftract-core --tests` | 287 `warning:` lines, all pre-existing tree-wide (unused imports/variables, `glyph-shapes.json` build-script note); **zero** name `truncated_flate` | 0 |
| R6 | `cargo test -p pdftract-core --lib parser::stream` | 171 passed / 0 failed (3513 filtered) | 0 |
| R7 | `cargo test -p pdftract-core --lib decoder::` | 23 passed / 0 failed (3661 filtered) | 0 |
| R8 | `--test stream_decoder_fixtures` | `test_each_filter_exercised` ok; `test_all_stream_decoder_fixtures` FAILED with the same 4 pre-existing mismatches: `flate_png_pred15_all_six` (48 B vs 48 B), `lzw_early_change_0/1` (10 B vs 0 B), `filter_array_a85_then_flate` (46 B vs 0 B); `flate_truncated` itself passes | 101 |

### Round-2 child-bead ledger

| Bead | Commit | Contribution |
|---|---|---|
| pdftract-c47fe3a3 (inspect) | none (verification-only) | Target/test/fixture identification, sidecar relationship, canonical naming, assertion semantics; clean-extraction observation of the :387 panic. |
| pdftract-e22f24e9 (focused test) | none (verification-only) | Filtered run exit 101 with `Total diagnostics: 0` — confirms the assert is reached and fails on emptiness, not on extraction error. |
| pdftract-a7ad64ba (regression + warnings) | none (verification-only) | Full-target 8/1, `cargo check` 0 with pre-existing-warning attribution, clean-extraction DoD record, upstream-current + orphan checks. |
| pdftract-82ef5e9d (this note) | this commit | Consolidation + independent re-derivation of every command at `48ac7db0`; fixture sha re-verified. |

### Verdict delta vs Round 1

**None.** All five parent (bf-hyhjnl) criteria resolve exactly as the Round 1 table
below records: assertion test FAIL at HEAD is a stable product gap
(`decode_stream` discards `DecodeResult.diagnostics`; only `DiagCode::StreamDecodeError`
emitters are `parser/objstm.rs:97` and `parser/xref.rs:1885`, neither on this
classic-xref fixture's path); no chain-caused regressions (R4/R6/R7 green elsewhere in
the suite, R8 mismatches pre-existing and unchanged); compiles cleanly (R1/R5);
canonical name `STREAM_DECODE_ERROR` (assertion block re-read this round).

### Limitations

- The repository-default DoD (`cargo test`) cannot pass at HEAD: fail-fast stops at
  8 pre-existing pdftract-cli unit failures (R2) unrelated to this chain. This is a
  standing condition of HEAD, not of this round; per-target runs R3–R8 cover the
  affected surface.
- The focused assertion test itself cannot pass at HEAD for the product-gap reason
  above; wiring the diagnostic through (Round 1 follow-up #1) is out of scope for a
  documentation bead.
- The shared working tree carries unrelated drift from other workers (including a
  tuple-binding rename in the target test file); every run consolidated here was
  executed in a pristine extraction, so none of it affects these results.

## Round 1 — 2026-09-24 original verification (pdftract-9356414f)

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

## Round 3 — 2026-09-25 post-fix re-verification and resolution

The diagnostic-plumbing fix child **pdftract-a2e5e10b** closed with fix commits
`9634a6ef` (`fix(pdftract-a2e5e10b): emit truncated flate diagnostics`) and
`50e9d654` (`fix(pdftract-a2e5e10b): retain decode diagnostics on empty recovery`).
The re-verification child **pdftract-1effa6c8** then closed at fixed verification
HEAD `50e9d654`. Both fix commits are ancestors of the current branch and were
pushed to `origin/main` before this consolidation. The clean git-archive
extraction used for this round was `.verify-pdftract-head.0mpacP`; the shared
dirty worktree was not used for the results below.

Every cargo command in this round was wrapped as
`timeout --kill-after=30s 600s <command>`.

### Post-fix commands and outcomes

| Command | Outcome | Exit |
|---|---|---|
| `cargo test -p pdftract-core --test test_truncated_flate_recovery test_truncated_flate_emits_stream_decode_error -- --exact` | **1 passed / 0 failed**; extraction remained non-fatal, page count was 1, and `STREAM_DECODE_ERROR` was present | 0 |
| `cargo test -p pdftract-core --test test_truncated_flate_recovery` | **9 passed / 0 failed** | 0 |
| `cargo test -p pdftract-core --lib parser::stream` | **171 passed / 0 failed** | 0 |
| `cargo test -p pdftract-core --lib decoder::` | **23 passed / 0 failed** | 0 |
| `cargo test -p pdftract-core --test stream_decoder_fixtures` | **13/17 fixtures passed**; the same four Round-2 baseline mismatches remained: `flate_png_pred15_all_six`, `lzw_early_change_0`, `lzw_early_change_1`, and `filter_array_a85_then_flate`; `flate_truncated` passed | 101 |
| `cargo check -p pdftract-core --tests` | Passed with pre-existing warnings; no warning named `truncated_flate` | 0 |
| `cargo build --all-targets` | Completed successfully | 0 |
| `cargo test` | **354 passed / 8 unrelated pre-existing `pdftract-cli` failures**; same standing limitation as Round 2 | 101 |

The focused assertion and full affected target are therefore green after the
fix. The fixture-target exit 101 is unchanged from Round 2 and is attributable
to the four pre-existing mismatches, not to the diagnostic-plumbing commits.
The repository-default `cargo test` exit 101 likewise stops on the known
unrelated `pdftract-cli` failures; the relevant core targets above are green.
The final bracketed orphan-process checks were empty.

### Round-3 child ledger

| Bead | Commit | Contribution |
|---|---|---|
| pdftract-a2e5e10b | `9634a6ef`, `50e9d654` | Plumbed truncated-FlateDecode diagnostics through extraction while retaining non-fatal partial recovery; focused and full target passed. |
| pdftract-1effa6c8 | none (verification-only) | Re-ran the fixed HEAD in a clean archive; confirmed the assertion and affected target pass, with no new regression. |
| pdftract-31d79e17 | this append | Consolidates Round 3 and updates the umbrella verdicts. |

### Umbrella bf-hyhjnl criteria after the fix

| Parent criterion | Post-fix verdict | Evidence |
|---|---|---|
| Test passes with new assertion | **PASS** | Focused test: 1/1 passed, exit 0. Full `test_truncated_flate_recovery`: 9/9 passed, exit 0. |
| No regressions in other tests | **PASS**, with pre-existing failures recorded | `parser::stream` 171/0 and `decoder::` 23/0 passed. The fixture target retained exactly the four Round-2 mismatches and introduced no new failure. |
| Test compiles cleanly | **PASS** | `cargo build --all-targets` and `cargo check -p pdftract-core --tests` exited 0; warnings were pre-existing and none named `truncated_flate`. |
| Assertion correctly validates STREAM_DECOMPRESS_ERROR presence | **PASS**, with naming correction | The repository's canonical code is `STREAM_DECODE_ERROR` / `DiagCode::StreamDecodeError`; the focused test now observes that diagnostic after the fix. `STREAM_DECOMPRESS_ERROR` remains the stale title spelling. |
| Verification documented in `notes/bf-4bx00-step4-verification.md` | **PASS** | This Round 3 section records the fix commits, clean-extraction commands, outcomes, and resolved verdicts. |

**Round-3 conclusion:** the prior product-gap FAIL is resolved. The umbrella
criterion “Test passes with new assertion” flips **FAIL → PASS** at fixed HEAD
`50e9d654`; the remaining exit-101 results are unchanged, explicitly attributed
pre-existing conditions.
