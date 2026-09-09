# pdftract-e6120947 — Build profile_yaml fuzz binary with libstdc++ toolchain environment

Split child 1/4 of umbrella bf-538xl2. Verified 2026-09-09.

## The recipe (this is the deliverable)

```bash
cd /home/coding/pdftract/fuzz
export PATH="$HOME/.cargo/bin:$PATH"     # real cargo; see "PATH note" below
export LD_LIBRARY_PATH=/nix/store/0p8b2lqk47fvxm9hc6c8mnln5l8x51q1-gcc-14.3.0-lib/lib

timeout --kill-after=30s 900s cargo fuzz build profile_yaml
# then run directly (child 3's 30-second run):
LD_LIBRARY_PATH=/nix/store/0p8b2lqk47fvxm9hc6c8mnln5l8x51q1-gcc-14.3.0-lib/lib \
  ./target/x86_64-unknown-linux-gnu/release/profile_yaml -max_total_time=30 fuzz/corpus/profile_yaml
```

- **Env var is the fix:** `LD_LIBRARY_PATH=/nix/store/0p8b2lqk47fvxm9hc6c8mnln5l8x51q1-gcc-14.3.0-lib/lib`
  (exact store path verified present on this box). Without it the binary exits **127**
  (`libstdc++.so.6: cannot open shared object file`). The sanitizer-built binary links
  libstdc++ but the default loader path only covers libgcc_s.
- **Binary path:** `fuzz/target/x86_64-unknown-linux-gnu/release/profile_yaml`
  (62,278,752 bytes after this build, 2026-09-09 09:03:30 -0400).
- **PATH note:** `~/.local/bin/cargo` is a wrapper that runs cargo under `systemd-run`,
  which discards cargo-fuzz's captured build output (and can remote-submit). Putting
  `~/.cargo/bin` first on PATH + an explicit redirect captures the full ~45 KB build log.
  This is a capture convenience — the binary itself works either way; only the env var is
  load-bearing for the exit-127 fix.

## Acceptance criteria

| AC | Result | Evidence |
|----|--------|----------|
| `profile_yaml` exists; `-help=1` exits 0 with `LD_LIBRARY_PATH` set | **PASS** | `cargo fuzz build profile_yaml` exit 0 ("Finished `release` profile [optimized + debuginfo] target(s) in 1m 31s"); `-help=1` → exit 0, printed libFuzzer usage |
| Exit-127 behavior WITHOUT the env var reproduced and documented once | **PASS** | Reproduced twice — on the pre-existing binary and again on the freshly built one: exit 127, stderr `error while loading shared libraries: libstdc++.so.6: cannot open shared object file: No such file or directory` |
| Recipe recorded in the verification note | **PASS** | This file, "The recipe" section above |
| No orphaned cargo/rustc/profile_yaml processes | **PASS** | `pgrep -af 'cargo\|rustc\|profile_yaml' \| grep -v bin/bash` → empty (exit 1) |

## Extra verification beyond the ACs

The fuzzer loop actually executes, not just the help text. With `-runs=1` on an empty
corpus dir: `INFO: Running with entropic power schedule (0xFF, 100)` … `Done 2 runs in
0 second(s)`, `INITED cov: 269 ft: 270`, exit 0.

## Notes for child 3 (the 30-second run)

- The build phase is ~1.5–3 min and mostly silent — not a hang. Incremental rebuilds
  after this build are faster, but build.rs generated-code fingerprint churn can still
  recompile; budget generously and wrap in `timeout --kill-after=30s 900s`.
- To capture build logs, `export PATH="$HOME/.cargo/bin:$PATH"` first, or invoke the
  binary directly as shown in the recipe.
- `pdftract-core`'s `profiles` feature compiles cleanly (it is pdftract-*cli*'s
  `--features profiles` that is broken at HEAD — irrelevant to this fuzz target, which
  depends on core only).
