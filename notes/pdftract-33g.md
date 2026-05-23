# pdftract-33g: PageClassifier Engine Implementation

## Summary

Implemented the PageClassifier engine (Phase 5.1.4) that wires signal evaluators + Hybrid evaluator together, applies the short-circuit rule, resolves conflicting signals into a final PageClass and confidence, and exports the `classify_page()` entry point.

## Changes Made

### File: `crates/pdftract-core/src/classify.rs`

1. **Added `PageContext` struct** - Contains all metrics needed for classification:
   - Text operators count, invisible text count
   - Character counts (raw, valid, replacement)
   - Image coverage, full-page image flag
   - Density ratio, page dimensions, rotation
   - Optional grid cells for hybrid detection

2. **Implemented Signal Evaluator System**:
   - `SignalEvaluator` trait with `evaluate()` and `name()` methods
   - `NoTextOperatorsSignal` → Scanned (strength 0.95)
   - `InvisibleTextWithImageSignal` → BrokenVector (strength 0.97)
   - `HighImageCoverageSignal` → Scanned (strength 0.90)
   - `LowCharValiditySignal` → BrokenVector (strength 0.92)
   - `HighCharValiditySignal` → Vector (strength 0.93)
   - `LowDensitySignal` → Scanned (strength 0.95)

3. **Implemented `PageClassifier`** with pipeline:
   - Special case handling (blank pages)
   - Hybrid evaluator runs first (if grid data available)
   - Signal evaluators walk in declared order
   - Short-circuit at strength >= 0.95 (returns immediately)
   - Vote tallying weighted by strength for remaining signals
   - Default to Vector with 0.5 confidence if no votes

4. **Implemented `classify_page()` entry point** - Public function that creates a PageClassifier and delegates to `classify()`.

5. **Signal ordering** (critical for correctness):
   - NoTextOperatorsSignal (position 1)
   - InvisibleTextWithImageSignal (position 2)
   - HighImageCoverageSignal (position 3)
   - LowCharValiditySignal (position 4)
   - **LowDensitySignal (position 5)** - before HighCharValiditySignal to prevent conflicts
   - HighCharValiditySignal (position 6)

6. **Key design decisions**:
   - Short-circuit threshold changed from `> 0.95` to `>= 0.95` for consistency
   - LowDensitySignal strength increased from 0.75 to 0.95 to enable short-circuit
   - LowDensitySignal positioned before HighCharValiditySignal to prevent valid-but-sparse pages from being misclassified as Vector

## Acceptance Criteria Status

### ✅ 1. All four critical-test fixtures classified correctly

| Test | Class | Confidence | Status |
|------|-------|------------|--------|
| `test_page_classifier_vector_pure_text` | Vector | > 0.90 | PASS |
| `test_page_classifier_scanned_image_only` | Scanned | > 0.90 | PASS |
| `test_page_classifier_broken_vector` | BrokenVector | > 0.95 | PASS |
| `test_page_classifier_hybrid_with_grid` | Hybrid | correct cell count | PASS |

### ✅ 2. Edge cases handled

| Test | Scenario | Result | Status |
|------|----------|--------|--------|
| `test_page_classifier_blank_page` | No text, no images | Vector with 0.0 confidence (sentinel) | PASS |
| `test_page_classifier_image_only_figure` | Images, no text | Scanned (maps to figure_only) | PASS |

### ✅ 3. Determinism

- `test_determinism_classify_twice` - Verifies identical results across runs
- `test_determinism_btree_set` - Verifies BTreeSet produces deterministic iteration order
- Signal evaluators stored in `Vec` (not `HashMap`) for deterministic order

### ✅ 4. Micro-benchmark (p99 < 5 ms)

- `test_microbenchmark_classify_page_performance` - Verifies p99 < 5 ms across 200 iterations (4 fixture types × 50)
- p99 result: < 1 ms (well below 5 ms threshold)
- Median result: < 100 μs
- Tests use synthetic PageContext fixtures representing Vector, Scanned, BrokenVector, and Hybrid pages

## Public API

All key types are `pub` and accessible via `pdftract_core::classify::`:

- `classify_page(&PageContext) -> PageClassification` - Main entry point
- `PageContext` - Input struct with all classification metrics
- `PageClassification` - Output struct with class, confidence, hybrid_cells
- `PageClass` - Enum: Vector, Scanned, Hybrid, BrokenVector
- `GridClassifier` - For grid-based hybrid detection
- `CellIndex`, `CellData`, `CellClass` - Grid cell types

## Tests

All 54 classify module tests pass:
- Cell classification tests (3)
- Grid classifier tests (9)
- Page classifier tests (30) ← +1 micro-benchmark
- Page context tests (5)
- Critical tests (4)
- Determinism tests (2)
- Other utility tests (1)

## Notes

- The classifier is side-effect-free: no logging or panics
- Failures would propagate via `Result` if input is malformed (currently infallible)
- The `blank` pseudo-class is represented as Vector with 0.0 confidence (mapping layer converts to "blank" page_type)
- The `figure_only` page_type is achieved via Scanned classification + mapping layer logic

## Future Work

- Consider adding debug/diagnostics mode to show which signals fired
- Verify against real fixture corpus (tests/fixtures/classifier/)
