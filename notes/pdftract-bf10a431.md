# pdftract-bf10a431 — README umbrella re-certification

Re-certification record for the four-child re-split of umbrella
**pdftract-cf349714** (grandparent **pdftract-d077ca93**). This child (4 of 4)
is the only writer in the chain: it records outcomes already derived by
children 1–3 and leaves one current record. It does not re-create the
deliverable and does not re-commit any evidence artifact under `notes/evidence/`.

## Deliverable provenance

Both artifacts already exist on origin/main and are untouched by this bead:

| Commit | Subject | Paths touched |
|---|---|---|
| `0364a3a89eabf5a02715b7ef7daba764f42d23d7` (`0364a3a8`) | docs(pdftract-cf349714): add notes/evidence manifest README | `notes/evidence/README.md` only (+57, single path) |
| `d2510888b2f102039e986b51cb58b2dee4a395a3` (`d2510888`) | docs(pdftract-e3e4847b): record README split-chain verification note | `notes/pdftract-cf349714.md` only (+75, the prior consolidated record) |

## The four checks of the chain

| # | Bead | Check | Outcome | Evidence closed on |
|---|---|---|---|---|
| 1 | `pdftract-87de95ea` | Manifest README content at HEAD covers the four required sections | PASS | Committed copy only (`git show HEAD:notes/evidence/README.md`); blob `8d7d1509ac7f2f47e2f5d9654c22490747a75dac`, unchanged since `0364a3a8`; headings verbatim `## Purpose`, `## Naming convention`, `## Freshness rule`, `## Evidence beads are measurement-only`. Five evidence-bearing PASS closes on 2026-09-08 (18:36, 18:48, 19:07, 19:50, 20:26Z), the last re-derived at HEAD `c5242b6c`. |
| 2 | `pdftract-a930c392` | Introducing commit `0364a3a8` is single-path with no `crates/` drift | PASS | Close 20:32:18Z: single parent `e9036d20`; path list exactly `A notes/evidence/README.md`; numstat `57 0`; `crates/` path count 0; diffstat `1 file changed, 57 insertions(+)`; subject match confirmed; ancestor of origin/main `c5242b6c`. |
| 3 | `pdftract-61ab92de` | `0364a3a8` and `d2510888` are ancestors of origin/main | PASS | Close 20:40:17Z: both `git merge-base --is-ancestor <sha> origin/main` → exit 0; `git ls-remote origin main` = `c5242b6c` = local HEAD, 0 ahead / 0 behind. |
| 4 | `pdftract-bf10a431` | Record this re-certification note and push it | this note | Single-path commit (`notes/pdftract-bf10a431.md` only) pushed to origin main; both child-3 ancestry checks re-run against the new tip after the push (see close reason for the tip SHA). |

Children 1 and 3 each carry evidence-bearing PASS closes that the dispatcher
reopened without a stated reason (child 1 five times, child 3 once at
20:42:18Z); the outcomes above are those closes' content, re-stated here.

## Close-readiness

**The parent umbrella `pdftract-cf349714` is close-ready from this note alone.**
Its deliverable (`notes/evidence/README.md` at `0364a3a8`) is on origin/main,
its prior consolidated record (`d2510888`) is on origin/main, all four chain
checks above are PASS with checkable evidence, and this file is the single
current record of that fact. The umbrella closes last, after this bead.

## Re-issue log — auto-split re-issues declined

**2026-09-09, 4th auto-split re-issue: DECLINED (no children created).** The
dispatcher re-issued bead `pdftract-bf10a431` as "failed 3 times in a row, too
big or complex — split into 3–5 children." Both premises are contradicted by
the record and by fresh re-derivation:

- The three counted "failures" are not work failures. The forensic log shows
  three evidence-bearing PASS closes on this bead — 20:58:54Z, 21:20:41Z,
  22:19:26Z on 2026-09-08 — each reopened within minutes with no stated
  reason. `failure-count:3` counts those reopens.
- The scope is one atomic action (one note file, one-path commit, one push).
  It has no decomposition into 3–5 independently completable children; any
  such split would fabricate micro-beads for work already merged, and would
  re-dispatch workers onto evidence artifacts this chain forbids re-committing.

Fresh re-derivation at tip `7b7792cc` (= `git ls-remote origin main`, 0
ahead / 0 behind), all PASS:

| Check | Result |
|---|---|
| `notes/pdftract-bf10a431.md` exists, matches committed state | PASS — 37 lines, `git diff HEAD` empty |
| Commit `e07d070d` touches only that one path | PASS — 1 file changed, +37; `crates/` path count 0 |
| `e07d070d` is an ancestor of origin/main | PASS — `git merge-base --is-ancestor` exit 0 |
| Cited deliverables `0364a3a8`, `d2510888` are ancestors of origin/main | PASS — both exit 0 |

Decline recorded in the single-path addendum commit on top of tip `7b7792cc`;
the bead is re-closed on this evidence. No file under `notes/evidence/` and no
file under `crates/` was touched, and no build or test was run.
