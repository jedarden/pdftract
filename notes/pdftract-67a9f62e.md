# pdftract-67a9f62e — wasm32 blocker list measured on pdftract-core itself

**Date:** 2026-09-15 · **Host:** codinghome · **Toolchain:** rustc/cargo 1.97.1, `wasm32-unknown-unknown` installed
**Bead:** `pdftract-67a9f62e` (child of umbrella `pdftract-f0d685f4`; follows the scratch-crate bead `pdftract-c3c31c30`)

The scratch-crate note (`notes/pdftract-c3c31c30.md`) measured each candidate
dependency **standalone** and ended on exactly the caveat this bead closes: "a
PASS here does not prove the dep works inside pdftract-core (call sites,
transitive conflicts)". This run measures the crate itself — pdftract-core's own
call sites and its real transitive graph — on the same edge the CI
`wasm32-check` leg enforces. **No pdftract-core source or Cargo.toml file was
edited** (gating is the next bead's job); every number below is a measurement.

**Prediction formed from the scratch note before measuring:** all six
"still-unconditional" deps (memmap2/parking_lot/rayon/tempfile/dirs/chrono)
should compile; zstd (behind `cache`) and nix (behind `remote`) should be absent
from the no-default graph entirely, so neither can block; the open question was
pdftract-core's own call sites, which isolation could not see.

## Method

- Every command is `timeout --kill-after=30s 600s cargo check -p pdftract-core …`
  (plain `cargo check`, no `--all-targets` — dev-dependencies must stay out of
  the library edge). Exit 0 = GREEN, 101 = compile failure. No run timed out.
- **Three measurement surfaces, to separate in-flight edits from what CI sees:**
  1. the shared working tree as dispatched (41 files under `crates/pdftract-core`
     are dirty from concurrent workers);
  2. a clean `git archive HEAD` extraction under `~/scratch` (no `.git`) —
     this is the state CI actually checks out;
  3. the two decisive GREEN results re-run from a **private
     `CARGO_TARGET_DIR`** (hardlink-seeded with `cp -al` of the 86 MB
     `wasm32-unknown-unknown` subtree of `/build/target-workers`), since the
     shared target dir is hot-swapped by other workers.
- All three surfaces agree on every row. Working tree and HEAD produced
  byte-identical failure signatures, so none of the in-flight edits touch this
  edge.
- Dep attribution: `cargo tree -p pdftract-core -e normal --no-default-features
  --features serde,decrypt,quick-xml --target wasm32-unknown-unknown`
  (188 dep edges, saved alongside the logs; quoted in the table below).

## Result matrix — pdftract-core

| # | Configuration | Host | wasm32-unknown-unknown |
|---|---|---|---|
| 1 | `--no-default-features` (bare; bead's baseline command) | **FAIL** exit 101 — 203 errors | **FAIL** exit 101 — 203 errors, byte-identical signature |
| 2 | `--no-default-features --features serde --locked` — **what the shipped CI leg actually runs** (`.ci/argo-workflows/pdftract-ci.yaml:2212-2214`, natively and on wasm32) | **PASS** (6.5 s) | **PASS** (4.9 s; re-confirmed from private target dir) |
| 3 | `--no-default-features --features serde,decrypt,quick-xml` — the **ADR-011-text** wasm feature set | not measured (not a CI edge; host №2 covers the host sanity check) | **PASS** (6.4 s; re-confirmed from private target dir) |
| 4 | `cargo check -p pdftract-wasm --locked --target wasm32-unknown-unknown` — CI leg 3, which pulls core with serde+decrypt+quick-xml | — | **PASS** |

**Final verdict: GREEN on every configuration CI enforces — both the ADR-011
text's set (№3) and the shipped template's `serde`-only set (№2), plus the
`pdftract-wasm` binding crate (№4). The residual blocker list for the wasm32
edge is empty.**

## The one failing configuration, fully named (№1)

Bare `--no-default-features` fails **identically on host and wasm32** — so it is
a feature-hygiene bug, not a wasm32 blocker, and it is not a configuration CI
runs. All 203 errors come from one root cause: pdftract-core's own un-gated
call sites of the *optional* `serde`/`serde_json` deps:

- 22 × `error[E0432]` + 28 × `error[E0433]` — 66 references to `serde_json`,
  34 to `serde`, e.g.:
  `error[E0433]: cannot find module or crate `serde_json` in this scope`
  `   --> crates/pdftract-core/src/output/sink.rs:304:22`
- 153 × `error: cannot find attribute `serde` in this scope`, e.g.:
  `--> crates/pdftract-core/src/confidence.rs:72:3` (`#[serde(rename_all = "lowercase")]`)

Blocking "crate": **pdftract-core itself** (28 files; heaviest:
`schema/mod.rs` 118 error lines, `extract.rs` 23, `output/ndjson/frames.rs` 18,
`parser/pages.rs` 7, `options.rs` 7 — full 28-file list preserved in the run
logs). Iterating per the bead — re-check with the dep's contribution enabled
(№2 = gating on `serde`) — **exhausts the error list**: nothing else fails,
on host or wasm32. ADR-011's *Consequences* section attributes the bare-config
failure to `conformance.rs`→`quick_xml` and `extract.rs`→`crate::encryption`;
at today's HEAD both of those are fixed and the residual un-gated family is
serde/serde_json (history tracked by the ADR's follow-up bead; not re-derived
here). This is exactly the gating work list for the next bead in the chain.

## Per-dependency verdicts — inside pdftract-core's real graph this time

`cargo tree` on the GREEN №3 config (`--target wasm32-unknown-unknown`) proves
each previously-suspected dep is not merely absent-an-obstacle but **present
and compiled** at its pinned version:

| Dependency | Version in the GREEN graph | Verdict |
|---|---|---|
| memmap2 | 0.9.10 | compiles in-graph (call sites construct no mmap source on wasm) |
| rayon | 1.12.0 | compiles in-graph (import is OCR-gated; not in this config anyway) |
| tempfile | 3.27.0 | compiles in-graph |
| dirs | 5.0.1 | compiles in-graph |
| parking_lot | 0.12.5 | compiles in-graph |
| chrono | 0.4.44 | compiles in-graph; `iana-time-zone` correctly target-gated ABSENT |
| dashmap / lru | 6.2.1 / 0.12.5 | compiles in-graph |
| rand → getrandom | 0.8.6 → 0.2.17 (+js) | compiles in-graph via the `cfg(target_arch="wasm32")`-gated `getrandom/js` dep (`Cargo.toml:69-70`) — the scratch note's one in-graph-vs-standalone divergence, now confirmed from inside the real graph |
| wasm-bindgen / js-sys | 0.2.122 / 0.3.99 | compile in-graph (getrandom's js backend) |
| zstd / zstd-sys | ABSENT | correctly excluded by `--no-default-features` (`cache` off) — the bf-2uw30r hard blocker cannot re-enter |
| nix, ureq, rustls | ABSENT | correctly excluded (`remote` off) — the hollow-crate call-site trap cannot fire |

Consistency with `pdftract-c3c31c30` (standalone) and `pdftract-4df4aefb`
(in-graph probe, 2026-09-14): **no contradictions**. This run adds the missing
dimension the scratch note named: core's own call sites compile on wasm32 once
`serde` is on.

## Divergence found en route: ADR-011 text vs shipped CI template

ADR-011 (plan.md:581-597) specifies the CI leg as
`--no-default-features --features serde,decrypt,quick-xml`. The shipped
template (updated by `pdftract-47d894cf`, which post-dates the ADR text) runs
`--features serde` only, on core, both targets, plus the full set via
`pdftract-wasm`. **Both formulations were measured here and both are GREEN**, so
the divergence is benign today; the ADR text is the stale one.

## Caveats

- Host rustc 1.97.1 vs the CI image `rust:1.83-bookworm` (same environment
  caveat as both predecessor beads).
- `cargo check` measures type-checking + build scripts, **not linking** — same
  bar as the CI leg and the scratch note.
- Shared-infrastructure controls: decisive GREENs re-verified from a private
  `CARGO_TARGET_DIR`; HEAD extraction isolates the result from concurrent
  workers' in-flight edits; `--locked` passed (Cargo.lock is committed and
  unmodified).
- Only the bare `--no-default-features` config was measured without `--locked`
  (the shipped template passes `--locked` on №2/№4 only); lockfile is identical
  either way.

## Reproduction

```bash
cd <checkout>                                   # or a `git archive HEAD` extraction
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features --target wasm32-unknown-unknown
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features --features serde --locked
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features --features serde --locked --target wasm32-unknown-unknown
timeout --kill-after=30s 600s cargo check -p pdftract-core --no-default-features --features serde,decrypt,quick-xml --target wasm32-unknown-unknown
timeout --kill-after=30s 600s cargo check -p pdftract-wasm --locked --target wasm32-unknown-unknown
cargo tree -p pdftract-core -e normal --no-default-features --features serde,decrypt,quick-xml --target wasm32-unknown-unknown
```

Raw logs (per-attempt stdout/stderr, the 28-file error-site list, the 188-edge
cargo tree) lived in `~/scratch/wasm32-core-67a9f62e/` and were removed after
this note was written, per ~/CLAUDE.md File Organization (creator owns removal).
No orphan processes were left (`pgrep -af 'pdftract[ ]mcp|TH[_]0|TH[-]0|cargo[ ]check'` → none).
