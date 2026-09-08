# pdftract-3c39e12c — consolidated handoff verification record

**Bead:** pdftract-f34a11cf (auto-split child 4 of 4 of pdftract-3c39e12c; umbrella chain
pdftract-fa233a5e → pdftract-b716bac5). This is the single place that says whether the
umbrella's children 2–4 may proceed. Consolidates children 1–3 of this split's results;
every section below was freshly re-verified at write time, not copied forward blindly.

**Split of pdftract-3c39e12c:**

| Child | Bead | Role | Status at consolidation |
|---|---|---|---|
| 1 | pdftract-082f2f0f | audit handoff note vs its sources | **closed** — result recorded on the bead |
| 2 | pdftract-ab076af2 | publication check (SHAs on forgejo main) | open — check re-executed fresh by this bead |
| 3 | pdftract-ca47706e | orphaned-process sweep | open — sweep re-executed fresh by this bead |
| 4 | pdftract-f34a11cf | this consolidation | this note |

Children 2 and 3 were still open and unassigned at write time, so their checks were
re-executed here (read-only git, `pgrep`) and the fresh results are what this record
carries. If those beads later record a different result, their bead notes supersede
this one for their own scope.

## (a) The verdict — quoted from `notes/b716bac5-child1.md`

> ## VERDICT: NO-GO
>
> cargo exited **101**. The `pdftract-core` lib test target does **not** compile, so the
> umbrella's children 2–4 must **not** proceed with any test run until this gate clears.

**Gate exit code: 101** — rustc compile failure, `${PIPESTATUS[0]}` of the single gated
invocation `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run`,
run 2026-09-08T12:28:32Z → 12:28:41Z at HEAD `0364a3a8` (raw log:
`notes/b716bac5-child1-gate-raw.log`, committed at `336bf8ee`, summary at line 2220:
``error: could not compile `pdftract-core` (lib test) due to 89 previous errors; 129
warnings emitted``). No re-run was performed for this note; this remains the **last
verified** compile check.

## (b) Child 1 — audit result: PASS, no correction commit

pdftract-082f2f0f (closed) audited `notes/b716bac5-child1-handoff.md` (commit `1702ba6b`)
against its three sources. All three checks **PASS**; the note matches its sources, so **no
correction commit exists or is needed**:

1. **Verdict quote: PASS** — handoff lines 13–16 quote `notes/b716bac5-child1.md` lines
   10–13 verbatim; handoff states "Gate exit code: 101".
2. **Error inventory vs the committed gate log: PASS** — parsing every error block and
   attributing it to its first `-->` span reproduces the handoff exactly: 89 total;
   classify.rs 54 {E0609 x52, E0277 x2}; font/type3_rasterizer.rs 29 {E0061 x24, E0599 x5};
   render/scanline.rs 5 {E0277 x2, E0308 x2, E0599 x1}; content_stream.rs 1 {E0061 x1};
   `source/mmap.rs` 0 errors and 0 mentions in the whole log.
3. **Certified SHAs: PASS** — `git log -1 -- <file>` returns `e9036d20` for the baseline
   (pdftract-3e8309f4), `336bf8ee` for the raw log (pdftract-68091d06), `10df3676` for the
   verdict note (pdftract-003ddcd1), matching the handoff table.

## (c) Child 2 — publication result: all certified SHAs are on forgejo main (fresh check)

Executed by this bead at write time (child 2's bead still open). Remote `origin` =
`https://git.ardenone.com/jedarden/pdftract.git` (Forgejo, the push target); `git fetch
origin` clean. With **origin/main tip observed at `576705dd`** (full SHA
`576705ddb9b8fea95367d5dbf21ff9abae14a11a`, == local HEAD, divergence `0 / 0`):

| Certified SHA | Artifact | Ancestor of origin/main |
|---|---|---|
| `e9036d20` | pre-gate baseline (pdftract-3e8309f4) | **yes** (`git merge-base --is-ancestor`) |
| `336bf8ee` | raw gate log, exit 101 (pdftract-68091d06) | **yes** |
| `10df3676` | verdict note (pdftract-003ddcd1) | **yes** |
| `1702ba6b` | the handoff commit itself (pdftract-3c39e12c) | **yes** |

The verdict and the handoff that certifies it are durably published on Forgejo `main`,
not just local.

## (d) Child 3 — process-sweep result: clean (fresh check)

Executed by this bead at write time (child 3's bead still open):
`pgrep -af "cargo test|pdftract"` returned **no `cargo test` process and no `pdftract`
binary**. The only matches were the needle harness `bash -c` wrappers whose command lines
contain the repo path — PIDs 2794581 (prompt pdftract-3c39e12c), 2930290 (pdftract-cf349714),
2961703 (this bead's own session), 3009481 (pdftract-1f1cf7a5) — plus the `pgrep` itself.
No process spawned by any bead in the gate chain is alive; nothing was killed because
nothing needed killing.

## (e) HANDOFF STATEMENT: **NO-GO** (as of the last verified check)

**The umbrella's children 2–4 may NOT start any `cargo test` run against this tree.** The
`pdftract-core` lib test target does not compile: cargo exit **101**, **89 errors across
4 files — classify.rs 54 / font/type3_rasterizer.rs 29 / render/scanline.rs 5 /
content_stream.rs 1**. Runtime evidence for the umbrella (origin bf-5o22rf criterion (a))
stays blocked until the gate clears.

Freshness of the NO-GO: every commit between the gate tree and the origin/main tip
observed here (`0364a3a8..576705dd`) touched only `.beads/` checkpoint state and `notes/`
— verified via `git log 0364a3a8..HEAD -- crates/ tests/ Cargo.toml Cargo.lock` (empty).
No new compile evidence exists, so the gate result stands as the last verified state, and
this bead did not re-run the gate.

### Per-file clearance list (carried forward from `notes/b716bac5-child1-handoff.md`)

All 89 errors are inside `#[cfg(test)]` test modules — zero production breakage.

| File | Errors | Working tree vs HEAD (re-confirmed at write time) | Clearance needed |
|---|---|---|---|
| `crates/pdftract-core/src/classify.rs` | 54 (E0609 x52, E0277 x2) | **clean** | a **commit to `main`** — broken at the pushed tip |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 (E0061 x24, E0599 x5) | modified (sibling in-flight edits) | local edits + commit |
| `crates/pdftract-core/src/render/scanline.rs` | 5 (E0277 x2, E0308 x2, E0599 x1) | **clean** | a **commit to `main`** — broken at the pushed tip |
| `crates/pdftract-core/src/content_stream.rs` | 1 (E0061 x1) | modified (sibling in-flight edits) | local edits + commit |

**59 of 89** errors (`classify.rs` + `render/scanline.rs`) are broken at the committed,
pushed tip of `main`, so the gate will not clear on its own via sibling commits: those two
test modules must catch up to the changed production signatures (`Result`-returning
`classify`, integer `edge.x`/`slope()`, 2-arg `detect_char_proc_type`, 7-arg
`execute_with_do`, no `CharProcType::Unknown`) **in a commit**.

Explicitly **not** a blocker: `crates/pdftract-core/src/source/mmap.rs` — zero errors and
zero mentions in the 2,220-line gate log (still dirty with sibling in-flight edits, none of
them implicated). The mmap observability work this chain exists to verify is clean.

## Compliance (this bead, pdftract-f34a11cf)

- No cargo invocation of any kind; the gate was **not** re-run. Verification was read-only
  git (`remote -v`, `fetch`, `rev-parse`, `merge-base --is-ancestor`, `log -- <path>`,
  `status --porcelain -- <paths>`) plus `pgrep`.
- No file under `crates/` was touched; the only file this bead writes is this note, staged
  alone (the tree carries sibling in-flight edits that are not ours).
- Orphan check at close time: see (d) — re-checked immediately before closing; clean.

## References

`notes/b716bac5-child1.md` (verdict, pdftract-003ddcd1); `notes/b716bac5-child1-gate-raw.log`
(raw log, pdftract-68091d06); `notes/b716bac5-child1-baseline.md` (pdftract-3e8309f4);
`notes/b716bac5-child1-handoff.md` (handoff, pdftract-3c39e12c, commit `1702ba6b`);
pdftract-fa233a5e; pdftract-b716bac5; pdftract-082f2f0f; pdftract-ab076af2; pdftract-ca47706e.

## Freshly re-verified — 2026-09-08T18:16:34Z (pdftract-a4eb4a16, split child 3 of 4)

Children 1–2 of the pdftract-f34a11cf split found nothing to correct (pdftract-c04460c3:
5/5 source audit PASS, no discrepancies; pdftract-02de535f: publication + freshness PASS),
so everything above stands unchanged apart from this stamp. Re-verified fresh at the stamp
time, read-only git only, no cargo: `git fetch origin` clean; **origin/main tip `d2510888`**
(full `d2510888b2f102039e986b51cb58b2dee4a395a3`) == local HEAD, 0 unpublished commits; all
five certified SHAs (7f2b93eb, e9036d20, 336bf8ee, 10df3676, 1702ba6b) still ancestors of
origin/main; `0364a3a8..origin/main` (14 commits) touches no `crates/` / `tests/` /
`Cargo.toml` / `Cargo.lock` path, so the (e) NO-GO freshness statement holds as-is and the
gate was not re-run — NO-GO remains the last verified compile state. Process sweep clean
(no `cargo test`, no `pdftract` binary).

## Final consolidation — 2026-09-08T19:16:44Z (pdftract-f34a11cf, the split umbrella)

Every bead in the pdftract-fa233a5e → pdftract-b716bac5 → pdftract-3c39e12c chain is now
**closed**; this section is the durable end-state record. The "open" rows in the split
table near the top of this file describe the state at the original consolidation and are
superseded by the results below — the note itself needed no correction, so this section is
append-only, as with the stamp above.

### Split of pdftract-3c39e12c (the parent split) — all closed

| Child | Bead | Role | Final result |
|---|---|---|---|
| 1 | pdftract-082f2f0f | audit handoff note vs its sources | **closed** 2026-09-08 — audit PASS, **no correction commit** |
| 2 | pdftract-ab076af2 | publication check | **closed** 2026-09-08T15:57:00Z — all four certified SHAs (`e9036d20`, `336bf8ee`, `10df3676`, `1702ba6b`) ancestors of origin/main at tip `7f2b93eb`; no push/repair needed |
| 3 | pdftract-ca47706e | orphaned-process sweep | **closed** 2026-09-08T16:17:37Z — **CLEAN**: 7 pgrep hits all NEEDLE dispatch wrappers matching on the literal substring "pdftract" in argv; zero cargo/rustc/pdftract/nextest binaries, zero `pdftract mcp`, zero TH-*, zero zombies |
| 4 | pdftract-f34a11cf | this consolidation | this bead |

### Split of pdftract-f34a11cf (this bead's own 4 children) — all closed

| Child | Bead | Role | Final result |
|---|---|---|---|
| 1 | pdftract-c04460c3 | audit the published note vs its cited sources | **closed** 2026-09-08T17:43:48Z — **5/5 PASS**, no discrepancies, no correction commit; sources in tree byte-identical to their certified commits (`child1.md`==`10df3676`, `child1-handoff.md`==`1702ba6b`, `child1-gate-raw.log`==`336bf8ee`) |
| 2 | pdftract-02de535f | forgejo publication + freshness | **closed** 2026-09-08T18:00:48Z — **PASS**: origin/main tip `d2510888`, local HEAD identical, 0 unpublished; all five certified SHAs ancestors; `crates/` untouched since the note's tip |
| 3 | pdftract-a4eb4a16 | republish with freshly verified state | **closed** 2026-09-08T18:26:18Z — append-only stamp (above), commit `05c9def6` |
| 4 | pdftract-78053a71 | close-time process sweep + final state | **closed** 2026-09-08T19:08:46Z — **CLEAN** (sweep 18:58:00Z–19:00:17Z): zero pdftract binaries, zero MCP servers, zero TH harness processes; nothing killed because nothing chain-spawned existed |

### Freshly verified at this stamp (read-only git; no cargo, gate not re-run)

`git fetch origin` clean; **origin/main tip observed at `5e8cf6ab`** (full
`5e8cf6ab82ce4856540e6c03d613cd9941c9017c`, subject "docs(pdftract-c0acafa1): auto-split
c0acafa1 path-set capture into 4-child umbrella chain") == local HEAD, divergence `0 / 0`.
All six record SHAs are ancestors of `origin/main` (`git merge-base --is-ancestor`):
`7f2b93eb` (this note's first consolidation), `e9036d20`, `336bf8ee`, `10df3676`,
`1702ba6b`, `05c9def6` (the a4eb4a16 stamp). `0364a3a8..origin/main` is **17 commits, none
touching `crates/`, `tests/`, `Cargo.toml` or `Cargo.lock`** — the freshness argument in
section (e) still holds, so **NO-GO remains the last verified compile state**.

### HANDOFF STATEMENT (unchanged): **NO-GO**

**No `cargo test` run may start against this tree.** The `pdftract-core` lib test target
does not compile — cargo exit **101**, **89 errors across 4 files: `classify.rs` 54 /
`font/type3_rasterizer.rs` 29 / `render/scanline.rs` 5 / `content_stream.rs` 1**. The
per-file clearance list in section (e) above (carried forward from
`notes/b716bac5-child1-handoff.md`) stands verbatim: `classify.rs` and `render/scanline.rs`
(59 of 89 errors) are broken at the committed, pushed tip of `main` and need a **commit**;
`font/type3_rasterizer.rs` and `content_stream.rs` need local edits + commit;
`source/mmap.rs` remains explicitly not a blocker. Runtime evidence for the umbrella
(origin bf-5o22rf criterion (a)) stays blocked until the gate clears.

### Close-time process sweep (this bead, immediately before closing): CLEAN

`pgrep -af 'cargo[ ]test|pdftrac[t]'` at 2026-09-08T19:16Z matched only four NEEDLE
dispatch `bash -c` wrappers — PIDs 492729 (prompt pdftract-ff5c577d), **517572 (this bead's
own session, prompt pdftract-f34a11cf)**, 524230 (pdftract-0078d4a6), 530264
(pdftract-bc29f726) — each matching only on the literal substring "pdftract" in the repo
path / prompt filename. Exact-match `ps -eo comm=` scan for
`cargo|rustc|pdftract|nextest` returned **0**; `pdftract[ ]mcp` and `TH[-_]0` probes
matched nothing but the probe's own shell. Nothing chain-spawned is alive; nothing was
killed.

## Freshly re-verified after reopen — 2026-09-08T19:42:47Z (pdftract-f34a11cf)

The 19:20:17Z close of pdftract-f34a11cf (commit `90ed37f6`) was reopened at 19:23:02Z with
no defect reason recorded, so this dispatch re-verified every acceptance-criteria fact from
scratch rather than re-asserting the earlier close. **Result: nothing above changed** — this
section is append-only, like the two before it.

- **Verdict and exit code re-confirmed at source:** `notes/b716bac5-child1.md` line 10
  `## VERDICT: NO-GO`, line 12 `cargo exited **101**`; handoff line 18 `Gate exit code: 101`;
  raw log at `336bf8ee` ends with `error: could not compile \`pdftract-core\` (lib test) due
  to 89 previous errors; 129 warnings emitted`. All three source files in the tree are
  byte-identical to their certified commits (`git diff --stat 10df3676 --` /
  `1702ba6b` / `336bf8ee` each empty).
- **Publication:** `git fetch origin` clean; **origin/main tip `a5521a88`** (full
  `a5521a88b71efe9657185204f1b9d56a54846c98`) == local HEAD, divergence `0 / 0`. All seven
  record SHAs are ancestors of origin/main via `git merge-base --is-ancestor`: `7f2b93eb`,
  `e9036d20`, `336bf8ee`, `10df3676`, `1702ba6b`, `05c9def6`, and `90ed37f6` (the final
  consolidation that was pushed before the reopen).
- **Freshness of NO-GO:** `0364a3a8..origin/main` is now **20 commits** (was 17 at the
  previous stamp) and **zero of them touch `crates/`, `tests/`, `Cargo.toml` or
  `Cargo.lock`** (`git log --oneline 0364a3a8..origin/main -- crates/ tests/ Cargo.toml
  Cargo.lock` is empty). Section (e)'s freshness argument therefore holds unchanged and the
  gate was **not** re-run — NO-GO remains the last verified compile state.
- **Note integrity:** working-tree copy of this file has zero drift from HEAD
  (`git status --porcelain -- notes/pdftract-3c39e12c.md` empty before this stamp).
- **Close-time process sweep:** CLEAN. The AC-named `pgrep -af "cargo test|pdftract"`
  matched only NEEDLE dispatcher `bash -c` wrappers for other beads' prompts (f4043374,
  3c39e12c, 87de95ea, b6d69433, 36076d6d, 686bdc40, this bead's own dispatcher 650246) plus
  the sweep shell — each matching on the literal substring "pdftract" in argv. Exact
  `ps -eo comm=` scan for `cargo|rustc|pdftract|nextest`: **0**. `pdftract[ ]mcp` and
  `TH[-_]0` probes: no match. Nothing chain-spawned is alive; nothing killed.

**HANDOFF STATEMENT (unchanged): NO-GO** — the `pdftract-core` lib test target does not
compile (cargo exit **101**, 89 errors: `classify.rs` 54 / `font/type3_rasterizer.rs` 29 /
`render/scanline.rs` 5 / `content_stream.rs` 1); no `cargo test` run may start against this
tree; the per-file clearance list in section (e) stands verbatim.
