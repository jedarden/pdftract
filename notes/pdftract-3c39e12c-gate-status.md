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

---

# Re-affirmation log

## 2026-09-08 (2nd check) — tip `e58fb369` — **NO-GO stands**

Re-derived from git alone after the tip advanced one docs commit past the 1st check
(`df7b7de2` → `e58fb369`, "consolidate single-owner conclusion for mmap re-run").
Still **no cargo run** — same rule, same gate evidence (raw log re-validated this
check: cargo summary line reads *"could not compile `pdftract-core` (lib test) due to
89 previous errors"*, exit 101; counting each diagnostic's **primary** `-->` location
reproduces the table exactly: classify.rs 54, type3_rasterizer.rs 29, scanline.rs 5,
content_stream.rs 1 = 89).

Fresh evidence at `e58fb369c951e5646e648913d3678fa859203eed`:

- `0364a3a8` still an ancestor of `origin/main`; range now **39 commits**, all
  docs-only (`0364a3a8..origin/main -- . ':(exclude)notes' ':(exclude).beads'` → empty).
- **Zero commits touch any of the four files** (`git log 0364a3a8..origin/main --
  <file>` empty for all four; `--since=2026-09-08` likewise empty).

| File | Errors at gate | Commits on `origin/main` since gate | Working tree |
|---|---|---|---|
| `classify.rs` | 54 | **none** | **clean** |
| `render/scanline.rs` | 5 | **none** | **clean** |
| `font/type3_rasterizer.rs` | 29 | none | modified (uncommitted, 10 lines) |
| `content_stream.rs` | 1 | none | modified (uncommitted, 305 lines) |

Rule applied unchanged: GO requires a fixing commit on `origin/main` for BOTH
`classify.rs` AND `render/scanline.rs`; both remain byte-identical to what the gate
compiled at `0364a3a8` and their combined 59 errors are still **committed at the tip**.
The two modified files are sibling in-flight edits, which the rule says can never clear
them.

> **VERDICT (2026-09-08, tip `e58fb369`): NO-GO stands.** No fixing commit SHA exists
> to quote. Children 2–4 of pdftract-3c39e12c must still not run tests until a commit
> to `main` fixes `classify.rs` and `render/scanline.rs`.
