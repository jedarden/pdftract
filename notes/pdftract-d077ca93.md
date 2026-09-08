# pdftract-d077ca93 — evidence workspace + pre-run baseline

Child 1 of 4 of the split of pdftract-ba4e7150. This bead only prepares the
evidence workspace and records the tree state that the later gated compile run
(child 3) will be interpreted against. Nothing under `crates/` was touched, no
cargo build was run, no socket bound, no server spawned.

## What was done

1. `mkdir -p notes/evidence`
2. Produced `notes/evidence/pdftract-d077ca93-baseline.txt` in this run via the
   bead-specified command block (output redirected to the file, not piped to the
   terminal). Content is fresh from this run, not copied from any prior attempt.
3. This note.

## Baseline file contents (verbatim)

From `notes/evidence/pdftract-d077ca93-baseline.txt`:

```
bead: pdftract-d077ca93
baseline HEAD: b2849bca
dirty crates/ files: 49
crates/ diffstat:  44 files changed, 2146 insertions(+), 535 deletions(-)
cargo: cargo 1.97.1 (c980f4866 2026-06-30)
nextest: ABSENT - use timeout-wrapped cargo
timeout: present
```

## Interpretation notes for downstream children

- **HEAD b2849bca is the commit the gated compile evidence must be read
  against.** 49 porcelain entries under `crates/` (44 tracked files in the
  diffstat — the porcelain count also covers deleted/untracked entries) are
  pre-existing worker edits in the shared checkout; this bead added none.
- **nextest is absent** on this box, so child 3's gated 600s run must use the
  timeout-wrapped `cargo test` fallback per repo CLAUDE.md test hygiene
  (`timeout --kill-after=30s 600s ...`), never a bare `cargo test`.
- `timeout` is present, so the hard wall-clock gate is enforceable.

## Acceptance criteria

- **PASS** — `notes/evidence/pdftract-d077ca93-baseline.txt` exists, is
  committed, and records baseline HEAD, dirty `crates/` count, and
  cargo/nextest/timeout availability. Baseline produced by this run; precise
  paths staged (no `git add -A`).
