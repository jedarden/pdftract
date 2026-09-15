# LZW fuzz coverage evidence (ADR-003 compensating control)

**Bead:** pdftract-fc90d8fa · **Date:** 2026-09-15 · **Status:** measured, wired into CI

This note is the evidence record behind the **Fuzzing** bullet in ADR-003
(`docs/adr/0003-lzw-advisory-exception.md`), which retains the unmaintained
`lzw` crate (RUSTSEC-2020-0144) partly on the strength of fuzz coverage over
the `LZWDecode` path. Before this bead, that claim was aspirational: the only
stream-decode fuzz target (`stream_decoder`) reached `LZWDecoder` with
`params = None` (default `/EarlyChange`, predictor 1), and nothing targeted
the `/DecodeParms` space or was seeded with real LZW streams.

## What was added

| Artifact | Purpose |
|---|---|
| `fuzz/fuzz_targets/lzw_decode.rs` | Target for the LZWDecode `/DecodeParms` space: `/EarlyChange` 0 and 1 (both `lzw` decoder variants), `/Predictor` 1 / 2 (TIFF) / 10–15 (PNG) via `apply_predictor`, `/Columns` × `/Colors` × `/BitsPerComponent` including the `MAX_ROW_BYTES` clamp (columns up to 65551 via the high byte). Each input is decoded twice: default 512 MiB document budget (INV-8: no panic) and a 100-byte budget (EC-10: bomb limit holds). Input capped at 6 B header + 16 KiB payload (`MAX_LZW_INPUT`), mirroring CI's `-max_len=10000`. |
| `fuzz/seeds/lzw_decode/` (10 files) | Real LZW streams extracted from tracked fixtures in `tests/fixtures/lzw_*.bin`, prefixed with the target's 6-byte parameter header. Both `/EarlyChange` variants and the TIFF-2 / PNG-10/12/15 predictor paths are represented, plus a truncated stream. |
| `scripts/gen_lzw_fuzz_seeds.py` | Regenerates the seeds from the fixtures (`--check` verifies byte-identity). Header encoding documented in both the script and the target. |
| `fuzz/Cargo.toml` | Registers the `lzw_decode` bin. |
| `.ci/argo-workflows/pdftract-nightly-fuzz.yaml` | Adds `fuzz-lzw-decode` to the DAG, the seed-corpus loop, and the report loop; drops `--features fuzzing` from the `cargo fuzz` invocations (no crate gates on it — the flag made cargo select a nonexistent feature on `pdftract-fuzz`); stale "5 targets" comments corrected to 8. |
| `declarative-config` `k8s/iad-ci/argo-workflows/pdftract-nightly-fuzz.yaml` | Adds `fuzz-lzw-decode` to the deployed DAG (between `fuzz-stream-decoder` and `fuzz-cmap-parser`), seed copy, and crash-collect loop; budget comment updated (6 × 17328 s + 900 s = 104868 s < `activeDeadlineSeconds: 108000`). |

All fuzzing runs under the memory ceiling established by bf-1g1fd / bf-5dnh1:
in-tree manifest = 1536 MB cgroup `MemoryMax` + libFuzzer
`-rss_limit_mb=1024 -malloc_limit_mb=1024 -max_len=10000`; deployed manifest
= libFuzzer `-rss_limit_mb=4096` under pod memory limits capped at the
cluster ceiling (commit `34db4ec5`).

## Measured evidence (2026-09-15, codinghome)

Toolchain: `cargo-fuzz 0.13.2`, `rustc 1.96.0-nightly (cbb9bb8bd 2026-03-13)`,
default address-sanitizer build.

```
$ LD_LIBRARY_PATH=<nix gcc-*-lib> cargo +nightly fuzz run lzw_decode -- \
    -max_total_time=150 -max_len=10000 -rss_limit_mb=1024 \
    -malloc_limit_mb=1024 -print_final_stats=1
# corpus seeded with all 10 fuzz/seeds/lzw_decode/*.bin

Done 1339822 runs in 151 second(s)
stat::number_of_executed_units: 1339822
stat::average_exec_per_sec:     8872
stat::new_units_added:          1273
stat::slowest_unit_time_sec:    0
stat::peak_rss_mb:              461
```

- **1,339,822 executions in 151 s** (8,872 exec/s) — zero crashes, zero
  leaks, zero timeouts, exit 0. No crash artifacts were produced
  (`fuzz/artifacts/lzw_decode/` empty).
- **Coverage grew by 1,273 units** over the seed corpus; the persistent
  corpus grew from 10 seeded files to 372 retained inputs.
- **Peak RSS 461 MB** — under the 1024 MB libFuzzer cap and the 1536 MB
  cgroup ceiling. Neither the 512 MiB document budget nor the 100-byte bomb
  budget leg ever aborted.
- **Seed provenance:** `python3 scripts/gen_lzw_fuzz_seeds.py --check` →
  `OK: all 10 seeds match tests/fixtures + header encoding`. The seed bytes
  are real decoder-accepted LZW streams, so INITED phase exercised both
  `/EarlyChange` variants and the predictor paths before mutation began.

The fixed CI invocation shape was verified locally — `cargo +nightly fuzz
run lzw_decode -s address -- <corpus> -max_total_time=8 -rss_limit_mb=1024
-malloc_limit_mb=1024 -artifact_prefix=…` (`+nightly` stands in for the
nightly-default CI image, which is what `-s address` requires): exit 0,
82,682 runs in 9 s, cov 526 / ft 1667 on the resumed corpus, peak rss
425 MB, zero crash artifacts.

## Adjacent defect found and fixed: deployed nightly job was a silent no-op

The deployed `pdftract-nightly-fuzz` manifest invoked:

```
cargo fuzz run -s none -V 120 -- /workspace/fuzz/corpus/"$TARGET" …
```

With cargo-fuzz 0.13.2 this **prints the version and exits 0 without
fuzzing**: `-V` is the version flag, there is no `TARGET` positional, and
`-s none` additionally overrode the manual `RUSTFLAGS="-Zsanitizer=address"`
export (cargo-fuzz constructs its own RUSTFLAGS). Verified locally:

```
$ cargo fuzz run lzw_decode -V 120 -- -runs=0
cargo-fuzz-run 0.13.2        # exits 0, no fuzzing
```

The step's failures were invisible because the DAG marks fuzz tasks
`continueOn: failed: true`. The manifest now calls
`cargo fuzz run "$TARGET" -s address -- …`, so every target in the deployed
job (including the new `fuzz-lzw-decode`) actually executes. Without this fix
the ADR's "runs nightly" claim would have remained decorative even after
wiring the target in.

## Reproduction

```bash
# seeds: regenerate/verify
python3 scripts/gen_lzw_fuzz_seeds.py --check

# short local run (NixOS: cargo-fuzz binaries need libstdc++ from the nix store)
lib="$(ls -d /nix/store/*gcc-*-lib/lib | sort -V | tail -1)"
mkdir -p fuzz/corpus/lzw_decode && cp fuzz/seeds/lzw_decode/*.bin fuzz/corpus/lzw_decode/
LD_LIBRARY_PATH="$lib" cargo +nightly fuzz run lzw_decode -- \
  -max_total_time=150 -max_len=10000 -rss_limit_mb=1024 -malloc_limit_mb=1024
```

## Limitations

- The local run above is a bounded smoke/coverage run; the sustained budget
  is the nightly CronWorkflow (~4.8 h per target).
- CI runs with `ASAN_OPTIONS=detect_leaks=0` (deployed manifest); leak
  detection is not part of the nightly signal.
- The target exercises `LZWDecoder` + `apply_predictor` directly, not the
  full `decode_stream` dispatch; the latter stays covered by
  `stream_decoder` (default parameters only).
