# pdftract-b716bac5 — child 1 chain HANDOFF: compile-gate verdict is published

**Bead:** pdftract-3c39e12c (grandchild 4 of 4 of pdftract-fa233a5e; umbrella pdftract-b716bac5).
This is the last child of the auto-split; it publishes the handoff statement so children 2–4 of
the umbrella never have to re-derive the verdict from scratch.

**Canonical verdict note:** `notes/b716bac5-child1.md` (bead pdftract-003ddcd1).
**Raw gate log:** `notes/b716bac5-child1-gate-raw.log` (bead pdftract-68091d06).
**Pre-gate baseline:** `notes/b716bac5-child1-baseline.md` (bead pdftract-3e8309f4).

## Verdict (quoted verbatim from `notes/b716bac5-child1.md`)

> ## VERDICT: NO-GO
>
> cargo exited **101**. The `pdftract-core` lib test target does **not** compile, so the
> umbrella's children 2–4 must **not** proceed with any test run until this gate clears.

**Gate exit code: 101** (rustc compile failure; `${PIPESTATUS[0]}` of the single gated
invocation `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run`, run
2026-09-08T12:28:32Z → 12:28:41Z at HEAD `0364a3a89eabf5a02715b7ef7daba764f42d23d7`).
Summary line, verbatim from the committed log (line 2220):
``error: could not compile `pdftract-core` (lib test) due to 89 previous errors; 129 warnings emitted``

## Commits this handoff certifies (children 1–3 of the split)

| Child | Bead | Artifact | Commit |
|---|---|---|---|
| 1 | pdftract-3e8309f4 | pre-gate git baseline (`notes/b716bac5-child1-baseline.md`) | `e9036d20` |
| 2 | pdftract-68091d06 | raw gate log, exit 101 (`notes/b716bac5-child1-gate-raw.log`) | `336bf8ee` |
| 3 | pdftract-003ddcd1 | verdict note (`notes/b716bac5-child1.md`) | `10df3676` |
| 4 | pdftract-3c39e12c | this handoff note | *(this commit)* |

Publication verified 2026-09-08 by `git fetch origin && git merge-base --is-ancestor`:
`e9036d20`, `336bf8ee`, and `10df3676` are **all ancestors of `origin/main`** (Forgejo,
`https://git.ardenone.com/jedarden/pdftract.git`), with local HEAD == `origin/main` ==
`9c9db3bf`. The verdict is durably published, not just local.

## HANDOFF STATEMENT: **NO-GO**

The umbrella's children 2–4 **may NOT proceed** with any `cargo test` run against this tree.
The `pdftract-core` lib test target does not compile (exit 101, 89 errors). Runtime evidence
for the umbrella (origin bf-5o22rf criterion (a)) is blocked until the gate clears.

### Files that must clear first (per-file error inventory, 89 errors / 4 files)

| File | Errors | Working tree vs HEAD | Clearance needed |
|---|---|---|---|
| `crates/pdftract-core/src/classify.rs` | 54 (E0609 x52, E0277 x2) | **clean** | a **commit to `main`** — errors are broken at the pushed tip |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 (E0061 x24, E0599 x5) | modified (sibling in-flight edits) | local edits + commit |
| `crates/pdftract-core/src/render/scanline.rs` | 5 (E0277 x2, E0308 x2, E0599 x1) | **clean** | a **commit to `main`** — errors are broken at the pushed tip |
| `crates/pdftract-core/src/content_stream.rs` | 1 (E0061 x1) | modified (sibling in-flight edits) | local edits + commit |

All 89 errors are inside `#[cfg(test)]` test modules — zero production breakage. **59 of 89
(`classify.rs` + `render/scanline.rs`) are broken at the committed, pushed tip of `main`**, so
the gate will not clear on its own via sibling commits: those two test modules must catch up to
the changed production signatures (`Result`-returning `classify`, integer `edge.x`/`slope()`,
2-arg `detect_char_proc_type`, 7-arg `execute_with_do`, no `CharProcType::Unknown`) in a commit.

Explicitly **not** a blocker: `crates/pdftract-core/src/source/mmap.rs` has **zero** errors and
zero mentions in the entire 2,220-line log — the mmap observability work this chain exists to
verify is clean; only the four sibling test modules above block the gate.

## Compliance (this bead)

- No cargo invocation of any kind was run; verification was read-only git (`git remote -v`,
  `git fetch origin`, `git log`/`git merge-base --is-ancestor`).
- No file under `crates/` was touched; the only file this bead writes is this note.
- Orphan check at close: `pgrep -af "cargo test|pdftract"` matched only this bead's own harness
  process and the pgrep itself — no `cargo test` run and no `pdftract` binary spawned by any
  bead in this chain is alive.
