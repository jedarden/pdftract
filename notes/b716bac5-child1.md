# pdftract-b716bac5 — child 1: pdftract-core lib test target compile gate — VERDICT

**Bead:** pdftract-003ddcd1 (grandchild 3 of 4 of pdftract-fa233a5e; umbrella pdftract-b716bac5)
**Input:** `notes/b716bac5-child1-gate-raw.log` — the verbatim combined stdout+stderr of the
single gated invocation captured by pdftract-68091d06 and committed at `336bf8ee`.
**Tree baseline:** `notes/b716bac5-child1-baseline.md` (pdftract-3e8309f4).
**This note is analysis only:** no cargo command of any kind was run to produce it; every
number below is parsed out of that one committed log.

## VERDICT: NO-GO

cargo exited **101**. The `pdftract-core` lib test target does **not** compile, so the
umbrella's children 2–4 must **not** proceed with any test run until this gate clears.

## Run record (the one invocation this verdict describes)

| Field | Value |
|---|---|
| Command | `set -o pipefail; timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run 2>&1 \| tee notes/b716bac5-child1-gate-raw.log \| tail -120` |
| UTC start | 2026-09-08T12:28:32Z |
| UTC end | 2026-09-08T12:28:41Z (~9 s wall — incremental build, deps cached by sibling runs) |
| cargo exit code | **101** (rustc compile failure; captured as `${PIPESTATUS[0]}`, not `tail`'s status — no timeout kill, so 124/137 does not apply) |
| Invocations | exactly **one**; no retries, no overlapping runs |
| rustc summary | ``error: could not compile `pdftract-core` (lib test) due to 89 previous errors; 129 warnings emitted`` (verbatim, log line 2220) |
| Tree | HEAD `0364a3a89eabf5a02715b7ef7daba764f42d23d7` on `main`, dirty working tree (64 pre-existing uncommitted paths from other in-flight work); nothing under `crates/` modified by the gate bead |

> This verdict **supersedes** the earlier 2026-09-08T09:20:54Z NO-GO capture of the same
> gate (prior content of this file, introduced by `a0a4e17a`, still present at
> `336bf8ee`~1). The inventory below is identical to that run's — third consecutive
> identical error set — but the provenance here is the *committed* raw log of
> pdftract-68091d06, not a re-run.

## Per-file error inventory — 89 errors, 4 files

Format: `file -> error codes -> counts`.

```
crates/pdftract-core/src/classify.rs            -> E0609 x52, E0277 x2                     -> 54
crates/pdftract-core/src/font/type3_rasterizer.rs -> E0061 x24, E0599 x5                   -> 29
crates/pdftract-core/src/render/scanline.rs     -> E0277 x2, E0308 x2, E0599 x1            -> 5
crates/pdftract-core/src/content_stream.rs      -> E0061 x1                                -> 1
-------------------------------------------------------------------------------------------
TOTAL                                                                                      -> 89
```

Code distribution across all 89: `E0609 x52, E0061 x25, E0599 x6, E0277 x4, E0308 x2`.

## `crates/pdftract-core/src/source/mmap.rs` — explicitly checked: **zero errors** (expected: none ✓)

Parsed every `error[E…]` primary span in the committed log: **no error points into
`source/mmap.rs`**. The file has **zero mentions in the entire 2,220-line log** — not even a
warning. The mmap observability work this chain exists to verify (commit `08ec8db8`, trace
logging under test, plus its in-flight test additions) is **not** the blocker; the gate
failure is entirely sibling test-module code in four other files.

## Git state at gate time (HEAD vs origin/main)

From the pre-gate baseline (pdftract-3e8309f4, captured 2026-09-08T11:48:28Z):

- Baseline HEAD `3dcfb6bee55b1ff11cb3e73dbb06e97b76494e13` = `origin/main` (Forgejo,
  re-fetched at capture), divergence `0 / 0` — exactly at the pushed remote tip.
- The gate ran ~40 min later at HEAD `0364a3a8`. Exactly **two** commits sit in between,
  both docs-only under `notes/` and touching **nothing under `crates/`**:
  `e9036d20` (adds `notes/b716bac5-child1-baseline.md` itself) and `0364a3a8` (adds
  `notes/evidence/README.md`). Both are ancestors of `origin/main` (verified read-only via
  `git show --stat`). The baseline's `crates/` inventory therefore describes the gate tree
  exactly.

Dirty-vs-clean split of the four error files at that tree — this decides who can clear the gate:

| File | Working tree vs HEAD | Consequence |
|---|---|---|
| `classify.rs` | **clean** | its 54 errors are **committed** at the pushed tip of `main` |
| `render/scanline.rs` | **clean** | its 5 errors are likewise **committed** at the pushed tip |
| `font/type3_rasterizer.rs` | modified (uncommitted sibling edits) | its 29 errors sit in a locally-edited state |
| `content_stream.rs` | modified (uncommitted sibling edits) | its 1 error sits in a locally-edited state |
| `source/mmap.rs` | modified (uncommitted sibling edits, **no errors**) | the mmap work under test is **not implicated** in the gate failure |

**Key finding for children 2–4: 59 of the 89 errors (`classify.rs` + `render/scanline.rs`)
are broken at the committed, pushed tip of `main`** — not merely in uncommitted in-flight
edits — so the gate will not clear on its own via sibling commits. The `#[cfg(test)]`
modules have to catch up to the changed production signatures.

At verdict-writing time (2026-09-08, this bead) the picture is unchanged: HEAD =
`origin/main` = `336bf8ee` (the commit carrying the raw log), and the tree is still dirty
with the same 49 in-flight paths under `crates/` (`source/mmap.rs` included).

## Error character — all 89 are `#[cfg(test)]` test code, zero production breakage

Every error's primary span lies inside the file's test module (verified: `classify.rs`
`#[cfg(test)]` starts at line 1607, errors span 2211–2793; `scanline.rs` 657, errors
904–938; `type3_rasterizer.rs` 2436, errors 2986–4007; `content_stream.rs` 2409, error
3347). Representative diagnostics, verbatim from the log:

- `E0609` `classify.rs:2211` — `assert_eq!(result.class, PageClass::Scanned)`: the test
  reads `.class` off a `Result<PageClassification, ClassificationError>`; rustc suggests
  `result.unwrap().class`. Tests predate the `Result`-returning production signature.
- `E0277` `classify.rs:2694` — `serde_json::to_string(&result)` needs
  `ClassificationError: Serialize`, which the production enum does not implement.
- `E0061` `content_stream.rs:3347` — `execute_with_do(...)` now takes 7 arguments; the test
  supplies 6 (missing `argument #7 … Option<&xref::XrefResolver>`).
- `E0061` `type3_rasterizer.rs:2986` / `2995` — `detect_char_proc_type` now takes 2
  arguments; tests still pass a third (`{integer}`).
- `E0599` `type3_rasterizer.rs:2987` — `CharProcType::Unknown` no longer exists.
- `E0308`/`E0277` `scanline.rs:904`, `926` — `edge.x` is now `i32` and `edge.slope()`
  returns `(i32, i32)`; tests still compare against floats (`10.0`, `1.0`).

`signature/mod.rs` and `word_boundary.rs`, which broke earlier runs, are clear in this log:
`signature/mod.rs` contributes only unused-`mut` **warnings**.

## Directive to children 2–4 of this split

Do **not** run `cargo test` against this tree — the gate is NO-GO. Runtime evidence for the
umbrella (origin bf-5o22rf criterion (a)) waits on the four test modules being brought up to
the changed signatures; 59 of 89 errors need a commit to `main`, not just local edits.

## Compliance

- No cargo command was run by this bead (verdict derived purely from the committed log).
- No file under `crates/` was touched; the only file this bead writes is this note.
- No error was "repaired" while inventorying it.
- Orphan check at close: no `cargo`/`pdftract` process spawned by this bead exists.

---

# RE-RUN 2026-09-14 — VERDICT: GO (supersedes both NO-GO captures above)

Re-derived at HEAD by pdftract-fa233a5e per the re-issue rule: a go/no-go artifact is a
*current* signal, and the 2026-09-08 captures were six days stale against continuous
sibling drift. Run exactly once, no retries, no overlapping invocation.

## Run record

| Field | Value |
|---|---|
| Command | `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run > /tmp/fa233a5e-gate-20260914.log 2>&1` (exit captured directly — no pipe eating the status) |
| UTC | 2026-09-14T12:40:53Z (completion) |
| exit code | **0** |
| rustc summary | ``Finished `test` profile [unoptimized + debuginfo] target(s) in 31.36s`` + ``Executable unittests src/lib.rs (/build/target-workers/debug/deps/pdftract_core-01467a2e479d2729)`` (verbatim) |
| Errors | **0** — 0 lines matching `^error`, 0 `error[E-NNNN]` codes in 1,335 lines of output; 169 warnings only (unused doc comments, unused vars, drop_bounds) |
| `source/mmap.rs` | no errors — trivially: zero errors anywhere in the target |
| Tree | HEAD `fff28e0c4a75a78d673986e18a5150d0c227b229` on `main`, == `origin/main`; 60 pre-existing dirty paths under `crates/` from sibling in-flight work; **no `crates/` file modified by this bead** |
| Raw log | `/tmp/fa233a5e-gate-20260914.log` (transient; not committed — GO needs no error inventory, and the run is reproducible in ~31 s) |

## What changed since the NO-GO captures

The 89-error set is gone at HEAD: `classify.rs` (E0609 x52, E0277 x2),
`font/type3_rasterizer.rs` (E0061 x24, E0599 x5), `render/scanline.rs` (E0277/E0308/E0599)
and `content_stream.rs` (E0061) test modules now compile — the sibling workers' signature
migrations landed. Supersedes `a0a4e17a` (f785930f) and `20189f01`/`10df3676` (0364a3a8).

## Directive to the umbrella's children 2–4

The gate is **GO**: the `pdftract-core` lib test target compiles at `fff28e0c`. Test runs
may proceed — for pdftract-b716bac5 that is the mmap runtime-evidence re-run
(`cargo test -p pdftract-core --lib source::mmap`, timeout-wrapped), still owed against
this fresh gate. Note the gate speaks only for compile: the ~300 pre-existing *runtime*
test failures recorded elsewhere at HEAD are unaffected by this verdict.

## Auto-split order on pdftract-fa233a5e — declined; the split already exists and is complete

The 2026-09-14 re-dispatch ordered a 3–5 child split of pdftract-fa233a5e. It was already
performed by a prior dispatch and is **complete**: sequential children
`3e8309f4 → 68091d06 → 003ddcd1 → 3c39e12c` (all `split-child`, all **closed** — last
child `3c39e12c` closed 2026-09-14T10:46Z), parent carrying `umbrella` and dependent on
the last child (`blocker: pdftract-3c39e12c, kind: blocks`). Creating a second-generation
chain would duplicate work units on a shared checkout. Record: `notes/fa233a5e-gate-reaffirm.md`
("that was declined, not forgotten"). This bead's own scope — one gate run, one artifact —
was executed instead, and the bead closes on this evidence.

## Compliance (this re-run)

- One gated invocation; no retry, no overlapping run, no bare `cargo test`.
- No file under `crates/` touched; the only tracked file this bead writes is this note.
- Orphan check at close: nothing spawned by this bead remains (gate ran foreground to completion).
