# Hybrid Fixture Generation Manifest

This manifest tracks the generation, modification, and verification status of all hybrid PDF fixtures in this directory.

## Provenance (root-level `hybrid-0NN` corpus)

The ten `hybrid-0NN*.pdf` files at the top level of this directory are the
Phase 5.5 hybrid classification corpus (KU-2, `docs/plan/plan.md`). This
section covers them; the subdirectory fixtures below (`receipt-overtext` …
`complex-overlap`) are an earlier placeholder generation — see the
annotations in "Generation Summary" and "Generation Script".

**Canonical generator: `tools/generate_hybrid_fixtures.py`** (repo root).
Stdlib-only Python 3 — no reportlab / Pillow / img2pdf — and deterministic:
no wall-clock timestamps in the emitted bytes unless one is injected via
`--date`.

```
python3 tools/generate_hybrid_fixtures.py [--out-dir DIR] [--fixture NAME ...]
```

History: each sidecar's `source.generation_method` cites a
`hybrid-NNN-generator.py` (and, for hybrid-010, a
`hybrid-010-generator-enhanced.py` variant) that lived in this directory
until those scripts were removed as generator debris in commit `2014ee74`
(bead bf-24po9b). `tools/generate_hybrid_fixtures.py` is a stdlib-only
reimplementation built from the sidecar specs (`source.generation_method`
and `hybrid_behavior` per fixture). `README.md` and the sidecars still cite
the removed scripts; those citations are historical records, not pointers
to in-tree files.

**Byte-identity is NOT preserved.** Regenerated PDFs are structurally
equivalent, not byte-identical: each contains one 1-bit grayscale
(DeviceGray, 1 bpc, FlateDecode) image XObject as the scanned layer plus
vector text operators, with the per-fixture vector features its sidecar
describes (underline rectangles and red checkbox strokes for hybrid-002,
45° CTM-rotated watermark for hybrid-004, Bezier-circle stamp for
hybrid-006, multi-angle CTM-rotated overlays for hybrid-008, ExtGState
`ca`/`CA` 0.5/0.7 for hybrid-009, 12-curveto circles / rectangles / lines
and a transparency layer for hybrid-010). The committed corpus has NOT been
regenerated; do not expect hashes to match.

Known deviations from the originals (the originals' sidecars describe
simulated scan patterns, so the same class of approximation applies to
them):

- The scanned layer is a synthetic dark-run line pattern ("horizontal line
  pattern simulating scanned text"), not rendered text. Background imagery
  the sidecars describe in prose (e.g. hybrid-002's "employee information
  form layout", hybrid-007's tax-form dividers) is approximated by the
  generic pattern plus divider lines.
- Label/stamp wording is representative, not identical to the originals.
- Rotated text uses `q`/`cm`/`Q` CTM transforms — the hybrid-008 sidecar's
  "CTM transformations". The removed originals emitted malformed `Tm`
  operators instead (e.g. `-45 -25 Tm`, two operands).
- Verification at HEAD (2026-09): all ten fixtures regenerate
  byte-identically across runs and parse (`pdftract extract` / `pdftract
  hash`, rc=0). Eight of ten yield their vector text to `pdftract extract`;
  hybrid-004 and hybrid-008 carry genuinely rotated text, which trips a
  pre-existing extractor panic (`layout/line.rs:374` `Option::unwrap()`).
  That panic is reproducible from the committed corpus itself — the
  committed `hybrid-008-rotated-vector.pdf` triggers it at HEAD.

## Primary Phase 5.5 corpus

The ten root-level fixtures below are the KU-2 Phase 5.5 classifier corpus.
The compiled `hybrid_corpus` target reports the observed class and the
8x8-grid coverage supplied by each sidecar; it does not expose an independent
per-cell PDF extraction count or confidence value. File sizes are the bytes
observed in the committed PDFs at the validation HEAD.

| Fixture | Expected PageClass | Observed result | Observed file size (bytes) | verification_status |
|---|---|---|---:|---|
| `hybrid-001-vector-header-over-scan.pdf` | Hybrid | Vector; 8/64 cells (12.50%) | 1191 | failed |
| `hybrid-002-vector-form-over-scan.pdf` | Hybrid | Hybrid; 48/64 cells (75.00%) | 1507 | verified |
| `hybrid-003-mixed-column-layout.pdf` | Hybrid | Vector; 0/64 cells (0.00%) | 1647 | failed |
| `hybrid-004-watermark-over-scan.pdf` | Hybrid | Scanned; 64/64 cells (100.00%) | 1416 | failed |
| `hybrid-005-vector-footer-over-scan.pdf` | Hybrid | Vector; 8/64 cells (12.50%) | 1460 | failed |
| `hybrid-006-stamp-annotation.pdf` | Hybrid | Hybrid; 12/64 cells (18.75%) | 1461 | verified |
| `hybrid-007-textbox-overlay.pdf` | Hybrid | Hybrid; 28/64 cells (43.75%) | 1407 | verified |
| `hybrid-008-rotated-vector.pdf` | Hybrid | Hybrid; 24/64 cells (37.50%) | 1435 | verified |
| `hybrid-009-transparent-vector.pdf` | Hybrid | Hybrid; 20/64 cells (31.25%) | 1653 | verified |
| `hybrid-010-complex-layered.pdf` | Hybrid | Scanned; 56/64 cells (87.50%) | 2027 | failed |

The five `failed` rows are documented classifier-tuning inputs in the
verification note for `pdftract-2946c023`; they match the test's
`EXPECTED_MISMATCH` registry.

The targeted hybrid-corpus test passed at the validation HEAD. The repository
default `cargo test` definition-of-done command remains red on eight
pre-existing `pdftract-cli` unit-test failures; this does not affect the
fixture-specific result and is recorded in the verification note.

## Fixture Metadata

Each hybrid fixture entry includes:
- `name`: Fixture directory name
- `description`: One-line summary of the hybrid case
- `vector_regions`: Description of vector text content
- `scanned_regions`: Description of scanned/image content
- `hybrid_cells_approx`: Approximate number of image-heavy cells (out of 64)
- `overlap_type`: How vector and scanned regions interact (separate/partial/complete)
- `test_focus`: What aspect of the hybrid pipeline this tests
- `generation_date`: When the fixture was generated
- `verification_status`: pending | verified | failed
- `notes`: Any additional context

## Fixtures

### receipt-overtext
- **description**: Scanned receipt body with vector price overlay text
- **vector_regions**: Price totals, tax calculations, subtotal (bottom 25% of page, rows 5-7)
- **scanned_regions**: Receipt body, line items, merchant info (top 75% of page, rows 0-4)
- **hybrid_cells_approx**: 24 cells (rows 5-7, all 8 cols)
- **overlap_type**: partial (vector prices over scanned totals area)
- **test_focus**: Merge rule with overlapping vector/OCR on price区域; vector confidence priority
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Classic e-receipt format where receipt is scanned but totals are overlaid as vector for machine readability. **Placeholder PDF created - requires reportlab for full hybrid generation.**

### letterhead-image
- **description**: Vector letterhead header + scanned letter body
- **vector_regions**: Company name, logo, address, contact info, date (top 15%, rows 0-1, cols 0-7)
- **scanned_regions**: Letter content, salutation, body paragraphs, signature (bottom 85%, rows 1-7)
- **hybrid_cells_approx**: 40 cells (rows 1-7, all cols, partial row 1)
- **overlap_type**: separate (clear boundary between header and body)
- **test_focus**: Header extraction precision; OCR on body only; non-overlapping merge
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Common business letter format; tests that vector header doesn't trigger OCR on header cells

### form-mixed
- **description**: Vector form fields over scanned form background
- **vector_regions**: Fillable text fields, checkboxes, dropdown indicators (scattered cells, ~15%)
- **scanned_regions**: Form labels, instructions, background design, field borders (most cells, ~85%)
- **hybrid_cells_approx**: 45 cells (majority of page, scattered vector overlay)
- **overlap_type**: partial (vector fields over scanned labels)
- **test_focus**: Scattered vector extraction through cell-level OCR; complex merge patterns
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Simulates PDF forms where the layout is scanned but fillable fields are vector overlays

### invoice-stamp
- **description**: Vector invoice line items + scanned approval stamp
- **vector_regions**: Invoice header, line items table, totals, calculations (80% of page)
- **scanned_regions**: Approval stamp, signature, handwritten note (20% overlap in bottom-right)
- **hybrid_cells_approx**: 12 cells (bottom-right corner where stamp overlaps)
- **overlap_type**: partial (stamp overlaps some vector totals)
- **test_focus**: High-confidence vector vs OCR merge; stamp region OCR priority
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Tests that high-confidence vector content is not replaced by OCR of overlapping stamp/signature

### document-annotation
- **description**: Scanned document with vector highlight annotations
- **vector_regions**: Highlight boxes (transparent yellow), margin notes, arrows (overlay layer)
- **scanned_regions**: Original document content, paragraphs, text (background layer)
- **hybrid_cells_approx**: 36 cells (most of page has highlights or annotations)
- **overlap_type**: complete (annotations cover entire page)
- **test_focus**: OCR priority for underlying content vs vector annotations; annotation preservation
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Simulates annotated academic papers; tests that OCR captures text under highlights while preserving annotation spans

### figure-caption
- **description**: Academic paper with vector figure caption + scanned figure
- **vector_regions**: Figure number, caption text, reference markers (bottom 10%, row 7)
- **scanned_regions**: Figure content, chart, graph, axes (top 90%, rows 0-6)
- **hybrid_cells_approx**: 8 cells (figure area only, rows 0-6)
- **overlap_type**: separate (clear boundary between figure and caption)
- **test_focus**: Precise caption extraction; figure OCR accuracy; minimal cell coverage
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Tests hybrid detection on low-hybrid-cell-count pages (8 cells = 12.5%, just below 15% threshold if miscounted)

### sidebar-image
- **description**: Newsletter with vector main text + scanned sidebar image
- **vector_regions**: Main article text, headlines, byline (70% width, left side, cols 0-4)
- **scanned_regions**: Sidebar image, photo, caption (30% width, right side, cols 5-7)
- **hybrid_cells_approx**: 24 cells (rightmost 3 columns, all rows)
- **overlap_type**: separate (vertical split, no overlap)
- **test_focus**: Column-aware hybrid cell detection; side-by-side merge without conflicts
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Tests column detection with hybrid content; verifies OCR runs only on sidebar columns

### watermark
- **description**: Vector text over scanned watermark background
- **vector_regions**: Main document text, paragraphs, headings (foreground, high contrast)
- **scanned_regions**: Watermark, logo, background pattern (background, page-wide, low opacity)
- **hybrid_cells_approx**: 64 cells (full page, watermark is page-wide)
- **overlap_type**: complete (watermark underlies all text)
- **test_focus**: Vector confidence vs OCR with low-contrast background; maximum hybrid cell count
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Worst-case for hybrid cell count (100% cells); tests that vector text is extracted despite watermark background

### multi-column-scan
- **description**: Multi-column document with vector headers + scanned columns
- **vector_regions**: Column headers, section titles, page number (top 20%, rows 0-1)
- **scanned_regions**: Multi-column body content, paragraphs (bottom 80%, rows 1-7)
- **hybrid_cells_approx**: 48 cells (body area, rows 1-7, all cols)
- **overlap_type**: partial (headers over first line of scanned content)
- **test_focus**: Column detection + hybrid cell grid alignment; multi-column OCR
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Newsletter/magazine format; tests that column detection works correctly when columns are hybrid

### complex-overlap
- **description**: Interleaved vector and scanned regions (checkerboard pattern)
- **vector_regions**: Scattered blocks in checkerboard pattern (32 alternating cells)
- **scanned_regions**: Complementary blocks in complementary checkerboard (32 other cells)
- **hybrid_cells_approx**: 32 cells (exactly half the page, every other cell)
- **overlap_type**: partial (checkerboard boundaries have mini-overlaps)
- **test_focus**: Worst-case merge rule performance; complex bbox overlap calculation
- **generation_date**: 2024-08-03
- **verification_status**: pending
- **notes**: Stress test for merge algorithm; 32 vector spans + 32 OCR regions with alternating pattern

## Generation Summary

- **Total fixtures**: 10
- **Generated (placeholder)**: 10
- **Generated (full)**: 0
- **Verified**: 0
- **Failed verification**: 0

**Note**: All fixtures have been generated as placeholder PDFs with ground truth .txt files and specification READMEs. For production-quality hybrid PDFs with proper vector+scan overlap, install `reportlab`/`Pillow`/`img2pdf` and run `generate_hybrid_fixtures.py`.

> **[Provenance update, 2026-09]** This note and the "Generation Script"
> section below apply only to the ten *subdirectory* placeholder fixtures
> (`receipt-overtext` … `complex-overlap`), which remain placeholders. The
> in-directory `generate_hybrid_fixtures.py` they cite was removed in
> commit `2014ee74` and has not been restored. The root-level `hybrid-0NN`
> corpus does **not** depend on reportlab; it is regenerated with
> `tools/generate_hybrid_fixtures.py` — see "Provenance" above.

## Generation Script

Run `python3 generate_hybrid_fixtures.py` to generate all pending fixtures.

The script will:
1. Create vector content using reportlab
2. Create scanned content by rendering text to images
3. Combine vector + scanned content per fixture specifications
4. Apply strategic overlap per fixture design
5. Output PDF + .txt ground truth for each fixture

> **[Provenance update, 2026-09]** This section is stale as written: it
> describes the removed in-directory `generate_hybrid_fixtures.py` (deleted
> in `2014ee74`), and reportlab is unavailable on the build machine. For
> the root-level `hybrid-0NN` corpus use
> `python3 tools/generate_hybrid_fixtures.py` (stdlib-only — see
> "Provenance").

## Verification Checklist

For each fixture, verify:

- [ ] PDF generates successfully without errors
- [ ] PDF is valid (passes `pdfinfo` and `pdfextract -v`)
- [ ] Vector text is selectable (pdfextract can extract it)
- [ ] Scanned regions require OCR (not extractable as vector)
- [ ] Hybrid cell count is ≥ 12 (15% threshold)
- [ ] Hybrid cells are correctly identified by classifier
- [ ] Merge rule produces no duplicate text
- [ ] Ground truth .txt matches expected extraction

## Classification Targets

Expected classification for each fixture:

| Fixture | Expected PageClass | Expected hybrid_cells | Expected cell confidence |
|---------|-------------------|------------------------|--------------------------|
| receipt-overtext | Hybrid | ~24 | 0.70-0.85 |
| letterhead-image | Hybrid | ~40 | 0.75-0.90 |
| form-mixed | Hybrid | ~45 | 0.65-0.80 |
| invoice-stamp | Hybrid | ~12 | 0.60-0.75 |
| document-annotation | Hybrid | ~36 | 0.70-0.85 |
| figure-caption | Hybrid | ~8 | 0.55-0.70 |
| sidebar-image | Hybrid | ~24 | 0.65-0.80 |
| watermark | Hybrid | ~64 | 0.80-0.95 |
| multi-column-scan | Hybrid | ~48 | 0.70-0.85 |
| complex-overlap | Hybrid | ~32 | 0.60-0.75 |

## Test Coverage

This fixture suite covers:

- **Hybrid cell threshold**: 8 to 64 cells (12.5% to 100% of page)
- **Overlap types**: separate, partial, complete
- **Vector confidence**: low (0.2), medium (0.5), high (0.9)
- **Merge patterns**: scattered, columnar, checkerboard, full-page
- **Real-world formats**: receipts, letters, forms, invoices, academic papers, newsletters
- **Edge cases**: minimum hybrid cells (figure-caption), maximum hybrid cells (watermark)

## Related Documentation

- `README.md`: Usage and test scenario documentation (its `hybrid-NNN-generator.py` citations are historical — see "Provenance")
- `tools/generate_hybrid_fixtures.py`: Canonical generator for the root-level `hybrid-0NN` corpus
- `docs/plan/plan.md` KU-2: Known Unknown this fixture suite resolves
- Phase 5.5: Page classifier tuning using these fixtures
