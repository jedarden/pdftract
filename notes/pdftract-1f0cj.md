# pdftract-1f0cj: ID-to-EI raw-bytes scanner verification

## Summary

The ID-to-EI raw-bytes scanner (`scan_inline_image_data` in `crates/pdftract-core/src/parser/inline_image.rs`) is already fully implemented and meets all acceptance criteria.

## Implementation Details

**Location**: `crates/pdftract-core/src/parser/inline_image.rs:335-390`

### How it works

1. **Cursor positioning**: Starts immediately after the `ID` keyword and its required whitespace byte
2. **Scanning algorithm**: Byte-by-byte scan looking for pattern `[ws, 0x45, 0x49]` where:
   - `ws` is any PDF whitespace byte (0x00, 0x09, 0x0A, 0x0C, 0x0D, 0x20)
   - `0x45` is 'E', `0x49` is 'I'
3. **Returns**: `(image_bytes: Vec<u8>, bytes_consumed: usize)` where:
   - `image_bytes` excludes the preceding whitespace and EI itself
   - `bytes_consumed` includes everything from ID end to EI end
4. **Lexer advancement**: `lexer.skip_bytes(bytes_consumed as u64)` positions cursor after EI

### Key design decisions

- **Whitespace-preceded rule**: The EI delimiter must be preceded by whitespace per PDF spec 8.9.7. This distinguishes the terminator from spurious `0x45 0x49` sequences that may appear in compressed image data.
- **End-of-stream handling**: If no EI is found, the scanner returns all remaining bytes and emits `InlineImageNoEi` diagnostic. This handles malformed PDFs gracefully.
- **Empty image**: Valid per spec - `ID EI` immediately returns empty slice.

## Acceptance Criteria Verification

| Criterion | Status | Notes |
|-----------|--------|-------|
| `ABCD<ws>EI` → returns `b"ABCD"` | PASS | Test at line 868-876 |
| `ABCDEI<ws>EI` → returns `b"ABCDEI"` | PASS | Test at line 879-888 (inner EI not preceded by ws) |
| No EI → returns remaining bytes + diagnostic | PASS | Test at line 902-917 |
| Lexer positioned after EI | PASS | Test at line 973-985 |

## Test Coverage

The module includes comprehensive tests in `crates/pdftract-core/src/parser/inline_image.rs:749-986`:

- `test_scan_inline_image_data_basic` - Basic case
- `test_scan_inline_image_data_with_embedded_ei` - EI in data not preceded by ws
- `test_scan_inline_image_data_empty` - Empty image
- `test_scan_inline_image_data_no_ei` - No terminator
- `test_scan_inline_image_data_various_whitespace` - All 6 ws bytes
- `test_scan_inline_image_data_binary_content` - Binary data with 0x45/0x49 bytes
- `test_scan_inline_image_data_lexer_position` - Lexer advancement verification

## Known Limitations

Per the task description's "Critical considerations":

> Image data may contain the pattern `<ws>EI` SPURIOUSLY (e.g., a JBIG2 stream might have such bytes); this is RARE but possible. Acceptable solution: trust the spec's filter+dimensions-determine-length convention OR adopt the whitespace-EI heuristic and accept that malformed images may cause early termination. The plan picks the whitespace heuristic; document as a known limitation.

This implementation uses the whitespace-EI heuristic. In the rare case that compressed image data contains a literal `<ws>EI` sequence, the scanner will terminate early. A more robust solution would use the inline image header's width/height/bpc/colorspace to compute the exact expected byte length, but that is deferred to a future version (v0.2.0+ per ADR).

## References

- Plan section: Phase 3.5 Parsing (line 1610-1620)
- PDF spec: ISO 32000-1:2008, section 8.9.7 "Inline Images"
