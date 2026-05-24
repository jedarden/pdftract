# pdftract-oh30a: Per-page readability aggregation (median weighted by char count)

## Implementation Summary

Implemented `aggregate_page_readability()` function that computes per-page readability as the char-weighted median of per-span scores.

### Files Changed

1. **Created** `crates/pdftract-core/src/layout/readability.rs`:
   - `ScoredSpan` trait for abstracting over different span representations
   - `aggregate_page_readability<T: ScoredSpan>()` function
   - Char-weighted median algorithm:
     - Collect `(score, char_count)` pairs from spans
     - Sort by score ascending
     - Compute cumulative character count
     - Return score at half-total-char point
   - Edge case handling: empty page (0.0), single span, all empty strings

2. **Modified** `crates/pdftract-core/src/layout/mod.rs`:
   - Added `pub mod readability;`
   - Exported `aggregate_page_readability` and `ScoredSpan`

3. **Modified** `crates/pdftract-core/src/schema/mod.rs`:
   - Added `readability: Option<f32>` field to `ExtractionQuality`
   - Updated `ExtractionQuality::new()` to initialize `readability: None`
   - Updated tests to include the new field

### Algorithm

The char-weighted median correctly weights longer spans more heavily:
- Sort spans by score (ascending)
- Walk sorted list accumulating character counts
- Return the score at the position where cumulative count exceeds half the total

Example from acceptance criteria:
- Spans: (100 chars, 0.9), (10 chars, 0.5), (100 chars, 0.8)
- Sorted: 0.5(10), 0.8(100), 0.9(100)
- Cumsum: 10, 110, 210
- Half = 105
- Score at cumsum >= 105 is **0.8** ✓

### Test Results

All readability module tests PASS (15/15):
- ✓ `test_single_span` - Single span returns its score
- ✓ `test_empty_page` - Empty page returns 0.0
- ✓ `test_all_unscored_spans` - No scored spans returns 0.0
- ✓ `test_mixed_scored_unscored` - Unscored spans excluded
- ✓ `test_char_weighted_median_example` - AC example from bead
- ✓ `test_char_weighted_median_even_split` - Equal spans
- ✓ `test_all_same_score` - All same score returns that score
- ✓ `test_empty_strings` - All empty strings returns 0.0
- ✓ `test_unicode_char_count` - Counts Unicode code points correctly
- ✓ `test_longer_span_dominates` - Long spans dominate median
- ✓ `test_all_perfect_scores` - All 1.0 returns 1.0
- ✓ `test_all_zero_scores` - All 0.0 returns 0.0
- ✓ `test_order_preservation` - Result independent of input order
- ✓ `test_nan_score_handling` - NaN scores handled gracefully
- ✓ `test_zero_width_joiner` - Combining marks counted correctly

### Validation

- [x] Code compiles: `cargo check --all-targets` ✓
- [x] All layout tests pass: `cargo test --lib layout` ✓ (53/53 passed)
- [x] All schema tests pass: `cargo test --lib schema` ✓ (26/26 passed)
- [x] Algorithm matches acceptance criteria exactly

### Commit

Files to commit:
- `crates/pdftract-core/src/layout/readability.rs` (new)
- `crates/pdftract-core/src/layout/mod.rs` (modified)
- `crates/pdftract-core/src/schema/mod.rs` (modified)

### Closing the bead

All acceptance criteria PASS:
- ✓ Page with 1 span of 100 chars at score 0.9: page score = 0.9
- ✓ Page with 3 spans: (100 chars, 0.9), (10 chars, 0.5), (100 chars, 0.8): char-weighted median = 0.8
- ✓ Empty page: page score = 0.0 (default)
- ✓ All-perfect spans: page score = 1.0

Ready to close.
