# pdftract-3c39e12c — gate-status re-affirmation at the pushed tip (2026-09-08)

**Bead:** pdftract-778fc59c (auto-split child 3 of 4 of pdftract-3c39e12c; umbrella
pdftract-b716bac5; depends on child 2 pdftract-d5598671). Read-only git check of
whether the recorded compile-gate verdict still stands at the live tree state.
**No cargo was run** — the gate ran once (notes/b716bac5-child1-gate-raw.log,
exit 101) and re-running it is this chain's over-budget failure mode.

## Recorded verdict being re-affirmed

From `notes/b716bac5-child1.md` (bead pdftract-003ddcd1) and
`notes/b716bac5-child1-handoff.md`: **NO-GO** — gate exit **101**, 89 errors confined
to 4 files, run 2026-09-08T12:28:32Z at gate HEAD
`0364a3a89eabf5a02715b7ef7daba764f42d23d7`.

## Verdict rule applied

**NO-GO stands unless a fixing commit is verifiably present on `origin/main` for BOTH
tip-broken files (`crates/pdftract-core/src/classify.rs` AND
`crates/pdftract-core/src/render/scanline.rs`).** In-flight local edits alone never
clear those two — at gate time both were clean (unmodified), so their 59 combined
errors are *committed* at the pushed tip and can only clear via a commit to `main`.

## Live state checked (2026-09-08)

- `git fetch origin` — no-op; local HEAD == `origin/main` ==
  `df7b7de2658052edf2c48e51722ccd336da36e62`.
- `0364a3a8` is an ancestor of `origin/main` (`git merge-base --is-ancestor` → yes);
  36 commits sit between gate HEAD and the current tip.
- **All 36 are docs-only** (`git log 0364a3a8..origin/main -- . ':(exclude)notes'
  ':(exclude).beads'` → empty; every commit is a `docs(...)` touching `notes/`).

## Per-file table

| File | Errors at gate | Commits on `origin/main` since gate (`git log --since=2026-09-08`, and `0364a3a8..origin/main`) | Working tree (`git status --porcelain`) |
|---|---|---|---|
| `crates/pdftract-core/src/classify.rs` | 54 (E0609 x52, E0277 x2) | **none** | **clean** — still broken as committed at the tip |
| `crates/pdftract-core/src/render/scanline.rs` | 5 (E0277 x2, E0308 x2, E0599 x1) | **none** | **clean** — still broken as committed at the tip |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 (E0061 x24, E0599 x5) | none | modified (uncommitted sibling in-flight edits, 10 lines changed) — same shape as at gate time |
| `crates/pdftract-core/src/content_stream.rs` | 1 (E0061 x1) | none | modified (uncommitted sibling in-flight edits, 305 lines changed) — same shape as at gate time |

Both rule-critical files (`classify.rs`, `render/scanline.rs`) have **zero** commits on
`origin/main` since the gate and **zero** local modifications — identical bytes to what
the gate compiled against at `0364a3a8`.

## Resulting verdict

> ## VERDICT: **NO-GO stands** — re-affirmed 2026-09-08

No fixing commit exists on `origin/main` for either tip-broken file, so the rule's GO
condition is not met (no fixing commit SHA exists to quote — `git log
0364a3a8..origin/main -- <either file>` is empty). The gate remains uncleared:
`pdftract-core` (lib test) does not compile at the pushed tip, and children 2–4 of
pdftract-3c39e12c must still not proceed with any test run until a commit to `main`
fixes `classify.rs` and `render/scanline.rs`.
