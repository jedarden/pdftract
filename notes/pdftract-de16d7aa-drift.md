# pdftract-de16d7aa — mmap trace-path audit: drift re-verification

**Bead:** pdftract-dee97c8d (child 1 of 4 in the re-split chain under
pdftract-de16d7aa; REFRESH of closed pdftract-216e4ddc, not a redo).
**Re-verified against:** working tree + HEAD of
`crates/pdftract-core/src/source/mmap.rs`, 2026-09-08 02:52 UTC.
**Inputs:** `notes/bf-5o22rf-child1.md` §1 — the closed audit's recorded refs.
**Scope:** code-path re-confirmation only. No tests executed (pdftract-b716bac5
owns runtime evidence). No production code changes.

## Headline: no drift — every recorded reference still resolves to the same code

The refresh brief predicted the closed note's refs had drifted
("advise_sequential now near 81-104, prefetch near 140-165"). **That prediction
is not borne out at the current tree**: every reference the closed audit
recorded is identical today, to the line, in both the working tree and HEAD.
All four audit conclusions still hold; the WARN rider (no runtime test
evidence) is unchanged and remains owned by pdftract-b716bac5.

Evidence that the file is in the same state the closed audit saw, not merely
similar:

- `git status --porcelain` on the file: still `M`, still exactly the in-flight
  diff the closed audit described — **137 insertions(+), 1 deletion(-)**,
  unchanged.
- `git diff -U0` hunk headers on the file: `@@ -202,0 +203 @@`,
  `@@ -204 +205 @@`, `@@ -448,0 +450,135 @@` — every hunk is inside
  `mod tests` (line ≥ 202). The entire audited region (lines 1-198) is outside
  the diff.
- `diff` of `git show HEAD:…mmap.rs | head -202` against the working tree's
  first 202 lines: **byte-identical**. The audit's conclusions are therefore
  independent of the uncommitted state — they hold at HEAD too.
- `git log --all --oneline -- …mmap.rs`: 5 commits, latest `08ec8db8` (the
  commit that landed the trace event). No ref, branch or stash-free state
  anywhere holds a version where `prefetch` sits at 140-165.
- File mtime `2026-09-07 07:42 EDT` (= 11:42 UTC): last touched ~12 h *before*
  the closed chain's own re-verification run (2026-09-07 23:32 UTC) and ~15 h
  before this refresh. The refs were stable across the whole window in which
  the drift was predicted to have occurred.

## Per-path verdicts

File: `crates/pdftract-core/src/source/mmap.rs` (645 lines in the working
tree, 22,283 bytes).

| # | Path | Note ref | Current ref | Drift | Verdict |
|---|---|---|---|---|---|
| 1 | `offset + length` overflow → `Err` | 85-87 | 85-87 | none | **still-holds** |
| 2 | range past EOF → `Err` | 89-94 | 89-94 | none | **still-holds** |
| 3 | `memmap2` `advise_range` failure → `io::Error` | 96-98 | 96-98 | none | **still-holds** |
| 4 | all three reach the single `tracing::trace!` event | 140 → 144-151 | 140 → 144-151 | none | **still-holds** |
| 5 | successful advise emits no event | 99 / 140 | 99 / 140 | none | **still-holds** |

### 1. Overflow path — STILL HOLDS

`mmap.rs:85-87`: `let end = start` / `.checked_add(length)` / `.ok_or_else(||
io::Error::new(io::ErrorKind::InvalidInput, "overflow"))?`. Short-circuits
before the EOF check, exactly as recorded. Unchanged.

### 2. Range-past-EOF path — STILL HOLDS

`mmap.rs:89-94`: `if end > self.mmap.len()` → `Err(InvalidInput, "range
extends beyond EOF")`, explicit early return. Unchanged.

### 3. Kernel `advise_range` failure — STILL HOLDS

`mmap.rs:96-98`: `self.mmap.advise_range(Advice::Sequential, start, length)
.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?` — a real memmap2
failure is mapped into the same `io::Error` channel as paths 1 and 2.
Unchanged.

### 4. All three reach the single trace event — STILL HOLDS

`prefetch` spans `mmap.rs:134-153` (recorded: 134-153). `if let Err(e) =
self.advise_sequential(offset, length)` at `mmap.rs:140`; the
`tracing::trace!` at `mmap.rs:144-151` carries all four documented fields —
`offset` (145), `length` (146), `file_len = self.len()` (147), `error = %e`
(148) — plus the message naming the failed call (149-150). There is no `?`,
wrapping, swallowing or remapping between `advise_sequential`'s return and
prefetch's `if let`, so every `Err` from paths 1-3 unconditionally reaches the
event while `prefetch` still returns `()`. It remains the **only**
`tracing::trace!` in the file (`grep -n 'tracing::trace!'` → line 144 alone).
Unchanged.

### 5. Success emits no event — STILL HOLDS

`advise_sequential` returns `Ok(())` at `mmap.rs:99`; `if let Err` at 140 does
not match `Ok`, so a successful advise is silent. The field-level pinning test
`test_prefetch_madvise_failure_is_traced` sits at `mmap.rs:536-583` (recorded:
536-583) — zero events asserted for in-range `prefetch(0, 10)`, exactly one
event with all four fields for past-EOF `prefetch(0, 100)`. The module-doc
contract at `mmap.rs:8-15` (advisory, not propagated, always observable at
trace level) is unchanged.

## Correction to the closed record (provenance, not behaviour)

`notes/bf-5o22rf-child1.md` §1 described the in-flight working-tree insertions
as "doc comment + dedicated tests". The actual uncommitted diff is
**test-module only**: the `BTreeMap`/`Mutex` imports (203, 205) plus the
135-line capture machinery and the field-level test (450-584). The
`# Prefetch error handling` module doc at `mmap.rs:8-15` is **committed** — it
ships in `08ec8db8`, not in the in-flight diff. This mislabel does not change
any conclusion; if anything it strengthens them, since it is precisely why no
production line number could have drifted from the uncommitted state.

## Verdict and follow-up

All four audit conclusions **STILL HOLD** at the current tree; nothing drifted.
The acceptance-criterion trigger "if any path no longer reaches the trace
event, file a follow-up bead" is **not** fired — no gap exists, so no follow-up
bead is filed. The only open item on this path is unchanged and out of scope
here: runtime test evidence for the trace event, owned by pdftract-b716bac5
(the `--lib` test target compile blockage recorded in
`notes/bf-5o22rf-child1.md` §2-3).

## Scorecard — anchor acceptance criteria (pdftract-19b81c8f, child 2 of 4)

Scored 2026-09-08 against the three acceptance criteria of the chain anchor
pdftract-de16d7aa, using artifacts that already exist. This is a scorecard, not
a re-synthesis: no test was executed, `notes/bf-5o22rf-child1.md` was not
rewritten, and no production file was touched. The only tree contact beyond
this section was a read-only spot-check that the audited refs still resolve
(see criterion 1).

| # | Anchor criterion | Verdict | Owning bead if not clean |
|---|---|---|---|
| 1 | `notes/bf-5o22rf-child1.md` exists and states criterion (a) PASS/FAIL with evidence | **PASS** | — |
| 2 | Any gap filed as a follow-up bead rather than fixed silently | **PASS** | — (the WARN itself is owned by pdftract-b716bac5) |
| 3 | No production code changes in this chain | **PASS** | — |

**Overall: 3/3 PASS.** No WARN/FAIL is introduced by this scorecard. The one
standing WARN in the chain (criterion (a) has no runtime test evidence) is not
a failure of any anchor criterion — criterion 1 only requires the note to state
the verdict with evidence, which it does — and it stays owned by
**pdftract-b716bac5** (Open, two precondition-failed releases recorded on the
bead as of this scoring). Scope does not expand here.

### Criterion 1 — PASS

- **Exists and is committed.** `git log --follow -- notes/bf-5o22rf-child1.md`
  → `e923802b` (creation, +114 lines) and `b19a9e52` (§4 follow-up link,
  +17 lines). `git status --porcelain` on the file is empty: the working-tree
  copy is exactly the committed one.
- **States the verdict explicitly with evidence.** §3: *"**PASS** — with a
  **WARN** rider on runtime test evidence."* Evidence is present in both
  required forms: file:line references (`mmap.rs:85-87` overflow, `89-94`
  past-EOF, `96-98` kernel `advise_range`, `140` → `144-151` the four-field
  `tracing::trace!`, `99` silent success, `536-583` the pinning test) and test
  output (§2 — three runs of the timeout-wrapped mmap suite, exit 101, 95
  identical errors, zero in `source/mmap.rs`; re-run re-verified 2026-09-07
  23:32 UTC with a byte-identical error list).
- **Still supported by child 1's refreshed audit** (this file, above, commit
  `a1e06a7d`, verified 2026-09-08 02:52 UTC): every recorded reference resolves
  identically in the working tree and at HEAD, all four audit conclusions still
  hold, and the drift prediction is not borne out. The refresh's one correction
  (the in-flight diff is test-module only, not "doc comment + tests") is
  provenance-only and *strengthens* the verdict. Spot-checked again during this
  scoring: `mmap.rs:85,87,89,94,96,98,99,140,144-148` still carry exactly the
  audited code, and `grep`-level state of the file is unchanged (still `M` in
  the working tree — the same in-flight diff, untouched by any chain commit).

### Criterion 2 — PASS

- The WARN rider's gap (no runtime evidence) is filed, not fixed silently:
  **pdftract-b716bac5** — "Re-run source::mmap tests for runtime evidence of
  the prefetch madvise trace event (bf-5o22rf WARN handoff)", filed by
  pdftract-2efded34, cross-referenced from the note in
  `notes/bf-5o22rf-child1.md` §4 and confirmed to exist via `bead show`
  (Status: Open, description cites this chain and the note).
- The refreshed audit found no drift, so the criterion's *other* trigger — a
  failure path that bypasses the trace event — did not fire; correctly, no
  second follow-up bead was filed and nothing was fixed silently in-tree.

### Criterion 3 — PASS

Every commit in this verification chain touches only `notes/` files; all are
`docs(...)`:

| Commit | Bead | Files |
|---|---|---|
| `e923802b` | pdftract-e6c4a13f | `notes/bf-5o22rf-child1.md` only (+114) |
| `b19a9e52` | pdftract-2efded34 | `notes/bf-5o22rf-child1.md` only (+17) |
| `a1e06a7d` | pdftract-de16d7aa (child 1) | `notes/pdftract-de16d7aa-drift.md` only (+114) |
| this commit | pdftract-19b81c8f (child 2) | `notes/pdftract-de16d7aa-drift.md` only |

Commit `08ec8db8` (the trace logging itself, bead bf-4chy94) predates the
chain — it is the artifact under audit, not a change made by it. `mmap.rs`
remains modified in the working tree, but that in-flight diff belongs to
sibling workers; no chain commit touches any file outside `notes/`.
