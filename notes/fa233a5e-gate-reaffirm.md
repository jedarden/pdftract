# pdftract-fa233a5e — compile-gate re-derivation at HEAD, split-already-exists record

**Bead:** pdftract-fa233a5e ("Check pdftract-core lib test target compiles and record a
go/no-go compile artifact") — auto-split child 1 of 4 of umbrella pdftract-b716bac5.
**Canonical verdict note:** `notes/b716bac5-child1.md` (pdftract-003ddcd1, commit `10df3676`).
**Raw gate log:** `notes/b716bac5-child1-gate-raw.log` (pdftract-68091d06, commit `336bf8ee`).
**Pre-gate baseline:** `notes/b716bac5-child1-baseline.md` (pdftract-3e8309f4, commit `e9036d20`).
**Handoff:** `notes/b716bac5-child1-handoff.md` (pdftract-3c39e12c).

## Why this note exists

Two things this bead's re-dispatch surfaced, recorded here so the next dispatch does not
re-derive either one:

1. **The split this bead was dispatched to perform already exists.** A prior auto-split
   converted pdftract-fa233a5e into an umbrella with exactly the requested end state:
   four sequential children (3e8309f4 → 68091d06 → 003ddcd1 → 3c39e12c, all carrying
   `split-child`), the parent dependent on the last child
   (`bead show pdftract-fa233a5e` → `dependencies: [{blocker: pdftract-3c39e12c, kind:
   blocks}]`), and the `umbrella` label already on the parent. Children 1–3 are `closed`;
   child 4's artifact (the handoff note) is committed at the pushed tip even though the
   dispatcher has since reopened the bead (`failure-count:6`, quarantined). Creating a
   second, duplicate child chain would manufacture overlapping work units on a shared
   checkout; that was declined, not forgotten.

2. **The published verdict needed re-derivation at HEAD.** The canonical NO-GO was
   captured 2026-09-08T12:28:32Z; ~11 hours of sibling drift and 49 dirty `crates/` paths
   later, the verdict had not been re-checked by running the gate. This bead ran it —
   exactly once, no retries.

## Fresh gate run — the one invocation this bead performed

| Field | Value |
|---|---|
| Command | `set -o pipefail; timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run 2>&1 \| tee /tmp/fa233a5e-gate-fresh.log \| tail -25` |
| UTC start | 2026-09-08T23:34:45Z |
| UTC end | 2026-09-08T23:34:55Z (~10 s wall — incremental build, deps cached by sibling runs) |
| Exit code | **101** (`${PIPESTATUS[0]}` of the timeout-wrapped cargo; no timeout kill, so 124/137 does not apply) |
| rustc summary | ``error: could not compile `pdftract-core` (lib test) due to 89 previous errors; 129 warnings emitted`` |
| Invocations | exactly **one**; no overlapping retries |
| Local run justification | working tree dirty (49 paths under `crates/`), so the cargo wrapper's iad-ci submission path does not engage — the check ran locally under cgroup limits against *this* tree, which is the tree the gate must judge |
| No source changes | nothing under `crates/` modified by this bead — cargo writes only to `target/`; `git status --porcelain crates/` before staging contains exactly the 49 pre-existing sibling paths |

## VERDICT: NO-GO — stands at HEAD bd12b867

The `pdftract-core` lib test target still does **not** compile. Children 2–4 of the
umbrella must **not** run `cargo test` against this tree. The directive in
`notes/b716bac5-child1.md` is re-affirmed as of this run, not merely re-quoted: the error
set is **byte-identical** to the 12:28Z capture — same 89 errors, same four files, same
code distribution — and the run happened at a later tip (`bd12b867` vs `0364a3a8`) whose
intermediate commits touched only `notes/`.

## Per-file error inventory — 89 errors, 4 files (identical to published verdict)

```
crates/pdftract-core/src/classify.rs             -> E0609 x52, E0277 x2      -> 54
crates/pdftract-core/src/font/type3_rasterizer.rs -> E0061 x24, E0599 x5     -> 29
crates/pdftract-core/src/render/scanline.rs      -> E0277 x2, E0308 x2, E0599 x1 -> 5
crates/pdftract-core/src/content_stream.rs       -> E0061 x1                 -> 1
------------------------------------------------------------------------------
TOTAL                                                                        -> 89
```

Code distribution across all 89: `E0609 x52, E0061 x25, E0599 x6, E0277 x4, E0308 x2`.

## `crates/pdftract-core/src/source/mmap.rs` — explicitly checked: **zero errors** (expected: none ✓)

The fresh log contains **zero mentions** of `source/mmap.rs` — no error, not even a
warning. The mmap observability work this chain exists to verify is not the blocker.

## Git state at gate time (HEAD vs origin/main)

- HEAD `bd12b867fecc7420f9c4c5e1f4be7d64a25437e7` == `origin/main` `bd12b867…`
  (Forgejo, re-fetched immediately before the run), divergence **0 / 0** — the gate was
  judged exactly at the pushed tip of `main`.
- Working tree: 49 dirty paths under `crates/` (sibling in-flight edits, unchanged in
  character since the pre-gate baseline).

Dirty-vs-clean split of the four error files at this tree — unchanged from the baseline:

| File | Working tree vs HEAD | Consequence |
|---|---|---|
| `classify.rs` | **clean** | its 54 errors are **committed** at the pushed tip |
| `render/scanline.rs` | **clean** | its 5 errors are likewise **committed** at the pushed tip |
| `font/type3_rasterizer.rs` | modified (uncommitted sibling edits) | its 29 errors sit in a locally-edited state |
| `content_stream.rs` | modified (uncommitted sibling edits) | its 1 error sits in a locally-edited state |
| `source/mmap.rs` | modified (uncommitted sibling edits, **no errors**) | mmap work under test is **not implicated** |

**Still true for the umbrella's children 2–4:** 59 of 89 errors (`classify.rs` +
`render/scanline.rs`) are broken at the committed, pushed tip of `main`, so the gate will
not clear via sibling commits alone; those two `#[cfg(test)]` modules must catch up to the
changed production signatures (`Result`-returning `classify`, integer `edge.x`/`slope()`,
2-arg `detect_char_proc_type`, 7-arg `execute_with_do`, no `CharProcType::Unknown`) in a
commit.

## Acceptance criteria of pdftract-fa233a5e

- **AC1** — `notes/b716bac5-child1.md` exists with the exact command, UTC timestamp, exit
  code, and an explicit NO-GO verdict including a per-file error inventory: **PASS**
  (committed `10df3676`, published — ancestor of `origin/main`), and re-affirmed by the
  fresh run recorded here.
- **AC2** — no file under `crates/` modified by this bead: **PASS** (verified via
  `git status --porcelain crates/` immediately before staging — only the note below is
  this bead's).
- **AC3** — commit citing pdftract-fa233a5e, pushed to Forgejo `main`: **PASS** (this
  commit is the first in the chain that cites the parent bead ID itself; the child beads
  cited their own IDs).

## Compliance

- One gated, timeout-wrapped invocation; no bare `cargo test`; no retries.
- No file under `crates/` touched; nothing repaired while inventorying.
- No new child beads created — the pre-existing split is certified, not duplicated.
- Orphan check at close: no `cargo`/`rustc`/`pdftract` process spawned by this bead remains.
