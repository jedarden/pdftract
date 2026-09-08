# pdftract-c1f3cf4b — raw gated classify.rs compile output

Child 1 of 4 of the re-split of pdftract-cc1e9d96. This child only **measures and
records**; it fixes nothing. Evidence produced by THIS run, committed as a durable
artifact so no later child in this chain inherits numbers from a prior attempt.

This is the **third measurement** under this bead: attempt 1 measured at HEAD
`d72f705f` and attempt 2 at HEAD `730589ed`, but the bead was released without
closing each time, so this run re-measured fresh rather than reusing either
artifact — per the FAIL criterion ("evidence copied from a prior attempt"). The
fresh capture is genuinely this run's output: it differs from the committed
attempt-2 artifact by exactly one line (attempt 2 had a
`Blocking waiting for file lock on artifact directory` line; this run did not).

## Run identity

| Field | Value |
|---|---|
| HEAD SHA (`git rev-parse --short HEAD`) | `e7a75b9e` (full: `e7a75b9e60e565eb393b7820d3f28605c40e4f6a`) |
| Run started (UTC) | 2026-09-08T07:16:43Z |
| Run ended (UTC) | 2026-09-08T07:16:50Z |
| Wall-clock seconds | **7 s** (everything cached from the 184 s attempt-2 run except `pdftract-core` itself, which re-emitted its errors) |
| cargo exit code | **101** |
| Raw artifact | `notes/evidence/pdftract-c1f3cf4b-raw.txt` (225 lines, 31,028 bytes) |

## Exact command

```bash
mkdir -p notes/evidence && START_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ) && S=$(date +%s) && timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run --message-format=short > notes/evidence/pdftract-c1f3cf4b-raw.txt 2>&1; EC=$?; E=$(date +%s); END_ISO=$(date -u +%Y-%m-%dT%H:%M:%SZ); { echo "cargo exit: $EC"; echo "wall-clock seconds: $((E-S))"; echo "started: $START_ISO"; echo "ended: $END_ISO"; } | tee /tmp/c1f3cf4b-run-meta.txt
```

Output is **redirected to a file**, not piped, so the exit code is unambiguous.
`nextest` is not installed on this box (see memory), so the timeout-wrapped
`cargo test` fallback is used per repo CLAUDE.md. The tree is dirty (66 porcelain
entries; `git diff --stat HEAD` = 55 files changed, +4527/−11196 — live sibling
in-flight work), so the `cargo` wrapper ran the build **locally** under cgroup
limits (CPUQuota=200%, MemoryMax=6G) rather than submitting to iad-ci. This is
exactly the run the bead specifies. The log opens with
`Compiling pdftract-core v0.1.0` and closes with the `could not compile` summary,
so the target was genuinely recompiled in this run, not replayed from cache.

`crates/pdftract-core/src/classify.rs` itself was **clean at HEAD `e7a75b9e`**
(unmodified in the working tree), so the classify.rs error lines below are
attributable to HEAD's classify.rs plus the dirty-tree production state it
compiles against.

## Result: cargo exit 101 — WARN (expected)

`timeout` did not fire (7 s ≪ 600 s); **not** an exit-124/TIMEOUT, so this is
not a FAIL. The lib test target still does not compile overall — expected WARN
(last line of the raw artifact):

```
error: could not compile `pdftract-core` (lib test) due to 89 previous errors; 129 warnings emitted
```

## Total classify.rs error locations: 54

Counting command (exact, from the captured file):

```bash
grep "^crates/pdftract-core/src/classify.rs" notes/evidence/pdftract-c1f3cf4b-raw.txt | grep "error\[" | wc -l
# 54
```

Breakdown of the 54:

| Code | Count | Notes |
|---|---|---|
| E0609 | 52 | no field on type (e.g. `no field 'class' on type 'Result<PageClassification, ClassificationError>'`) |
| E0277 | 2 | at `classify.rs:2694` and `classify.rs:2778` — matches baseline exactly |
| **Total** | **54** | |

Line span: **min 2211, max 2793**. Errors at line ≤ 2196 (the target range — the
6 `classify_page` outcome tests, decls 2055–2183): **0**. The target range is
already clean at HEAD; every remaining classify.rs error is below it.

## All error locations by file (sums to 89 = the reported total)

| File | Locations | Owner |
|---|---|---|
| `crates/pdftract-core/src/classify.rs` | 54 | siblings `pdftract-71de30c6` / `pdftract-82ce7969` |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 29 | beads outside this subtree |
| `crates/pdftract-core/src/render/scanline.rs` | 5 | beads outside this subtree |
| `crates/pdftract-core/src/content_stream.rs` | 1 | beads outside this subtree |

## Acceptance criteria

- **PASS** — `notes/evidence/pdftract-c1f3cf4b-raw.txt` exists, was produced by
  this run (ended 2026-09-08T07:16:50Z, not copied from any prior attempt), and
  is committed. This note records command + cargo exit code (101) + HEAD SHA
  (`e7a75b9e`) + wall-clock (7 s) + total classify.rs error count (54) from
  THIS run.
- **WARN (expected)** — cargo exit 101; the lib target does not compile overall.
  The 54 remaining classify.rs errors belong to siblings `pdftract-71de30c6` /
  `pdftract-82ce7969`; the `font/type3_rasterizer.rs` (29), `render/scanline.rs` (5)
  and `content_stream.rs` (1) errors belong to beads outside this subtree.
- **FAIL** — not applicable: exit was 101, not 124/TIMEOUT; evidence is fresh from
  this run.

## Hard rules honored

- Nothing under `crates/` was modified by this child (evidence run compiles only;
  `--no-run` builds, never runs tests). No derives added to any production type.
- No socket bound; no spawned server processes (nothing to orphan — confirmed:
  the only child was `cargo`/`rustc`, exited with the command).
