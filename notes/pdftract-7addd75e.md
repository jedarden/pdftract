# pdftract-7addd75e — Restore hybrid fixture generator provenance under tools/

Date: 2026-09-23
Bead: `pdftract-7addd75e` (parent bf-1xlu7d; KU-2 at `docs/plan/plan.md` line 725)

## What was done

1. **`tools/generate_hybrid_fixtures.py` (new)** — stdlib-only (zlib, math,
   argparse, pathlib) Python 3 generator for the ten root-level
   `hybrid-0NN*.pdf` hybrid fixtures. Deterministic: no wall-clock timestamps
   unless injected via `--date YYYYMMDDHHMMSS`. CLI:
   `--out-dir DIR` (default `tests/fixtures/hybrid`), repeatable `--fixture`
   (accepts `001`, `hybrid-001`, or the full stem), `--date`.
2. **`tests/fixtures/hybrid/GEN_MANIFEST.md`** — new "Provenance (root-level
   `hybrid-0NN` corpus)" section naming the tools script as canonical,
   recording that the originally-cited `hybrid-NNN-generator.py` /
   `hybrid-010-generator-enhanced.py` scripts were removed in `2014ee74`
   (bf-24po9b), and stating explicitly that byte-identity is NOT preserved
   (structural equivalence is). Stale reportlab-era sections ("Generation
   Summary" note, "Generation Script") and the Related Documentation pointer
   are annotated as scoped to the legacy subdirectory placeholders.
3. The ten committed PDFs are untouched (`git status` clean under
   `tests/fixtures/hybrid/*.pdf`).

## Design notes

- Per-fixture builders follow each sidecar's `source.generation_method`:
  1-bit DeviceGray FlateDecode image XObject scanned layer + vector layer
  (text ops, underline rects, red checkbox strokes, 45° watermark, Bezier
  circle stamp, multi-angle rotations, ExtGState ca/CA 0.5/0.7,
  12-curveto circles for hybrid-010).
- Rotated text uses `q`/`cm`/`Q` CTM transforms (hybrid-008's sidecar says
  "CTM transformations"). The removed originals used malformed `Tm`
  operators instead (`-45 -25 Tm` — two operands; `15 15 15 1 0 0 Tm` —
  raw degrees as matrix components).
- Unrotated text uses `Td` positioning inside `BT..ET` (BT resets the text
  matrix), matching the original idiom. Empirically, `Tm` combined with
  any explicit `rg`/`gs` makes pdftract suppress ALL page text (overlay
  classification); `Td` + color extracts fine.

## Verification (working tree at 06e1abf5 + this change)

Commands run from `/var/tmp` scratch dirs (`/var/tmp/hybrid-regen`), never
overwriting the committed corpus (`~/scratch` is a symlink to the full
`/data` mount, per memory):

- `python3 tools/generate_hybrid_fixtures.py --out-dir /var/tmp/hybrid-regen`
  → 10 files, 2.7–4.5 KB each. Exit 0.
- Determinism: second run into `/var/tmp/hybrid-regen2`; `diff -r` →
  byte-identical.
- `--fixture` selection: `--fixture 004 --fixture hybrid-010` writes exactly
  those two. Exit 0.
- Structure: every regenerated PDF has exactly 1 `/Subtype /Image` object
  and 2–15 ` Tj` operators (grep -a counts).
- `pdftract extract <pdf> --text -` (debug binary
  `/build/target-workers/debug/pdftract`): rc=0 for all 10; vector text
  extracted for 8/10 (e.g. 001 "ACME Corp / Annual Report 2024", 006
  incl. red "APPROVED"/"Official Seal", 009 incl. gs-transparent overlays,
  010 incl. all columns/annotation).
- `pdftract hash <pdf>`: rc=0 for all 10 (fingerprint value is coarse —
  identical across single-page-letter PDFs including committed originals;
  used only as a parse signal).
- `git status tests/fixtures/hybrid/` → clean; committed PDFs byte-unchanged.

## WARN / findings

- **WARN — hybrid-004 and hybrid-008 extract no text at HEAD.** Both carry
  genuinely rotated vector text; pdftract panics at
  `crates/pdftract-core/src/layout/line.rs:374:26`
  (`Option::unwrap()` in `group_lines_into_blocks`) when a page mixes a
  rotated/skewed text line with axis-aligned lines. This is PRE-EXISTING:
  the committed `tests/fixtures/hybrid/hybrid-008-rotated-vector.pdf`
  triggers the same panic at HEAD (its bogus 6-operand `Tm` matrices
  produce skewed lines). The removed hybrid-004 original only dodged it
  because its malformed 2-operand `Tm` degrades to non-rotated text.
  Regenerated fixtures are sidecar-faithful (real rotated text), so they
  exercise the same extractor path. Follow-up bead filed for the panic.
- **WARN — scanned imagery is approximated.** As the sidecars themselves
  describe simulated patterns ("horizontal line pattern simulating scanned
  text"), the generator emits a deterministic synthetic dark-run pattern;
  prose-described layouts (e.g. hybrid-002's form imagery) are approximated
  with the generic pattern plus divider lines. Recorded in GEN_MANIFEST
  "Provenance" deviations.
- `pdftract classify` requires the `profiles` feature, which the available
  debug binary is not built with — classification-level verification not
  possible here; extract/hash evidence used instead.

## Artifacts

- `tools/generate_hybrid_fixtures.py`
- `tests/fixtures/hybrid/GEN_MANIFEST.md`
