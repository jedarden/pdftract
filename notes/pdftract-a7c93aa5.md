# pdftract-a7c93aa5 — ADR-010/ADR-011 reconciled with the verified wasm32 evidence

**Date:** 2026-09-15 · **Bead:** `pdftract-a7c93aa5` (child of umbrella `pdftract-f0d685f4`; closes the chain after `pdftract-c3c31c30` → `pdftract-67a9f62e` → `pdftract-c1fceb36`)

**Deliverable:** plan.md revision **1.5** — a Verification amendment appended to ADR-011
(docs/plan/plan.md, after its Invalidation trigger), a Status-pointer update on both
ADR-010 and ADR-011, and a new revision-history row. ADR-011's decision is **Confirmed
(Go)**; no superseding ADR was warranted because no finding is feature-gate-resistant.

## Claim-by-claim reconciliation — ADR-011 (plan.md:581-597 as written 2026-09-14)

| # | ADR-011 claim | Verdict | Evidence |
|---|---|---|---|
| 1 | Status: spike "fails outright on `zstd-sys`" | **CONFIRMED** — cc-rs finds no wasm32 C toolchain; byte-identical error re-measured standalone and at HEAD | `notes/pdftract-c3c31c30.md` row 6; `docs/notes/bf-2uw30r.md` HEAD matrix |
| 2 | Status/ADR-010 trigger "names `memmap2` explicitly" | **NUANCED** — the trigger fired on a right fact (zstd) for a wrong rationale: memmap2 compiles standalone and in-graph | `notes/pdftract-c3c31c30.md` row 1; `notes/pdftract-67a9f62e.md` per-dep table |
| 3 | D1: default-on `cache` feature gating zstd; native unchanged | **CONFIRMED** — `default = ["serde","decrypt","quick-xml","cache"]`, `cache = ["dep:zstd"]`; zstd ABSENT from the no-default wasm graph | `crates/pdftract-core/Cargo.toml` (read this bead); `notes/pdftract-67a9f62e.md` |
| 4 | D1: no `#[cfg(target_arch = "wasm32")]` in core | **CONFIRMED** — 0 introduced; only the sanctioned Cargo.toml `getrandom/js` target-dep block + two pre-existing x86_64 test cfgs | `notes/pdftract-c1fceb36.md` AC §, `grep target_arch` |
| 5 | D2: in-memory input; "mmap sources compile for wasm32, simply never constructed there" | **CONFIRMED** — measured both standalone and inside core's real graph | `notes/pdftract-c3c31c30.md` row 1; `notes/pdftract-67a9f62e.md` |
| 6 | D3: rayon's only import is OCR-gated; vector path serial | **CONFIRMED** | same two notes (rayon rows) |
| 7 | D4: `pdftract-wasm` scaffold, `default-features=false, features=[serde,decrypt,quick-xml]`, publish=false | **CONFIRMED** — verified in tree this bead; CI leg E5 green | `crates/pdftract-wasm/Cargo.toml` + `src/lib.rs`; re-run below |
| 8 | D5: CI leg runs `--features serde,decrypt,quick-xml` **on core** | **DIVERGED (benign)** — shipped leg runs `--features serde` on core (both targets) + full set via `-p pdftract-wasm`; both formulations measured GREEN; shipped template authoritative | `notes/pdftract-67a9f62e.md` §divergence; `.ci/argo-workflows/pdftract-ci.yaml:2211-2219` (read this bead) |
| 9 | D6: bindings + demo deferred until native extraction green | **CONFIRMED** — scaffold still exports only `pdftract_version()` | `crates/pdftract-wasm/src/lib.rs` (read this bead) |
| 10 | Context: spike's blocker table "factually wrong once tested"; the 8 named deps compile | **CONFIRMED + mechanism added** — nix fails by hollow-crate (E0433 at call sites), chrono green even without `wasmbind` | `notes/pdftract-c3c31c30.md` divergence table + new-facts § |
| 11 | Context: `rand`→`getrandom` fixed via target-gated `js` dep | **CONFIRMED + sharpened** — standalone rand FAILS without js; the only in-graph/standalone divergence | `notes/pdftract-c3c31c30.md` feature-flip rows |
| 12 | Alternatives: "the only hard blocker is one self-contained module" | **CORRECTED** — two C-backed blockers (zstd-sys behind `cache`, ring behind `remote`) + nix hollow-crate; all outside the wasm set, so Go unaffected | `docs/notes/bf-2uw30r.md` HEAD matrix; `notes/pdftract-c3c31c30.md` row 5 |
| 13 | Consequences: bare `--no-default-features` fails via quick_xml/encryption un-gated refs | **SUPERSEDED** — at HEAD the residual family was serde/serde_json call sites; `pdftract-c1fceb36` gated them → bare config now GREEN host+wasm32, 0 new native test failures | `notes/pdftract-67a9f62e.md` §one-failing-config; `notes/pdftract-c1fceb36.md` E2 + tests § |
| 14 | Consequences: declarative-config mirror sync pending | **DONE since** — mirror carries the identical leg | `declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml` (grepped this bead: 4 wasm32-check hits, same commands) |
| 15 | Consequences: tier4 YAML indentation defect tracked separately | **FIXED since** — in-tree template parses cleanly | `docs/notes/bf-2uw30r.md` HEAD re-verification § |
| 16 | Invalidation trigger (leg goes red in a feature-gate-proof way) | **NOT FIRED** — leg green; strict bare baseline additionally green | re-verified this bead, below |

## Tree facts verified this bead (guidance #5 — record what IS)

- `crates/pdftract-core/Cargo.toml`: features `default=[serde,decrypt,quick-xml,cache]`,
  `cache=[dep:zstd]`, `remote=[dep:url,dep:ureq,dep:nix]`, `ocr`, `full-render`,
  `profiles=[dep:serde_yaml]`, `receipts`/`shape-db`/`cjk`/`proptest`/`fuzzing` (cfg-only);
  `[target.'cfg(target_arch="wasm32")'.dependencies] getrandom = { features = ["js"] }`.
- `crates/pdftract-wasm`: workspace member (`Cargo.toml` members list), publish=false,
  cdylib+rlib, pulls core with serde+decrypt+quick-xml, exports `pdftract_version()` only.
- CI: `wasm32-check` leg at `.ci/argo-workflows/pdftract-ci.yaml:2187-2221`
  (rust:1.83-bookworm) runs `--features serde` on core natively + wasm32, then
  `-p pdftract-wasm --locked --target wasm32-unknown-unknown`; the declarative-config
  mirror carries the identical commands.
- Live re-verification at HEAD `cdd6efd4`, shared tree, `--locked`, this bead:

  ```
  cargo check -p pdftract-core --no-default-features --locked --target wasm32-unknown-unknown   → exit 0 (2.4s)
  cargo check -p pdftract-core --no-default-features --features serde --locked --target wasm32-unknown-unknown → exit 0 (3.2s)
  cargo check -p pdftract-wasm --locked --target wasm32-unknown-unknown                        → exit 0 (3.7s)
  ```

## Doc change made

- plan.md revision row **1.5** (2026-09-15) appended to the change table; no history rows rewritten.
- ADR-010 Status: one-sentence addition citing the measured ground (zstd fact right, memmap2 rationale wrong).
- ADR-011 Status: "Verified 2026-09-15 …" pointer added.
- ADR-011 body: **Verification amendment** bullet appended after Invalidation trigger, containing (a) the final position (Go, confirmed, residual blockers enumerated), (b) the verified blocker table with per-row evidence pointers, (c) three numbered corrections to stale assumptions (claims 12/13/8+14+15 above), (d) the concrete feature-flag surface table (names + what each gates + measured status), and (e) the residual-gaps paragraph (no link-level build yet, bindings/demo deferred per D6, no wasm-runtime tests, rust 1.83-vs-1.97 caveat).

## Acceptance criteria

- **PASS** — plan.md carries the reconciliation as a labeled Verification amendment inside ADR-011, every blocker claim traceable to an empirical note from this chain (cited per row).
- **PASS** — the wasm feature-flag surface is enumerated concretely in the record (feature → on/off in wasm build → what it gates → measured status; residual native-only deps and gaps named).
- **PASS** — this note contains the claim-by-claim reconciliation table (§ above).

## Caveats

- The re-verification runs used the shared working tree (dirty with unrelated concurrent
  polish edits) rather than a `git archive HEAD` extraction; the predecessor notes
  established tree/HEAD equivalence for this edge (byte-identical signatures), and the
  only commits since `c1fceb36`'s final-HEAD verification are docs-only (`0769ad58`,
  `cdd6efd4`).
- Shared `CARGO_TARGET_DIR` (/build/target-workers) was used for the re-checks; the
  decisive measurements in the chain used private/isolated target dirs.
- No new build/test artifacts were produced by this bead beyond the doc edits.
