# Verification Note: pdftract-5o6hx

## Bead: Hyphenation repair (end-of-line hyphen + next line first word -> joined, hyphen stripped)

## Implementation Summary

Implemented `repair_hyphenation(block: &mut Block<S>, column_width: f64) -> u32` in `crates/pdftract-core/src/layout/correction.rs` that:

1. **Detects hyphenation** within consecutive line pairs in a block:
   - Line N's last span text ends with `-`, `‐` (U+2010), `‑` (U+2011), or soft hyphen (U+00AD)
   - Line N's last span bbox.x1 is within `0.05 * column_width` of column right edge
   - Line N+1's first span text starts with a lowercase letter (continuation)
   - Both spans are in the same column

2. **Repairs by joining**:
   - Strips trailing hyphen from line N's last word
   - Prepends stripped word to line N+1's first word
   - Updates line N's last span text with `joined_word + " "`
   - Removes first word from line N+1's first span
   - Cleans up empty spans/lines

3. **Returns count** of repairs performed (u32)

## Files Modified

- `crates/pdftract-core/src/layout/correction.rs`: Added `repair_hyphenation` function, `HasBBox` trait, `HyphenableSpan` trait, and test infrastructure
- `crates/pdftract-core/src/layout/mod.rs`: Exported `repair_hyphenation` and `HyphenableSpan`
- `crates/pdftract-core/src/schema/mod.rs`: Fixed test cases to include `column: None` field (pre-existing issue)

## Key Implementation Details

### Traits
- `HasBBox`: Provides bbox access for position-based detection
- `HyphenableSpan`: Combines `CorrectableText` + `HasBBox` for spans needing hyphenation repair
- Blanket implementation allows any span type implementing both traits to work

### Borrow Checker Safety
- Extracts data first before mutations to avoid multiple mutable borrows
- Uses separate scopes for current/next line mutations
- Calculates span indices separately to avoid double borrowing

### Hyphen Detection
Supports multiple hyphen types:
- ASCII hyphen `-`
- Unicode hyphen `‐` (U+2010)
- Non-breaking hyphen `‑` (U+2011)
- Soft hyphen (U+00AD)

## Invariants Enforced

✅ **INV**: do NOT join across blocks (function operates on single block)
✅ **INV**: capital-start of next line indicates NOT a continuation (checked)
✅ **INV**: mid-line hyphens (not at right edge) are NOT joined (checked via bbox)
✅ **INV**: lines in different columns are NOT joined (checked via column field)

## Test Coverage

Added 9 comprehensive tests:
1. `test_hyphenation_join_basic`: Basic join "hyphen-" + "ation" -> "hyphenation"
2. `test_hyphenation_capital_start_no_join`: Capital "More" -> no join
3. `test_hyphenation_not_at_right_edge`: Mid-line hyphen -> no join
4. `test_hyphenation_different_columns`: Different columns -> no join
5. `test_hyphenation_soft_hyphen`: Soft hyphen (U+00AD) -> joined
6. `test_hyphenation_non_breaking_hyphen`: Non-breaking hyphen (U+2011) -> joined
7. `test_hyphenation_empty_span_removed`: Empty span cleanup
8. `test_hyphenation_multi_word_continuation`: Multi-word continuation handling
9. `test_hyphenation_multiple_repairs`: Multiple repairs in same block

## Compilation Status

✅ `cargo check --lib` - PASSED
✅ `cargo clippy --lib -p pdftract-core` - PASSED (no warnings for correction module)
✅ `cargo fmt` - PASSED

Note: Test compilation has pre-existing errors in other modules (schema, stream) unrelated to this implementation.

## Integration

The function is exported via `crate::layout::correction::repair_hyphenation` and can be used in the correction pipeline (Phase 4.7) after mojibake repair.

## References

- Plan section: Phase 4.7 Correction pipeline step 2 (line 1796)
- Critical test: "Hyphenated word spanning line break: joined correctly, hyphen stripped" (line 1791)
