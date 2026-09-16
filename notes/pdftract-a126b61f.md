# pdftract-a126b61f — re-issue 6: duplicate auto-split declined, evidence close (umbrella head)

**Dispatch:** 2026-09-16, worker `claude-code-glm-5.3-flash-glm-icg`, claim epoch 6.
**Order:** Auto-Split — "This bead has failed 5 times in a row" → create 3-5 children.

## Why the split order is a false premise (split declined)

- Attempt 5 (2026-09-15T19:26Z) resolved **verified_success**. `failure-count:5` is a
  reopen artifact (documented treadmill: reason-less reopens bump the counter —
  forensic.jsonl for this bead shows 1 closed / 1 reopened / 5 attempt_resolved), not
  five real failures.
- The bead is **already an umbrella** (`umbrella` + `auto-split-parent` labels present
  before this dispatch) with a complete child chain — **all four children closed**:

| Child | Scope | Status |
|---|---|---|
| `pdftract-b5cb36ca` | default_rust gate green at HEAD | closed |
| `pdftract-4785df89` | typed diagnostic JSON serialization stability, pinned tests | closed |
| `pdftract-a58276cf` | legacy `Vec<String>` byte-for-byte compatibility | closed (re-issue 5 evidence close, commit `81c7541a`) |
| `pdftract-dd40e63a` | canonical/compatibility contract documentation audit | closed (commits `016ff6c2`, `0bbc1385`) |

- The required umbrella edge already existed: parent blocked-by terminal child
  `pdftract-dd40e63a`.
- Repair made this dispatch: restored the missing historical chain edge
  `pdftract-a58276cf` blocked-by `pdftract-4785df89` (both beads closed before and
  after the edge add; chain now fully linked).
- Creating 3-5 new children would duplicate already-closed verification work — same
  decline rationale as `6c545ff0` (8c4bed00 split re-check), `dbbd2e0a`/`81c7541a`
  (a58276cf re-issues).

## Re-verification at HEAD `0bbc1385`

Method: clean no-`.git` extraction of HEAD (`git archive` → `~/scratch/a126b61f-verify`,
removed after the run — the working tree was NOT used because it carries another
worker's in-flight edits to two of the pinned test files). The only code delta since
the last clean run (`5d4d9b02`) is commit `016ff6c2`, which touched **doc comments and
doc-examples only** in `diagnostics.rs`, `diagnostics_compat.rs`, `extract.rs`,
`schema/mod.rs` (verified by diff review; no behavior change).

| Command (run in the clean extraction) | Result |
|---|---|
| `cargo test --lib diagnostics::` | 29 passed, 0 failed (exit 0) |
| `cargo test --lib diagnostics_compat` | 9 passed, 0 failed (exit 0) |
| `cargo test --no-fail-fast --test diagnostics_serialization_format --test diagnostics_surface_mirror --test diagnostics_catalog_drift --test diagnostics_helper_test` | 46 passed, 0 failed (7+8+5+26, exit 0) |

**84/84 diagnostics tests green.** Known pre-existing failure excluded by the
`diagnostics::` filter: `output::ndjson::pipeline::tests::test_footer_errors_carry_structured_diagnostics`
("No /Root reference in trailer") — a corpus-level extraction failure pre-dating this
bead by ≥1800 commits (see plan TH notes / workspace memory), not a diagnostics-model
test. It is the sole reason the unfiltered `--lib diagnostics` substring run is not
used as the fenced command.

## Acceptance criteria re-derived at HEAD

- **PASS — public typed diagnostic model + severity enum with stable JSON field
  names:** `crates/pdftract-core/src/diagnostics.rs` (`Severity` L92, `DiagCode`
  L145, `Diagnostic` L2752, `DiagnosticsCollector` L3199); `DiagnosticJson`
  round-trip pinned by `diagnostic_json_round_trip_tests` (4/4) and
  `diagnostics_serialization_format` (7/7).
- **PASS — legacy `Vec<String>` byte-for-byte compatible:**
  `crates/pdftract-core/src/diagnostics_compat.rs` (`to_legacy_string` L93,
  `to_legacy_strings` L103); golden
  `golden_legacy_bytes_match_inline_producer_expression` in the 9/9 compat suite;
  independently verified by closed child `pdftract-a58276cf`.
- **PASS — unit tests cover typed-to-legacy conversion, optional fields, and
  supported severities:** compat suite 9/9 (order/length/duplicates preserved;
  every supported severity renders its documented string; optional JSON fields
  omitted when `None`); round-trip suite 4/4 (absent optional fields omitted from
  serialization; every code round-trips through structured JSON; display form
  matches documented string shape).
- **PASS — canonical/compatibility contract recorded in code-level API
  documentation:** module docs in `diagnostics.rs` and `diagnostics_compat.rs`;
  `docs/errors-array-format.md`; `docs/integrations/diagnostics-codes.md`;
  documentation drift fixed by `pdftract-dd40e63a` (`016ff6c2`).

## Disposition

- Split **declined** — duplicate of an existing, fully-closed 4-child chain.
- No code change needed this dispatch; bead `notes` field updated as the
  gate-visible closure evidence (closure contract option 2), plus this note for
  git history.
- Downstream `pdftract-f9d10c44` ("Thread structured diagnostic context through
  extraction emission sites") is unblocked by this close.
