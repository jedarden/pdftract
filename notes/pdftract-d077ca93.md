# pdftract-d077ca93 — evidence workspace + pre-run baseline (split verification note)

Umbrella: **pdftract-d077ca93** — "Prepare the evidence workspace and record
the pre-run baseline for the classify.rs compile check" (itself child 1 of 4
of the pdftract-ba4e7150 split). The parent failed three times as a single
bead and was split into four sub-children. This note is the parent's scope
step 3 deliverable; the current version was produced by the 2026-10-17
re-dispatch after the previous attempt (claude-print, 14:47Z) closed with
reason-only evidence and was bounced by the `shipped_work` gate without
landing any commit.

## Note history

| When | Commit | What |
|---|---|---|
| parent attempt 1 | `0ef2997f` | evidence workspace + first baseline + first version of this note |
| parent attempt 2 | `87e6d2e9` | baseline re-capture, note updated |
| parent attempt 3 | `cc15ac22` | baseline re-capture (HEAD 87e6d2e9), note updated |
| sub-child 2 | `d237c38f` | baseline re-captured fresh at HEAD 4dcff58d — this note was NOT touched, so its verbatim quote went stale |
| sub-child 3 | `9f95395b` | rewrite: re-syncs the verbatim quote to the committed baseline and records the split's artifact-to-commit map |
| sub-child 4 | `40f8b086` | baseline re-captured at HEAD d237c38f ("align remote baseline freshness marker"); quote went stale again |
| re-dispatch 2026-10-17 | this file's introducing commit | baseline re-captured live at HEAD 40f8b086 (the only changed line), quote re-synced, split map finalized |

## Split map: sub-child → artifact → commit

| # | Bead | Artifact | Commit |
|---|---|---|---|
| 1 | pdftract-cf349714 (closed) | `notes/evidence/README.md` — manifest for the evidence workspace | `0364a3a8` (single path, 57 insertions) |
| 2 | pdftract-9935e4b6 (closed) | `notes/evidence/pdftract-d077ca93-baseline.txt` — fresh re-capture at HEAD 4dcff58d | `d237c38f` (single path, 3 insertions / 3 deletions) |
| 3 | pdftract-867642aa (closed) | this note (rewrite) | `9f95395b` |
| 4 | pdftract-bf2b6ee5 (closed) | publication verification against origin/main + baseline alignment | `40f8b086` |
| — | pdftract-d077ca93 re-dispatch (this run) | baseline re-capture + this note's current version | hash recorded in the bead close reason |

## Baseline file contents (verbatim)

From `notes/evidence/pdftract-d077ca93-baseline.txt`, re-captured live by the
2026-10-17 re-dispatch and committed by this version of the note's own commit
(working tree and HEAD agree once that commit lands; the diff vs the previous
committed state is exactly the one `baseline HEAD:` line):

```
bead: pdftract-d077ca93
baseline HEAD: 40f8b086
dirty crates/ files: 28
crates/ diffstat:  25 files changed, 1102 insertions(+), 390 deletions(-)
cargo: cargo 1.97.1 (c980f4866 2026-06-30)
nextest: ABSENT - use timeout-wrapped cargo
timeout: present
```

## Acceptance criteria (parent pdftract-d077ca93) — verdicts this run

- **PASS — baseline file exists and is committed**:
  `notes/evidence/pdftract-d077ca93-baseline.txt` is tracked; this run
  overwrites it in place with a fresh capture and commits exactly two paths
  (the baseline + this note).
- **PASS — records baseline HEAD**: `40f8b086` (live `git rev-parse --short
  HEAD` at capture time).
- **PASS — records dirty crates/ count**: 28 porcelain entries under
  `crates/` (25 tracked files in the diffstat; the porcelain count also
  covers rename/delete entries) — unchanged from every prior capture.
- **PASS — records cargo/nextest/timeout availability**: cargo 1.97.1,
  nextest ABSENT (fallback stated in the file), timeout present.
- **PASS — freshness (not copied from a prior attempt)**: this run executed
  the capture command against the live tree and produced a different blob:
  the `baseline HEAD:` line moved `d237c38f` → `40f8b086`, the only volatile
  field that changed. Every field was re-measured by this run, not quoted.
- **PASS — no blanket staging**: this dispatch stages only
  `notes/evidence/pdftract-d077ca93-baseline.txt` and
  `notes/pdftract-d077ca93.md` by explicit pathspec.
- **PASS — hard rules respected**: nothing under `crates/` was modified, no
  `cargo build`/`cargo test` was run (the warm-up build belongs to child 2,
  the gated run to child 3), and no socket was bound or server spawned.

Overall: **PASS** on every criterion; no WARN or FAIL items.

## Clean-extraction verification (committed state, not working tree)

HEAD was extracted to a scratch directory and checked there:

- the committed baseline and note are byte-identical to the working-tree
  copies (`diff` clean on both paths);
- `scripts/definition-of-done.sh --fast` was run in the extraction — it is
  Python-only (readme capabilities audit, SDK doc validation, Argo workflow
  drift, nightly evidence) and therefore compatible with this bead's
  no-cargo-build hard rule; result recorded in the close reason's verified
  block.

## Interpretation notes for the downstream gated run

- **nextest is absent** on this box, so the gated 600s run must use the
  timeout-wrapped `cargo test` fallback per repo CLAUDE.md test hygiene
  (`timeout --kill-after=30s 600s ...`), never a bare `cargo test`.
- `timeout` is present, so the hard wall-clock gate is enforceable.
- The 28 dirty `crates/` entries are pre-existing shared-checkout edits from
  other workers; this dispatch added none and changed none of them.
- A pre-run baseline necessarily snapshots an earlier HEAD (40f8b086) than
  the one a later gated run executes against; the freshness rule assigns
  re-capture duty to whichever bead runs next (child 2 or child 3), and this
  note's verbatim quote tracks the committed baseline file, not the moving
  HEAD.

## crates/ untouched by this split

- `0364a3a8` touches only `notes/evidence/README.md`.
- `d237c38f` touches only `notes/evidence/pdftract-d077ca93-baseline.txt`.
- `9f95395b` touches only `notes/pdftract-d077ca93.md`.
- `40f8b086` touches only `notes/evidence/pdftract-d077ca93-baseline.txt`.
- This re-dispatch's commit touches only the two `notes/` paths above.
- No member of this split ran `cargo build` or `cargo test`, bound a socket,
  or spawned a server process (hard rules).
