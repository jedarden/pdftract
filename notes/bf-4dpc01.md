# bf-4dpc01 — Land the existing no-panic refactor in sauvola_binarize and verify the build

**Date:** 2026-09-17 · **Worker:** claude-code-glm-5.3-glm-pdftract · **Fix commit:** `6420b40e`

## Dispatch premise, corrected

The auto-split dispatch (failure-count:5) claimed the refactor existed "uncommitted in the
working tree" and that the bead had "failed 5 times in a row". Both were false:

- The refactor was **already committed and pushed** — `c4b56fe9` ("chore: restore local work
  preserved through git-history cleanup"); the sauvola.rs blob on `origin/main` was
  byte-identical to local HEAD (`ff8b7637`) before this dispatch touched anything.
- The bead's entire forensic history (`.beads/checkpoint/forensic.jsonl`, events keyed by
  `issue_id`) was 3 `label_added` (quarantine machinery) + 3 `updated` — **zero attempts,
  zero closes, zero reopens**. The `failure-count:5` label had nothing behind it.

Per the workspace closure contract the terminal action for a satisfied re-issue bead is an
evidence close, not a split of already-landed work (see memory: auto-split treadmill,
2026-09-16 workspace degradation). No split was performed.

## What verification found (clean no-.git extraction of HEAD)

Extraction: `git archive HEAD | tar -x` → `/tmp/pdftract-verify-4dpc01/t`; private target dir
`/data/tmp-4dpc01/tgt` hardlink-warmed from `/build/target-workers/debug` (CARGO_INCREMENTAL=0,
`/data` was at 100% — build output kept minimal). Build env (ex44 has no system
leptonica/tesseract; nix store + bindgen env required):

```
PKG_CONFIG_PATH=/nix/store/g5q4zigcidyrm3lgprzs33vdycvdvlam-leptonica-1.87.0/lib/pkgconfig:/nix/store/i8prwglibnxgyw1g21fv7qxzd9a4zhch-tesseract-5.5.2/lib/pkgconfig
LIBCLANG_PATH=/nix/store/hc5di09q89yykzmh8z9ra7agykrh4ji9-clang-21.1.8-lib/lib
BINDGEN_EXTRA_CLANG_ARGS=-isystem /nix/store/14vllvzrm1wwkpvkn31fbmi7qz85mmn0-glibc-2.42-67-dev/include
LD_LIBRARY_PATH=<lept lib>:<tess lib>:<clang lib>
```

### Acceptance criteria

| Criterion | Result |
|---|---|
| No `panic!` macro in the three conversion/binarize failure branches | **PASS** (pre-existing) — only `panic!` occurrence in the file is prose inside a comment (line 128). Three `warn!` + `return image.clone()` degraded-result branches at 143/181/201 match the three former panic sites; doc comment documents degraded behavior (77–83); `assert!(window_size % 2 == 1)` precondition retained (out of scope per bead). |
| `cargo build -p pdftract-core --features ocr` succeeds | **WARN → fixed in-bead file, remaining drift out of scope.** First build attempt exposed 2 real E0308 errors **in sauvola.rs itself** (199:37, 216:24): `pixDestroy` takes `PIX **`, code passed `*mut Pix`. Fixed in `6420b40e` (`&mut` coercion). After the fix sauvola.rs is error-free; **49 pre-existing errors remain across 8 other files** — ocr.rs (20, `tesseract::TessBaseAPI` API drift at 0.15.2), preprocess.rs (8, `Pix` unresolved), hybrid.rs (7, `HybridSpan` missing), lib.rs (2, E0432 imports of `preprocess::*`/`hybrid::*`), forms/xfa.rs (6), render/image_compositing.rs (4), ocr/preprocessing/otsu.rs (1), + cascades. Owned by follow-up bead **pdftract-a293e323** (created; bf-41jlc7 now blocked by it). |
| Existing sauvola unit tests pass | **WARN (blocked, infra/scope).** The crate cannot compile under `--features ocr` until pdftract-a293e323 fixes the 49 drift errors, so the 8 `#[test]` fns in sauvola.rs (264–566) cannot run. Never run: not a regression of this bead. |
| Conventional Commits commit citing bf-41jlc7, pushed | **PASS** — `6420b40e` `fix(bf-4dpc01): pass pointer-to-pointer to pixDestroy in sauvola_binarize` (refs bf-41jlc7), pushed to `origin` (= Forgejo; the bead's `git push forgejo main` is stale — no `forgejo` remote exists). `nextest` does not exist on this box (memory: use timeout-wrapped `cargo test`). |

### Gate-configuration safety

`cargo check -p pdftract-core` (default features) at the fixed HEAD: **exit 0, 0 errors**
(the changed lines sit inside `#[cfg(feature = "ocr")]`; verified anyway — one transient
build-script failure on first invocation did not reproduce).

## Deferred, tracked

- **pdftract-a293e323** — the 49-error ocr compile drift (full inventory + repro env in its
  description). bf-41jlc7 is blocked by it; the split-child #3 regression test cannot run
  until it lands.
- Cosmetic: sauvola.rs:32/33 unused imports `DiagCode`/`Luma` under `--features ocr`
  (warnings only; possibly used by the non-ocr cfg path — left untouched deliberately).
