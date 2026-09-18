# bf-4flrb9 — conversion-failure no-panic regression test in sauvola.rs

**Date:** 2026-09-18 · **Worker:** claude-code-glm-5.3-glm-pdftract · **Commit:** (this dispatch)
**Parent:** bf-41jlc7 (split-child #3 of 4) · **Trigger spike:** bf-5v0q2f (closed)

## What was added

`test_sauvola_conversion_failure_no_panic_returns_degraded_input` in
`crates/pdftract-core/src/ocr/preprocessing/sauvola.rs` (`#[cfg(test)] mod tests`,
ocr-gated with the module like every other test in the file):

- Input: `GrayImage::new(0, 0)` — the deterministic `grayimage_to_pix` failure
  trigger confirmed by the bf-5v0q2f spike (leptonica 1.87.0 `pixCreate` rejects
  zero width/height → conversion returns `Err`).
- Asserts under `std::panic::catch_unwind(AssertUnwindSafe(...))` that
  `sauvola_binarize(&degenerate, 15, 0.34)` returns `Ok` (no panic), and that the
  returned image equals the unbinarized input (the degraded result).
- Zero-dim input is load-bearing: a non-empty "degenerate" image would exercise
  the GrayImage→Pix copy-loop heap overflow (pdftract-20dc6118) instead of the
  failure branch. Spike instruction followed verbatim.

## Latent FFI errors fixed in the same file (necessary for the file to ever compile)

bf-5v0q2f's spike could not see these; they are **masked in every in-repo
`--features ocr` build** because the ~100-error compile drift (pdftract-a293e323)
aborts rustc before type-checking this module. They surfaced immediately in the
standalone harness (see below), which type-checks sauvola.rs with nothing masked:

1. `sauvola_binarize` null-result branch called `pixDestroy(pix)` with
   `pix: *mut Pix` — `pixDestroy` takes `*mut *mut Pix` (E0308). Binding is now
   `let mut pix` and the call is `pixDestroy(&mut pix)`.
2. `binary_pix` was an immutable binding while both existing cleanup sites pass
   `&mut binary_pix` (E0596). Binding is now `mut`.

Both are the same drift class bf-4dpc01 fixed at two other sites (commit
`6420b40e`) — those were type-checked only because of their position; these were
never reached in-repo.

## Verification

The in-repo `cargo test -p pdftract-core --features ocr sauvola` **cannot run at
HEAD for pre-existing reasons unrelated to this bead**:

- Compile: ~100 drift errors across ~20 files (hybrid.rs 42, preprocess.rs 36,
  ocr.rs 35, signature/mod.rs 24, …) — pdftract-a293e323, open; grown from the
  49 errors bf-4dpc01 inventoried on 2026-09-17. `sauvola.rs` itself shows only
  the 2 known unused-import warnings, no errors.
- Runtime: 8 of the 9 pre-existing sauvola tests SIGSEGV individually
  (`--test-threads=1`, one at a time) — the pdftract-20dc6118 copy-loop heap
  overflow (non-empty images write 4×len bytes into a len-byte Pix buffer).
  Only `test_sauvola_even_window_panics` (panics before any Pix work) survives.

So the exact committed sauvola.rs bytes were verified in a standalone harness at
`/var/tmp/bf-4flrb9/harness` (removed after verification; same target cache at
`/var/tmp/bf-4flrb9/tgt`): a `sauvola-harness` lib that `#[path]`-includes the
real file verbatim, plus verbatim copies of `grayimage_to_pix`/`pix_to_grayimage`
and a shape-matched Diagnostic stub (message: `Cow<'static, str>` so
`d.message.as_ref()` resolves identically). Documented deviations: stub
`with_static_no_offset` takes `impl Into<Cow<'static, str>>` (the verbatim
`format!` call site needs it — the real `&'static str` signature is part of the
a293e323 drift), and the copied `grayimage_to_pix` got the same `&mut` pixDestroy
correction as the real file.

Build env (bf-4dpc01 recipe): `PKG_CONFIG_PATH` = nix leptonica-1.87.0 +
tesseract-5.5.2 pkgconfig, `LIBCLANG_PATH` = clang-21.1.8-lib,
`BINDGEN_EXTRA_CLANG_ARGS="-isystem <glibc-2.42-67-dev/include>"`,
`LD_LIBRARY_PATH` = lept + tess + clang lib.

Results:

- `cargo test --features ocr --lib test_sauvola_conversion_failure_no_panic_returns_degraded_input`
  → **PASS 3/3, exit 0**; leptonica's `Error in pixCreateHeader: width must be > 0`
  on stderr proves the failure path fired; degraded result asserted.
- Harness `cargo test --features ocr` (whole sauvola suite, 11 tests): the new
  test passes; the binary then SIGSEGVs in the pre-existing non-empty-image tests
  (pdftract-20dc6118), as itemized above.
- `rustfmt --edition 2021 --check` on sauvola.rs: clean.
- Default-feature tree unaffected (changes are ocr-gated): `cargo check -p
  pdftract-core` clean at the committed HEAD (fresh git-archive extraction).

## Acceptance criteria

| Criterion | Result |
|---|---|
| Regression test exists exercising the conversion-failure path, asserts no-panic + degraded result | **PASS** (harness-verified on exact committed bytes) |
| `cargo nextest run -p pdftract-core --features ocr sauvola` passes including the new test | **WARN — blocked, pre-existing, out of scope.** nextest is not installed on this box (memory; bf-4dpc01 precedent — timeout-wrapped `cargo test` is the runner). The suite cannot compile under `--features ocr` until pdftract-a293e323 lands, and cannot pass at runtime until pdftract-20dc6118 fixes the copy-loop overflow. The new test itself passes in isolation. |
| Commit citing bf-41jlc7, pushed | **PASS** (pushed to `origin` = Forgejo; the bead's `git push forgejo main` is stale — no `forgejo` remote exists) |
