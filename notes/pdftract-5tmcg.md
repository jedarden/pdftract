# pdftract-5tmcg: Page Tree Flattener with Inherited Attributes

## Summary

Implemented page tree flattener with inherited attribute resolution (MediaBox, CropBox, Resources, Rotate) plus content stream concatenation preparation.

## Implementation

The `flatten_page_tree` function in `crates/pdftract-core/src/parser/pages.rs` implements:

1. **Recursive page tree walk** with depth-first traversal
2. **Inherited attribute accumulator** tracking MediaBox, CropBox, Resources, Rotate across /Pages ancestors
3. **PageDict output** containing all resolved page attributes
4. **Error recovery** for malformed files

### Key Features

- **Cycle detection**: Uses HashSet<ObjRef> to detect circular references in /Kids arrays
- **Depth limiting**: MAX_PAGES_DEPTH = 16 to prevent stack overflow
- **EC-09 compliance**: Missing MediaBox defaults to US Letter (612 x 792 points) with STRUCT_MISSING_KEY diagnostic
- **Rotate validation**: Non-multiples of 90 are clamped to nearest multiple with STRUCT_INVALID_ROTATE diagnostic
- **Page count validation**: Cross-checks against /Count; emits STRUCT_INVALID_PAGE_COUNT on mismatch

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| 3-level /Pages inheritance | PASS | `test_flatten_three_level_inheritance` verifies grandparent MediaBox inheritance |
| EC-09: missing MediaBox defaults | PASS | `test_ec09_missing_mediabox_defaults_to_us_letter` |
| /Pages tree with cycles | PASS | `test_cycle_detection_in_page_tree` |
| /Rotate = 45 clamped to 0 | PASS | `test_invalid_rotate_clamped` |
| Page count validation | PASS | `test_page_count_mismatch` |
| proptest: random shapes never panic | PASS | All fuzz tests in proptests module |
| INV-8: no panics on invalid input | PASS | Proptests cover arbitrary PdfObject input |

## Files Modified

- `crates/pdftract-core/src/parser/pages.rs` - Added cycle detection test

## Tests

All 189 lib tests pass:
- 17 page-specific unit tests
- 4 property tests (fuzzing)
- All other modules unaffected
