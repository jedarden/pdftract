# pdftract-4d95d233 — Add degraded low-quality OCR fixture

**Date:** 2026-09-20 · **Commit:** 3881351f · **Status:** complete

## What the bead asked vs. what was already there

The fixture itself (`low-quality/degraded-200dpi.pdf`, 601,199 bytes, committed
2026-09-01 via bf-2ngt6/bf-cc7ru) already existed, as did
`degraded-200dpi-ground-truth.txt`, `degraded-200dpi-ocr.txt`, the source
document, the `FIXTURE_MANIFEST` row in `scripts/measure-wer.sh`, and
provenance prose in `tests/fixtures/scanned/README.md` +
`tests/fixtures/scanned/GEN_MANIFEST.md`.

What was missing — and what this bead adds:

1. **`tests/fixtures/scanned/low-quality/degraded-200dpi.txt`** — the canonical
   `<name>.txt` ground-truth name the bead requires ("the exact PDF/TXT pair").
   Every other fixture directory carries a ground-truth `<name>.txt`; the
   degraded fixture only had the long `-ground-truth.txt` name. The new file is
   a **byte-identical copy** of `degraded-200dpi-ground-truth.txt` (verified
   with `cmp`), mirroring the invoice/letter precedent where both names exist.
2. **`scripts/measure-wer.sh`** — the degraded `FIXTURE_MANIFEST` row now
   points at the canonical `low-quality/degraded-200dpi.txt` (matching the
   receipt row's convention). Gate arithmetic unchanged: 8.10% before and after
   the switch proves the two ground truths are interchangeable.
3. **Documentation** (README.md + GEN_MANIFEST.md): explicit 200 DPI
   confirmation, provenance, and the expected-quality contract.

## Acceptance criteria

| Criterion | Result | Evidence |
|---|---|---|
| Exact PDF/TXT pair exists | **PASS** | `low-quality/degraded-200dpi.pdf` (601,199 B) + `low-quality/degraded-200dpi.txt` (39 lines, 321 words); `cmp` vs `-ground-truth.txt` → identical |
| PDF metadata or documented source confirms 200 DPI | **PASS** | Page is 612×792 pt (8.5×11 in) with exactly one `/Subtype /Image` XObject, 1700×2200 px, `/DCTDecode` → 1700/8.5 = 2200/11 = **200.0×200.0 PPI** (derived with python3 stdlib from the committed bytes; documented in GEN_MANIFEST "Resolution" and README attribution) |
| Ground truth covers the page text | **PASS** | GT = 321 words; all 39 non-empty GT lines appear verbatim in the first ~2000 chars of `source-document-abraham-lincoln-public-domain.txt`, which is exactly what `tools/create_degraded_200dpi.py` renders ("Take first ~2000 characters for a single-page fixture") |
| Provenance and expected-quality notes recorded | **PASS** | Abraham Lincoln 1860 Cooper Union address via Project Gutenberg, public domain in the USA (source doc + README attribution + GEN_MANIFEST Source line); expected-quality contract documented: WER well above the clean 3% gate, soft target < 10%, "remains usable for OCR regression coverage", manifest class `degraded`, never gates |
| Fixture does not make the clean 300 DPI WER gate fail | **PASS** | `scripts/measure-wer.sh` live (tesseract 5.5.2, nix): exit 0, 5 clean fixtures ≤ 3%, aggregate 0.64%; degraded-200dpi reported 9.97% non-gating. Re-run from the clean extraction post-commit: recorded exit 0, `--self-test` 6/6, live exit 0 |

## WER numbers, both modes (measured this dispatch, 2026-09-20)

- **Recorded** (committed `*-ocr.txt` vs GT): 12 SUB / 1 DEL / 13 INS /
  32 errors / 321 words = **8.10%**
- **Live** (fresh `pdfimages -png` + `tesseract 5.5.2 stdout -l eng`):
  16 SUB / 2 DEL / 14 INS / 32 errors / 321 words = **9.97%**

README's table cites the live figure (9.97%); GEN_MANIFEST's WER Results table
cites the recorded figure (8.10%). Both are correct for their mode and both sit
under the 10% soft target; the README caption now names which figure is which.

## Verification commands (clean extraction of 3881351f at /var/tmp/pdftract-4d95d233-E5x7)

```
scripts/measure-wer.sh --recorded            exit 0  (GATE: PASS, 5 clean <= 3%, aggregate 0.64%)
scripts/measure-wer.sh --self-test           exit 0  (6 passed, 0 failed)
nix-shell -p tesseract poppler-utils python3 --run '<extract>/scripts/measure-wer.sh'
                                             exit 0  (live OCR, degraded 9.97% non-gating)
cmp degraded-200dpi.txt degraded-200dpi-ground-truth.txt   identical
python3 stdlib PDF inspection                1 image 1700x2200 px on 8.5x11 in = 200.0x200.0 PPI
```

## Notes and scope decisions

- **`cargo build/test` not claimed.** This bead touches zero Rust files
  (fixture .txt + shell manifest row + two docs). The tree at HEAD carries
  ~300 pre-existing test failures documented in `docs/plan/plan.md` TH notes;
  `cargo build --all-targets` was run from the clean extraction as the compile
  half of the language default. Per repo CLAUDE.md the close fence keeps to
  commands that genuinely pass at HEAD.
- Fixture discovery (`crates/pdftract-cli/tests/fixture_discovery.rs`) walks
  `tests/fixtures/` for **PDF** files only; adding a `.txt` cannot change its
  results, and no PDF was added.
- The `--self-test` sha256 fingerprint check is within-run (before/after), so
  adding a file to the corpus tree does not trip it.
