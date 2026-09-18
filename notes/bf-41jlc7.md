# bf-41jlc7 — sauvola_binarize no-panic on Pix conversion failure (parent consolidation)

**Date:** 2026-09-18 · **Worker:** claude-code-glm-5.3-glm-pdftract · **Dispatch:** bf-1ijxwx (split-child #4 of 4, verification note)
**Plan:** docs/plan/plan.md:146 — "**Error model:** All parse errors are recoverable and produce diagnostic entries in the `errors` array; no `panic!` in library code."
**Parent:** bf-41jlc7 (open at dispatch) · **Verdict: both parent acceptance criteria PASS.**

## Split children (all closed)

| # | Bead | Title | Commit(s) | Status |
|---|---|---|---|---|
| 1 | bf-4dpc01 | Land the existing no-panic refactor in sauvola_binarize and verify the build | `c4b56fe9`, `6420b40e` | closed — notes/bf-4dpc01.md |
| 2 | bf-5v0q2f | Establish a deterministic trigger for the grayimage_to_pix failure path | (spike, no repo change) | closed — evidence on bead notes |
| 3 | bf-4flrb9 | Add the conversion-failure no-panic regression test to sauvola.rs | `b0a380ff` | closed — notes/bf-4flrb9.md |
| 4 | bf-1ijxwx | This consolidation note | (this dispatch) | this note |

All three commits are ancestors of HEAD == `origin/main` (`b0a380ff`); verified with
`git merge-base --is-ancestor` at dispatch time. The affected file is
`crates/pdftract-core/src/ocr/preprocessing/sauvola.rs` (module `ocr::preprocessing`,
pub fn `sauvola_binarize`, reachable from the OCR preprocessing path).

## (1) The no-panic refactor — split-child #1 (bf-4dpc01)

`sauvola_binarize` previously called `panic!` when `grayimage_to_pix()` returned
`Err` (the removed comment even said "for now we panic to surface the issue
early"). The refactor landed via `c4b56fe9` (restore of the rewritten file after
the git-history blob strip) plus `6420b40e` (the `pixDestroy` `PIX**` E0308 fix
exposed when the file first type-checked under `--features ocr`).

All three former panic sites (GrayImage→Pix conversion, null `pixSauvolaBinarize`
result, Pix→GrayImage back-conversion) now record the failure and return the
**unbinarized input as a degraded result** — `tracing::warn!` + `return
image.clone()` at sauvola.rs:143/181/201 at HEAD. The only retained abort is the
`assert!(window_size % 2 == 1)` API precondition, which bf-4dpc01 recorded as out
of scope (caller contract, not a conversion failure).

### Grep evidence at HEAD (AC a)

Run in this dispatch's clean `git archive HEAD` extraction
(`/var/tmp/bf-1ijxwx/extract`):

```
$ grep -n 'panic!' crates/pdftract-core/src/ocr/preprocessing/sauvola.rs
128:        // table: "no `panic!` in library code"), each such failure is surfaced as a
```

**Exactly one hit, and it is prose inside a comment** quoting the plan's error
model — there is no executable `panic!` anywhere in the file, hence none in the
grayimage_to_pix failure branch. The branch itself (sauvola.rs:139-151 at HEAD):

```rust
let mut pix = match grayimage_to_pix(image) {
    Ok(p) => p,
    Err(diag) => {
        diagnostics.extend(diag);
        warn!(
            "sauvola_binarize: GrayImage→Pix conversion failed; returning \
             unbinarized input as a degraded result ({})", ...
        );
        return image.clone();
    }
};
```

## (2) The deterministic failure-path trigger — split-child #2 (bf-5v0q2f)

Spike result (full text on the bead's `notes` field): `GrayImage::new(0, 0)` — or
any zero width/height — deterministically drives `grayimage_to_pix` to `Err`.
Mechanics: leptonica 1.87.0 `pixCreate(0, *, 8)` returns NULL ("Error in
pixCreateHeader: width must be > 0"), so the null-result branch fires with
`DiagCode::ImgUnsupportedFormat`. No UB on the trigger path: the `Err` returns
before the copy loop, and a 0-dim image has `len == 0` anyway. An i32-wrap width
also triggers but costs a 2 GB allocation — zero dims is the cheap deterministic
trigger. The spike also ground-truthed the adjacent copy-loop heap overflow
(bytes written as `*mut l_uint32` words addressed by byte index → 4×len bytes
into a len-byte Pix buffer for every NON-empty image) and filed it as
**pdftract-20dc6118** with ASAN evidence — which is why the regression test MUST
use the zero-dim trigger and must not use a non-empty "degenerate" image.

## (3) The conversion-failure regression test — split-child #3 (bf-4flrb9)

Commit `b0a380ff` added
`test_sauvola_conversion_failure_no_panic_returns_degraded_input`
(sauvola.rs:597, inside the `#[cfg(test)]` module, ocr-gated with the file):

- Input `GrayImage::new(0, 0)` (the bf-5v0q2f trigger).
- `std::panic::catch_unwind(AssertUnwindSafe(...))` around
  `sauvola_binarize(&degenerate, 15, 0.34)`; `result.expect(...)` fails the test
  if the function panicked.
- Asserts the degraded result: returned image equals the unbinarized input
  (width, height, and raw bytes).
- Same commit also fixed three latent FFI-signature errors in the file
  (`pixDestroy` `*mut *mut Pix` E0308 ×2 sites, `mut` binding E0596), which were
  masked in every in-repo `--features ocr` build by the pdftract-a293e323 drift
  aborting rustc before type-checking this module.

### Fresh re-run at this dispatch's HEAD (AC b)

bf-4flrb9 verified the test 3/3 PASS exit 0 in a standalone harness on the exact
committed bytes. This dispatch **re-derived the same result at HEAD** rather than
citing the earlier run: HEAD (`b0a380ff`) is the very commit that introduced the
test, and the included file's sha256 is byte-identical to what bf-4flrb9
verified:

```
sha256(sauvola.rs at HEAD) = 4910e2c8e8594945ed86bf1b6214ead0fc8cf12ac8ef55246042fa4069c9fc4a
```

The harness (assembled at `/var/tmp/bf-1ijxwx/harness`, recipe per
notes/bf-4flrb9.md, functions sed-extracted from the extraction — no manual
transcription) `#[path]`-includes the real sauvola.rs verbatim next to verbatim
copies of `grayimage_to_pix` / `pix_to_grayimage` and a shape-matched diagnostics
stub. Documented deviations (all part of the pdftract-a293e323 drift, not this
bead's scope): stub `with_static_no_offset` takes `impl Into<Cow<'static, str>>`
(the verbatim `format!` call site needs it), `pixDestroy(&mut pix)` correction in
the copied `grayimage_to_pix`, and a module-scope `use
leptonica_plumbing::leptonica_sys::Pix;` (the real preprocess.rs signatures
reference `Pix` with no in-scope import — one of its 8 inventoried drift errors).

Why a harness at all: `cargo test -p pdftract-core --features ocr` cannot run
in-repo for pre-existing reasons owned by other beads — compile drift (~100
errors across ~20 unrelated files, pdftract-a293e323, open) and, once compiled,
8/9 sibling sauvola tests SIGSEGV via the pdftract-20dc6118 overflow.

Output (runner: timeout-wrapped `cargo test`, private CARGO_TARGET_DIR,
nix leptonica 1.87.0 + tesseract 5.5.2 + clang bindgen env, exit code 0):

```
$ cargo test --features ocr --lib test_sauvola_conversion_failure_no_panic_returns_degraded_input -- --nocapture
running 1 test
Error in pixCreateHeader: width must be > 0        <- leptonica: the failure path fired
Error in pixCreateNoInit: pixd not made
Error in pixCreate: pixd not made
test sauvola::tests::test_sauvola_conversion_failure_no_panic_returns_degraded_input ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
```

The stderr lines are leptonica's own pixCreate errors — direct proof the
conversion-failure branch was exercised, after which the no-panic and
degraded-result assertions passed.

## Gate-safety at the extraction (default features, unrelated to this bead's ocr-gated file)

- `cargo build -p pdftract-core` in the clean extraction: **exit 0** (74
  pre-existing warnings, 0 errors).
- `cargo test -p pdftract-core --lib` in the same extraction: **3474 passed /
  134 failed, exit 101** — all pre-existing failures documented in the plan TH
  notes; none touch sauvola (ocr-gated, not compiled under default features).
  Recorded to show the baseline, not as a pass.
- No `scripts/definition-of-done.sh` exists in the repo, so the language-default
  check is scoped as above (workspace-wide `cargo test` carries ~300 PRE-EXISTING
  failures and integration tests fail-fast — see plan TH notes; not re-run here,
  not attributable to this bead).

## Acceptance criteria (parent bf-41jlc7)

| Criterion | Result |
|---|---|
| (a) The grayimage_to_pix failure branch in sauvola_binarize contains no panic call; conversion failure handled non-fatally (degraded result) — grep evidence | **PASS** — `grep -n 'panic!'` on sauvola.rs at HEAD returns a single hit, a comment quoting the plan's rule (line 128); the Err branch is `extend` + `warn!` + `return image.clone()` (lines 139-151). |
| (b) A test drives the conversion-failure path and asserts the call returns without panicking | **PASS** — `test_sauvola_conversion_failure_no_panic_returns_degraded_input` (commit `b0a380ff`), re-run fresh at this dispatch's HEAD extraction: 1 passed, exit 0, leptonica stderr proving the Err path fired. |

## WARN items (infra, pre-existing, out of scope — none block this close)

- **WARN** — `cargo nextest` is not installed on this machine (repo memory +
  bf-4dpc01 precedent); the dispatch's "include the cargo nextest output" is
  satisfied with the actual runner, timeout-wrapped `cargo test`. Exit-code and
  pass/fail semantics are equivalent for this single-test selection.
- **WARN** — the in-repo `--features ocr` suite cannot compile until
  pdftract-a293e323 (open, ~100 drift errors in ~20 files unrelated to
  sauvola.rs) and cannot run to completion until pdftract-20dc6118 (open,
  copy-loop heap overflow; 8/9 sibling tests SIGSEGV). Neither defect touches the
  conversion-failure branch or its test; both are tracked and owned.
- **WARN** — the harness deviations listed above (three drift items needed to
  type-check verbatim copies outside the drifted crate).

## Commits and push

- This dispatch adds only this note (`notes/bf-41jlc7.md`); the substantive work
  is `c4b56fe9` + `6420b40e` (bf-4dpc01) and `b0a380ff` (bf-4flrb9), all pushed.
- Push remote is `origin` (= Forgejo, the only remote); the beads' "git push
  forgejo main" text is stale — no `forgejo` remote exists (bf-4dpc01/bf-4flrb9
  precedent).
- Commit: `docs(bf-41jlc7): parent verification note — no-panic refactor, deterministic trigger, regression test` (pushed to `origin/main`).

## After this bead closes

Parent bf-41jlc7's remaining blocker is **pdftract-a293e323** (ocr compile
drift). Its substantive acceptance criteria are both satisfied and evidenced
above, so the parent is closeable as an umbrella on the strength of this note +
the closed split children; the a293e323 blocker gates the broader `--features ocr`
build health, not the no-panic criteria this parent owns.
