# pdftract-cae26b95 — non-owner reasons for the two ADD-regression-test beads: verification

**Bead:** pdftract-cae26b95 (split child 1 of 5 of pdftract-cd0c29e6; coordination-only,
read-only). **Guard honored:** no mmap-suite run, no cargo test of any kind, no production
code change, no bead filed, no other bead's status touched. Method: `bead show` of both
sibling beads only, compared against the two recorded evidence locations — parent
pdftract-cd0c29e6 Notes (coordination check 2026-09-08T12:18Z) and
`notes/pdftract-cd0c29e6.md` (commit 3dcfb6be, check 2026-09-08T11:41Z).

## Conclusion

**Both recorded non-owner reasons are CONFIRMED — no substantive drift.** Neither bead has
moved since the reasons were recorded: pdftract-a8f096bc is rev 1, untouched since creation
2026-09-07T08:53Z; pdftract-931e6649 is rev 3, last updated 2026-09-07T12:01Z — both
pre-date the 11:41Z and 12:18Z passes, and both beads are still Open as recorded.

## Confirmed one-liners (deliverable)

- **pdftract-a8f096bc** — ADD-regression-test bead (bf-5o22rf child 2 of 5, Open): its
  deliverable is NEW capturing-subscriber tests asserting the prefetch trace event as
  code+commit — a different artifact class from re-running the existing mmap suite, so it
  produces no bf-5o22rf criterion-(a) runtime evidence itself.
- **pdftract-931e6649** — ADD-regression-test bead (bf-3a1p3b split child 1 of 4, Open):
  same artifact class — one new test plus an optional dev-dependency; its only cargo run
  is a side-effect check on its own new test, never the owned criterion-(a) evidence
  re-run, which belongs solely to pdftract-b716bac5 → pdftract-030e8414.

## Cross-check detail

| Bead | Recorded reason (12:18Z Notes / 11:41Z note file) | Own scope (`bead show`) | Verdict |
|---|---|---|---|
| pdftract-a8f096bc | new test authoring, different artifact class from a re-run, produces no criterion-(a) runtime evidence itself | "Add regression tests asserting the MmapSource prefetch failure trace event"; bf-5o22rf child 2 of 5; capture-and-assert tests over the existing `tracing::trace!` block; the timeout-wrapped run is its own acceptance criterion | CONFIRMED |
| pdftract-931e6649 | same ADD-a-regression-test class; only cargo run is a side-effect check on its own new tests, not the owned evidence re-run | "Add a regression test for the prefetch madvise-failure trace event"; bf-3a1p3b split child 1 of 4; one new test + tracing-subscriber dev-dependency; timeout-wrapped `cargo test -p pdftract-core source::mmap` green is its own acceptance criterion | CONFIRMED |

### Minor imprecisions noted (not substantive drift)

1. **Timestamps:** the task cites the "2026-09-08T12:18Z recorded reasons" together with
   commit 3dcfb6be, but the committed note file records an 11:41Z check — the 12:18Z pass
   lives only in the parent bead's Notes. Substance is identical across both passes.
2. **"Only its own new tests":** both beads' acceptance criteria run
   `cargo test -p pdftract-core (…--lib) source::mmap` — the whole module, not literally
   only their own new tests. The recorded phrase describes the run's *purpose*
   (self-verification of the bead's own change), not the command's coverage; the ownership
   conclusion is unaffected — the single owner of the criterion-(a) re-run remains
   pdftract-b716bac5's chain (child 2, pdftract-030e8414), and neither ADD bead competes
   with it.
