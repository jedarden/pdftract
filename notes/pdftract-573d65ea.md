# pdftract-573d65ea — Land or discard stranded uncommitted diagnostics.rs additions

**Outcome: STRANDED → LANDED.** The residue was ownerless; it is now landed at
HEAD, gate-verified, and pushed.

## Ownership determination (done FIRST, before touching anything)

Verdict: **stranded** — residue of `pdftract-d79310ab`'s three failed attempts
(the parallel dispatch of the same scope as parent `pdftract-b75df06a`; the
known double-dispatch pattern). Evidence:

1. **Attempt history** (`bead why --id pdftract-d79310ab`, cross-checked in
   `.beads/checkpoint/forensic.jsonl`):
   - 2026-09-14T19:42:44Z `work_failure / gate:default_rust` (actor glm-pdftract)
   - 2026-09-14T21:01:32Z `indeterminate / timeout` (actor glm-seam)
   - 2026-09-15T02:02:57Z `indeterminate / timeout` (actor glm53-adc)
   - then `released` + `dependency_added` by the dispatcher.
2. **The bead is parked, not active**: `pdftract-d79310ab` is `in_progress` but
   effectively **blocked** behind `pdftract-46b044b3` (created 02:27:49Z by the
   dispatcher, still **open, unassigned**, untouched since creation). It cannot
   re-enter the ready frontier until that closes.
3. **No live worker**: `pgrep` for needle workers at dispatch time showed zero
   workers with `--workspace /home/coding/pdftract` (all running workers target
   FABRIC, jedarden.com, ibkr-mcp, agent-archivist, tradegraph-platform,
   irreversible-command-gate, .needle/roam-only). Heartbeats
   (`.beads/heartbeats.jsonl`) carry no per-bead claims; last pdftract
   heartbeat is my own dispatch's.
4. **Files cold**: the three unstaged files' mtimes were 2026-09-14
   21:49–23:08 EDT — the window of d79310ab's second/third attempts — untouched
   for ~15 h before my dispatch.
5. **Prior strand's finding refined**: `pdftract-e3db48eb` (tradegraph,
   12:27Z) had called the residue "in-flight under ACTIVE bead
   pdftract-d79310ab". That bead's activity had in fact ended at 02:02Z with
   an indeterminate timeout; tradegraph correctly declined to sweep the files
   into its own verification-only pass, but ownership was never settled. This
   bead settles it.

## What the residue actually was (re-derived at HEAD)

The edits were written 2026-09-14 evening, BEFORE the sibling landings
(c034d6ca 05:29Z, 9497b721, 23cf6bb4, 56a4f6c7 07:19Z). Reconciling against
current HEAD revealed the residue is **one atomic feature spanning four
paths** — not three:

- `crates/pdftract-core/src/diagnostics.rs` (+389): `DiagCode::ALL` (113
  variants, cfg-gated cjk arms), `DiagCode::from_name` (inverse of `name()`),
  free `suggested_action(code) -> Option<&'static str>` over
  `DIAGNOSTIC_CATALOG`, the canonical `impl From<&Diagnostic> for
  DiagnosticJson` (hint populated from the catalog), and a 4-test
  `diagnostic_json_round_trip_tests` module.
- `crates/pdftract-core/src/schema/mod.rs` (staged, −24/+2): **the staged
  deletion of schema's interim `From` impl (`hint: None`)** — the other half
  of the impl move into `diagnostics.rs`. Verified worktree == index for this
  path, so committing it swept nothing else. I added a doc pointer on
  `DiagnosticJson` to the conversion's new home.
- `crates/pdftract-core/tests/diagnostics_catalog_drift.rs` (+131):
  category↔`DiagCode::category()` cross-check, `category_severity_
  classification_is_stable` severity map, `actionable_catalog_codes_have_
  structured_hints`, documented-codes-must-exist-in-catalog guard. I added an
  `ALL.len() == DIAGNOSTIC_CATALOG.len()` pin.
- `crates/pdftract-core/tests/test_helpers/diagnostics.rs` (+29/−13): doc
  updates for the `CODE:`-prefixed legacy string form and
  `metadata.diagnostics_detailed` as the object-form source. I sharpened one
  doc example.

Landing only the three unstaged paths **does not compile**: E0119, two
conflicting `From<&Diagnostic> for DiagnosticJson` impls (diagnostics.rs:2875
vs schema/mod.rs:856) — reproduced in the first gate run. The four together
are the move the original author intended; the staged half was stranded
residue of the same dead attempts, not an active worker's edit (no live
workers, see above).

**Contract validation against landed code**: HEAD's `From` impl set
`hint: None` while the landed `docs/integrations/diagnostics-codes.md`
("emitted diagnostics include a hint today") and the landed
`diagnostics_compat.rs` field table (`hint` — "only when known") both specify
the catalog hint, and landed `extract.rs` feeds `metadata.diagnostics_detailed`
through `DiagnosticJson::from` — so at HEAD the documented hint contract was
not actually flowing. This landing makes impl, docs, and compat table agree.
`DiagCode::Display == name()` makes the moved impl's `code: name()` mapping
byte-identical to the old `code: to_string()`.

## Verification (private extraction, non-killed runs)

Isolation: `git archive HEAD` extraction at `/build/pdftract-573d65ea-check`
plus exactly these four files — excludes the ~60 other workers' uncommitted
files in the shared checkout. Every run `timeout --kill-after=30s 570s`-wrapped.

| Check | Result |
|---|---|
| `cargo check --all-targets` (final bytes) | **PASS, exit 0**, 0 errors |
| `--lib diagnostic_json_round_trip` (default feats) | **4 passed, 0 failed** |
| `--test diagnostics_catalog_drift` | **5 passed, 0 failed** |
| `--test diagnostics_serialization_format` (landed contract) | **7 passed, 0 failed** |
| `--test diagnostics_surface_mirror` (landed contract) | **8 passed, 0 failed** |
| round-trip + drift under `--features cjk` | **4 + 5 passed, 0 failed** |
| `rustfmt --check` on the four files | clean (1 line joined) |

`every_code_round_trips_through_structured_json` /
`from_name_resolves_every_code_and_rejects_unknown` are the dynamic proof that
`ALL` (113 entries), `from_name`, and `name()` agree across every variant —
static cross-checks agreed (my first static checker had regex bugs around
digit-containing names like `ASCII85`; the test run is authoritative).

Predispatch snapshot `.needle-predispatch-sha` = `468ba16b` == HEAD at
dispatch; the landed blob differs from it by the full feature (538/37 lines),
i.e. real work, not a no-op.

## WARN (pre-existing, out of scope)

- Bare `cargo test --all-targets` was NOT run tree-wide: HEAD carries ~300
  pre-existing test failures unrelated to diagnostics (repo memory), and the
  shared tree holds ~60 other workers' uncommitted files. Gate + the four
  diagnostics-relevant suites are the scoped verification.
- `diagnostics_surface_mirror`'s extraction-driven leg self-skips (pre-existing
  tree-wide `extract_pdf` failure — see memory `pdftract-extract-broken-everywhere`).
- `pdftract-d79310ab` remains in_progress/blocked behind open unassigned
  `pdftract-46b044b3`; its assignee may find this work already landed — that is
  the point (landing stranded work unblocks the chain: verify-children
  `pdftract-3ebde655` / `pdftract-42d7064c` / `pdftract-e3aa1009`).
- Lineage: local main == origin/main (`0 0` at fetch) — the pre-scrub
  divergence in repo memory no longer applies; plain `git push origin main`.

## Artifacts

- Commit: `fix(pdftract-573d65ea): ...` — four explicit pathspecs, no `git add -A`.
- Private extraction removed after success (owner cleans up, per house rules).
