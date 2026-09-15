# pdftract-c3c31c30 — wasm32 dependency blockers measured in isolation (scratch crate)

**Date:** 2026-09-15 · **Host:** codinghome · **Toolchain:** rustc/cargo 1.97.1, `wasm32-unknown-unknown` installed
**Bead:** `pdftract-c3c31c30` (child of umbrella `pdftract-f0d685f4`)

Every row of the "Expected Additional Blockers (Not Yet Tested)" table in
`docs/notes/bf-2uw30r.md` was tested empirically, one dependency at a time, in a
throwaway scratch crate at `~/scratch/wasm32-blocker-check-c3c31c30` (removed after
this note was written; errors are quoted inline). Unlike the two earlier
verifications recorded in that note (2026-09-14 per-crate probing inside
pdftract's graph, and the 2026-09-15 per-feature matrix at HEAD), this run
measured each candidate **standalone**: no pdftract-core feature unification, no
`cfg(target_arch = "wasm32")` gates, no pdftract-wasm graph — only the candidate,
its own dependency tree, and the target.

## Method

- Scratch crate with the eight candidates as optional deps behind per-dep
  features, each pinned with `=` to pdftract's `Cargo.lock` version
  (`zstd-sys` additionally force-pinned to `=2.0.16`, see caveat below).
- One run per feature:
  `timeout --kill-after=30s 300s cargo check --target wasm32-unknown-unknown --features <dep>`
  with the crate's own `CARGO_TARGET_DIR` (the global `/build/target-workers`
  override would otherwise pollute results).
- Control run first: the empty crate (no features) checks **PASS** for the
  target, so every FAIL below is attributable to the dependency, not the
  harness. Exit 0 = PASS, 101 = compile/build-script failure.
- A minimal `probe_*` fn per feature references the dep in item position, so
  the enabled dep is genuinely part of the compilation (the measurement is the
  dep's own tree compiling, not the probe).

## Verified table — the eight candidates

| # | Dependency | Pinned version | Configuration tested | wasm32-unknown-unknown `cargo check` | Evidence / actual error |
|---|---|---|---|---|---|
| 1 | memmap2 | 0.9.10 | default | **PASS** | `Checking memmap2 v0.9.10` — compiles with *no* libc in its wasm graph at all |
| 2 | rayon | 1.12.0 | default | **PASS** | rayon-core 1.13.0 + full crossbeam stack (utils/epoch/deque) all compile |
| 3 | tempfile | 3.27.0 | default | **PASS** | fastrand 2.5.0 + once_cell compile; tempfile's `getrandom 0.4.3` lock edge is target-gated out on wasm32 and was never compiled |
| 4 | dirs | 5.0.1 | default | **PASS** | dirs-sys 0.4.1 + option-ext compile |
| 5 | nix | 0.29.0 | `features = ["fs"]` (matches pdftract-core's `remote` decl) | **FAIL — call-site** | the crate itself **compiles** for wasm32; every unix module is `#[cfg(unix)]`-gated out of existence, so any real use fails: `error[E0433]: cannot find `unistd` in `nix`` |
| 6 | zstd (zstd-sys) | 0.13.3 / **2.0.16** | default | **FAIL — build script** | `error: failed to run custom build command for `zstd-sys v2.0.16+zstd.1.5.7`` → `error occurred in cc-rs: failed to find tool "clang": No such file or directory (os error 2)` — byte-identical to the original `bf-2uw30r` spike error. (No clang/clang-18/zig exists on this host.) |
| 7 | parking_lot | 0.12.5 | default | **PASS** | parking_lot_core 0.9.12, lock_api, scopeguard, smallvec compile |
| 8 | chrono | 0.4.44 | default features (= pdftract's plain `chrono = "0.4"`) | **PASS** | pulls wasm-bindgen 0.2.128 + js-sys 0.3.105 via the default `wasmbind` feature; all check green |

## Feature-flip rows (guidance item 3)

| Configuration | Result | Nuance |
|---|---|---|
| chrono 0.4.44, `default-features = false` + `std` | **PASS** | pure date math, no clock, no JS glue (0.4 s) |
| chrono 0.4.44, `clock` + `std`, **no** `wasmbind` | **PASS** | `iana-time-zone` never even enters the wasm compile — it is target-gated out — so `wasmbind` is **not load-bearing** for check-level compilation on wasm32-unknown-unknown |
| rand 0.8.6, plain | **FAIL** | `getrandom` 0.2.17: `compile_error!: the wasm*-unknown-unknown targets are not supported by default, you may need to enable the "js" feature` (getrandom src/lib.rs:346) |
| rand 0.8.6 + `getrandom` `js` feature | **PASS** | js-sys 0.3.105, rand_core, rand_chacha, ppv-lite86 all check green |

The rand pair is the load-bearing demonstration of why isolation matters:
rand **passes inside pdftract-core's graph only because** pdftract-core declares a
`cfg(target_arch = "wasm32")`-gated `getrandom = { features = ["js"] }` dependency
(`crates/pdftract-core/Cargo.toml:69-70`). Measured standalone, the same
dependency fails. Neither `rand` nor `getrandom` was in bf-2uw30r's candidate
list, but the flip was explicitly requested and it is the one case where
"in-graph PASS" and "standalone PASS" genuinely diverge.

## Divergences from the bf-2uw30r hypothesis table (acceptance criterion 2)

Called out explicitly, hypothesis → measured:

| bf-2uw30r said | Measured here | Verdict |
|---|---|---|
| memmap2 — high-confidence blocker | **PASS** standalone | hypothesis wrong (consistent with the 2026-09-14 correction) |
| rayon — high-confidence blocker | **PASS** standalone | hypothesis wrong (consistent with the 2026-09-14 correction) |
| tempfile — high-confidence blocker | **PASS** standalone | hypothesis wrong (consistent with the 2026-09-14 correction) |
| dirs — high-confidence blocker | **PASS** standalone | hypothesis wrong (consistent with the 2026-09-14 correction) |
| nix — high-confidence blocker | **FAIL**, but by a *different mechanism*: the crate compiles; its unix API surface is `#[cfg(unix)]`-absent, so failures are E0433 at call sites | blocker confirmed, mechanism corrected |
| zstd — confirmed blocker | **FAIL**, byte-identical cc-rs/clang error at pdftract's exact `zstd-sys` 2.0.16 pin | confirmed (re-verified by one method with all other rows) |
| parking_lot — moderate concern | **PASS** | concern retired |
| chrono — moderate concern | **PASS** in all three configurations | concern retired |

Consistency with the two prior experimental passes (2026-09-14
`pdftract-4df4aefb`, 2026-09-15 `pdftract-f0d685f4`): **no contradictions**.
This run adds the missing measurement dimension — per-crate isolation — and
retires the remaining "it only ever passed inside a graph that already carried
pdftract's wasm fixes" caveat for the six passing rows. ADR-011's claims about
`rayon` (compiles; only import is OCR-gated) and `memmap2` (compiles; mmap
sources simply unused on wasm) now rest on isolated measurements as well.

## New facts this run contributes

1. **nix's wasm32 failure mode is "hollow crate", not compile error.** nix
   0.29.0 (with `fs`) type-checks for wasm32-unknown-unknown; what breaks is
   any use of it. For gating purposes this is *worse* than a build failure for
   detection (nothing goes red until a call site exists) and it means "does the
   dep compile?" and "can the dep be used?" are genuinely different questions.
   Keeping `nix` behind the `remote` feature remains correct.
2. **chrono's `wasmbind` default is not what makes it check-green on wasm32** —
   `clock` without `wasmbind` checks green too, because `iana-time-zone` is
   target-gated out of the wasm graph entirely. The JS glue only matters for
   actually *running* `Local::now()` in a JS host.
3. **zstd-sys 2.1.0 (current upstream) ships a `wasm-shim/`** (stdlib.h/string.h
   rerun-if-changed entries visible in its build-script output) — upstream has
   a deliberate wasm32 C-compilation route. It still fails identically without
   a clang that targets wasm, so the verdict is unchanged on this host and in
   CI, but a future zstd-sys bump + toolchain install is a plausible path if
   compressed-cache support in wasm ever becomes desirable. Measured at both
   2.0.16 (pdftract's pin) and 2.1.0 — same failure.
4. **tempfile's getrandom edge is 0.4.3 and never compiles on wasm32** — its
   wasm fallback avoids getrandom entirely. (Not directly tested as a row:
   the crate was never in the wasm compile, so this is an observation about
   the dependency edge, not a getrandom-0.4 verdict.)

## Caveats

- `cargo check` measures type-checking + build-script execution, **not
  linking**. Graphs containing wasm-bindgen/js-sys (chrono default, rand+js)
  check green but still require wasm-bindgen post-processing to produce a
  loadable module — the same bar the CI `wasm32-check` leg uses, so the method
  matches the established decision record.
- Transitive-drift vs pdftract's `Cargo.lock` (direct candidates were `=`-pinned;
  transitives resolved fresh): libc 0.2.189 (pdftract: 0.2.183), fastrand 2.5.0
  (2.4.1), wasm-bindgen 0.2.128 (0.2.122), zstd-sys initially resolved 2.1.0 and
  was re-pinned to `=2.0.16` and re-run — both pins FAIL identically.
- rustc 1.97.1 host vs CI's rust:1.83 image (same environment caveat as the
  umbrella's HEAD verification).
- Direction of inference (per the bead's critical considerations): a FAIL here
  is a hard blocker for the dep; a PASS here does not prove the dep works
  inside pdftract-core (call sites, transitive conflicts) — and nix is the
  existence proof that even "compiles" ≠ "usable". Measuring the crates inside
  pdftract-core is the next bead in this chain.

## Reproduction

```bash
# throwaway crate: each candidate optional behind a per-dep feature, =pinned
# to pdftract's Cargo.lock versions; chrono flips via package renames
# (chrono_min = no-default+std, chrono_clock = clock+std w/o wasmbind);
# rand-js = rand + getrandom{features=["js"]}
cd <scratch-crate>
export CARGO_TARGET_DIR="$PWD/target"          # beat the global /build/target-workers override
cargo check --target wasm32-unknown-unknown                     # control: PASS
for f in memmap2 rayon tempfile dirs nix zstd parking_lot chrono \
         chrono-min chrono-clock rand rand-js; do
  timeout --kill-after=30s 300s \
    cargo check --target wasm32-unknown-unknown --features "$f"
done
```

All 13 runs completed (no timeouts, exit 124 nowhere); no orphan processes left
behind. The scratch crate and its logs were deleted after this note was
written, per ~/CLAUDE.md File Organization (creator owns removal).
