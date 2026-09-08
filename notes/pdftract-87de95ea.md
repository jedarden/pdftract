# pdftract-87de95ea — auto-split dispatch DECLINED (2026-09-08)

## What arrived

The NEEDLE auto-split path dispatched pdftract-87de95ea as "failed 3 times in a
row, split into 3–5 children, convert to umbrella." This note records why the
split was **not** performed.

## Why the premise is false

`failure-count:3` is reopen churn, not three failed attempts. The forensic
checkpoint (`grep pdftract-87de95ea .beads/checkpoint/forensic.jsonl`) shows:

| Time (2026-09-08) | Event | Detail |
|---|---|---|
| 18:36:43 | closed | PASS, full evidence (worker `glm-armor`) |
| 18:39:39 | reopened by `system` | **no reason given** |
| 18:48:43 | closed | PASS, full evidence (worker `glm-seam`) |
| 18:51:21 | reopened by `system` | **no reason given** |
| 19:07:21 | closed | PASS, full evidence + provenance cross-checks (worker `glm-spaxel2`) |
| 19:08:14 | reopened by `system` | **no reason given** |

All three close reasons satisfy the bead's acceptance criteria as written:
four section headings quoted verbatim, blob hash quoted, working-tree drift
stated. None of the reopens carries a reason. This is the same pathology
recorded in `notes/pdftract-1f1cf7a5.md` and commit `a5521a88`: evidence-bearing
closes being reopened as `verification-failed`, then the failure count being
read as task size.

## Independent re-derivation at decline time (2026-09-08 ~19:34Z, HEAD = a5521a88)

Read-only, scoped to commits per the bead's hard rules:

- `git rev-parse HEAD:notes/evidence/README.md` →
  `8d7d1509ac7f2f47e2f5d9654c22490747a75dac` (same blob all three prior closes
  saw; the file is untouched since introducing commit `0364a3a8`).
- All four required sections present in the HEAD copy, headings verbatim:
  1. `## Purpose` — evidence artifacts for gated compile-check runs of this repo.
  2. `## Naming convention` — `<bead-id>-<kind>.txt` with worked examples
     (`pdftract-d077ca93-baseline.txt`, kinds `baseline`/`meta`/`raw`).
  3. `## Freshness rule` — every capture produced fresh by the run that commits
     it, never copied forward from a prior attempt.
  4. `## Evidence beads are measurement-only` — no `crates/` changes, no
     `cargo build`/`cargo test`.
  (A fifth section, `## Committing`, is also present and not one of the four.)
- Working tree matches HEAD for this path: `git diff HEAD -- notes/evidence/README.md`
  empty; `git status --porcelain -- notes/evidence/` empty. No drift, none fixed.

**Verdict: PASS.** The bead's acceptance criteria are met at HEAD right now.

## Why splitting is the wrong action

1. The task is atomic: one committed file, one check, evidence in the close
   reason. Its "children" could only be per-section re-reads — notes about a
   README, each destined for the same reopen churn.
2. The bead is itself `split-child` 1 of 4 of the existing re-split of umbrella
   `pdftract-cf349714`; a second split nesting under a fresh re-split deepens
   the treadmill, it does not converge.
3. Sibling `pdftract-a930c392` (child 2) is blocked on this bead. Adding a
   "parent depends on last child" edge here can only postpone the whole chain's
   closure further.
4. The repo's own README (freshness rule; measurement-only rule) argues against
   manufacturing artifact chains for an end state that already exists.

## Actions taken

- No child beads created; no `umbrella` label added; no dependency edges added;
  no SPLIT_COMPLETE emitted (no split was performed).
- Bead left open and released for the operator to decide, per the precedent of
  `pdftract-1f1cf7a5` (commit `a5521a88`).
- Read-only hard rules honored by this decline: no source changes, no build,
  no socket, no spawned server.

## Recommendation

**Close pdftract-87de95ea** on the strength of the three prior evidence-bearing
closes plus the re-derivation above — the acceptance criteria are met and
closing unblocks `pdftract-a930c392`. The fix that actually ends this treadmill
is upstream of the workers: the automated verifier is reopening valid closes
without a reason (4+ instances across the fleet on 2026-09-08); until that is
corrected, `failure-count` on verification-only beads measures the verifier,
not the work.
