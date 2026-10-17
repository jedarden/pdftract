# pdftract-d077ca93 — evidence workspace + pre-run baseline (split verification note)

Umbrella: **pdftract-d077ca93** — "Prepare the evidence workspace and record
the pre-run baseline for the classify.rs compile check" (itself child 1 of 4
of the pdftract-ba4e7150 split). The parent failed three times as a single
bead and was split into four sub-children. This note is the parent's scope
step 3 deliverable, rewritten by sub-child 3 (**pdftract-867642aa**) after
the artifact sub-children landed their commits.

## Note history

| When | Commit | What |
|---|---|---|
| parent attempt 1 | `0ef2997f` | evidence workspace + first baseline + first version of this note |
| parent attempt 2 | `87e6d2e9` | baseline re-capture, note updated |
| parent attempt 3 | `cc15ac22` | baseline re-capture (HEAD 87e6d2e9), note updated |
| sub-child 2 | `d237c38f` | baseline re-captured fresh at HEAD 4dcff58d — this note was NOT touched, so its verbatim quote went stale |
| sub-child 3 (this) | this file's introducing commit | rewrite: re-syncs the verbatim quote to the committed baseline and records the split's artifact-to-commit map |

## Split map: sub-child → artifact → commit

| # | Bead | Artifact | Commit |
|---|---|---|---|
| 1 | pdftract-cf349714 (closed) | `notes/evidence/README.md` — manifest for the evidence workspace | `0364a3a8` (single path, 57 insertions) |
| 2 | pdftract-9935e4b6 (closed) | `notes/evidence/pdftract-d077ca93-baseline.txt` — fresh re-capture at HEAD 4dcff58d | `d237c38f` (single path, 3 insertions / 3 deletions) |
| 3 | pdftract-867642aa (this bead) | this note | the commit introducing this version (hash recorded in the bead close reason; subject `docs(pdftract-867642aa): verification note for evidence baseline`) |
| 4 | pdftract-bf2b6ee5 (open) | publication verification against origin/main | pending — runs after this note lands |

## Baseline file contents (verbatim)

From `notes/evidence/pdftract-d077ca93-baseline.txt`, as committed by
`d237c38f` and unchanged through `618542bc` (HEAD immediately before this
note's commit; `git diff HEAD -- notes/evidence/` is empty, so the
working-tree copy is byte-identical to the committed one):

```
bead: pdftract-d077ca93
baseline HEAD: 4dcff58d
dirty crates/ files: 28
crates/ diffstat:  25 files changed, 1102 insertions(+), 390 deletions(-)
cargo: cargo 1.97.1 (c980f4866 2026-06-30)
nextest: ABSENT - use timeout-wrapped cargo
timeout: present
```

## Acceptance criteria (parent pdftract-d077ca93) — verdicts

- **PASS — baseline file exists and is committed**:
  `notes/evidence/pdftract-d077ca93-baseline.txt` is tracked at HEAD,
  introduced by `0ef2997f`, last rewritten by `d237c38f`.
- **PASS — records baseline HEAD**: `4dcff58d`.
- **PASS — records dirty crates/ count**: 28 porcelain entries under
  `crates/` (25 tracked files in the diffstat; the porcelain count also
  covers rename/delete entries).
- **PASS — records cargo/nextest/timeout availability**: cargo 1.97.1,
  nextest ABSENT (with the timeout-wrapped-cargo fallback stated in the
  file), timeout present.
- **PASS — freshness (not copied from a prior attempt)**: `d237c38f`'s diff
  re-measured every volatile field — HEAD 87e6d2e9 → 4dcff58d, dirty files
  49 → 28, diffstat 44 files/2146+/535− → 25 files/1102+/390− — so the
  content demonstrably came from that run, satisfying the workspace
  freshness rule in `notes/evidence/README.md`.
- **PASS — no blanket staging**: `0364a3a8` and `d237c38f` each touch
  exactly one path under `notes/`; this note's commit stages only
  `notes/pdftract-d077ca93.md`.

Overall: **PASS** on every parent criterion; no WARN or FAIL items.

Observation (informational, not a criterion): a pre-run baseline necessarily
snapshots an earlier HEAD (4dcff58d) than the one a later gated run executes
against; the freshness rule assigns re-capture duty to whichever bead runs
next, and this note's verbatim quote tracks the committed baseline file, not
the moving HEAD.

## Interpretation notes for the downstream gated run

- **nextest is absent** on this box, so the gated 600s run must use the
  timeout-wrapped `cargo test` fallback per repo CLAUDE.md test hygiene
  (`timeout --kill-after=30s 600s ...`), never a bare `cargo test`.
- `timeout` is present, so the hard wall-clock gate is enforceable.
- The 28 dirty `crates/` entries are pre-existing shared-checkout edits from
  other workers; the split that produced this note added none and changed
  none of them.

## crates/ untouched by this split

- `0364a3a8` touches only `notes/evidence/README.md`.
- `d237c38f` touches only `notes/evidence/pdftract-d077ca93-baseline.txt`.
- This note's commit touches only `notes/pdftract-d077ca93.md`.
- No sub-child of this split ran `cargo build` or `cargo test`, bound a
  socket, or spawned a server process (hard rules).
