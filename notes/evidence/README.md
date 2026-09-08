# notes/evidence/ — run evidence artifacts

## Purpose

This directory holds evidence artifacts produced by gated compile-check and
measurement runs of this repo — for example a pre-run baseline captured
before a compile gate, or the raw output of a gated `cargo` invocation. Each
artifact is the durable record of one specific run, committed by the bead
that owned that run, so a bead's close reason can cite concrete, checkable
output instead of prose.

## Naming convention

    <bead-id>-<kind>.txt

- `<bead-id>` — the bead that produced and committed the capture, e.g.
  `pdftract-d077ca93`.
- `<kind>` — what the capture is, e.g. `baseline` (pre-run workspace state
  the run is interpreted against), `meta` (run summary: HEAD, exit code,
  wall clock, diffstat), `raw` (full command output).

Artifacts currently present (snapshot for orientation — the convention above
is the authoritative rule):

| File | Kind | Contents |
|---|---|---|
| `pdftract-ba4e7150-meta.txt` | `meta` | gated compile-check summary: HEAD, exit 101, wall clock, dirty `crates/` diffstat |
| `pdftract-ba4e7150-raw.txt` | `raw` | full output of that gated compile check |
| `pdftract-c1f3cf4b-raw.txt` | `raw` | full output of that bead's gated run |
| `pdftract-d077ca93-baseline.txt` | `baseline` | pre-run state: HEAD, dirty `crates/` files, diffstat, toolchain, nextest availability |

## Freshness rule

Every capture is produced fresh by the run that commits it. Never copy an
artifact forward from a prior attempt: if the same measurement is needed
again for a new run, re-capture it at the current HEAD and commit the new
file (or replace the old one in the same commit that documents the
re-capture). A stale capture is worse than none — it makes a close reason
cite evidence that no longer corresponds to any real run.

## Evidence beads are measurement-only

A bead whose deliverable is a capture in this workspace changes nothing
else:

- NO modifications under `crates/` (or any other source path).
- NO `cargo build` or `cargo test` invocations of its own.

The artifact records state observed at commit time; actually compiling
belongs to the gated-run bead the evidence describes. A commit serving an
evidence bead therefore touches only files inside this directory.

## Committing

Stage evidence files explicitly (`git add notes/evidence/<file>`). Blanket
staging (`git add -A`, `git add .`, `git commit -a`) is forbidden in this
repo — it sweeps in unrelated in-flight work from other beads.
