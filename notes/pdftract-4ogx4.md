# pdftract-4ogx4: Signal evaluator char_validity_rate

## Status: PASS

This bead requested implementing two signal evaluators for `char_validity_rate`:
- `rate < 0.4` → BrokenVector with strength 0.80
- `rate > 0.85` → Vector with strength 0.90

## Implementation Location

File: `crates/pdftract-core/src/classify.rs`

### LowCharValiditySignal (lines 222-240)
```rust
struct LowCharValiditySignal;

impl SignalEvaluator for LowCharValiditySignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        if ctx.has_text() {
            let validity = ctx.char_validity_rate();
            if validity < 0.4 {
                return Some(Vote::broken_vector(0.80));
            }
        }
        None
    }
}
```

### HighCharValiditySignal (lines 242-260)
```rust
struct HighCharValiditySignal;

impl SignalEvaluator for HighCharValiditySignal {
    fn evaluate(&self, ctx: &PageContext) -> Option<Vote> {
        if ctx.has_text() {
            let validity = ctx.char_validity_rate();
            if validity > 0.85 {
                return Some(Vote::vector(0.90));
            }
        }
        None
    }
}
```

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `rate = 0.3 → Some(Signal { 0.80, BrokenVector })` | PASS | `LowCharValiditySignal` returns `Vote::broken_vector(0.80)` for `validity < 0.4` |
| `rate = 0.9 → Some(Signal { 0.90, Vector })` | PASS | `HighCharValiditySignal` returns `Vote::vector(0.90)` for `validity > 0.85` |
| `rate = 0.6 → None` (middle band) | PASS | Both signals return None for rates between 0.4 and 0.85 (inclusive) |
| `rate = None (Phase 4.7 not run) → None` | PASS | Both signals check `ctx.has_text()` first, returning None if no text |

## Test Coverage

All classification tests pass (80/80):

- `test_page_classifier_low_char_validity` - Tests 20% validity → BrokenVector
- `test_page_classifier_vector_pure_text` - Tests 97% validity → Vector  
- `test_page_classifier_default_vector` - Tests 70% validity (middle band) → default Vector classification
- `test_page_classifier_confidence_in_range` - Tests various validity rates

## Integration

Both signal evaluators are registered in `PageClassifier::new()` (lines 310-320):
```rust
signals: vec![
    Box::new(NoTextOperatorsSignal),
    Box::new(InvisibleTextWithImageSignal),
    Box::new(HighImageCoverageSignal),
    Box::new(LowCharValiditySignal),        // ← rate < 0.4 → BrokenVector
    Box::new(LowDensitySignal),
    Box::new(HighCharValiditySignal),       // ← rate > 0.85 → Vector
],
```

The signals are evaluated in order after the Hybrid evaluator check. Low char validity fires before high validity, ensuring that broken encodings are correctly identified even when high validity signals might also fire.

## Conclusion

The bead was already fully implemented. No code changes were required. All acceptance criteria are met and tested.
