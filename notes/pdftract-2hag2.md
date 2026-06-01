# Verification Note: pdftract-2hag2

## Bead: Signal evaluator: all_tr3_with_full_page_image (Tr=3 + image >= 95% page -> BrokenVector definitive)

## Summary

The `all_tr3_with_full_page_image` signal evaluator was implemented in commit `39ca6a3` as part of bead pdftract-2b7ff (image_coverage_fraction). The implementation is correct and all acceptance criteria are met.

## Implementation Location

- **File**: `crates/pdftract-core/src/classify.rs`
- **Function**: `pub fn all_tr3_with_full_page_image(ctx: &PageContext) -> Option<Vote>` (lines 339-357)
- **Tests**: Lines 2459-2695

## Acceptance Criteria Verification

### 1. text_op_count=10, tr3_op_count=10, full_page_image=true → Some(Vote { 0.99, BrokenVector })
- **Test**: `test_all_tr3_with_full_page_image_exact_match` (line 2462)
- **Status**: PASS

### 2. text_op_count=10, tr3_op_count=5 → None (mix of Tr=3 and visible)
- **Test**: `test_all_tr3_with_full_page_image_mixed_tr3` (line 2516)
- **Status**: PASS

### 3. text_op_count=0 → None (no text)
- **Test**: `test_all_tr3_with_full_page_image_no_text` (line 2532)
- **Status**: PASS

### 4. full_page_image=false → None
- **Test**: `test_all_tr3_with_full_page_image_no_full_page_image` (line 2548)
- **Status**: PASS

### 5. Unit tests
- **Total tests**: 10 tests covering all edge cases
- **Additional tests**:
  - Exactly 95% coverage (threshold edge case)
  - Just below 95% threshold
  - Multiple images (one large enough)
  - Zero page area (division by zero guard)
  - Empty image areas
  - Definitive short-circuit verification
  - Standard US Letter and A4 page sizes
- **Status**: PASS

## Implementation Review

### Signature
```rust
pub fn all_tr3_with_full_page_image(ctx: &PageContext) -> Option<Vote>
```
**Status**: ✓ Correct

### Logic
```rust
let all_tr3 = ctx.text_op_count > 0 && ctx.tr3_op_count == ctx.text_op_count;
let page_area = ctx.width * ctx.height;
let full_page_image = if page_area > 0.0 {
    ctx.image_xobject_areas.iter().any(|&area| area / page_area >= 0.95)
} else {
    false
};
if all_tr3 && full_page_image {
    return Some(Vote::broken_vector(0.99));
}
None
```
**Status**: ✓ Correct

### Key Features
- All text operators must be Tr=3 (not just some) - enforced by `tr3_op_count == text_op_count`
- Single image XObject covering >= 95% of page area - uses `iter().any()` to check if ANY image meets threshold
- Definitive strength 0.99 for short-circuit behavior in PageClassifier
- Division by zero guard when `page_area <= 0.0`
- Returns `None` for all non-matching cases

## Integration

The signal is integrated into the PageClassifier via `InvisibleTextWithImageSignal` (lines 195-207):
```rust
struct InvisibleTextWithImageSignal;

impl SignalEvaluator for InvisibleTextWithImageSignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        all_tr3_with_full_page_image(ctx)
    }

    fn name(&self) -> &'static str {
        "all_tr3_with_full_page_image"
    }
}
```

## References

- EC-12: Tr=3 + full-page-image is a deterministic BrokenVector signal
- Plan section: Phase 5.1.2
- Commit: 39ca6a3 (feat(pdftract-2b7ff): implement image_coverage_fraction signal evaluator)

## Conclusion

**PASS**: All acceptance criteria are met. The implementation is correct, complete, and fully tested.
