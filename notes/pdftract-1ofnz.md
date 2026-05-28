# pdftract-1ofnz: RTL direction detection (unicode-bidi majority bidi class)

## Summary

Implemented `detect_line_direction(line_text) -> LineDirection` function in `crates/pdftract-core/src/layout/line.rs`.

## Implementation Details

**Location:** `crates/pdftract-core/src/layout/line.rs:458-496`

**Algorithm:**
1. Walk each character in the text
2. Count L (Left-to-Right) vs R/AL (Right-to-Left/Arabic Letter) using `unicode_bidi::bidi_class`
3. All other bidi classes (EN, ES, ET, AN, CS, NSM, BN, B, S, WS, ON, etc.) are ignored per INV
4. Return:
   - `LineDirection::Ltr` if LTR count > RTL count OR both counts are zero (empty/neutral-only)
   - `LineDirection::Rtl` if RTL count > LTR count
   - `LineDirection::Mixed` if counts are equal (and both > 0)

**Key design decision:** Empty strings and neutral-only text (digits, punctuation) default to Ltr per bead acceptance criteria.

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| "Hello, World!" -> Ltr | PASS | Test: `test_detect_line_direction_latin_text` |
| "مرحبا بالعالم" -> Rtl | PASS | Test: `test_detect_line_direction_arabic_text` |
| Mixed Latin+Arabic: Mixed or dominant | PASS | Tests: `test_detect_line_direction_mixed_latin_arabic`, `test_detect_line_direction_latin_more_than_arabic`, `test_detect_line_direction_arabic_more_than_latin` |
| "123 456" digits only: Ltr default | PASS | Test: `test_detect_line_direction_digits_only` |
| "" -> Ltr | PASS | Test: `test_detect_line_direction_empty_string` |

## Additional Test Coverage

- `test_detect_line_direction_punctuation_only`: Punctuation-only text -> Ltr
- `test_detect_line_direction_latin_dominant`: Latin with punctuation/digits -> Ltr
- `test_detect_line_direction_arabic_dominant`: Arabic with digits -> Rtl
- `test_detect_line_direction_hebrew_text`: Hebrew text -> Rtl
- `test_detect_line_direction_cyrillic_text`: Cyrillic text -> Ltr

## Tests Executed

```bash
cargo nextest run --package pdftract-core --lib 'layout::line::tests::test_detect_line_direction'
```

**Result:** 12/12 tests passed (all RTL direction detection tests)
**Module tests:** 44/44 tests passed (entire line module)

## Code Changes

**Files modified:**
1. `crates/pdftract-core/src/layout/line.rs`: Added `detect_line_direction` function with comprehensive documentation and tests
2. `crates/pdftract-core/src/layout/header_footer.rs`: Fixed pre-existing compilation error (removed nonexistent `reading_order_rank` field from test helper)

**Commit:** `4ab89e1` feat(pdftract-1ofnz): implement detect_line_direction with unicode-bidi

## INV Compliance

- Numerals are bidi-neutral and do not drive direction
- Punctuation is neutral
- Empty lines default to Ltr

## References

- Plan section: Phase 4.2 RTL detection (line 1668)
