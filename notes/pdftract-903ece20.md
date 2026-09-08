# pdftract-903ece20 — 1800s warm-up `--no-run` build (child 2 of 4, split of pdftract-ba4e7150)

## Command

Run from the repo root (`/home/coding/pdftract`), 2026-09-08 07:17 EDT:

```
timeout --kill-after=60s 1800s cargo test -p pdftract-core --lib --no-run --message-format=short \
  > /tmp/pdftract-903ece20-warmup.txt 2>&1
```

(nextest is not installed on this box; `cargo` is the cgroup-limited wrapper, CPUQuota=200% / MemoryMax=6G.)

## Result: PASS

| Item | Value |
|---|---|
| Exit code | **101** (compile error — expected; any exit satisfies this bead) |
| Wall-clock | **12 seconds** (cap was 1800s; `timeout` did not fire, no kill) |
| Log | `/tmp/pdftract-903ece20-warmup.txt` — 225 lines (not committed; not evidence) |

## What happened

- The cargo target cache was **already warm** from the 2026-09-07 runs, so the command needed only
  12s to drive the `pdftract-core` lib-test unit to its failure point. **0 files** under
  `target/debug` were written by this run (verified with `find target/debug -newermt <run-start>`).
- The lib (non-test) target is cached and present: `libpdftract_core-93f5e5a2f3c2cf29.rlib` (82 MB)
  + `.rmeta` (9.7 MB), and a cached lib-test binary `pdftract_core-53002a33ca364ac3` (10.8 MB),
  all from Sep 7. `target/` totals 3.8 GB.
- The `--no-run` build of the **lib test** target fails with the pre-existing **89 errors** — the
  same count the parent (pdftract-ba4e7150) recorded across the 4 test modules:

  | Error | Count |
  |---|---|
  | E0609 (no field on type) | 52 |
  | E0061 (wrong arg count) | 25 |
  | E0599 (no method) | 6 |
  | E0277 (trait bound) | 4 |
  | E0308 (mismatched types) | 2 |

  First errors are at `crates/pdftract-core/src/classify.rs:2211` ff. (`Result` not unwrapped
  before field access). 129 warnings emitted on the lib test target.

## Implication for child 3 (gated run)

The cache is warm: a gated 600s `--no-run` re-run reaches this same compile-error wall in seconds,
so the exit-124 risk this bead exists to absorb is nil. Child 3 should expect **exit 101** on the
same 89 errors unless child of the fix lands first — its gate standard should judge against that,
not against a timeout.

## Acceptance criteria

- **PASS** — the command returned within the 1800s cap (12s) with exit 101, recorded above with
  wall-clock seconds.
- Not applicable: the WARN branch (exit 124 + partial cache) did not occur — no kill happened and
  the cache is complete.
- **HARD RULES honored** — nothing under `crates/` was modified; the log stays in `/tmp`;
  no socket bound, no server process spawned; no orphans left
  (`pgrep -af 'pdftract mcp|TH_0|TH-0'` clean).
