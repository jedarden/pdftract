# pdftract-5vhp: Word Boundary Reconstruction Research Note

## Summary

Brought `docs/research/word-boundary-reconstruction.md` to v1.0 final-pass status with complete documentation of the adaptive word-boundary algorithm.

## Changes Made

### File: `docs/research/word-boundary-reconstruction.md`

**Before:** 202 lines
**After:** 899 lines

Added/expanded sections:

1. **Document Header** — Version 1.0, Final status, date, plan reference
2. **Section 3.1** — Tc/Tw/Tz Formula — Complete Specification
   - Explicit formula: `expected_advance = (w_g / 1000 * font_size + Tc + Tw_if_space) * Tz / 100`
   - Table of all parameters (w_g, font_size, Tc, Tw, Tw_if_space, Tz)
   - Critical implementation notes (Tz multiplicative, Tw only for U+0020, Tc universal)
3. **Section 3.2** — Text-Space vs. Device-Space Comparison
   - Per plan line 1550: comparison in text space before CTM transformation
4. **Section 4** — Complete rewrite with adaptive algorithm specification
   - Initial threshold: `0.25 * font_size`
   - After 20 glyphs: compute median, set threshold = `1.5 * median`
   - Outlier exclusion: filter gaps > `4.0 * threshold`
   - Recalibration rules (per-font reset, bootstrap behavior)
   - Why median (not mean) — robust against outliers
5. **Section 11** — Complete Algorithm — Pseudo-Code (NEW)
   - Data structures: `TextState`, `Glyph`, `WordBoundaryState`
   - Main loop: processing Tj, TJ, Td, TD, Tm, T*, Tf operators
   - Word boundary detection function
   - Adaptive threshold computation
   - Glyph advance calculation
6. **Section 12** — Edge Cases and Special Handling (NEW)
   - Zero-width joiners (ZWJ)
   - Combining marks
   - CJK text (script detection, disable adaptive threshold)
   - Justified text (variance detection)
   - Monospaced fonts (wider initial threshold)
   - Diagonal/rotated text (text-space comparison invariance)
   - RTL/bidi text (negative gaps)
   - Ligatures (no internal spaces)
   - Soft hyphens (conditional rendering)
   - Tab stops (layout gap classification)
7. **Section 13** — Validation Methodology (NEW)
   - Test corpus location: `tests/fixtures/word-boundary-corpus/` (141 PDFs)
   - Ground truth format (tokens, gaps in JSONL)
   - Validation metrics (precision, recall, F1, space error rate)
   - Per-category acceptance criteria (8 categories)
   - Regression test suite: `tests/word_boundary_test.rs`
   - Continuous validation (CI metrics artifact)
   - Manual validation for new PDFs
8. **Section 14** — Summary and Implementation Checklist (NEW)
   - Core algorithm recap
   - Implementation checklist (20 items)
   - Edge cases verified (10 items)
   - Validation status (corpus, integration test, metrics, CI)
   - References to plan lines and phases

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| docs/research/word-boundary-reconstruction.md updated with complete Tc/Tw/Tz formula | **PASS** | Section 3.1 with explicit formula and parameter table |
| Pseudo-code listing present | **PASS** | Section 11: Complete Algorithm — 5 functions with data structures |
| Edge cases called out (ZWJ, combining marks, CJK, justified text, monospaced) | **PASS** | Section 12: 10 edge cases with detailed handling |
| Validation methodology specified with corpus location | **PASS** | Section 13: corpus at `tests/fixtures/word-boundary-corpus/`, 141 PDFs, 8 categories |
| File grows to approx 350+ lines | **PASS** | 899 lines (from 202) |

## References

- Plan line 1529: adaptive threshold + Tc/Tw/Tz reference
- Plan line 1547: Word boundary threshold specification
- Plan line 1550: Comparison space (text space)
- Plan line 1551: Recalibration window scope (reset on font switch)
- Plan line 1552: Bootstrap behavior (first 20 glyphs)

## Commits

- `6b7a8c2 docs(pdftract-5vhp): bring word-boundary-reconstruction.md to v1.0 final-pass`

## Notes

- No code changes required; documentation-only bead.
- All acceptance criteria PASS.
- Document is now authoritative source of truth for Phase 3.2 word boundary reconstruction implementation.
