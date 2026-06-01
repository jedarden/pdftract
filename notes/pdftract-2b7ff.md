# Verification Note: pdftract-2b7ff

## Bead: image_coverage_fraction signal evaluator

## Status: PASS ✅

The `image_coverage_fraction` signal evaluator was already implemented in the codebase at `/home/coding/pdftract/crates/pdftract-core/src/classify.rs` (lines 359-413).

## Implementation Details

### Signature
```rust
pub fn image_coverage_fraction(ctx: &PageContext) -> Option<Vote>
```

### Algorithm
1. Compute page area: `page_area_pt2 = ctx.width * ctx.height`
2. Guard against zero/negative page area (returns `None`)
3. Sum all `image_xobject_areas` to get total image coverage
4. Compute coverage fraction: `total_image_area / page_area_pt2`
5. Clamp to `[0.0, 1.0]` to handle overlapping images defensively
6. If `coverage_fraction > 0.85`: return `Some(Vote::scanned(0.85))`

### Trade-offs Documented in Code
The implementation uses `sum` instead of `union` for simplicity, with a clear comment noting:
- 5 overlapping copies of one image = sum of 5x area but union is 1x area
- This is acceptable for the 0.85 threshold (conservative signal)
- Revisit with Klee's algorithm (~O(N log N)) if accuracy demands

## Acceptance Criteria Verification

| AC | Status | Notes |
|---|--------|-------|
| Page with one image covering 90% area → Some(Vote { 0.85, Scanned }) | ✅ PASS | Lines 2698-2713 test this exact case |
| Page with multiple small images totaling 50% → None | ✅ PASS | Lines 2715-2730 test this exact case |
| Page with no images → None | ✅ PASS | Lines 2732-2743 test this exact case |
| Coverage clamped to 1.0 on overlapping images | ✅ PASS | Lines 2745-2766 test 5x overlapping images |

## Additional Tests Verified

| Test Case | Status |
|-----------|--------|
| Exactly 85% threshold (just above) | ✅ PASS (lines 2769-2783) |
| Just below 85% threshold | ✅ PASS (lines 2785-2797) |
| Zero page area | ✅ PASS (lines 2799-2810) |
| Negative page area | ✅ PASS (lines 2812-2823) |
| Multiple images totaling 90% | ✅ PASS (lines 2839-2856) |

## Conclusion

The implementation is complete, correct, and thoroughly tested. All acceptance criteria pass. The bead is ready to close.

## Files Reviewed

- `/home/coding/pdftract/crates/pdftract-core/src/classify.rs` - Main implementation (lines 359-413)
- `/home/coding/pdftract/crates/pdftract-core/src/classify.rs` - Tests (lines 2696-2889)
