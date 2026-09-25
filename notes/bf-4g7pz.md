# bf-4g7pz — Verify assert_exit_code method (umbrella close-out)

## Dispatch context

Auto-split dispatch #4 (attempt 12 overall, `failure-count:18`). The split
premise was false: this bead had already been decomposed three times
(attempts 9–11, 2026-09-25) into direct children, and every descendant is
closed:

- 12/12 direct blockers of bf-4g7pz: **closed** (verified via `bead show` per id)
- 41/41 beads labeled `parent-bf-4g7pz`: **closed** (`bead list --json --limit 999999`)

Blockers: pdftract-032dbdad, pdftract-1ee8b1a6, pdftract-601fc793,
pdftract-6096241c, pdftract-9896685b, pdftract-a7bd6bcc, pdftract-a7ec487d,
pdftract-b22b01ca, pdftract-e745de7d, pdftract-e94d6756, pdftract-f14a82a8,
pdftract-f1879d7b.

The forensic event log shows the treadmill mechanism: each prior attempt added
exactly one child (`dependency_added`), resolved `indeterminate`/`decomposed`
with `action=none`, was released, and failure-count bumped — no prior attempt
executed the terminal close. Splitting again would have created a fourth set
of children for already-completed work. Terminal action performed instead:
evidence close of the umbrella head.

## Acceptance criteria — all PASS at HEAD 36f80e179f4a2d699b5075d3febdb5c4c2520a74

| Criterion | Evidence |
|---|---|
| assert_exit_code method exists | `crates/pdftract-core/src/extract.rs:2967` — `pub fn assert_exit_code(&self, expected: i32) -> Result<(), AssertionError>`; doc example at :2963. Committed at HEAD (`git show HEAD:crates/pdftract-core/src/extract.rs`); file clean in the working tree. |
| Ok(()) when exit code matches | `test_extraction_result_assert_exit_code_success`, `_matching_values` — pass |
| Err(AssertionError) on mismatch | `test_extraction_result_assert_exit_code_mismatch`, `_mismatching_values` — pass |
| Handles various exit codes (0, 1, etc.) | `_matching_values`/`_mismatching_values` iterate varied expected/actual pairs; plus `_out_of_domain`, `_with_errors`, `_with_multiple_errors`, `_error_message` — 8/8 pass |

Method semantics (extract.rs:2967–2976 at HEAD): `actual = if
self.metadata.error_count == 0 { 0 } else { 1 }`; `Ok(())` on match,
`Err(AssertionError { expected, actual, description })` otherwise. The
0/1-only derivation means the two mismatched pairs in `_mismatching_values`
exhaust the supported variation domain (noted in child
pdftract-e94d6756/f14a82a8's closes).

## Verification runs (2026-09-25)

1. **Shared checkout** (`/home/coding/pdftract`; `extract.rs` byte-identical
   to HEAD — no working-tree diff):
   `timeout --kill-after=30s 600s cargo test --lib -p pdftract-core assert_exit_code`
   → `8 passed; 0 failed` — exit 0.
2. **Clean no-.git extraction of HEAD** (`git archive 36f80e17… | tar -x` to
   `/var/tmp/bf4g7pz-head-COXmBo`, isolated target dir
   `/build/pdftract/verify-bf4g7pz` per the cargo wrapper's one-target-dir
   rule; no iad-ci offload since no `.git`):
   `cargo test --lib -p pdftract-core assert_exit_code`
   → `8 passed; 0 failed` — exit 0 (build 1m 23s).
   This replicates the environment of the close-reason `verified:` fence
   re-run (clean extraction of HEAD). Both the extraction dir and the target
   dir were removed after success; `df` before/after healthy.

Tests exercised (both runs, same 8):
`extract::tests::test_extraction_result_assert_exit_code_{success,
matching_values, mismatch, mismatching_values, out_of_domain, with_errors,
with_multiple_errors, error_message}`.

WARN (infra, pre-existing, out of scope): full definition-of-doD
`cargo test --no-fail-fast` is known to wedge ~7min in on
`test_notification_no_response` (`crates/pdftract-cli/tests/mcp-stdio.rs:355`,
blocking `fill_buf` poll) and the tree carries ~300 pre-existing test
failures (plan TH notes) — so only focused filtered runs are cited, per
test-hygiene rules.

## Orphan check

`pgrep -af 'pdftract[ ]mcp|TH[_]0|TH[-]0'` → empty.

## Closure actions

- `bead update bf-4g7pz --notes … --fencing-token 12` (closure-contract
  option 2 — verification-only dispatch, notes field previously empty)
- `bead close bf-4g7pz --reason … --fencing-token 12` with `verified:` fence
  carrying only genuinely green commands
- `bead sync flush-only`
- This note committed (`notes/bf-4g7pz.md`, explicit pathspec) and pushed to
  `origin` (Forgejo)
