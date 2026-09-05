# bf-84xr8: Unmapped Glyph Generation Implementation

**Task:** Implement unmapped glyph generation logic
**Date:** 2026-09-05 (rework of the 2026-07-03 attempt)
**Status:** ✅ COMPLETE (generator + fixture + structural tests; end-to-end extraction blocked, see WARN-1)

## Summary

Implemented the unmapped glyph generation logic in the canonical encoding fixture
generator, `tools/generate_encoding_fixtures.py`, so `no-mapping.pdf` carries the
seven unmapped glyph names selected in bf-ad2pp (notes/bf-68f9i-glyphs.md) and laid
out in notes/bf-68f9i-design.md, plus three standard AGL names as a control group.

## Why this bead was reopened

Two earlier commits claim this work but neither changed the generator:

- `e4f97650` (bf-3cwge) added `tests/fixtures/encoding/generate_unmapped_glyphs.py`.
  That file was later **deleted** when fixture generators were consolidated
  (`e43936bf` moved generators to `tools/`); the script no longer exists.
- `a9d13438` (bf-84xr8) says it "updated `create_no_mapping_pdf()`" but the commit
  touches **only** `tests/fixtures/PROVENANCE.md`. The generator kept its placeholder
  glyphs `/g00 … /g05`.
- `2f06440d` (bf-84xr8) fixed `/Differences` in the now-deleted standalone script.

So the shipped generator still emitted the six placeholder names and a content
stream built from bare glyph names (`/g00 /g01 /g02 /g03 Tj`), which are not valid
PDF show operators — text is shown with hex or literal strings, not by naming glyphs.

## What changed

### `tools/generate_encoding_fixtures.py`

- Added `NO_MAPPING_GLYPHS`, the design doc's 10-row character-code table, each row
  carrying `(code, name, unmapped, expected_extraction)`; and `NO_MAPPING_LINES`, the
  three-line layout. The generator is now driven by these tables rather than by
  hard-coded bytes, so "configure the generator to use these glyph names" is a data
  edit, not a rewrite.
- `/Differences` is now assembled as `[0 /g001 /g002 /g003 /CustomA /CustomB
  /NotAGlyph /glyph_0041 /A /B /space]`. The previous code emitted the base code as
  if it were the first glyph name, which silently dropped `/g001` from the array.
- The content stream uses hex string show operators — `<000102> Tj`,
  `<03040506> Tj`, `<070809> Tj` — instead of bare glyph names.
- `/Length` and every xref offset are computed from the assembled objects. The old
  body hard-coded both, so any edit to the table desynced the xref from the objects
  it described.
- Ground truth is derived from the table via `no_mapping_ground_truth()` instead of
  being a separate literal that could drift: 7 × U+FFFD followed by `"AB "`.

### Regenerated fixture

- `tests/fixtures/encoding/no-mapping.pdf` — SHA256
  `4e35868ca3c5a66b46e38f4fe52c3bea00bb4bc77a5a5532992f4fa11da46d6d`
- `tests/fixtures/encoding/no-mapping.txt` — ground truth
- `tests/fixtures/PROVENANCE.md` — corrected the Content line (the old one claimed a
  trailing `AB A` and listed `/uni0041`, which the design doc marks optional and the
  10-code table omits) and recorded this regeneration.

### New test: `crates/pdftract-core/tests/no_mapping_fixture_structure.rs`

Seven tests that pin the acceptance criteria to the committed fixture bytes and to
the real AGL resolver, so the criteria stay verifiable independently of the
extraction pipeline (see WARN-1):

| Test | Asserts |
|---|---|
| `fixture_has_all_glyph_names_in_differences_array` | all 10 names, correct order, base code 0 |
| `fixture_declares_no_tounicode_cmap` | no `/ToUnicode` anywhere in the file |
| `fixture_content_stream_shows_all_ten_character_codes` | the three `<…> Tj` operators are present |
| `fixture_font_is_type1_and_not_embedded` | `/Subtype /Type1`, no `/FontDescriptor` |
| `unmapped_glyph_names_have_no_unicode_representation` | `unicode_for_glyph_name` → `None` for all 7 |
| `mapped_control_glyphs_resolve_through_agl` | `/A`→U+0041, `/B`→U+0042, `/space`→U+0020 |
| `non_agl_algorithmic_prefix_is_rejected` | `/glyph_0041` does not resolve |

`cargo test -p pdftract-core --test no_mapping_fixture_structure` → **7 passed; 0 failed.**

## Acceptance criteria

| Criterion | Result |
|---|---|
| Generator configured to output ≥3 unmapped glyphs from design doc | **PASS** — 7 unmapped glyphs, from the design table |
| Unmapped glyphs properly encoded (no CMAP/ToUnicode mapping) | **PASS** — no `/ToUnicode`; font is `/Type1`, not embedded |
| Generated PDF contains the unmapped glyph names in font subset | **PASS** — all 7 present in `/Differences`; pinned by test |
| Verification that these glyphs are truly unmapped | **PASS** at Level 2 — all 7 absent from `crates/pdftract-core/build/agl.json` (586 entries) and rejected by the algorithmic parser; see WARN-1 for Levels 1/3/4 |
| Changes committed to git | **PASS** |

Verification that the names are unmapped, checked against the repo's own AGL data:

```
--- unmapped (must be ABSENT from AGL) ---
  /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041   present=False
--- mapped control (must be PRESENT) ---
  /A -> A   /B -> B   /space ->                 present=True
RESULT: PASS
```

`glyph_0041` deserves a note: it contains hex digits and looks algorithmic, but
`parse_algorithmic` in `crates/pdftract-core/src/font/agl.rs:102` accepts only the
`uniXXXX` (exactly 4 hex digits) and `uXXXXXX` prefixes, so the `glyph_` prefix is
correctly rejected.

## WARN-1: end-to-end text extraction is broken tree-wide (pre-existing)

`PdfExtractor` currently produces no text for **any** PDF, so I could not demonstrate
U+FFFD emission by running the extractor. Evidence that this is not caused by my
change:

- A control PDF written by ReportLab 4.5.1 — `canvas.drawString("Hello World")`, xref
  verified well-formed — opens successfully but `extract_page(0)` returns
  `Document 'PdfExtractor' is empty or contains no content`.
- `tests/fixtures/encoding/no-mapping.pdf` (mine), `unmapped-glyphs.pdf` and
  `unmapped-comprehensive.pdf` fail the same way. These have valid xref tables
  (verified independently in Python).
- `agl-only.pdf`, `fingerprint-match.pdf`, `shape-match.pdf`,
  `test_working_copy.pdf`, `tests/fixtures/classify_page_simple.pdf` and
  `tests/fixtures/json_schema/simple-text.pdf` fail earlier with `No /Root reference
  in trailer` — and their `startxref` values genuinely do not point at an `xref`
  keyword, so that error is the correct diagnosis for them.
- Net: `crates/pdftract-core/tests/encoding_recovery.rs` fails 5 of 6 tests both
  before and after my change. Verified by restoring the original fixture and
  re-running: same 5 failures on the untouched fixture.

My structural tests were written specifically so this blockage does not hide the
fixture criteria.

## Related pre-existing issue (not this bead)

`crates/pdftract-core/tests/cmap_unmapped_glyphs.rs` fails 5 of 7 tests because
`build/unmapped-glyph-names.json` does not exist at the workspace root. `build.rs`
falls back to a default set containing only `.notdef`, so `is_unmapped_glyph_name("g001")`
returns `false` and the `/Differences` overlay does not filter the g-series names.
That JSON file is the deliverable of a follow-up to bf-3vo80 and is deliberately not
created here — adding it would change `is_unmapped_glyph_name()` behaviour across the
codebase, which is outside this bead's scope.

## Side effect to be aware of

Running `python3 tools/generate_encoding_fixtures.py` regenerates **all four**
encoding fixtures and overwrites the curated `agl-only.txt` and `shape-match.txt`
ground truths (which carry explanatory comments and different expected text) with
bare strings, and produces a `fingerprint-match.txt` that is not checked in. During
development this clobbered those files; they were restored from git before commit and
only the `no-mapping` pair was regenerated, by importing the module and calling
`create_no_mapping_pdf()` directly. The generator's other three fixtures have drifted
from their checked-in counterparts and should not be trusted to reproduce them — worth
a bead of its own.

## Files

- `tools/generate_encoding_fixtures.py` — generator logic
- `tests/fixtures/encoding/no-mapping.pdf` — regenerated fixture
- `tests/fixtures/encoding/no-mapping.txt` — regenerated ground truth
- `tests/fixtures/PROVENANCE.md` — provenance + new SHA256
- `crates/pdftract-core/tests/no_mapping_fixture_structure.rs` — new structural tests
- `notes/bf-84xr8.md` — this note
