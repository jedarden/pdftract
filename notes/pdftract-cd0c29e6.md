# pdftract-cd0c29e6 — mmap-re-run sibling beads: duplicate-ownership check

**Bead:** pdftract-cd0c29e6 (split child 2 of 4 of umbrella pdftract-c08a9ede; blocked by
child 1, the owner-status check pdftract-adb806f2, which closed).
**Type:** read-only coordination check. **Guard honored:** no mmap-suite run, no cargo
test of any kind, no production code change, no bead filed, no other bead's status touched.
**Check performed:** 2026-09-08T11:41Z (re-verification of the 11:09 UTC pass recorded in
the bead notes; all conclusions re-confirmed unchanged against the live store at rev HEAD
`ddc65fb4`).

## Conclusion

**pdftract-b716bac5 and its serial 4-child chain remain the SINGLE owner of the
mmap-suite runtime re-run. No duplicate or competing owner exists.**

Runtime evidence has NOT yet landed — no commit to point at. The latest gate state is
child 1's NO-GO capture (`notes/b716bac5-child1.md`, 09:20 UTC: `cargo test -p
pdftract-core --lib --no-run` exit 101, 89 errors in 4 sibling TEST modules, zero in
`source/mmap.rs`); child 1 pdftract-fa233a5e is InProgress right now (claimed by
`glm-tgp`, updated 11:36 UTC) re-working the gate. Per the task guard, no duplicate
re-run bead was filed.

## Non-owner reasons (one line each)

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

## Owner chain state at check time

- pdftract-b716bac5 — **Open**, unassigned, unchanged since 05:07 UTC. Sole owner.
- pdftract-fa233a5e (child 1, compile go/no-go gate) — **InProgress**, claimed by
  glm-tgp 11:36 UTC. Never executes the mmap suite.
- pdftract-030e8414 (child 2, the ONLY bead that executes the mmap suite) — Open,
  serialized behind child 1.
- pdftract-ec0e6526 (child 3, verdict into notes/bf-5o22rf-child1.md) — Open, blocked.
- pdftract-38700c39 (child 4, cross-link + push) — Open, blocked.

`notes/b716bac5-child2.md` is absent; `notes/bf-5o22rf-child1.md` still ends on the
"append a runtime-evidence section here" handoff — both confirm the evidence has not
landed and no worker other than the owner chain has produced it.
