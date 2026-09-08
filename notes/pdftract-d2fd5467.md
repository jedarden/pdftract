# pdftract-d2fd5467 — verify the recorded non-owner reasons for the three anchor beads

**Bead:** pdftract-d2fd5467 (split child 3 of 5 of umbrella pdftract-cd0c29e6;
coordination-only, read-only).
**Authority confirmed against:** the parent's coordination check **2026-09-08T12:18Z**
(recorded in `bead show pdftract-cd0c29e6` Notes) and its committed twin
`notes/pdftract-cd0c29e6.md` (commit `3dcfb6be`, which records the earlier 11:41Z pass).
**Method:** `bead show` of each anchor bead only — no test execution, no cargo of any
kind, no production code change, no bead filed, no other bead status touched. Guard honored.

## Verdict: all three one-liners CONFIRMED — no drift

| Anchor | Status (re-checked today) | 12:18Z one-liner | Result |
|---|---|---|---|
| bf-5o22rf | Open | "the PARENT acceptance bead — it owns criterion (a) itself, stays open until children close, and is what the re-run serves, not a runner of it" | **CONFIRMED** |
| bf-3a1p3b | Open | "sibling umbrella whose remaining children are http_range/doc/consolidation — its own notes rule out re-touching mmap.rs ('do not re-implement the mmap.rs logging')" | **CONFIRMED** (one phrasing nuance, below — conclusion unaffected) |
| pdftract-de16d7aa | Open | "chain anchor, audit-only child 1 ('do NOT re-implement') that produced the WARN handoff which created the re-run need" | **CONFIRMED** |

## Per-bead evidence

**bf-5o22rf — parent acceptance bead.** Title "Add Observability to Prefetch madvise
Errors" (`crates/pdftract-core/src/source/mmap.rs:125-128`). Its description carries the
ACCEPTANCE CRITERIA block itself ("a) Log madvise failures at trace level for debugging")
and no implementation or run instructions — it holds criterion (a), it does not execute
anything. `pdftract-de16d7aa`'s description names it: "Parent: bf-5o22rf … child 1 of 5
in the auto-split chain." Status Open, i.e. still holding until children close, exactly
as recorded. That the re-run serves this bead rather than competing with it is explicit
in `notes/bf-5o22rf-child1.md` §4: follow-up bead `pdftract-b716bac5` — "Re-run
source::mmap tests for runtime evidence of the prefetch madvise trace event
(**bf-5o22rf WARN handoff**)". Single-owner conclusion (b716bac5 chain) untouched.

**bf-3a1p3b — sibling umbrella.** Its Notes open "Split into 4 sequenced children
(umbrella) after 3 failed attempts" and close with the exact guard sentence the
one-liner quotes: "do not re-implement the mmap.rs logging." The mmap.rs logging
already landed under bf-4chy94 / commit `08ec8db8`, which is why the umbrella forbids
re-touching it. *Nuance, not drift:* the one-liner's shorthand "remaining children are
http_range/doc/consolidation" names three of the four children; all four are still Open
(pdftract-931e6649 regression test, pdftract-e1793e09 http_range, pdftract-259b0053
doc, pdftract-4b4a85ff consolidation). The omitted fourth child is not unaccounted for —
pdftract-931e6649 has its own dedicated non-owner line in the same 12:18Z record (and
its own row in the 11:41Z table in `notes/pdftract-cd0c29e6.md`). The load-bearing
claim — none of this umbrella's children is the mmap-suite runtime re-run owner, and
its notes forbid re-touching mmap.rs — holds.

**pdftract-de16d7aa — chain anchor, audit-only child 1.** Its description states
verbatim "child 1 of 5 in the auto-split chain. This bead is the chain anchor", scopes
the work as "Audit the existing implementation … Record the finding; do NOT
re-implement", and its acceptance criteria require "No production code changes in this
bead." The WARN handoff it produced is on disk: `notes/bf-5o22rf-child1.md` §3 verdict
"PASS — with a **WARN** rider on runtime test evidence" (the pinning test never
executed; `--lib` test target fails to compile on unrelated sibling edits), §4 routing
that WARN to follow-up bead pdftract-b716bac5 — the very re-run bead whose single
ownership the parent umbrella confirms. Authorship nuance: the note's header credits
the writing to pdftract-e6c4a13f, "child 3 of the split chain under pdftract-de16d7aa"
(built on child 1's audit pdftract-216e4ddc + child 2's test run pdftract-ff35ed32);
`notes/bf-5o22rf-child1.md` is de16d7aa's own designated deliverable per its guidance
step 4, so the WARN is correctly attributed to de16d7aa's chain. Status Open, matching
both records.

## Consistency between the two records

`notes/pdftract-cd0c29e6.md` (commit `3dcfb6be`) records the 11:41Z pass (2,697 beads
swept, 16 new since the 05:15Z ownership check); the parent bead Notes record the
12:18Z re-verification (2,705 beads, 24 new). The three anchor one-liners are the same
claims in both, reworded — the 11:41Z table rows and the 12:18Z Notes lines agree on
substance for all three anchors, and all three anchor statuses are unchanged (still
Open) as of this re-check. The duplicate-sweep itself is not re-run here: it belongs to
the parent umbrella and its other split children.
