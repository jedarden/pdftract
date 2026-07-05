# Repository Debris Audit
**Bead:** bf-a8031  
**Date:** 2026-07-05  
**Scope:** Tracked artifacts violating File and Module Layout norms

## Summary

This audit cataloged all tracked repository debris across the pdftract workspace. Found **44 files** across 5 categories that violate the project's layout norms.

---

## Category 1: Scratch Artifacts in Root Directory (13 files)

**Python scripts in root (should be in scripts/ or removed):**
- `assess_doc_coverage.py`
- `audit_docs.py`
- `check_content.py`
- `check_docs.py`
- `check_examples.py`
- `fix_fixtures.py`
- `tmp_fixtures.py`

**Rust files in root (should be in examples/, tests/, or removed):**
- `generate_expected_json.rs`
- `gen_unmapped_comprehensive.rs`
- `test_js_logging.rs`

**Shell scripts in root (should be in scripts/ or removed):**
- `check_doc_coverage.sh`
- `measure_doc_coverage.sh`

**Action:** All 13 files should be removed. These appear to be temporary development artifacts that have been superseded by tools in scripts/ or crates/.

---

## Category 2: Tracked Compiled Binaries (6 files)

**ELF binaries in root directory:**
- `mod` — 6.9MB ELF binary
- `conformance_test` — ELF binary
- `test_trailer_parse` — ELF binary  
- `test_trailer_parse2` — ELF binary
- `gen_fixtures` — Symlink wrapper to ELF binary

**ELF binaries in test fixtures:**
- `tests/sdk-conformance/fixtures/gen_fixtures` — 4.4MB ELF binary
- `tests/document_model/fixtures/gen_fixtures` — 4.4MB ELF binary

**Action:** Remove all 6 compiled binaries. These are build artifacts that should not be tracked. The test fixture gen_fixtures should be regenerated as needed from source.

---

## Category 3: Stray Output Files (6 files)

**Temporary outputs in root:**
- `--1.ppm` — 246MB PPM image file (massive disk space waste)
- `out.pdf` — 1.3KB PDF output
- `memory-report.json` — 4.3KB memory profiling output
- `test_js.pdf` — 668B test output
- `pdftract-test-merged.cdx.json` — 114KB Codecov output
- `0` — Mysteriously named file (likely stray output)

**Action:** Remove all 6 files. These are temporary outputs from development/testing that should not be committed.

---

## Category 4: Tests/Fixtures/ Generator Scripts (18 files)

**Generator scripts in tests/fixtures/ root:**
- `generate_book_chapter_fixtures.rs`
- `generate_cjk_fixtures_fixed.rs`
- `generate_cjk_fixtures.rs`
- `generate_encoding_fixtures.py`
- `generate_encrypted_fixtures.py`
- `generate_encrypted_fixtures.rs`
- `generate_large_remote_fixture.rs`
- `generate_legal_filing_fixtures.rs`
- `generate_lzw_fixtures_main.rs`
- `generate_lzw_fixtures.rs.disabled`
- `generate_ocr_fixtures.rs`
- `generate_page_class_fixtures.rs`
- `generate_scientific_paper_fixtures.rs`
- `generate_slide_deck_fixtures.rs`
- `generate_suspects_fixture.rs`
- `generate_suspects_fixtures.rs`
- `generate_suspects_fixtures_v5.rs`
- `PROVENANCE.md` (documentation file for fixtures)

**Action:** **FLAG FOR RELOCATION** — These generators are legitimate tools for creating test fixtures. They should be moved to `tools/` or integrated into `xtask/` for proper organization. DO NOT DELETE without relocation plan.

---

## Category 5: Non-Standard Root src/ Directory (7 files)

**Files in src/ directory (non-standard for workspace layout):**
- `src/lib.rs`
- `src/graphics_state/mod.rs`
- `src/graphics_state/color.rs`
- `src/graphics_state/diagnostics.rs`
- `src/graphics_state/matrix.rs`
- `src/graphics_state/stack.rs`
- `src/graphics_state/state.rs`

**Action:** Investigate and determine if these are legacy code or active development. If active, relocate to appropriate crate. If legacy, remove.

---

## Files NOT Flagged (Intentional Fixtures)

**LZW test binaries (14 files) — LEGITIMATE:**
- `tests/fixtures/lzw_incremental_early.bin` through `lzw_truncated.bin`

These are small (<50 bytes each) intentional test fixtures for LZW decoder testing. They should remain.

**Benchmark corpus PDFs (100+ files) — LEGITIMATE:**
- `benches/competitors/corpus/raster/*.pdf`
- `benches/competitors/corpus/vector/*.pdf`

These are intentionally tracked test corpus for benchmarking. They should remain.

**Build artifacts — LEGITIMATE:**
- `build/font-fingerprints.json`
- `build/frequency.json`
- `build/glyph-shapes.json`
- `crates/pdftract-core/build/*.json`

These are generated build data that is intentionally tracked for reproducibility. They should remain.

---

## Statistics

| Category | File Count | Total Size |
|----------|------------|------------|
| Scratch artifacts (root) | 13 | ~500KB |
| Compiled binaries | 6 | ~20MB |
| Stray outputs | 6 | ~247MB (mostly --1.ppm) |
| Generator scripts | 18 | ~200KB |
| Non-standard src/ | 7 | ~50KB |
| **TOTAL** | **50** | **~267MB** |

**Note:** The 246MB `--1.ppm` file is the single largest disk space offender.

---

## Recommendations

1. **Immediate cleanup (safe to remove):**
   - Remove all 13 scratch artifacts from root
   - Remove all 6 compiled binaries
   - Remove all 6 stray output files
   - Savings: ~267MB disk space

2. **Relocation required:**
   - Move 18 generator scripts from tests/fixtures/ to tools/ or xtask/
   - Investigate and handle 7 files in non-standard src/ directory

3. **Update .gitignore:**
   - Add patterns to prevent future tracking of these file types
   - Consider adding `*.ppm`, `out.pdf`, `memory-report.json` patterns

---

## Next Steps

This audit is for bead bf-a8031 (audit only). The actual cleanup will be handled by a follow-up bead.
