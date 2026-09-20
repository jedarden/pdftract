# bf-33zjo — OCR acceptance corpus for WER <3% gate (verification note)

Re-verification 2026-09-14 at local HEAD `c763122a` (an ancestor of `origin/main`;
`tests/fixtures/scanned` tree hash `6624327d` is byte-identical across `c763122a`,
`origin/main`, and the corpus commit `4caeaba1`, so this verification covers the
published corpus exactly). This is an independent re-derivation of the
2026-09-13 close of the same bead, which was auto-reopened without a recorded
reason ~1 minute after closing.

## Criterion 1 — ≥5 fixtures across ≥4 document types: PASS

Six fixture sets across six types, all committed:

| Type | Scanned PDF | Pages | Ground truth |
|---|---|---|---|
| receipt | `receipt/receipt-300dpi-scanned.pdf` | 1 | `receipt/receipt-300dpi.txt` (143 words) |
| invoice | `invoice/invoice-300dpi.pdf` | 1 | `invoice/invoice-300dpi-ground-truth.txt` (105 words) |
| letter | `letter/letter-300dpi.pdf` | 1 | `letter/letter-300dpi-ground-truth.txt` (227 words) |
| form | `form/form-300dpi.pdf` | 3 | `form/form-300dpi-ground-truth.txt` (264 words) |
| report (multi-page) | `multi-page/report-300dpi.pdf` | 11 (≥5 required) | `multi-page/report-300dpi-ground-truth.txt` (1383 words) |
| degraded (low-quality) | `low-quality/degraded-200dpi.pdf` | 1 | `low-quality/degraded-200dpi-ground-truth.txt` (321 words) |

Legacy compatibility copies (`documents/invoice|form`, `multi-page/doc-10page`)
also present. Page counts via `pdfinfo`; specs and OCR-stability curation rules
in `tests/fixtures/scanned/GEN_MANIFEST.md`.

## Criterion 2 — `scripts/measure-wer.sh` exits 0 (WER ≤3% clean 300 DPI): PASS

Re-measured 2026-09-14 with **freshly generated OCR output** (not the committed
`*-ocr.txt` references): `pdfimages -png` per page + `tesseract 5.5.2 stdout -l
eng`, concatenated in page order, then `scripts/measure-wer.sh <ocr> <gt>`:

| Fixture | WER | Exit | Gate |
|---|---|---|---|
| receipt-300dpi | 0.00% | 0 | PASS |
| invoice-300dpi | 0.00% | 0 | PASS |
| letter-300dpi | 0.00% | 0 | PASS |
| form-300dpi | 2.27% | 0 | PASS |
| report-300dpi (11p) | 0.87% | 0 | PASS |
| degraded-200dpi | 9.97% | 1 | n/a — intentionally outside the gate (<10% degraded target) |

All five clean 300 DPI fixtures exit 0, matching the 2026-09-13 measurements
exactly (deterministic pipeline). Environment: `nix-shell -p tesseract
poppler-utils python3` (the nix attribute is `poppler-utils`; plain `poppler`
does not install the tools — this cost one failed attempt, OCR output empty →
WER 100% everywhere before the fix).

## Criterion 3 — `cargo nextest run --features ocr --test ocr`: FAIL at HEAD — pre-existing, owned by `pdftract-ecad3b80` (InProgress)

Same conclusion as the 2026-09-13 close, re-proven on a pristine
`git archive HEAD` tree (extracted to `/data/build/bf33zjo-verify/head`, private
hardlink-warmed `CARGO_TARGET_DIR`, so no concurrent worker's dirty files are
involved):

1. **nextest is not installed on this box** — timeout-wrapped `cargo test` is
   the documented fallback (see memory/CLAUDE.md test-hygiene).
2. **No test target named `ocr` exists** — the real target is `ocr_integration`
   (`crates/pdftract-core/tests/ocr_integration.rs`).
3. `cargo test -p pdftract-core --features ocr --test ocr_integration` fails to
   compile the lib: **50 errors** on the pristine tree. Build environment is
   proven sufficient (`notes/bf-1nww72.md` recipe re-derived with churned nix
   store paths: leptonica 1.87.0 + tesseract 5.5.2 `lept.pc`/`tesseract.pc`,
   clang-21.1.8-lib for bindgen 0.64, glibc-2.42-67-dev headers — bindgen and
   both sys crates build cleanly); the errors are source-level API drift:
   `src/ocr.rs` written against the pre-0.13 tesseract API vs pinned
   tesseract 0.15.2 (E0432 `use tesseract::...`), hybrid `HybridSpan`/`Span`
   type moves (E0425/E0433), preprocess `Pix` imports, `quick-xml` 0.36 drift
   in `forms/xfa.rs`, imageproc 0.26 drift. Distribution: ocr.rs 20,
   preprocess.rs 8, hybrid.rs 7, forms/xfa.rs 6, render/image_compositing.rs 4,
   ocr/preprocessing/{sauvola,otsu}.rs 3, lib.rs 2.
4. Without the feature the same target compiles and exits ok running 0 tests
   (`ok. 0 passed` — every test is `#[cfg(feature = "ocr")]`-gated), so the
   failure is exclusively the feature-gated source bitrot.

This defect predates the corpus work (documented at HEAD `8d2c48e0` on
2026-09-09 with the same failure signature in `notes/bf-1nww72.md`) and is not
caused by this bead's artifacts (fixtures are data files; the corpus commits
touch only `tests/fixtures/scanned/` and `scripts/measure-wer.sh`). Follow-up
bead **`pdftract-ecad3b80`** owns the fix and is InProgress with an assignee as
of 2026-09-14 — bf-33zjo closing does not strand it.

## Verdict

Criteria 1 and 2 PASS with fresh evidence at HEAD. Criterion 3 FAIL is
pre-existing product bitrot, out of this bead's corpus scope, and actively
owned by `pdftract-ecad3b80` — recorded as the criterion-3 handoff, matching
the documented WARN/FAIL convention for out-of-scope items.

---

# Re-verification 2026-09-20 — dispatch bead `pdftract-0093e34d`

Re-derived at HEAD `8c305ea0` (== `origin/main`, `git rev-list origin/main..HEAD`
empty). Method: pristine `git archive HEAD | tar -x` extraction at
`/data/build/bf33zjo-verify-0093e34d/head` (no `.git`, so the shared checkout's
~40 dirty files from other workers are irrelevant); private
`CARGO_TARGET_DIR=/data/build/bf33zjo-verify-0093e34d/target` warm-seeded with
`cp -a` from `/build/target-workers`; `tests/fixtures/scanned` tree hash
`fb4f3341` byte-identical between HEAD and `origin/main`. Live OCR toolchain via
`nix-shell -p tesseract poppler-utils python3` (tesseract 5.5.2, poppler
26.06.0). The temporary extraction, build cache, and raw logs were removed after
verification; everything durable is inline below.

## Criterion 1 — ≥5 fixtures across ≥4 document types: PASS

Six manifested fixtures across six document types (manifest in
`scripts/measure-wer.sh` `FIXTURE_MANIFEST`; specs in
`tests/fixtures/scanned/GEN_MANIFEST.md`):

| Type | Scan PDF | Pages | Ground truth (words) |
|---|---|---|---|
| receipt | `receipt/receipt-300dpi-scanned.pdf` | 1 | `receipt/receipt-300dpi.txt` (143) |
| invoice | `invoice/invoice-300dpi.pdf` | 1 | `invoice/invoice-300dpi-ground-truth.txt` (105) |
| letter | `letter/letter-300dpi.pdf` | 1 | `letter/letter-300dpi-ground-truth.txt` (227) |
| form | `form/form-300dpi.pdf` | 1 | `form/form-300dpi-ground-truth.txt` (166) |
| report (multi-page) | `multi-page/report-300dpi.pdf` | 11 | `multi-page/report-300dpi-ground-truth.txt` (1383) |
| degraded (low-quality) | `low-quality/degraded-200dpi.pdf` | 1 | `low-quality/degraded-200dpi-ground-truth.txt` (321) |

Note: the form fixture was redesigned 2026-09-18 (bead `pdftract-d2201653`) from
the 3-page/264-word layout to a single page/166 words, superseding the 09-14
table above. Legacy `documents/` copies host no manifested fixture (the script's
NOTE about it is informational).

## Criterion 2 — `scripts/measure-wer.sh` exits 0 with clean 300 DPI ≤3%: PASS

Live OCR (fresh `pdfimages -png` + `tesseract 5.5.2 stdout -l eng` per page,
concatenated in page order) run from the extraction:

```
FIXTURE            PAGES  WORDS  SUB  DEL  INS      WER  GATE
receipt-300dpi         1    143    0    0    0    0.00%  PASS (<=3%)
invoice-300dpi         1    105    0    0    0    0.00%  PASS (<=3%)
letter-300dpi          1    227    0    0    0    0.00%  PASS (<=3%)
form-300dpi            1    166    1    0    0    0.60%  PASS (<=3%)
report-300dpi         11   1383    8    3    1    0.87%  PASS (<=3%)
degraded-200dpi        1    321   16    2   14    9.97%  REPORTED (soft target <10%) (non-gating)

clean aggregate: 13 errors / 2024 reference words = 0.64% (micro-average; threshold 3.00%)
GATE: PASS — 5 clean fixture(s) at or under 3%
```

Exit code 0. Corroborating runs, both exit 0:

- `scripts/measure-wer.sh --recorded` — committed `*-ocr.txt` references pass the
  same gate (degraded 8.10% recorded).
- `scripts/measure-wer.sh --self-test` — 6/6 checks passed, including fixture-tree
  immutability (sha256 fingerprint unchanged).

**Degraded-fixture note (the explicitly allowed exception):** `degraded-200dpi`
is the intentionally degraded 200 DPI fixture. It is measured and reported but
never gates; 9.97% live (8.10% recorded) is below its 10% soft target from
`GEN_MANIFEST.md`. Its presence above the 3% clean threshold is by design, not a
failure.

## Criterion 3 — OCR integration test: FAIL at HEAD — pre-existing, owned by `pdftract-ecad3b80` (open, unassigned)

1. **nextest is not installed on this box** — timeout-wrapped `cargo test` is the
   documented fallback (repo CLAUDE.md test-hygiene).
2. **No test target named `ocr` exists**: `cargo test -p pdftract-core --test ocr`
   → exit 101, `error: no test target named 'ocr' in 'pdftract-core' package`.
   The real target is `ocr_integration`
   (`crates/pdftract-core/tests/ocr_integration.rs`, autodiscovered — no
   `[[test]]` section in the Cargo.toml).
3. `cargo test -p pdftract-core --features ocr --test ocr_integration` in the
   pristine extraction → **exit 101: `pdftract-core` (lib) failed to compile with
   48 rustc errors** (55 warnings). New since the 09-14 attempt: the C
   dependencies now build — `leptonica-sys 0.4.9` and `tesseract-sys 0.6.3`
   compile cleanly with the corrected bindgen environment (recipe below) — so the
   failure is purely source-level API drift, same signature as 09-14: tesseract
   0.15.2 API use in `src/ocr.rs`, `HybridSpan`/`Span` type moves out of
   `crate::hybrid`, `Pix` imports, `lexer::Token::Array` removal. Error-location
   mentions by file: ocr.rs 26, preprocess.rs 14, hybrid.rs 10,
   parser/pages.rs 7, forms/xfa.rs 6, render/image_compositing.rs 5,
   parser/lexer/mod.rs 4, output/json.rs 3, plus ≤2 each in ~20 more files
   (schema, sauvola, otsu, lib.rs, reading_order, extract, classify, table/*,
   receipts/*, xref, page_class, ndjson/frames, options, contrast, markdown,
   line, correction, document, confidence, audit, span).
4. Without the feature the same target compiles and exits 0 running 0 tests
   (`cargo test -p pdftract-core --test ocr_integration` → `ok. 0 passed; 0
   failed`; every test is `#[cfg(feature = "ocr")]`-gated), so the failure is
   exclusively the feature-gated source bitrot.

This defect is unchanged in kind from the 09-14 finding and predates this
dispatch. **`pdftract-ecad3b80` owns the fix and is open/unassigned as of
2026-09-20.** This dispatch is verification-only and performed no source fixes
(no unrelated work), so criterion 3 remains the recorded handoff.

### OCR build env recipe (2026-09-20 — corrects `notes/bf-1nww72.md` for leptonica 1.87)

```bash
export PKG_CONFIG_PATH="<tesseract-5.5.2-pkg>/lib/pkgconfig:<leptonica-1.87.0>/lib/pkgconfig"
export LIBCLANG_PATH="<clang-21.1.8-lib>/lib"
export BINDGEN_EXTRA_CLANG_ARGS="-isystem <glibc-2.42-67-dev>/include"
```

Store paths churn on nix GC (re-derive with the globs in `notes/bf-1nww72.md`;
live 2026-09-20 values: leptonica `g5q4zigcidyrm3lgprzs33vdycvdvlam`, tesseract
`k7v448ggp0yii5blnp0ibgnyad3kdnzx`, clang-lib `imqc1fmvi4gpzvv7b0qw839rhimyfc4i`,
glibc-dev `2rgc5gn3a2qids4bhjaq2g1l1r2i1637`). **Critical correction:** do NOT
add `-I <clang-lib>/lib/clang/21/include`. With the nix clang wrapper,
libclang auto-discovers its resource dir from `LIBCLANG_PATH`; adding the
builtin include as a plain user `-I` breaks clang's include-next ordering, so
`stdatomic.h`/`stdint.h` resolve to nothing and leptonica 1.87's `environ.h:70`
fails with `unknown type name 'atomic_int'` (reproduced standalone with the
plain wrapper clang; `atomic_int x; intptr_t y;` compiles fine without the extra
`-I`). `bf-1nww72`'s recipe pre-dates leptonica 1.85→1.87, which introduced the
C11 atomics block — that is why its extra `-I` was harmless then and fatal now.
Use `-isystem` for the glibc-dev include only.

## Criterion 4 — evidence note cites actual paths and results: PASS

This section is that record: every command, exit code, fixture path, and WER
value above was produced by this dispatch at HEAD `8c305ea0`.

## Criterion 5 — no parent closure, no unrelated cleanup: HELD

Parent `bf-33zjo` was not closed or modified by this dispatch. No source files
were touched; the only artifact is this note.

## Verdict (2026-09-20)

Criteria 1, 2, 4 PASS; criterion 5 held. Criterion 3 FAIL is pre-existing
product bitrot (ocr-feature API drift, 48 errors at HEAD), out of this
dispatch's verification scope, and owned by open bead `pdftract-ecad3b80` —
same handoff as the 09-14 record, now with the sys-crate build environment
solved and the failure isolated to source drift alone.
