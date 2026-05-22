# pdftract-5cqy: Xref Stream Parser Implementation

## Summary

Implemented xref stream parser for PDF 1.5+ cross-reference streams with full support for:
- `/W` field widths (type_w, obj_w, gen_w)
- Type 0 (free), Type 1 (in-use), Type 2 (compressed in ObjStm) entries
- `/Index` subsection boundaries with default `[0 /Size]`
- Big-endian multi-byte field decoding
- Zero-width field handling
- FlateDecode decompression with PNG predictor support
- Proper error handling and diagnostics (INV-8 compliant)

## Implementation Location

- File: `crates/pdftract-core/src/parser/xref.rs`
- Function: `parse_xref_stream(source: &dyn PdfSource, stream_obj_offset: u64) -> XrefSection`
- Lines: 1252-1569

## Key Features

1. **Indirect object parsing**: Uses Phase 1.2's `ObjectParser::parse_indirect_object()` to read the xref stream object
2. **Stream decompression**: Uses Phase 1.5's `decode_stream()` for FlateDecode with predictor support
3. **Field width handling**: Supports any `/W [type_w obj_w gen_w]` configuration including zero-width fields
4. **Multi-subsection support**: Handles `/Index [first_1 count_1 first_2 count_2 ...]` arrays
5. **Big-endian decoding**: `read_big_endian_field()` helper for 1-8 byte fields
6. **Trailer dict extraction**: Copies relevant keys (Root, Info, ID, Encrypt, Prev) from stream dict

## Test Results

All xref stream tests pass:
- `test_parse_xref_stream_simple`: PASS - /W [1 4 2] /Index [0 6] with 6 entries
- `test_parse_xref_stream_multi_subsection`: PASS - /Index [0 3 100 2] produces correct entries
- `test_parse_xref_stream_type2_compressed`: PASS - Type-2 entries route through ObjStm
- `test_parse_xref_stream_field_width_zero_gen`: PASS - /W [1 4 0] (gen always 0)
- `test_parse_xref_stream_with_predictor`: PASS - FlateDecode + PNG predictor
- `test_parse_xref_stream_invalid_entry_type`: PASS - Unknown types emit diagnostics
- `test_parse_xref_stream_missing_size`: PASS - Emits appropriate diagnostic
- `test_parse_xref_stream_invalid_w_array`: PASS - Emits appropriate diagnostic
- `proptest_parse_xref_stream_no_panic`: PASS - Random bytes never panic
- `proptest_parse_xref_stream_random_offset_no_panic`: PASS - Random offsets never panic
- `test_debug_xref_stream_parsing`: PASS - Debug helper test

## INV-8 Compliance

Verified: No `unwrap()`, `expect()`, or `panic!()` in production xref stream parsing code.
- Line 206: `.unwrap_or(false)` is safe (handles poisoned lock gracefully)
- All other `unwrap()`/`panic!` calls are in `#[cfg(test)]` modules (allowed per INV-8)

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Simple test /W [1 4 2] /Index [0 6] | ✅ PASS | 6 entries decoded correctly |
| Type-2 ObjStm routing | ✅ PASS | Compressed entries parse correctly |
| Multi-subsection /Index [0 3 100 2] | ✅ PASS | Entries at 0,1,2,100,101 |
| Predictor (FlateDecode + PNG) | ✅ PASS | Stream decoder handles transparently |
| Field width /W [1 4 0] | ✅ PASS | Zero-width gen field defaults to 0 |
| proptest random bytes | ✅ PASS | No panics on random input |
| INV-8 maintained | ✅ PASS | No production panics |

## Integration Points

- **Phase 1.2**: Uses `ObjectParser::parse_indirect_object()` for reading the xref stream object
- **Phase 1.5**: Uses `decode_stream()` for decompression with filter/predictor support
- **Object resolution**: Type-2 entries return `XrefEntry::Compressed { obj_stm_nr, index }` for ObjStm resolver

## References

- Plan section: Phase 1.3 line 1089-1123 (xref streams)
- PDF spec 7.5.8 (Cross-Reference Streams)
- Bead: pdftract-5cqy
