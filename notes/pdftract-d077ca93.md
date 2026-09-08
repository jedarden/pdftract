# pdftract-d077ca93 — evidence workspace + pre-run baseline

Child 1 of 4 of the split of pdftract-ba4e7150. This bead only prepares the
evidence workspace and records the tree state that the later gated compile run
(child 3) will be interpreted against. Nothing under `crates/` was touched, no
cargo build was run, no socket bound, no server spawned.

## Provenance of this capture

Two prior attempts of this bead each committed a baseline but never closed the
bead (commits `0ef2997f` and `87e6d2e9`), leaving it InProgress. Per the
acceptance criteria ("baseline content copied from a prior attempt" is a
FAIL), this run **re-produced the baseline from scratch**: the bead-specified
command block was executed again, output redirected to the file. The stale
prior baseline recorded HEAD `0ef2997f`; the fresh capture below records the
current tree at HEAD `87e6d2e9` — a different HEAD, proving the content came
from this run rather than being reused. (Both commits between the captures
were notes-only, so the dirty-`crates/` count and diffstat are unchanged,
which cross-checks that no `crates/` churn occurred in between.)

## What was done (this run)

1. `mkdir -p notes/evidence` (already existed from prior attempts; re-run
   idempotently)
2. Produced `notes/evidence/pdftract-d077ca93-baseline.txt` fresh in this run
   via the bead-specified command block (output redirected to the file, not
   piped to the terminal)
3. This note

## Baseline file contents (verbatim)

From `notes/evidence/pdftract-d077ca93-baseline.txt`:

```
bead: pdftract-d077ca93
baseline HEAD: 87e6d2e9
dirty crates/ files: 49
crates/ diffstat:  44 files changed, 2146 insertions(+), 535 deletions(-)
cargo: cargo 1.97.1 (c980f4866 2026-06-30)
nextest: ABSENT - use timeout-wrapped cargo
timeout: present
```

## Interpretation notes for downstream children

- **HEAD 87e6d2e9 is the commit the gated compile evidence (child 3) must be
  read against.** 49 porcelain entries under `crates/` (44 tracked files in
  the diffstat — the porcelain count also covers deleted/untracked entries)
  are pre-existing worker edits in the shared checkout; this bead added none
  and changed none.
- **nextest is absent** on this box, so child 3's gated 600s run must use the
  timeout-wrapped `cargo test` fallback per repo CLAUDE.md test hygiene
  (`timeout --kill-after=30s 600s ...`), never a bare `cargo test`.
- `timeout` is present, so the hard wall-clock gate is enforceable. The
  warm-up build that de-risks the 600s gate is child 2's job, not this
  bead's.

## Acceptance criteria

- **PASS** — `notes/evidence/pdftract-d077ca93-baseline.txt` exists, is
  committed, and records baseline HEAD, dirty `crates/` count, and
  cargo/nextest/timeout availability. Baseline produced by this run (prior
  attempt's stale capture at `0ef2997f` replaced, not reused); precise paths
  staged (no `git add -A`).
