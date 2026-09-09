# pdftract-3c39e12c — gate-status re-affirmation at the pushed tip (2026-09-08)

**Bead:** pdftract-778fc59c (auto-split child 3 of 4 of pdftract-3c39e12c; umbrella
pdftract-b716bac5; depends on child 2 pdftract-d5598671). Read-only git check of
whether the recorded compile-gate verdict still stands at the live tree state.
**No cargo was run** — the gate ran once (notes/b716bac5-child1-gate-raw.log,
exit 101) and re-running it is this chain's over-budget failure mode.

## Recorded verdict being re-affirmed

From `notes/b716bac5-child1.md` (bead pdftract-003ddcd1) and
`notes/b716bac5-child1-handoff.md`: **NO-GO** — gate exit **101**, 89 errors confined
to 4 files, run 2026-09-08T12:28:32Z at gate HEAD
`0364a3a89eabf5a02715b7ef7daba764f42d23d7`.

## Verdict rule applied

**NO-GO stands unless a fixing commit is verifiably present on `origin/main` for BOTH
tip-broken files (`crates/pdftract-core/src/classify.rs` AND
`crates/pdftract-core/src/render/scanline.rs`).** In-flight local edits alone never
clear those two — at gate time both were clean (unmodified), so their 59 combined
errors are *committed* at the pushed tip and can only clear via a commit to `main`.

## Live state checked (2026-09-08)

- `git fetch origin` — no-op; local HEAD == `origin/main` ==
  `df7b7de2658052edf2c48e51722ccd336da36e62`.
- `0364a3a8` is an ancestor of `origin/main` (`git merge-base --is-ancestor` → yes);
  36 commits sit between gate HEAD and the current tip.
- **All 36 are docs-only** (`git log 0364a3a8..origin/main -- . ':(exclude)notes'
  ':(exclude).beads'` → empty; every commit is a `docs(...)` touching `notes/`).

## Per-file table

| File | Errors at gate | Commits on `origin/main` since gate (`git log --since=2026-09-08`, and `0364a3a8..origin/main`) | Working tree (`git status --porcelain`) |
|---|---|---|---|
| `crates/pdftract-core/src/classify.rs` | 54 (E0609 x52, E0277 x2) | **none** | **clean** — still broken as committed at the tip |
| `crates/pdftract-core/src/render/scanline.rs` | 5 (E0277 x2, E0308 x2, E0599 x1) | **none** | **clean** — still broken as committed at the tip |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 (E0061 x24, E0599 x5) | none | modified (uncommitted sibling in-flight edits, 10 lines changed) — same shape as at gate time |
| `crates/pdftract-core/src/content_stream.rs` | 1 (E0061 x1) | none | modified (uncommitted sibling in-flight edits, 305 lines changed) — same shape as at gate time |

Both rule-critical files (`classify.rs`, `render/scanline.rs`) have **zero** commits on
`origin/main` since the gate and **zero** local modifications — identical bytes to what
the gate compiled against at `0364a3a8`.

## Resulting verdict

> ## VERDICT: **NO-GO stands** — re-affirmed 2026-09-08

No fixing commit exists on `origin/main` for either tip-broken file, so the rule's GO
condition is not met (no fixing commit SHA exists to quote — `git log
0364a3a8..origin/main -- <either file>` is empty). The gate remains uncleared:
`pdftract-core` (lib test) does not compile at the pushed tip, and children 2–4 of
pdftract-3c39e12c must still not proceed with any test run until a commit to `main`
fixes `classify.rs` and `render/scanline.rs`.

---

# Re-affirmation log

## 2026-09-08 (2nd check) — tip `e58fb369` — **NO-GO stands**

Re-derived from git alone after the tip advanced one docs commit past the 1st check
(`df7b7de2` → `e58fb369`, "consolidate single-owner conclusion for mmap re-run").
Still **no cargo run** — same rule, same gate evidence (raw log re-validated this
check: cargo summary line reads *"could not compile `pdftract-core` (lib test) due to
89 previous errors"*, exit 101; counting each diagnostic's **primary** `-->` location
reproduces the table exactly: classify.rs 54, type3_rasterizer.rs 29, scanline.rs 5,
content_stream.rs 1 = 89).

Fresh evidence at `e58fb369c951e5646e648913d3678fa859203eed`:

- `0364a3a8` still an ancestor of `origin/main`; range now **39 commits**, all
  docs-only (`0364a3a8..origin/main -- . ':(exclude)notes' ':(exclude).beads'` → empty).
- **Zero commits touch any of the four files** (`git log 0364a3a8..origin/main --
  <file>` empty for all four; `--since=2026-09-08` likewise empty).

| File | Errors at gate | Commits on `origin/main` since gate | Working tree |
|---|---|---|---|
| `classify.rs` | 54 | **none** | **clean** |
| `render/scanline.rs` | 5 | **none** | **clean** |
| `font/type3_rasterizer.rs` | 29 | none | modified (uncommitted, 10 lines) |
| `content_stream.rs` | 1 | none | modified (uncommitted, 305 lines) |

Rule applied unchanged: GO requires a fixing commit on `origin/main` for BOTH
`classify.rs` AND `render/scanline.rs`; both remain byte-identical to what the gate
compiled at `0364a3a8` and their combined 59 errors are still **committed at the tip**.
The two modified files are sibling in-flight edits, which the rule says can never clear
them.

> **VERDICT (2026-09-08, tip `e58fb369`): NO-GO stands.** No fixing commit SHA exists
> to quote. Children 2–4 of pdftract-3c39e12c must still not run tests until a commit
> to `main` fixes `classify.rs` and `render/scanline.rs`.

## 2026-09-08 (3rd check) — tip `a0dc9569` — **NO-GO stands**

Re-derived from git alone after the tip advanced five more docs commits
(`e58fb369` → `a0dc9569`). Still **no cargo run**. Gate raw log re-validated this
check: `# cargo exit code: 101` in the header and the verbatim summary line at
log line 2220 — *"could not compile `pdftract-core` (lib test) due to 89 previous
errors; 129 warnings emitted"*. Counting each diagnostic's **primary** `-->`
location reproduces the table exactly: classify.rs 54, type3_rasterizer.rs 29,
scanline.rs 5, content_stream.rs 1 = 89. (Do not naively `grep '^\s*-->' | sort |
uniq -c` the log: that counts secondary spans too — "required by this bound",
lifetimes, etc. — and inflates every file. Anchor on the `^error` line, take the
*first* `-->` after it.)

### ⚠ Trap found this round: `--since=2026-09-08` is NOT the fixing-commit test

`git log origin/main --oneline --since=2026-09-08 -- crates/pdftract-core/src/classify.rs`
**returns two commits**:

- `ff006426` "fix(pdftract-6fb49cdb): unwrap classify_page Result in blank_page and image_only_figure tests"
- `f40117c3` "fix(pdftract-72b4cbdf): land Result unwraps in 4 classify_page outcome tests"

**These are not fixes.** Both are ancestors of the gate HEAD (`git merge-base
--is-ancestor ff006426 0364a3a8` → yes; same for `f40117c3`) — commit dates
2026-09-07 21:31 −0400 = 2026-09-08 01:31Z, over ten hours *before* the gate ran
(12:28Z). The gate already compiled them; they are the *origin* of the 54
classify.rs errors, which is why the summary blames `Result<classify::PageClassification, _>`
field access on `classify_page`. The correct fixing-commit test is the range
`git log 0364a3a8..origin/main -- <file>` (empty here), not `--since`. A future
re-check that reads `--since` output as "a commit landed, maybe GO" would flip
this verdict wrongly to GO.

### Fresh evidence at `a0dc95697b96c757dcb80b7e172f3aa5fb8327eb`

- `git fetch origin` — no-op; local HEAD == `origin/main` == `a0dc9569`.
- `0364a3a8` still an ancestor of `origin/main`; range now **44 commits**, all
  docs-only (`0364a3a8..origin/main -- . ':(exclude)notes' ':(exclude).beads'` →
  empty; `0364a3a8..origin/main -- crates/` → empty). Zero commits touch any of
  the four inventory files in the range.

| File | Errors at gate | Commits on `origin/main` since gate (`0364a3a8..origin/main`) | Working tree |
|---|---|---|---|
| `classify.rs` | 54 | **none** — the two `--since` hits above are pre-gate ancestors | **clean** |
| `render/scanline.rs` | 5 | **none** (`--since` also empty; last touch 2026-08-16) | **clean** |
| `font/type3_rasterizer.rs` | 29 | none | modified (uncommitted, 10 lines) |
| `content_stream.rs` | 1 | none | modified (uncommitted, 305 lines) |

Rule applied unchanged: GO requires a fixing commit on `origin/main` for BOTH
`classify.rs` AND `render/scanline.rs`. Both remain byte-identical to what the
gate compiled at `0364a3a8` (clean working tree, zero commits in range) — their
combined 59 errors are still **committed at the pushed tip**. The two modified
files remain sibling in-flight edits, which the rule says can never clear them.

> **VERDICT (2026-09-08, tip `a0dc9569`): NO-GO stands.** No fixing commit SHA
> exists to quote. Children 2–4 of pdftract-3c39e12c must still not run tests
> until a commit to `main` fixes `classify.rs` and `render/scanline.rs`.

## 2026-09-08 (4th check) — tip `589f8c55` — **NO-GO stands**

Re-derived from git alone after the tip advanced four more docs commits
(`a0dc9569` → `589f8c55`). Still **no cargo run**. This check exists because the
auto-split dispatcher re-issued the bead a 4th time (reopen as
`verification-failed`, `failure-count:3`, quarantine expired 23:51Z); the
`failure-count` is the prior evidence-bearing closes being reopened, not real
work failures — hence re-derive and re-close rather than split.

Fresh evidence at `589f8c55`:

- `git fetch origin` — no-op; local HEAD == `origin/main` == `589f8c55`.
- `0364a3a8` still an ancestor of `origin/main`; range now **48 commits**, all
  docs-only (`0364a3a8..origin/main -- . ':(exclude)notes' ':(exclude).beads'` →
  empty). Zero commits touch any of the four inventory files in the range.
- Fixing-commit test done with the **range** form, per the trap finding above
  (`git log 0364a3a8..origin/main -- <file>` → empty for all four). Plain
  `--since=2026-09-08` returned empty for all four in this shell too — but that
  is *not* load-bearing: the range test is the authoritative one.

| File | Errors at gate | Commits on `origin/main` since gate (`0364a3a8..origin/main`) | Working tree |
|---|---|---|---|
| `classify.rs` | 54 | **none** | **clean** — still broken as committed at the tip |
| `render/scanline.rs` | 5 | **none** | **clean** — still broken as committed at the tip |
| `font/type3_rasterizer.rs` | 29 | none | modified (uncommitted sibling in-flight edits, 10 lines) |
| `content_stream.rs` | 1 | none | modified (uncommitted sibling in-flight edits, 305 lines) |

Rule applied unchanged: GO requires a fixing commit on `origin/main` for BOTH
`classify.rs` AND `render/scanline.rs`. Both remain byte-identical to what the
gate compiled at `0364a3a8` (clean working tree, zero commits in range) — their
combined 59 errors are still **committed at the pushed tip**. The two modified
files remain sibling in-flight edits, which the rule says can never clear them.

> **VERDICT (2026-09-08, tip `589f8c55`): NO-GO stands.** No fixing commit SHA
> exists to quote. Children 2–4 of pdftract-3c39e12c must still not run tests
> until a commit to `main` fixes `classify.rs` and `render/scanline.rs`.

### Auto-split re-issue declined (this check)

The dispatcher's 4th issue asks to split this bead into 3–5 children and convert
it to an umbrella. Declined: this bead is itself a leaf `split-child` (child 3 of
4 of pdftract-3c39e12c), its scope is a single read-only note, and every
acceptance criterion is already met at HEAD (see the 1st–4th checks above and the
three citing commits `8ee27f95`, `60d981cf`, `bd12b867`). Splitting a satisfied
leaf would multiply redispatch surface, not shrink it. The terminal action is
this evidence close.
