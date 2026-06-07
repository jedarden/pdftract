# pdftract-4brcu: List Detection Implementation

## Summary

The list detection implementation was already complete in `crates/pdftract-core/src/layout/list.rs`. This task verified that the implementation meets all acceptance criteria.

## Implementation Details

### Location
- File: `crates/pdftract-core/src/layout/list.rs`
- Module: `pdftract_core::layout::list`
- Exported via: `crates/pdftract-core/src/layout/mod.rs`

### Key Functions

1. **`classify_list<S, L>(block: &Block<S>) -> bool`**
   - Returns `true` when ≥80% of block's lines start with bullet/numbered pattern
   - Empty blocks return `false`

2. **`starts_with_bullet(line_text: &str) -> bool`**
   - Pattern: `^\s*[•‣◦⁃\-\*]\s`
   - Matches Unicode bullets and ASCII marks

3. **`starts_with_number(line_text: &str) -> bool`**
   - Pattern: `^\s*\d+[.\)]\s`
   - Matches "1.", "2)", etc.

### Regex Patterns
```rust
BULLET_RE:    r"^\s*[•‣◦⁃\-\*]\s"
NUMBER_RE:    r"^\s*\d+[.\)]\s"
```

## Acceptance Criteria Verification

All acceptance criteria PASS:

| # | Criterion | Test | Result |
|---|-----------|------|--------|
| 1 | 3 "* Item" lines → List | `test_classify_list_three_bullet_items` | PASS |
| 2 | 3 "1. First/2. Second/3. Third" lines → List | `test_classify_list_three_numbered_items` | PASS |
| 3 | 1 "* Solo" line → List | `test_classify_list_single_bullet_item` | PASS |
| 4 | 4/5 "- " starts → List | `test_classify_list_four_of_five_bullet_items` | PASS |
| 5 | 2/5 "- " starts → NOT List | `test_classify_list_two_of_five_bullet_items` | PASS |

## Test Results

```
running 20 tests
test layout::list::tests::test_classify_list_empty_block ... ok
test layout::list::tests::test_classify_list_exactly_80_percent ... ok
test layout::list::tests::test_classify_list_four_of_five_bullet_items ... ok
test layout::list::tests::test_classify_list_just_below_80_percent ... ok
test layout::list::tests::test_classify_list_mixed_bullet_and_numbered ... ok
test layout::list::tests::test_classify_list_single_bullet_item ... ok
test layout::list::tests::test_classify_list_three_bullet_items ... ok
test layout::list::tests::test_classify_list_three_numbered_items ... ok
test layout::list::tests::test_classify_list_two_of_five_bullet_items ... ok
test layout::list::tests::test_classify_list_unicode_bullets ... ok
test layout::list::tests::test_classify_list_zero_matching ... ok
... (9 more helper tests for starts_with_bullet/starts_with_number)

test result: ok. 20 passed; 0 failed
```

## Notes

- Lettered (a., b.) and Roman (I., II.) lists are NOT covered in v0.1.0 (as per plan)
- Indented sub-bullets (nesting) is deferred (as per plan)
- Unicode bullets (•, ‣, ◦, ⁃) are matched as literal codepoints (per INV)

## Git Status

No file changes required - implementation was already complete and passing.
