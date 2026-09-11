# pdftract-3c39e12c — 5th auto-split re-issue DECLINED (evidence close)

**Date:** 2026-09-11 (dispatch 12:34:32Z, quarantine round 5 expired 12:33:48Z)
**Bead:** Publish the compile-gate verdict handoff and verify a clean workspace
**Re-issue:** 5th auto-split order (split into 3–5 children + umbrella conversion).
Prior declines: `afe0317a` (1st), `9b6553a0` (2nd), `9976c4ff` (3rd), `0442ee12`
(4th, evidence close at rev 24, checkpoint `a6725f75`). Not executed.

## Why the split is declined on the merits

1. **The bead is already satisfied and was already decomposed once.** The 1st
   split generation closed as `b6d69433` (publication), `d5598671` (orphans),
   `778fc59c` (gate status, itself re-affirmed 8×, last at `dcd16fdc`),
   `f141d045` (split record, `cc83905b`); their blocker dependencies were
   removed 2026-09-09T19:33:21Z (forensic seq 6013–6015). A further split
   generation would duplicate closed work one-for-one against the
   duplicate-bead rule and manufacture treadmill fodder.
2. **`failure-count:9` is not 9 failures.** forensic.jsonl (counted by
   `event.issue_id`, not string grep) shows **6 closed / 6 reopened** — every
   reopen an `actor: system` reopen of an evidence-bearing close, no verifier
   reason. Zero genuine work failures on record; the deliverable has passed
   every determination. The label counter and the event log disagree, which
   is itself evidence the counter is bumping on reason-less reopens.
3. **The label set is self-contradictory** (`split-child` AND `umbrella`,
   `quarantine:failure-count:5`…`:9` stacked) — dispatcher artifacts of the
   re-arm loop, not state a worker should reconcile by inventing children.

## Re-derivation at HEAD (2026-09-11, read-only; no cargo, no crates/ edits)

Local HEAD == `origin/main` == `4a64c0ae` (fetched this session, 0/0
ahead/behind — no divergence episode this round). All four acceptance
criteria PASS:

1. **Handoff note** — `notes/b716bac5-child1-handoff.md` tracked and clean;
   quotes `VERDICT: NO-GO` verbatim from `notes/b716bac5-child1.md`, gate
   exit code **101**, certifies child SHAs `e9036d20` / `336bf8ee` /
   `10df3676`. PASS.
2. **Publication** — `git fetch origin && git merge-base --is-ancestor` exits
   0 for `1702ba6b` (the handoff-citing commit) vs `origin/main`. PASS.
3. **Orphan sweep** — self-match-safe EREs `cargo[ ]test` and
   `pdftract[ ]mcp` both return no matches (rc=1); `ps -eo comm=` over all
   `pdftract`-string hits classifies them as `needle`/`bash` (this worker's
   own harness shells) — zero cargo/rustc/pdftract binaries. PASS.
4. **Citing commit pushed** — `1702ba6b` message cites pdftract-3c39e12c and
   is an ancestor of `origin/main`. PASS.

## NO-GO stands (10th determination)

- 85 commits in `0364a3a8..origin/main` (`4a64c0ae`), **zero** touching
  `crates/` at all.
- Fixing-commit test `git log 0364a3a8..origin/main -- crates/pdftract-core/src/classify.rs
  crates/pdftract-core/src/render/scanline.rs` is **empty**.
- GO rule (fixing commit on `origin/main` for BOTH classify.rs AND
  render/scanline.rs) is unmet → **VERDICT: NO-GO stands.** The published
  handoff remains current; the gate-status sibling (`778fc59c`) owns the
  next re-affirmation if one is ordered.

## If re-issued again

Re-derive at HEAD and re-close with evidence. The load-bearing facts are:
`1702ba6b` (and the chain SHAs) on `origin/main`, the empty fixing-commit
test for the two clean-tree inventory files, and the clean per-comm orphan
sweep. None of them depend on this bead remaining open.
