# pdftract-f0d685f4 — umbrella evidence index: wasm32 feasibility spike closed out

**Date:** 2026-09-16 · **Bead:** `pdftract-f0d685f4` (umbrella head, `auto-split-parent`)

## What this bead asked for vs. what closed

The parent scope had four deliverables. All four landed via the auto-split chain
created 2026-09-15; every child is closed with an evidence-bearing outcome, and the
terminal child carries the go/no-go decision the parent required.

| Parent deliverable | Child (closed) | Outcome | Evidence |
|---|---|---|---|
| Test the hypothesized blocker table (memmap2, rayon, tempfile, dirs, nix, zstd, parking_lot, chrono) | `pdftract-c3c31c30` — verify the untested wasm32 dependency blockers in an isolated scratch crate | verified_success 2026-09-15T14:09Z | commit `5ca97b49` · `notes/pdftract-c3c31c30.md` |
| Incremental per-dependency wasm32 check of pdftract-core itself, recorded pass/fail per crate | `pdftract-67a9f62e` — measure the per-dependency wasm32 blocker list of pdftract-core itself | verified_success 2026-09-15T15:16Z | commit `7c5368fc` · `notes/pdftract-67a9f62e.md` |
| Feature-gated cfg for the native-only dependencies | `pdftract-c1fceb36` — gate the verified wasm32-blocking dependencies in pdftract-core | closed (chain of fixes `166b401f`, `3aa2d58a`, `b1a08bbf`, repair `cebe1e1f`) | `notes/pdftract-c1fceb36.md` · strict-baseline note `cdd6efd4`: wasm32 strict baseline green, 0 new native test failures |
| Go/no-go decision updating/superseding ADR-010 with the actual blocker list + feature-flag surface | `pdftract-a7c93aa5` — reconcile ADR-010/ADR-011 with the verified wasm32 evidence and close the feasibility question | verified_success 2026-09-16T00:34Z | commit `b099716d` (HEAD) · `notes/pdftract-a7c93aa5.md` |

## Verified final state

- **Decision: Go, confirmed.** plan.md revision 1.5 carries a Verification amendment
  inside ADR-011: 16 ADR-011 claims reconciled claim-by-claim against the empirical
  notes (CONFIRMED / NUANCED / CORRECTED / SUPERSEDED per row), ADR-010 Status
  annotated (zstd fact right, memmap2 rationale wrong), no superseding ADR warranted.
- **Blocker list is measured, not hypothesized:** two C-backed blockers outside the
  wasm set (zstd-sys behind `cache`, ring behind `remote`) plus nix hollow-crate;
  the spike note's 8-dep "expected blockers" table is factually resolved — those
  deps compile (chrono green even without `wasmbind`; rand needed the `js` feature,
  the only standalone/in-graph divergence).
- **Feature-flag surface verified live:** at HEAD, bare
  `cargo check -p pdftract-core --no-default-features --target wasm32-unknown-unknown`
  → exit 0, `--features serde` variant → exit 0, `-p pdftract-wasm` → exit 0
  (re-measured by `pdftract-a7c93aa5` at `cdd6efd4`; no code changed since except
  docs `b099716d`, so the verdict stands).
- **CI leg:** `wasm32-check` at `.ci/argo-workflows/pdftract-ci.yaml:2187-2221`,
  mirrored identically into declarative-config.

## Why the umbrella closes now

A later auto-split order (2026-09-16) requested decomposing this parent again, but
the decomposition already existed — four `split-child` children, all closed — and
the terminal child is the decision record itself. Re-splitting would duplicate
closed work. This note is the umbrella's evidence index; the close reason cites it.

## Acceptance criteria (parent scope, mapped to children)

- **PASS** — blocker table exercised with verified pass/fail per crate (`c3c31c30`, `67a9f62e`).
- **PASS** — feature-gated cfg landed for the verified native-only blockers, native
  behavior unchanged (0 new native test failures) (`c1fceb36`).
- **PASS** — results recorded in docs/notes (`notes/pdftract-c3c31c30.md`,
  `notes/pdftract-67a9f62e.md`, `notes/pdftract-c1fceb36.md`, `docs/notes/bf-2uw30r.md` HEAD matrix).
- **PASS** — go/no-go decision produced, ADR-010/ADR-011 reconciled with the actual
  blocker list and feature-flag surface (`a7c93aa5`, plan.md rev 1.5).
