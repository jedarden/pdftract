# pdftract-3c39e12c — consolidated split record (all 4 children)

**Bead:** pdftract-f141d045 (auto-split child **4 of 4** of pdftract-3c39e12c, umbrella
pdftract-b716bac5 — the LAST child of this split; closing it unblocks the parent).
**Scope:** consolidation only — this note restates what children 1–3 already established and
certifies where each conclusion lives. No cargo invocation, no source edits, no re-derivation
of any verdict; every claim below is quoted or pointed at, never re-run.
**Purpose:** the parent (pdftract-3c39e12c) failed 4x because each attempt re-derived the
verdict from scratch. This is the one file a future worker reads instead of four.

## 1. Per-child table

| Split child | Bead | Deliverable note | Certifying commit(s) | Conclusion carried |
|---|---|---|---|---|
| 1 | pdftract-b6d69433 | `notes/pdftract-3c39e12c-publication.md` | `905c229c` (original); re-verified at later tips by `eeb0d65c`, `85b60661` | **PASS** — all four chain SHAs (`e9036d20`, `336bf8ee`, `10df3676`, `1702ba6b`) are ancestors of Forgejo `origin/main`; `git rev-list --count origin/main..HEAD` = 0 at every check, including two re-verifications as the tip advanced (`d68d9dfe`, `20fac636`) |
| 2 | pdftract-d5598671 | `notes/pdftract-3c39e12c-orphans.md` | `92b1ecf0` (original); re-verified by `c41e5bb9`, `b8d3f12b` | **CLEAN, 5 sweeps across 3 fully disjoint process generations** — no `cargo`/`rustc`/`nextest`/`pdftract` process from any chain bead alive; every sweep hit was a `bash` NEEDLE harness wrapper or the check shell itself; zero processes killed |
| 3 | pdftract-778fc59c | `notes/pdftract-3c39e12c-gate-status.md` | `8ee27f95` | **NO-GO stands, re-affirmed at pushed tip `df7b7de2`** — 36 commits between gate HEAD `0364a3a8` and that tip are all docs-only; zero fixing commits touch the two tip-broken files; both are byte-identical to what the gate compiled against |
| 4 | pdftract-f141d045 | `notes/pdftract-3c39e12c-split-record.md` (this file) | *(this commit)* | Consolidation: children 1–3's evidence in one place + the restated handoff below |

The split's parent deliverable itself — the handoff note
`notes/b716bac5-child1-handoff.md` — was published by pdftract-3c39e12c as `1702ba6b`.

## 2. Verdict and gate exit code (quoted verbatim)

From `notes/b716bac5-child1-handoff.md` (which itself quotes `notes/b716bac5-child1.md`):

> ## VERDICT: NO-GO
>
> cargo exited **101**. The `pdftract-core` lib test target does **not** compile, so the
> umbrella's children 2–4 must **not** proceed with any test run until this gate clears.

**Gate exit code: 101** — `${PIPESTATUS[0]}` of the single gated invocation
`timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run`, run
2026-09-08T12:28:32Z → 12:28:41Z at gate HEAD `0364a3a89eabf5a02715b7ef7daba764f42d23d7`.
Summary line, verbatim from the committed raw log (`notes/b716bac5-child1-gate-raw.log`, line 2220):

``error: could not compile `pdftract-core` (lib test) due to 89 previous errors; 129 warnings emitted``

## 3. Restated handoff statement for the umbrella's children 2–4

**NO-GO — the umbrella's children 2–4 may NOT proceed with any `cargo test` run against this
tree.** The `pdftract-core` lib test target does not compile (exit 101, 89 errors). Runtime
evidence for the umbrella (origin bf-5o22rf criterion (a)) stays blocked until the gate clears.
Child 3 re-affirmed this at the pushed tip `df7b7de2` (2026-09-08), so the statement below is
current, not merely the original publication-time wording.

### Files that must clear first (per-file inventory, 89 errors / 4 files)

| File | Errors at gate | Clearance needed | Status per child 3 (at tip `df7b7de2`) |
|---|---|---|---|
| `crates/pdftract-core/src/classify.rs` | 54 (E0609 x52, E0277 x2) | a **commit to `main`** — errors are broken at the pushed tip | still broken: **no** fixing commit on `origin/main`, working tree **clean** |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 (E0061 x24, E0599 x5) | local edits + commit | uncommitted sibling in-flight edits only — no commit |
| `crates/pdftract-core/src/render/scanline.rs` | 5 (E0277 x2, E0308 x2, E0599 x1) | a **commit to `main`** — errors are broken at the pushed tip | still broken: **no** fixing commit on `origin/main`, working tree **clean** |
| `crates/pdftract-core/src/content_stream.rs` | 1 (E0061 x1) | local edits + commit | uncommitted sibling in-flight edits only — no commit |

The NO-GO rule, per child 3: **NO-GO stands unless a fixing commit is verifiably present on
`origin/main` for BOTH tip-broken files (`classify.rs` AND `render/scanline.rs`)** — 59 of the
89 errors are *committed* at the pushed tip and can only clear via a commit to `main`; local
in-flight edits alone never clear them. All 89 errors are inside `#[cfg(test)]` test modules
(zero production breakage), and `crates/pdftract-core/src/source/mmap.rs` — the observability
work this chain exists to verify — has zero errors and is explicitly **not** a blocker.

## 4. Canonical pointers

| Artifact | Path | Certifying commit |
|---|---|---|
| Verdict note (canonical) | `notes/b716bac5-child1.md` | `10df3676` (bead pdftract-003ddcd1) |
| Raw gate log (2,220 lines, exit 101) | `notes/b716bac5-child1-gate-raw.log` | `336bf8ee` (bead pdftract-68091d06) |
| Pre-gate git baseline | `notes/b716bac5-child1-baseline.md` | `e9036d20` (bead pdftract-3e8309f4) |
| Handoff statement (original) | `notes/b716bac5-child1-handoff.md` | `1702ba6b` (bead pdftract-3c39e12c) |
| Gate-status re-affirmation (newest determination) | `notes/pdftract-3c39e12c-gate-status.md` | `8ee27f95` (bead pdftract-778fc59c) |

## 5. Consistency of the two handoff statements

There are two GO/NO-GO statements in the chain and they **agree — both NO-GO, no conflict to
arbitrate**:

- `notes/b716bac5-child1-handoff.md` (commit `1702ba6b`, published 2026-09-08 at tip
  `9c9db3bf`) issued the original NO-GO from the gate run itself.
- `notes/pdftract-3c39e12c-gate-status.md` (commit `8ee27f95`, child 3) is the **newer**
  determination: it re-affirmed NO-GO at the later tip `df7b7de2` after verifying that all
  36 intervening commits are docs-only and that neither `classify.rs` nor
  `render/scanline.rs` gained a fixing commit or a local modification. The gate verdict is
  therefore still live at the tip this note was written at — the restated statement in
  section 3 reflects child 3's version, which supersedes the original only in *freshness*,
  not in outcome.

Nothing here re-runs the gate. If a future worker needs to know whether NO-GO *still* holds,
the check is the one child 3 defined — read-only git (`git fetch origin`, then
`git log 0364a3a8..origin/main -- crates/pdftract-core/src/classify.rs
crates/pdftract-core/src/render/scanline.rs`; empty ⇒ NO-GO stands) — not another cargo run.

## References

- pdftract-f141d045 (this bead); parent pdftract-3c39e12c; umbrella pdftract-b716bac5;
  chain root pdftract-fa233a5e; split children pdftract-b6d69433 / pdftract-d5598671 /
  pdftract-778fc59c
- All eight notes listed in sections 1 and 4 above
