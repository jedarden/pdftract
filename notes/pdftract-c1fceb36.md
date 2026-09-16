# pdftract-c1fceb36 — wasm32-blocking dependencies gated behind capability features

**Date:** 2026-09-16 (work started 2026-09-15) · **Host:** codinghome · **Toolchain:** rustc/cargo 1.97.1, `wasm32-unknown-unknown` installed
**Bead:** `pdftract-c1fceb36` (child of umbrella `pdftract-f0d685f4`; follows `pdftract-c3c31c30` (scratch-crate table) and `pdftract-67a9f62e` (core measurement); feeds `pdftract-a7c93aa5` (ADR reconciliation))

## What the measurements implied (bead guidance #1)

Both predecessor beads measured GREEN on wasm32-unknown-unknown, standalone and
in-graph, for **all six** deps this bead's description suspected (memmap2,
parking_lot, rayon, tempfile, dirs, chrono). Per the bead's own guidance —
*"If a dep measured GREEN on wasm32 in both, do NOT gate it"* — **none of the
six were gated.** `zstd` was already gated behind `cache` (commit `b565b6e4`)
and `nix` behind `remote`; both were left as-is.

The one configuration the predecessors measured RED — the bead's strict
baseline, `cargo check -p pdftract-core --no-default-features --target
wasm32-unknown-unknown` — failed **identically on host**, with every error
coming from pdftract-core's own un-gated call sites of the *already-optional*
`serde`/`serde_json` deps (the `serde` capability feature has existed in
`crates/pdftract-core/Cargo.toml` all along; only the call-site gating was
missing). That call-site gating is therefore the work this bead landed.

## What was landed, and where it came from

The gating itself was found **stranded, uncommitted, in the shared working
tree** — left by this bead's two timed-out prior attempts. At dispatch, the
tree carried 285 `cfg(feature = "serde")`-family sites vs **0** at HEAD. The
stranded work was reviewed (idiomatic `#[cfg_attr(feature = "serde", …)]` +
gated `use` lines; compiler-proven equivalent under default features) and
landed in three commits, each file carrying a one-line provenance comment so
every staged blob differs from the predispatch snapshot (commit-hook rule):

| Commit | Content |
|---|---|
| `166b401f` | 24 core files: every serde/serde_json touchpoint gated on the existing `serde` capability feature (attrs via `#[cfg_attr]`); default feature set unchanged |
| `3aa2d58a` | Companion API surface the gated files call: `content_stream.rs` (6-arg `process_with_mode`, `Glyph::is_hidden`), `parser/resources.rs` (`ResourceDict::warm_indirect_properties`), `parser/xref.rs` (`XrefResolver::from_section_with_source`), `pdftract-cli/src/grep/worker.rs` (updated call sites) — plus an `AssertUnwindSafe` fix in five `document.rs` tests (below) |
| `b1a08bbf` | Restored full `extract.rs` call sites after a concurrent repair had minimally reverted them (crossing with `pdftract-d1c76b2a`, below) |

**Latent bug fixed en route (`3aa2d58a`):** five `document.rs` tests called
`std::panic::catch_unwind` on closures capturing `&XrefResolver`, which holds a
non-`RefUnwindSafe` `dyn PdfSource` — so `cargo check --all-targets` failed
with E0277 **on the shared tree as well as at HEAD** (verified on the tree
before fixing). Wrapped in `std::panic::AssertUnwindSafe`, matching the
established pattern already used three times in `extract.rs`. Runtime behavior
of the tests is unchanged.

**Concurrent-worker crossing (disclosed):** between my first and second
commits, another worker landed `cebe1e1f` (bead `pdftract-d1c76b2a`) — a
minimal repair of the HEAD breakage my first partial landing had caused —
reverting three `extract.rs` call sites to definitions that no longer existed
once my `3aa2d58a` landed, which re-broke HEAD (E0061, 5-arg call vs 6-arg
definition). `b1a08bbf` restored the tree's full `extract.rs`, converging HEAD
with the shared tree. Their bead posted an addendum (`0769ad58`) confirming
the default_rust gate green at the converged HEAD `b1a08bbf`. No history was
rewritten; every step is a normal forward commit.

## Verification matrix

Every command `timeout --kill-after=30s 600s`-wrapped (tests: 2400s). "Tree" =
shared working tree as dispatched; "final HEAD" = fresh `git archive` extraction
of `b1a08bbf` (what CI and the default_rust gate check out).

| # | Edge | Tree | Final HEAD |
|---|---|---|---|
| E1 | `-p pdftract-core --no-default-features` (host) | exit 0 | **exit 0** |
| E2 | E1 `--target wasm32-unknown-unknown` — **the strict baseline** | exit 0 | **exit 0** |
| E3 | `--no-default-features --features serde --locked --target wasm32-unknown-unknown` (shipped CI leg) | exit 0 | **exit 0** |
| E4 | `--no-default-features --features serde,decrypt,quick-xml --locked --target wasm32-unknown-unknown` (ADR-011 text set) | exit 0 | **exit 0** |
| E5 | `cargo check -p pdftract-wasm --locked --target wasm32-unknown-unknown` (CI leg 3) | exit 0 | **exit 0** |
| E6 | `cargo check -p pdftract-core` (native defaults) | exit 0 | **exit 0** |
| E7 | workspace gate replica `cargo check --all-targets --quiet` | exit 0 (after `3aa2d58a` fix; failed E0277 before, on tree too) | **exit 0** |

**Before/after for the strict baseline (E2), same method, real output:**

- Pre-landing HEAD (`git archive` of `2b83ad6c`-lineage before my commits):
  **exit 101**, 204 error lines — signature: 153 × `cannot find attribute
  'serde'`, 28 × `cannot find module or crate 'serde_json'`, 22 × E0432 —
  matching the predecessor note's characterization exactly.
- Final HEAD: **exit 0**, `Finished 'dev' profile … target(s)`.

## Native tests — no new failures vs baseline

`cargo test -p pdftract-core --all-targets --no-fail-fast`, identical
procedure on twin extractions (pre-landing HEAD vs final HEAD), private
`CARGO_TARGET_DIR` each:

| Run | Result lines | FAILED |
|---|---|---|
| before (pre-landing HEAD) | 89 binaries (77 integration) | 211 |
| after (final HEAD) | 89 binaries (77 integration) | 210 |

- **New failures in after: 0** (order-insensitive set diff of FAILED lines).
- **Retired: 1** — `output::json::tests::test_convert_diagnostics` now passes
  (side effect of the landed stranded diagnostics work, not of the gating).
- Remaining failures are the documented pre-existing families (parser::xref,
  render::scanline, font::type3_rasterizer, TH-03 password/ipv6, …).

## Acceptance criteria

- **PASS** — wasm32 strict-baseline check GREEN at committed HEAD (E2), with
  the before-state's full failure output preserved above; residual blocker
  list: **empty**.
- **PASS** — native default features build (E6/E7) and test with **no new
  failures** vs the pre-landing run (0 new / 1 retired).
- **PASS** — zero `#[cfg(target_arch = "wasm32")]` introduced:
  `git show 166b401f -- crates/pdftract-core/src | grep -c target_arch` → **0**
  (the single hit in `git show 166b401f` is commit-message prose; the only
  pre-existing refs at HEAD are the `Cargo.toml` `getrandom/js` manifest block
  — ADR-011's own sanctioned exception — and two pre-existing `x86_64` test
  cfgs in `ocr.rs`/`preprocess.rs`).

## CI contract

The CI leg's enforced feature contract did not change (`--features serde` and
the `pdftract-wasm` full set both stay green), so per guidance #5 no
`wasm32-check` CI leg or `pdftract-wasm` edits were made. The bare
`--no-default-features` edge is now additionally green; if the reconciliation
bead (`pdftract-a7c93aa5`) wants it enforced, adding one line to the CI leg is
a natural follow-up, deliberately not taken here.

## Reproduction

```bash
cd <checkout>
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features --target wasm32-unknown-unknown
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features --features serde --locked --target wasm32-unknown-unknown
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features --features serde,decrypt,quick-xml --locked --target wasm32-unknown-unknown
timeout --kill-after=30s 600s cargo check -p pdftract-wasm --locked --target wasm32-unknown-unknown
timeout --kill-after=30s 600s cargo check -p pdftract-core
git archive HEAD | tar -x -C <dir> && cargo check --manifest-path <dir>/Cargo.toml --all-targets --quiet
grep -rn 'target_arch' crates/pdftract-core/src   # no wasm32 cfg in code
```

## Caveats

- Host rustc 1.97.1 vs the CI image `rust:1.83-bookworm` (same environment
  caveat as both predecessor beads).
- `cargo check` measures type-checking + build scripts, not linking — same bar
  as the CI leg and both predecessor notes.
- 35 files of unrelated stranded polish remain uncommitted in the shared tree
  (cli, core tests, cache/lru, decoder/jbig2, …); they are not demanded by any
  committed call site (workspace replica green without them) and are out of
  this bead's scope.
- Commit `0769ad58` (docs-only) landed on HEAD after the final-HEAD extraction
  began; it touches documentation only and cannot affect these results.
