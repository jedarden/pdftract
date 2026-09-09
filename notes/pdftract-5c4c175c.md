# pdftract-5c4c175c — 10-second profile_yaml fuzz baseline corpus + coverage snapshot

Split child 2/4 of umbrella bf-538xl2. Run executed 2026-09-09 (~10:10 -0400) on the lab box.

This note is the baseline that the umbrella's "Corpus size increases from 10-second
baseline" acceptance criterion is measured against. Children 3/4 cite the numbers below.

## Baseline (cite these numbers)

| Metric | Value |
|---|---|
| Corpus files BEFORE | **4914** (`ls fuzz/corpus/profile_yaml \| wc -l`) |
| Corpus files AFTER | **5110** (+196 new inputs written by this run) |
| Final coverage (cov) | **3756** |
| Final features (ft) | **19085** |
| libFuzzer corp counter at DONE | 3255/671Kb (pruned in-memory set — see note below) |
| Total runs | 116,681 in 11 wall-clock second(s) |
| Exit code | **0** |
| New crash artifacts | **0** (29 files before → 29 after; name+mtime listing byte-identical) |
| Log | `/home/coding/pdftract/logs/fuzz/profile_yaml-baseline-10s.log` (1275 lines; `logs/fuzz/` is gitignored by design, `.gitignore:37`) |

Stats lines quoted verbatim from the log:

```
#4915	INITED cov: 3754 ft: 19018 corp: 3233/658Kb exec/s: 4915 rss: 329Mb
...
#115719	NEW    cov: 3756 ft: 19085 corp: 3255/671Kb lim: 4096 exec/s: 11571 rss: 366Mb L: 55/4091 MS: 1 CrossOver-
#116681	DONE   cov: 3756 ft: 19085 corp: 3255/671Kb lim: 4096 exec/s: 10607 rss: 366Mb
Done 116681 runs in 11 second(s)
```

## Exact command

Recipe from pdftract-e6120947 (the `LD_LIBRARY_PATH` var is the load-bearing part; the
PATH override just keeps cargo-fuzz's captured output out of the systemd-run wrapper):

```bash
cd /home/coding/pdftract/fuzz
export PATH="$HOME/.cargo/bin:$PATH"
export LD_LIBRARY_PATH=/nix/store/0p8b2lqk47fvxm9hc6c8mnln5l8x51q1-gcc-14.3.0-lib/lib

timeout --kill-after=30s 900s cargo fuzz run profile_yaml -- -max_total_time=10 \
  > ../logs/fuzz/profile_yaml-baseline-10s.log 2>&1
echo "EXIT_CODE=$?"     # → 0
```

`-max_total_time=10` was an integer, as required — libFuzzer truncates fractional values
to 0 (= unlimited) and such a run never returns.

## Notes for children 3/4

- **The baseline corpus size is the directory file count, now 5110.** libFuzzer's own
  `corp:` counter (3255 at DONE) is the *pruned in-memory* feature-maximal set, not the
  number of files on disk — it read all 4914 seeds (`INITED #4915`), kept 3233 of them in
  its working set, and wrote 196 new inputs back into `fuzz/corpus/profile_yaml/`. Measure
  the umbrella AC with `ls fuzz/corpus/profile_yaml | wc -l`, not with the `corp:` field.
- The run rebuilt `pdftract-core` first (the worktree had a newer edit to
  `font/type3_rasterizer.rs` than the pre-existing binary), so the coverage numbers reflect
  the worktree source as of this run, not the 09:03 stale binary. Build warnings are in the
  log head; the fuzz loop output follows them.
- No sanitizer banners (`==pid==`), `SUMMARY:`, `ERROR:`, or panic lines in the log —
  INV-8 held across 116,681 executions of `load_profile_yaml`.
- `fuzz/artifacts/profile_yaml/` held 29 pre-existing artifacts from earlier runs
  (2026-09-06/08); this run added none. A before/after name+mtime listing is preserved at
  `/home/coding/scratch/pdftract-5c4c175c/artifacts-{before,after}.txt`.
- A 10-second run costs ~11 s of fuzzing plus an incremental build check (fast when the
  binary is fresh, ~1.5–3 min after a `pdftract-core` source edit).

## Acceptance criteria

| AC | Result | Evidence |
|----|--------|----------|
| 10-second run exits 0, exit code captured | **PASS** | `EXIT_CODE=0` under `timeout --kill-after=30s 900s`; `Done 116681 runs in 11 second(s)` |
| Baseline numbers recorded (before/after corpus count, final cov + ft, stats line verbatim) | **PASS** | Table + verbatim block above |
| Zero crash artifacts created by this run | **PASS** | artifacts dir 29 → 29; `diff` of before/after listings empty |
| Baseline recorded in `notes/<this-bead>.md` for children 3/4 | **PASS** | This file |

## Extra verification

- Log contains the libFuzzer banner and 1275 lines of run output (redirected, so the
  sandbox-swallowed-stdout problem does not apply).
- No orphaned processes left behind; the run was bounded by `timeout` and completed on its
  own (`-max_total_time=10` is a clean stop, not a kill).
