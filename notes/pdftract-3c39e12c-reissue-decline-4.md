# pdftract-3c39e12c — 4th auto-split re-issue DECLINED (evidence close)

**Date:** 2026-09-10 (dispatch 04:21:38Z, quarantine round 4 expired 04:18:28Z)
**Bead:** Publish the compile-gate verdict handoff and verify a clean workspace
**Re-issue:** 4th auto-split order (split into 3–5 children + umbrella conversion).
Prior declines: `afe0317a` (1st), `9b6553a0` (2nd), `9976c4ff` (3rd). Not executed.

## Why the split is declined on the merits

1. **The bead is already satisfied and already an umbrella.** It carries a closed
   4-child decomposition from the 1st split generation: `b6d69433` (publication),
   `d5598671` (orphans), `778fc59c` (gate status, itself re-affirmed 8×, last at
   `dcd16fdc`), `f141d045` (split record, `cc83905b`). A second split generation
   would duplicate closed work one-for-one against the org duplicate-bead rule.
2. **`failure-count:8` is not 8 failures.** forensic.jsonl shows every reopen is an
   `actor: system` reopen of an evidence-bearing close, minutes later, with no
   verifier reason (last pair: close 12:13:31Z → reopen 12:18:26Z on 2026-09-09).
   Zero genuine work failures on record — the deliverable has passed every time.
3. **New dispatcher behavior this round:** at 2026-09-09T19:33:21Z the dispatcher
   removed all three blocker dependencies (`003ddcd1`, `f141d045`, `f34a11cf`,
   forensic seq 6013–6015) to re-arm dispatch after the 4th quarantine expiry.
   Dependency-stripping does not change the facts below.

## Re-derivation at HEAD (2026-09-10, read-only; no cargo, no crates/ edits)

All four acceptance criteria PASS at `origin/main` = `50198b13` (fetched this
session; local HEAD `e39cff1c` is diverged from origin, see below):

1. **Handoff note** — `notes/b716bac5-child1-handoff.md` tracked and clean; quotes
   `VERDICT: NO-GO` verbatim from `notes/b716bac5-child1.md`, gate exit code
   **101**, certifies child SHAs `e9036d20` / `336bf8ee` / `10df3676`. PASS.
2. **Publication** — `git merge-base --is-ancestor` exits 0 for all eight chain
   SHAs vs `origin/main`: `1702ba6b` (handoff citing commit), `10df3676`,
   `336bf8ee`, `e9036d20`, `afe0317a`, `9b6553a0`, `9976c4ff`, `dcd16fdc`. PASS.
3. **Orphan sweep** — self-match-safe ERE `cargo[ ]test|pdftract[ ]mcp|nextest`
   matched only this check's own shell (comm=bash); `ps -eo comm=` shows zero
   cargo/rustc/pdftract/nextest binaries. PASS.
4. **Citing commit pushed** — `1702ba6b` is an ancestor of `origin/main`. PASS.

## NO-GO stands (9th determination)

- 71 commits in `0364a3a8..origin/main`, **zero** touching `crates/`.
- Fixing-commit test `git log 0364a3a8..origin/main -- <4 inventory files>` is
  **empty** for `src/classify.rs` (54 errs), `src/render/scanline.rs` (5 errs),
  `src/font/type3_rasterizer.rs`, `src/content_stream.rs`.
- GO rule (fixing commit on `origin/main` for BOTH classify.rs AND
  render/scanline.rs) is unmet → **VERDICT: NO-GO stands.** The published
  handoff remains current.

## Divergence note

Local `main` and `origin/main` diverged this round (sibling bead `pdftract-8835d51a`
recorded a non-FF push rejection at `50198b13`). This note was committed locally,
then reconciled with a **merge commit** (no force-push) and pushed per org rules.

## If re-issued again

Re-derive at HEAD and re-close with evidence. The load-bearing facts are: the
eight chain SHAs above on `origin/main`, the empty fixing-commit test, and the
clean orphan sweep. None of them depend on this bead remaining open.
