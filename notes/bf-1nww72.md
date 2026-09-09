---
name: bf-1nww72-ocr-build-env-recipe
description: Verified env recipe that makes leptonica-sys build on this NixOS box, with proof from cargo check -p pdftract-core --features ocr
metadata:
  type: project
---

# OCR build-environment recipe (leptonica-sys on this NixOS box)

Beads: pdftract-33abd0ac (this record) under umbrella bf-1nww72. Supersedes the
stale "missing OCR system dependencies" BLOCKED note on bf-1nww72 — that claim
is FALSE. `notes/bf-3tedi.md` documents the CLI syntax but NOT this recipe; that
gap is why bf-1nww72 failed 3x.

## The recipe

Three env vars are all that is missing. Everything else is already on the box:
tesseract 5.5.0 at `/home/coding/.nix-profile/bin/tesseract`, tessdata at
`/home/coding/.nix-profile/share/tessdata`, leptonica 1.85.0 and clang 18.1.8
in the nix store.

```bash
export PKG_CONFIG_PATH=/nix/store/3ili60pi8p3l10a1pnxxdd98f4s63jxb-leptonica-1.85.0/lib/pkgconfig
export LIBCLANG_PATH=/nix/store/hw7m1zkrmb6mcl8m37307b5x8w4rb39s-clang-18.1.8-lib/lib
export BINDGEN_EXTRA_CLANG_ARGS="-I/nix/store/dc9s2cqvwslmx5lsfidnn60v9af044zw-glibc-2.40-66-dev/include -I/nix/store/hw7m1zkrmb6mcl8m37307b5x8w4rb39s-clang-18.1.8-lib/lib/clang/18/include"
```

Why each:

- `PKG_CONFIG_PATH` — lets leptonica-sys 0.4.9's build script find `lept.pc`
  (that dir contains exactly one file, `lept.pc`).
- `LIBCLANG_PATH` — bindgen needs `libclang`; only the clang **lib** split is
  present, and its `lib/` subdir is the right value.
- `BINDGEN_EXTRA_CLANG_ARGS` — Nix store compilers see no default include path,
  so bindgen must be told where glibc headers live and where clang's own
  builtins (`stddef.h` etc.) live. Use **glibc 2.40-66**: it matches this
  system (`ldd --version` → GNU libc 2.40). A `glibc-2.42-67-dev` store path
  also exists and does NOT match — mixing it in risks header/libc mismatch.

**These paths are content-addressed and churn on nix store GC.** If a path is
gone, re-derive equivalents:

```bash
ls -d /nix/store/*leptonica-1.85*        # -> use <that>/lib/pkgconfig
ls -d /nix/store/*clang-18*-lib          # -> use <that>/lib, and <that>/lib/clang/18/include
ldd --version | head -1                  # pick the matching *glibc-<that version>*-dev
ls -d /nix/store/*glibc-*-dev
```

## Proof (2026-09-09, HEAD 8d2c48e0)

Command (use the real cargo — the PATH wrapper discards stderr):

```bash
$HOME/.cargo/bin/cargo check -p pdftract-core --features ocr --lib
```

With the recipe exported, the run got PAST the leptonica-sys build script.
All five prior failure signatures are absent (0 grep hits in the full log):
`failed to run custom build command`, `Package lept was not found`,
`unable to find libclang`, `stdio.h`, `stddef.h` (and no `panic`). 43 crates
compiled, including:

```
   Compiling leptonica-sys v0.4.9
```

Then compilation progressed into pdftract-core sources and stopped there —
the expected outcome:

```
error: could not compile `pdftract-core` (lib) due to 50 previous errors; 36 warnings emitted
```

Error codes: 16x E0308, 15x E0425, 7x E0599, 4x E0433, 3x E0432, 1x each
E0061/E0277/E0515/E0606 (+2 uncoded). File distribution (error spans):

| errors | file |
|-------:|------|
| 26 | `crates/pdftract-core/src/ocr.rs` |
| 14 | `crates/pdftract-core/src/preprocess.rs` |
| 10 | `crates/pdftract-core/src/hybrid.rs` |
| 7 | `crates/pdftract-core/src/parser/pages.rs` |
| 6 | `crates/pdftract-core/src/forms/xfa.rs` |
| 5 | `crates/pdftract-core/src/render/image_compositing.rs` |
| 4 | `crates/pdftract-core/src/ocr/preprocessing/sauvola.rs` |
| 4 | `crates/pdftract-core/src/parser/lexer/mod.rs` |
| 2 | `crates/pdftract-core/src/ocr/preprocessing/otsu.rs` |
| 1-2 each | `lib.rs`, `parser/xref.rs`, `parser/stream.rs` |

Toolchain: cargo/rustc 1.98.0-nightly. `leptonica-plumbing 1.4.0` and
`leptonica-sys 0.4.9` per Cargo.lock.

## Handoff to pdftract-c327a210 (fix ocr compile errors)

The 50 errors above are SOURCE errors — bitrot in `cfg(feature = "ocr")` code,
not environment. Environment is proven sufficient; nothing further to install.
One cheap first fix, found while diagnosing:

- `ocr.rs:23` does `use tesseract::{PageSegMode, TessBaseAPI}` → E0432 because
  `tesseract = { version = "0.15", optional = true }` (`crates/pdftract-core/Cargo.toml:19`)
  is never enabled: the `ocr` feature (line 71) lists `dep:image`,
  `dep:imageproc`, `dep:leptonica-plumbing` but **not** `dep:tesseract`.
  `tesseract 0.15.2` is already in Cargo.lock. Add `"dep:tesseract"` to the
  `ocr` feature to clear the import; `tesseract-sys` shares the same pkg-config
  discovery needs, so the same recipe applies to it.
- The rest of the table matches the parent bead's bitrot list (quick-xml 0.31
  APIs vs locked 0.36, `lexer::Token::Array` nonexistent, E0308/E0515 in
  preprocess.rs/hybrid.rs/pages.rs/xfa.rs).

## Verdict

PASS on this bead's ACs: recipe recorded with concrete store paths; empirical
proof captured that leptonica-sys builds and compile reaches pdftract-core;
note committed. The 50 source errors are a separate, owned work item
(pdftract-c327a210) — they are success criteria here, not blockers.
