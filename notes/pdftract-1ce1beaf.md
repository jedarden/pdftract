# pdftract-1ce1beaf — Dated duplicate-owner sweep, mmap-suite runtime re-run (re-run 3)

**Date of this sweep:** 2026-09-08 (bead created 12:53:48Z; re-run 2 executed ~20:35Z; **this
re-run 3 executed ~21:00Z**, after the bead's `verification-failed` reopen)
**Scope of enumeration:** every bead created after **2026-09-08T05:15Z** (the prior ownership check), all statuses.
**Bead:** split child 4 of 5 of pdftract-cd0c29e6 (auto-split umbrella, coordination-only, read-only).
**Guard honored:** no cargo invocation, no test execution, no crates/ edits, no beads filed, no other bead status touched. Read-only `bead`/`git`/`jq` commands plus this note.

## Result (re-run 3, ~21:00Z)

**SWEEP RESULT 2026-09-08 (re-run 3): NONE FOUND.**

- Enumeration re-run from live store state, all four statuses queried separately
  (`open` / `in_progress` / `deferred` / `closed`, `--limit 999999`, JSONL — one object per
  bead, so no jq array concat): **2753 beads total, 2753 unique IDs** (disjoint, complete).
  Post-05:15Z count: **72 — identical to re-run 2.** The newest bead in the entire store is
  pdftract-f141d045 at **2026-09-08T19:44:55Z**, so *zero* beads have been created since the
  re-run 2 window ended, let alone since its execution. Every one of the 72 IDs is checked
  present in the re-run 2 classification below — nothing new to classify.
- Owner chain re-verified live, still all `open`: pdftract-b716bac5 (rev 13), fa233a5e
  (rev 11), 030e8414 / ec0e6526 / 38700c39 (rev 1 each). The runtime-run child 030e8414
  remains open and unexecuted.
- Dependency edges re-grepped across all 72: **zero** blockers point at any owner-chain bead
  (b716bac5, fa233a5e, 030e8414, ec0e6526, 38700c39) — re-run 2's finding holds.
- Status mix of the 72: 28 closed / 42 open / 2 in_progress — the two in_progress are this
  bead and pdftract-61ab92de (a Class B read-only README/publication certification); neither
  executes tests.
- No commit has touched `crates/pdftract-core/src/source/` since 05:15Z (git log re-checked),
  so criterion-(a) runtime evidence is still not landed and no "duplicate re-run already
  satisfied" branch applies.

## Result (re-run 2, ~20:35Z)

**SWEEP RESULT 2026-09-08 (re-run 2): NONE FOUND.**
No bead created after 2026-09-08T05:15Z is a second owner of the bf-5o22rf criterion-(a)
mmap-suite runtime re-run. **pdftract-b716bac5 and its serial chain remain the single owner:**
fa233a5e (compile gate) → 030e8414 (run source::mmap tests) → ec0e6526 (runtime verdict) →
38700c39 (cross-link). Verified live at sweep time: all five chain beads still `open`; runtime
evidence is NOT yet landed (gate verdict NO-GO, cargo exit 101, 89 errors, 0 in source/mmap.rs),
so no "closed with evidence commit" branch applies and no duplicate re-run bead is warranted.

## Method

`bead list --status <open|in_progress|deferred|closed> --json --limit 999999` (plain `bead list`
is claim-ordered and page-caps at 100, so statuses were queried separately), filtered on
`created_at >= 2026-09-08T05:15:00Z` via jq. **72 candidates** total — the exact 24 beads the
prior 12:18Z sweep covered (created 05:47Z–12:01Z), plus **48 new** (created 12:53Z–19:44Z).
Cross-checks: descriptions grepped for `mmap` / `source::mmap` / test-execution language;
dependency edges grepped for the owner chain (b716bac5, fa233a5e, 030e8414, ec0e6526, 38700c39)
— **zero** post-05:15Z bead depends on the owner chain.

## Prior 24 (05:47Z–12:01Z) — prior classifications confirmed, still hold

None is a second owner; same three nearest-neighbor classes as the 12:18Z sweep:

- **classify.rs compile-evidence chains (16):** pdftract-c1f3cf4b (+ fe9aa999, 2c5787e0,
  d53efad3), pdftract-ba4e7150 (+ 15511c0b, 180898ae, 52170e18), pdftract-d077ca93
  (+ 903ece20, 2f8f6402, 54df39bb), pdftract-cf349714 (+ 9935e4b6, 867642aa, bf2b6ee5) —
  `--no-run` compile measurement of classify.rs, a different artifact; produces no criterion-(a)
  runtime evidence.
- **gate re-split of the owner's own chain (4):** pdftract-3e8309f4, 68091d06, 003ddcd1,
  3c39e12c — children of the gate pdftract-fa233a5e, which is the owner b716bac5's own
  precondition sub-work, not a competitor.
- **coordination children of pdftract-c08a9ede / cd0c29e6 (4):** pdftract-adb806f2,
  eb967150, c02b4878, pdftract-cd0c29e6 itself — ownership checks and note/verifier work.

## New 48 (created after 12:18Z) — each classified NOT-OWNER

**Class A — cd0c29e6's own split family, coordination-only (9).**
pdftract-cae26b95, pdftract-d2fd5467, pdftract-1f1cf7a5, pdftract-3128538a, pdftract-1ce1beaf
(this bead; unique-ref `split:cd0c29e6-childN`), plus 1f1cf7a5's own split children
pdftract-62d653ec, pdftract-3df8a46a, pdftract-686bdc40, pdftract-0078d4a6.
*Reason:* they only confirm/consolidate the already-recorded non-owner reasons and produce this
dated sweep; none executes or re-scopes the re-run — by construction they are the
anti-duplication coordination work itself.

**Class B — notes/evidence README + commit-0364a3a8 forensics, read-only git/bead-state
certification (28).**
cf349714's original split: pdftract-0a91c85a, ab6a49c2, 758dff02, 974805d9.
cf349714's re-split: pdftract-eaa42093, e3e4847b, f4043374, 87de95ea, a930c392, 61ab92de,
bf10a431. ab6a49c2's split: pdftract-1f91ea1a, 50b9386b, b00f33d6, 92636b5d.
50b9386b/c0acafa1 re-splits: pdftract-c0acafa1, c06c214e, 6954bfba, 46f4ffb9, ff5c577d,
7a8da184, bc29f726, 0cf5f8c4. f4043374's split: pdftract-36076d6d, 540e5add, 1d0280c3,
a1d88582. Plus 17:23Z quartet on the published note: pdftract-c04460c3, 02de535f, a4eb4a16,
78053a71.
*Reason:* every one is an explicit single-purpose read-only check (diffstat/path-set/message-shape
of one docs commit, ancestor checks on origin/main, README-content and bead-terminal-state
certification, orphan `pgrep` sweeps); descriptions forbid cargo and crates/ edits — they certify
documentation artifacts, they do not run the mmap suite.

**Class C — owner-chain handoff publication + hygiene (11).**
pdftract-082f2f0f (verify handoff note vs committed verdict), ab076af2 (handoff commits on
forgejo main), ca47706e (orphan-process sweep, "no cargo invocation"), f34a11cf (write
consolidated handoff note), b6d69433 (certify forgejo publication per commit), d5598671
(orphan-process hygiene), 778fc59c ("do NOT re-run the gate… git archaeology, not a compile"),
f141d045 (consolidate split record — note-writing only), plus the c04460c3/02de535f/a4eb4a16
audit-republish trio already listed in Class B.
*Reason:* they publish, audit and re-stamp the owner chain's existing NO-GO verdict; several
state in so many words that re-running the gate/suite is "this chain's over-budget failure
mode" — service to the single owner, not competing ownership.

## Conclusion

Consistent with the 12:18Z sweep and closed sibling pdftract-adb806f2 (10:58Z): the single owner
of the bf-5o22rf criterion-(a) mmap-suite runtime re-run is **pdftract-b716bac5** via its chain
fa233a5e → 030e8414 → ec0e6526 → 38700c39. Still open, evidence still not landed, gate still
NO-GO (exit 101; classify.rs 54 / type3_rasterizer.rs 29 / scanline.rs 5 / content_stream.rs 1
errors). Nothing to file; a duplicate re-run bead would be a second owner and is explicitly
not created by this bead.
