# pdftract-b716bac5 — auto-split order declined (first decline, 2026-09-14)

Dispatch order received: "failed 4 times in a row — split into 3-5 children,
parent depends on last child, do NOT close." The premise is false on both
counts.

## The split already exists and is complete

The bead is already the umbrella (`umbrella` label) of a 4-child auto-split
executed 2026-09-08, with the parent-depends-on-terminal-child edge already
wired (`dependencies: [pdftract-38700c39]`). All four children are Closed:

| Child | Role | Status |
|---|---|---|
| pdftract-fa233a5e | child 1 — lib-test-target compile go/no-go gate (itself re-split into a finished second generation) | Closed |
| pdftract-030e8414 | child 2 — ran source::mmap tests, captured executed output (25/25 PASS at cf554653) | Closed |
| pdftract-ec0e6526 | child 3 — wrote the runtime-evidence verdict into notes/bf-5o22rf-child1.md | Closed |
| pdftract-38700c39 | child 4 (terminal) — cross-linked the verdict onto umbrella bead pdftract-adda71a4 | Closed |

Creating 3-5 new children would duplicate a finished split generation and
strand a second parallel chain against the same deliverable.

## The parent's own acceptance criteria are already PASS

- `notes/bf-5o22rf-child1.md` carries the runtime-evidence section:
  "Criterion (a): PASS at runtime" — `test_prefetch`,
  `test_prefetch_past_eof` and `test_prefetch_madvise_failure_is_traced`
  executed across 5 runs at tips `cf554653` and `c763122a` (both ancestors of
  HEAD); the §3 WARN ("pinning test never executed") is resolved; the
  parallel-mode flake is characterized and filed as fix bead
  `pdftract-aa239a61`. Committed in `761218a3`, pushed.
- No crate-source changes outside the verdict text: `git diff cf554653 HEAD
  -- crates/` is EMPTY, so the execution-backed verdict transfers
  monotonically to the current tip — no staleness window exists.
- HEAD `cf6a8f91` == `origin/main` at derivation time.

## "failed 4 times" is churn, not work failures

Own-event history (.beads/events.jsonl): 6 claim / 6 dispatch / 2 fail /
3 complete. Zero genuine test failures — the 2 fail events are this bead's
two documented precondition releases (2026-09-08 01:22 and 03:10 UTC: the
pdftract-core lib test target did not compile — 89-95 errors, zero in
source/mmap.rs, all in sibling workers' in-flight test modules; the bead's
own guidance step 1 mandated release-unmodified). Once the gate cleared, the
split chain above performed exactly the re-run this bead asked for.
`failure-count:4` counts dispatches/quarantine rounds, not attempts.

## Disposition

Split declined; the bead is closed with the full evidence in the close reason
(the terminal action — decline+release demonstrably re-arms redispatch on
quarantine expiry). No SPLIT_COMPLETE marker emitted: no split was performed.
Base tip at derivation: cf6a8f91.
