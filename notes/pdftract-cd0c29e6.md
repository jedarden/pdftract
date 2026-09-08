# pdftract-cd0c29e6 — mmap-re-run sibling beads: duplicate-ownership check

**Bead:** pdftract-cd0c29e6 (split child 2 of 4 of umbrella pdftract-c08a9ede; blocked by
child 1, the owner-status check pdftract-adb806f2, which closed). Now also the
consolidation record for cd0c29e6's own auto-split (children 1–5, ref `split:cd0c29e6-childN`).
**Type:** read-only coordination check. **Guard honored:** no mmap-suite run, no cargo
test of any kind, no production code change, no bead filed, no other bead's status touched.
**Check performed:** 2026-09-08T11:41Z (re-verification of the 11:09 UTC pass recorded in
the bead notes; all conclusions re-confirmed unchanged against the live store at rev HEAD
`ddc65fb4`).
**Consolidation performed:** 2026-09-08T22:29Z by pdftract-3128538a (split child 5 of 5),
merging the child 1–4 verdicts below into the final single-owner conclusion (commit
`e58fb369`), and re-affirmed at **2026-09-08T23:04Z** on the bead's re-dispatch: every
status in this file re-checked against the live store at that time, and child 4's sweep
re-run 5 (22:51Z, `a0dc9569`) folded in. No conclusion changed.

## Conclusion (consolidated 22:29Z, re-affirmed 2026-09-08T23:04Z)

**pdftract-b716bac5 and its serial 4-child chain (pdftract-fa233a5e compile gate →
pdftract-030e8414 run source::mmap → pdftract-ec0e6526 verdict → pdftract-38700c39
cross-link/push) remain the SINGLE owner of the bf-5o22rf criterion-(a) mmap-suite
runtime re-run. No duplicate or competing owner exists — confirmed independently by the
2026-09-08T12:18Z coordination pass, children 1–3 of this split, and child 4's dated
sweep re-runs through re-run 5 (22:51Z, commit `a0dc9569`).**

**b716bac5 status re-checked at consolidation time (2026-09-08T23:04Z, on re-dispatch;
unchanged from the 22:29Z consolidation): Open, rev 13, unchanged since
2026-09-08T05:07:27Z — runtime evidence has still NOT landed and there is no evidence
commit to cite.** The suite has never executed. `notes/bf-5o22rf-child1.md` still ends on
the "append a runtime-evidence section here" handoff (line 128; last touched by
`b19a9e52`), `notes/b716bac5-child2.md` is still absent, and the gate remains NO-GO —
latest composition from child 4's sweep re-run 4: exit 101, 89 errors, all in sibling
TEST modules (classify.rs 54, font/type3_rasterizer.rs 29, scanline.rs 5,
content_stream.rs 1), zero in `source/mmap.rs`. Child 4's re-run 5 additionally
re-confirmed at HEAD `2241ea6b` that no commit has touched
`crates/pdftract-core/src/source/` since 05:15Z, so no "duplicate re-run already
satisfied" branch applies. The gate bead fa233a5e, InProgress under `glm-tgp` at 11:36Z,
has been released back to Open (updated 11:49Z) and is now itself carrying a re-split
(3e8309f4 / 68091d06 / 003ddcd1 / 3c39e12c). Per the task guard, no duplicate re-run
bead was filed.

## Non-owner reasons (one line each)

Statuses in the table below re-checked 2026-09-08T23:04Z — all six still Open, and each
row's reason has since been independently CONFIRMED with no drift by this split's
children 1–3 (see "Split-family consolidation" below).

| Bead | Status | Why it is NOT the re-run owner |
|---|---|---|
| pdftract-a8f096bc | Open | ADD-regression-test bead (bf-5o22rf child 2 of 5) — delivers NEW capturing-subscriber tests as code+commit; different artifact class from a re-run, which produces execution evidence for the already-present suite with zero code changes. |
| pdftract-931e6649 | Open | ADD-regression-test bead (bf-3a1p3b split child 1 of 4) — same artifact class: one new test + optional dev-dependency; runs the suite only to verify its own new test, never to land bf-5o22rf criterion-(a) runtime evidence. |
| pdftract-adda71a4 | Open | Terminal umbrella consolidation (child 5 of 5) — writes notes/bf-5o22rf.md from other children's evidence; consumes the re-run verdict, executes nothing itself. |
| bf-5o22rf | Open | Parent umbrella holding criteria (a)/(b) — stays open until its children close; all execution is delegated to children, so it owns no runnable work. |
| bf-3a1p3b | Open | Sibling parent umbrella (own 4-child split) — its note forbids re-touching the mmap.rs logging; its children cover http_range/docs/consolidation, not the mmap suite. |
| pdftract-de16d7aa | Open | Chain anchor / verification audit (child 1 of 5) — already recorded its PASS-on-analysis verdict and the WARN that spawned pdftract-b716bac5; the re-run exists precisely because that WARN was out of its scope. |

## Duplicate sweep

**Dated 2026-09-08T11:41Z — result: NONE FOUND.** Full `bead list --json` sweep of all
2,697 beads; 16 created after the 2026-09-08T05:15Z ownership-check cutoff, none a second
owner of the re-run:

- **12× classify.rs compile cluster** (c1f3cf4b, fe9aa999, 2c5787e0, d53efad3, ba4e7150,
  15511c0b, 180898ae, 52170e18, d077ca93, 903ece20, 2f8f6402, 54df39bb): all run/fix/tabulate
  the gated classify.rs compile check in TEST code. Zero of the 16 titles or descriptions
  mention mmap/madvise/prefetch/bf-5o22rf/b716bac5 (regex-checked). They measure and clear
  the gate that BLOCKS the re-run and produce no executed-test output.
- **4× pdftract-c08a9ede coordination children** (adb806f2 — closed, cd0c29e6 — this bead,
  eb967150, c02b4878): record ownership conclusions only; execute nothing by design.

**Re-pass 2026-09-08T12:18Z — result: NONE FOUND.** The parent bead's second coordination
pass (closed 12:19:16Z, then reopened by dispatch churn) re-enumerated post-cutoff beads
across open/in_progress/deferred/closed separately (plain `bead list` is priority-ordered
and truncates): 24 beads created after the 05:15Z cutoff out of 2,705 total, still no
second owner. Nearest neighbors unchanged, plus the gate re-split of the owner's own
chain (pdftract-68091d06 / 003ddcd1 / 3c39e12c — children of the gate fa233a5e, i.e. the
owner's own precondition sub-work, not competitors).

**Child-4 sweep re-runs, through re-run 5 dated 2026-09-08T22:51Z (commit `a0dc9569`) —
result: NONE FOUND every time.** Split child 4 (pdftract-1ce1beaf) extended coverage to
every bead created after 12:18Z: 48 new + 24 prior, each classified NOT-OWNER into three
classes — (A) cd0c29e6's own split family, 9 coordination-only beads including this
consolidation; (B) 28 read-only git/bead-state forensics on the notes README and commit
`0364a3a8`; (C) 11 owner-chain handoff publication and hygiene beads. Zero post-05:15Z
bead depends on the owner chain; zero executes or re-scopes the re-run. Re-run 5
re-enumerated the live store at 22:51Z — 2,753 beads total, all unique IDs, **72 created
after 05:15Z (identical to re-runs 2–4, all 72 already classified)**, newest bead
store-wide still pdftract-f141d045 at 19:44:55Z (store static since), and zero dependency
edges into any owner-chain bead. Full per-bead classification:
`notes/pdftract-1ce1beaf.md`.

## Split-family consolidation (child 5 of 5, pdftract-3128538a)

pdftract-cd0c29e6 was auto-split into five children (unique-ref `split:cd0c29e6-childN`);
children 1–4 each verified one slice of the non-owner record above against the live
store, and this section merges their verdicts. Per-child results:

| Child | Bead | Verdict | Evidence |
|---|---|---|---|
| 1 | pdftract-cae26b95 (closed 13:45Z) | **CONFIRMED, no drift** — the two ADD-regression-test beads a8f096bc and 931e6649 are a different artifact class (new tests as code+commit); neither lands criterion-(a) runtime evidence | `notes/pdftract-cae26b95.md`, commit `9c9db3bf` |
| 2 | pdftract-1f1cf7a5 (Open at rev 18, 22:46Z — dispatch/quarantine churn continues; declined the dispatch-7 auto-split re-issue at rev 17 and re-closed on committed evidence, reopened again; failure-count 6 is double-claim churn per child 4 re-run 5; work itself complete) | **CONFIRMED, no drift** — adda71a4 is umbrella NOTE consolidation only; its final suite check is a confirmation step feeding a hygiene criterion, not the owned re-run | `notes/pdftract-1f1cf7a5.md` (committed); its own split children 62d653ec, 3df8a46a, 686bdc40, 0078d4a6 all closed |
| 3 | pdftract-d2fd5467 (closed — PASS close 14:38Z reopened 78s later by churn, re-closed on committed evidence 20:29Z) | **CONFIRMED, no drift** — the three anchor beads bf-5o22rf (parent acceptance bead the re-run serves), bf-3a1p3b (sibling umbrella whose notes end "do not re-implement the mmap.rs logging"), de16d7aa (audit-only chain anchor whose WARN rider spawned b716bac5) | `notes/pdftract-d2fd5467.md`, commit `576705dd` |
| 4 | pdftract-1ce1beaf (Open — dispatcher churn after evidence-bearing closes; five dated sweeps committed) | **NONE FOUND** — dated duplicate-owner sweeps re-run 1–5 (latest 22:51Z, `a0dc9569`), zero second owners across all post-cutoff creation | `notes/pdftract-1ce1beaf.md` |
| 5 | pdftract-3128538a (this bead) | Merges 1–4 into the consolidated conclusion at the top of this file; re-affirmed live at 23:04Z on re-dispatch | this note |

One imprecision child 1 flagged in the committed version of this note is resolved here:
the note previously documented only the 11:41Z pass, while the 12:18Z pass lived in the
parent bead's Notes. Both are now recorded above; substance identical, conclusion
unaffected.

## Owner chain state

At the 11:41Z check:

- pdftract-b716bac5 — **Open**, unassigned, unchanged since 05:07 UTC. Sole owner.
- pdftract-fa233a5e (child 1, compile go/no-go gate) — **InProgress**, claimed by
  glm-tgp 11:36 UTC. Never executes the mmap suite.
- pdftract-030e8414 (child 2, the ONLY bead that executes the mmap suite) — Open,
  serialized behind child 1.
- pdftract-ec0e6526 (child 3, verdict into notes/bf-5o22rf-child1.md) — Open, blocked.
- pdftract-38700c39 (child 4, cross-link + push) — Open, blocked.

Re-checked at consolidation time 2026-09-08T23:04Z (re-affirmation pass; identical to the
22:29Z consolidation): b716bac5 still **Open** (rev 13, unchanged since 05:07Z — no
runtime evidence landed, no commit to cite); fa233a5e now **Open** again (rev 11,
released 11:49Z, its gate re-split 3e8309f4/68091d06/003ddcd1/3c39e12c carries the
compile work); 030e8414, ec0e6526 and 38700c39 all still **Open** (rev 1 each, unchanged
since 04:59Z). The serial order and single ownership are intact.

`notes/b716bac5-child2.md` is absent (re-confirmed 23:04Z); `notes/bf-5o22rf-child1.md`
still ends on the "append a runtime-evidence section here" handoff (line 128, last commit
`b19a9e52`) — both confirm the evidence has not landed and no worker other than the owner
chain has produced it.
