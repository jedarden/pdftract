# pdftract-nf172: Phase 3.5 Inline Image skip (coordinator)

## Summary

Coordinator bead for Phase 3.5: detect and skip BI/ID/EI inline image sequences in content streams. All 3 children have been completed and the inline image parsing functionality is fully implemented.

## Children Completed

All 3 children are CLOSED:
- **pdftract-1sxpa**: BI/ID inline image header parser - CLOSED (commit 4ac8479)
- **pdftract-1f0cj**: ID-to-EI raw-bytes scanner with whitespace-preceded EI detection - CLOSED
- **pdftract-axcri**: Inline image -> ImageXObject record in page image list - CLOSED

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| All 3 children closed | **PASS** | `bf show` confirms all 3 children are closed |
| Inline image followed by text operators parsed correctly | **PASS** | `parse_inline_image()` in render.rs advances lexer past EI; subsequent tokens parse normally |
| Embedded "EI" bytes not treated as terminator | **PASS** | Test `test_scan_inline_image_data_with_embedded_ei` passes - input `b"ABCDEI\nEI"` correctly returns `b"ABCDEI"` (inner EI not preceded by ws) |

## Implementation Overview

The inline image implementation works as follows:

1. **Header parsing** (pdftract-1sxpa): `parse_inline_image_header()` parses BI...ID dictionary with shorthand key expansion
2. **Data scanning** (pdftract-1f0cj): `scan_inline_image_data()` byte-by-byte scan for whitespace-preceded EI
3. **ImageXObject recording** (pdftract-axcri): `collect_image_xobjects()` records both XObject and inline images with CTM-derived bbox

## Test Results

All 14 inline_image tests pass:
- `test_scan_inline_image_data_basic` - Basic BI...ID...EI parsing
- `test_scan_inline_image_data_with_embedded_ei` - EI in data not preceded by ws
- `test_scan_inline_image_data_empty` - Empty image
- `test_scan_inline_image_data_no_ei` - No terminator handling
- `test_scan_inline_image_data_various_whitespace` - All 6 ws bytes before EI
- `test_scan_inline_image_data_binary_content` - Binary data with 0x45/0x49 bytes
- `test_scan_inline_image_data_lexer_position` - Lexer advancement verification
- `test_parse_basic_header` - Basic header parsing
- `test_shorthand_expansion` - Shorthand key expansion
- `test_id_whitespace_validation` - ID whitespace requirement
- `test_parse_header_with_array_filter` - Array filter chains
- `test_parse_header_with_missing_value` - Malformed header recovery
- `test_inline_image_header_new` - Header construction
- `test_inline_image_header_has_required_fields` - Field presence

## Integration with Content Stream Parser

The `collect_image_xobjects()` function in `render.rs` integrates inline image parsing into the content stream interpreter:
- BI keyword triggers `parse_inline_image()` which consumes the entire BI/ID/EI sequence
- Lexer is positioned after EI, allowing subsequent text operators to parse correctly
- ImageXObject entries are added to page image list for Phase 4.4 figure detection

## Known Limitations

Per the plan's "Critical considerations":
- The whitespace-EI heuristic may terminate early if compressed image data contains `<ws>EI` (rare)
- A more robust solution would compute expected byte length from width/height/bpc/colorspace (deferred to v0.2.0+)

## References

- Plan section: Phase 3.5 Inline Images (lines 1592-1600)
- ISO 32000-1 sec 8.9.7
