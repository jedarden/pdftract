# pdftract-1ce1beaf — Dated duplicate-owner sweep, mmap-suite runtime re-run (re-run 7)

**Date of this sweep:** 2026-09-08/09 (bead created 2026-09-08T12:53:48Z; re-run 2 executed
~20:35Z; re-run 3 executed ~21:00Z; re-run 4 executed ~22:13Z; re-run 5 executed 22:51Z;
re-run 6 executed 2026-09-09T01:06Z; **this re-run 7 executed 2026-09-09T05:34Z**, on the
eighth dispatch — the fifth auto-split order, re-issued on expiry of the quarantine-round:2
set at 01:21:36Z)
**Scope of enumeration:** every bead created after **2026-09-08T05:15Z** (the prior ownership check), all statuses.
**Bead:** split child 4 of 5 of pdftract-cd0c29e6 (auto-split umbrella, coordination-only, read-only).
**Guard honored:** no cargo invocation, no test execution, no crates/ edits, no beads filed, no other bead status touched. Read-only `bead`/`git`/`jq` commands plus this note.

## Result (re-run 7, 2026-09-09T05:34Z)

**SWEEP RESULT 2026-09-09 (re-run 7): NONE FOUND.**

- Dispatch context: eighth dispatch, arrived as an **auto-split order** again
  (`failure-count:6`, `quarantine-round:2`, `cycling`, `quarantined`,
  `quarantine-until:2026-09-09T05:21:36Z`, expired → redispatch re-armed at 05:28:53Z). The
  count still decomposes into dispatch churn, not task failure: re-run 6's evidence-bearing
  close landed at 01:16:26Z and was **reopened 5m09s later** (01:21:35Z, `actor: system`, no
  stated reason), then quarantined and re-dispatched on expiry — the seventh consecutive
  close-and-reopen cycle on an answered question.
- Fresh enumeration from live store state at 05:34Z (all four statuses queried separately,
  `--limit 999999`, JSONL): **2753 beads total, 2753 unique IDs** (disjoint, complete), **still
  72 created after 05:15Z — identical to re-runs 2, 3, 4, 5 and 6.** Newest bead store-wide is
  STILL pdftract-f141d045 at **2026-09-08T19:44:55Z**: zero beads created in the ~9h49m since,
  so no new candidate can exist and there is nothing new to classify.
- Programmatic set-diff of all 72 live IDs against this note: **zero uncovered** — every
  candidate already carries a classification below.
- Owner chain re-verified live, still all `open`: pdftract-b716bac5 (rev 13), fa233a5e
  (rev 14), 030e8414 (rev 4), ec0e6526 (rev 1), 38700c39 (rev 1) — runtime child 030e8414
  remains open and unexecuted.
- Dependency edges re-checked across all 72 from list JSON: **zero** blockers point at any
  owner-chain bead (b716bac5, fa233a5e, 030e8414, ec0e6526, 38700c39) — re-runs 2–6 hold.
- Status mix of the 72: **31 closed / 39 open / 2 in_progress** — the in-progress pair is this
  bead and pdftract-a1d88582 (Class B read-only commit forensics). No member executes tests.
- No commit has touched `crates/pdftract-core/src/source/` since 05:15Z (git log re-checked at
  HEAD `9b6553a0`), so criterion-(a) runtime evidence is still not landed and no "duplicate
  re-run already satisfied" branch applies.
- Sibling churn datapoint: consolidation child pdftract-3128538a (re-run 6 recorded it closed)
  was closed 23:11:07Z and **reopened 43 s later** by `system` with no reason — the same
  wheel this bead is on.

### Split declined (this dispatch's order)

Declined for the four reasons recorded under re-run 4 and repeated under re-runs 5 and 6,
unchanged: the bead's own acceptance criteria say **"no beads filed, no other bead status
touched"** — the split order violates the contract it dispatches against; the task is atomic
(one enumeration pass + one dated note), now satisfied by six consecutive executions of this
bead (re-runs 2–7); a third coordination level multiplies the redispatch churn that *is* the
failure count (sibling 1f1cf7a5 already at `failure-count:6` / `cycling` / `quarantined`, and
3128538a reopened 43 s after its close); and same-day sibling beads filed dated split-decline
notes and closed on committed evidence instead. The terminal action remains the evidence close
of this bead.

## Result (re-run 6, 2026-09-09T01:06Z)

**SWEEP RESULT 2026-09-09 (re-run 6): NONE FOUND.**

- Dispatch context: seventh dispatch, arrived as an **auto-split order** again
  (`failure-count:5`, `quarantined`, `cycling`, `quarantine-until:2026-09-09T00:58:29Z`,
  expired → redispatch re-armed). The count still decomposes into dispatch churn, not task
  failure: every prior close (re-runs 2–5, commits `45dd32ae` / `efb27707` / `df7b7de2` /
  `a0dc9569`) carried a dated PASS result and was reopened with no stated reason; the log shows
  the 20:54:29/20:54:40Z double-claim 11 s apart on one shared checkout and two
  `exit_code: 0` failure events landing after evidence-bearing commits existed.
- Fresh enumeration from live store state at 01:06Z (all four statuses queried separately,
  `--limit 999999`, JSONL): **2753 beads total, 2753 unique IDs** (disjoint, complete), **still
  72 created after 05:15Z — identical to re-runs 2, 3, 4 and 5.** Newest bead store-wide is
  STILL pdftract-f141d045 at **2026-09-08T19:44:55Z**: zero beads created in the ~5h22m since,
  so no new candidate can exist and there is nothing new to classify.
- Programmatic set-diff of all 72 IDs against this note: **zero uncovered** — every candidate
  already carries a classification below.
- Owner chain re-verified live, still all `open`: pdftract-b716bac5 (rev 13), fa233a5e
  (rev 14), 030e8414 (rev 4), ec0e6526 (rev 1), 38700c39 (rev 1) — the rev bumps on
  fa233a5e/030e8414 are label/redispatch churn, not work; runtime child 030e8414 remains open
  and unexecuted.
- Dependency edges re-checked in list JSON across all 72: **zero** blockers point at any
  owner-chain bead (b716bac5, fa233a5e, 030e8414, ec0e6526, 38700c39) — re-runs 2–5 hold.
- Status mix of the 72: **30 closed / 41 open / 1 in_progress** — the sole in-progress bead is
  this bead itself. No member executes tests.
- No commit has touched `crates/pdftract-core/src/source/` since 05:15Z (git log re-checked at
  HEAD `0e04fc38`), so criterion-(a) runtime evidence is still not landed and no "duplicate
  re-run already satisfied" branch applies.

### Split declined (this dispatch's order)

Declined for the four reasons recorded under re-run 4, unchanged: the bead's own acceptance
criteria say **"no beads filed, no other bead status touched"** — the split order violates the
contract it dispatches against; the task is atomic (one enumeration pass + one dated note),
now satisfied by five consecutive executions of this bead (re-runs 2–6); a third coordination
level multiplies the redispatch churn that *is* the failure count (parent cd0c29e6, sibling
1f1cf7a5 already at `failure-count:6` / `cycling` / `quarantined`); and same-day sibling beads
filed dated split-decline notes and closed on committed evidence instead. The terminal action
remains the evidence close of this bead — its sole blocker pdftract-d2fd5467 and sole
dependent pdftract-3128538a are both already closed, so closure retires the unit rather than
starving a chain.

## Result (re-run 5, 22:51Z)

**SWEEP RESULT 2026-09-08 (re-run 5): NONE FOUND.**

- Dispatch context: sixth dispatch, arrived 22:45:07Z as an **auto-split order**
  (`failure-count:4`, `quarantine-until:2026-09-08T22:43:48Z`, expired). The count decomposes
  from `.beads/events.jsonl` exactly as re-run 4 recorded: 20:43:33Z `glm-spaxel2` success
  (re-run 2), 21:19:05Z `glm-tgp` failure-after-commit (re-run 3, `efb27707`), 21:41:06Z
  `glm-spaxel2` failure (no new commit), 22:13:48Z `glm-armor` **`outcome: success`** (re-run 4,
  `df7b7de2`) — after which the evidence-bearing close was reopened again and this dispatch
  was queued. Re-run 4's close succeeded; the reopen is dispatcher churn, not task failure.
- Fresh enumeration from live store state at 22:51Z (all four statuses queried separately,
  `--limit 999999`, JSONL): **2753 beads total, 2753 unique IDs** (disjoint, complete), **72
  created after 05:15Z — identical to re-runs 2, 3 and 4.** Newest bead store-wide is still
  pdftract-f141d045 at **2026-09-08T19:44:55Z**: zero beads created since the re-run 2/3/4
  window, so no new candidate can exist.
- Programmatic set-diff of all 72 IDs against this note (bare 8-hex form, since continuation
  children are listed unprefixed): **ALL 72 already classified below — nothing uncovered,**
  nothing new to classify.
- Owner chain re-verified live, still all `open`: pdftract-b716bac5 (rev 13), fa233a5e
  (rev 11), 030e8414 / ec0e6526 / 38700c39 (rev 1 each). The runtime-run child 030e8414
  remains open and unexecuted.
- Dependency edges re-grepped across all 72: **zero** blockers point at any owner-chain bead
  (b716bac5, fa233a5e, 030e8414, ec0e6526, 38700c39) — re-runs 2–4 hold.
- Status mix of the 72: **30 closed / 41 open / 1 in_progress** — the sole in-progress bead is
  this bead itself. (d5598671 closed since re-run 4; the 61ab92de in-progress noted there is
  no longer open.) No member executes tests.
- No commit has touched `crates/pdftract-core/src/source/` since 05:15Z (git log re-checked at
  HEAD `2241ea6b`), so criterion-(a) runtime evidence is still not landed and no "duplicate
  re-run already satisfied" branch applies.

### Split declined (this dispatch's order)

Declined for the four reasons recorded under re-run 4, unchanged: the bead's own acceptance
criteria say **"no beads filed, no other bead status touched"**; the task is atomic (one
enumeration pass + one dated note, now satisfied at HEAD four times); a third coordination
level multiplies the redispatch churn that *is* the failure count (parent cd0c29e6 at
`failure-count:4`, sibling 1f1cf7a5 at `failure-count:6` / `cycling` / `quarantined`, this bead
now reissued on the expiry of a quarantine that re-arms each cycle); and seven same-day sibling
beads have filed dated split-decline notes and closed on committed evidence instead. Terminal
action is the evidence close of this bead, which unblocks the one dependent
(pdftract-3128538a, umbrella child 5, the consolidation bead); sole blocker pdftract-d2fd5467
is already closed.

## Result (re-run 4, ~22:15Z)

**SWEEP RESULT 2026-09-08 (re-run 4): NONE FOUND.**

- Dispatch context: this fifth dispatch arrived as an **auto-split order** (`failure-count:3`,
  `quarantine-until:2026-09-08T22:01:06Z`, expired). The failure count decomposes, from
  `.beads/events.jsonl`, into dispatch churn — not task failure:
  - 20:30:15Z claim `glm-spaxel2` → 20:43:33Z **`outcome: success`** (re-run 2, commit `45dd32ae`).
  - 20:54:29Z claim `glm-tgp` and 20:54:40Z claim `glm-roam-16` — a **double-claim 11 s apart**
    on one shared checkout. `glm-tgp` ran 20:54:30→21:19:05Z, committed re-run 3 (`efb27707`,
    21:17:45Z), then reported `outcome: failure` (exit_code 0). `glm-roam-16` has no dispatch
    event logged.
  - 21:31:07Z reclaim `glm-spaxel2` → 21:41:06Z `outcome: failure` (exit_code 0), no new commit.
  - Both failure events carry `exit_code: 0` and land after evidence-bearing commits exist at
    HEAD — the same shape as the six same-day sibling declines
    (`d5598671`, `1f1cf7a5`, `7a8da184`, `ff5c577d`, `686bdc40`, `87de95ea`).
- Fresh enumeration from live store state (all four statuses queried separately,
  `--limit 999999`, JSONL): **2753 beads total, 2753 unique IDs**, **72 created after 05:15Z** —
  identical to re-runs 2 and 3. The newest bead store-wide is still pdftract-f141d045 at
  **2026-09-08T19:44:55Z**, so *zero* beads have been created since re-run 3's window.
  Programmatic set-diff of all 72 eight-hex IDs against this note: **NONE uncovered** — every
  candidate already carries a not-owner classification below; nothing new to classify.
- Owner chain re-verified live, still all `open`: pdftract-b716bac5 (rev 13), fa233a5e (rev 11),
  030e8414 / ec0e6526 / 38700c39 (rev 1 each). The runtime-run child 030e8414 remains open and
  unexecuted.
- Dependency edges re-grepped across all 72: **zero** blockers point at any owner-chain bead
  (b716bac5, fa233a5e, 030e8414, ec0e6526, 38700c39) — re-runs 2 and 3 hold.
- Status mix of the 72: 29 closed / 41 open / 2 in_progress (this bead and pdftract-d5598671,
  whose worker also declined its auto-split order at 22:05Z, commit `b8d3f12b`). Neither
  in-progress bead executes tests.
- No commit has touched `crates/pdftract-core/src/source/` since 05:15Z (git log re-checked), so
  criterion-(a) runtime evidence is still not landed and no "duplicate re-run already satisfied"
  branch applies.

### Split declined (this dispatch's order)

The order to create 3–5 children and convert this bead to an umbrella is **declined**, on record:

1. This bead's own acceptance criteria say **"no beads filed, no other bead status touched"** —
   the split order directly violates the contract it dispatches against.
2. The task is atomic: one enumeration pass + one dated note. Its acceptance criteria are
   satisfied at HEAD three times over (re-runs 2, 3, 4). Chained children ("enumerate" →
   "classify" → "write note") are not independently closable units; each would only add a
   claim/dispatch/commit cycle to an already-answered question.
3. A third coordination level multiplies the churn instead of fixing it: parent cd0c29e6 is at
   `failure-count:4`, and its already-split child pdftract-1f1cf7a5 is at **`failure-count:6`,
   labeled `cycling` and `quarantined`** — the split is the failure mode here, not the remedy.
   Each new child is a fresh redispatchable unit in that loop.
4. Fleet precedent, same day: six sibling beads filed dated split-decline notes and closed on
   committed evidence instead; no `SPLIT_COMPLETE` was emitted for those either.

Terminal action is the evidence close below, which unblocks the one dependent
(pdftract-3128538a, umbrella child 5, the consolidation bead). Sole blocker pdftract-d2fd5467 is
already closed.

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
