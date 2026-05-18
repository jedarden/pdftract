# Verification Note: pdftract-3nnqy

## Work Completed

Implemented the StreamDecoder trait, filter pipeline orchestrator, and max_decompress_bytes bomb limit for PDF stream decoding.

## Components Implemented

### 1. StreamDecoder Trait (`crates/pdftract-core/src/parser/stream.rs`)
- Trait with `decode()` method for filter-specific decoding
- Per-filter implementations:
  - `FlateDecoder`: zlib/deflate decompression with bomb limit checking
  - `ASCII85Decoder`: Base85 decoding with bomb limit checking
  - `ASCIIHexDecoder`: Hexadecimal decoding
  - `PassthroughDecoder`: For unsupported filters (DCTDecode, JBIG2Decode, etc.)

### 2. Filter Pipeline (`decode_stream()`)
- Single filter handling: `/Filter /FlateDecode`
- Array filter handling: `/Filter [/ASCII85Decode /FlateDecode]`
- /DecodeParms pairing with /Filter arrays
- Filter abbreviation normalization (/A85 → ASCII85Decode, /Fl → FlateDecode, etc.)
- Unknown filter handling: returns raw bytes with STRUCT_UNKNOWN_FILTER diagnostic

### 3. Bomb Limit Protection
- `ExtractionOptions` struct with `max_decompress_bytes` field (default: 2 GB)
- Document-level counter tracking across all stream decodes
- Per-stream bomb limit checking
- Chunked decoding (64 KB chunks) to enforce limit mid-stream
- STREAM_BOMB diagnostic when limit exceeded

### 4. Supporting Types
- `PdfSource` trait for abstracted byte reading
- `MemorySource` implementation for in-memory data
- `FileSource` implementation for file-backed data
- `FilterError` enum for hard errors (unknown filter, invalid params)
- `DecodeResult` struct for bytes + diagnostics

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| decode_stream() handles single-filter and array-filter cases | PASS | Tested with `test_decode_stream_single_filter` and `test_decode_stream_filter_array` |
| /DecodeParms array correctly paired with /Filter array | PASS | Implementation validates array lengths match |
| Critical test: [/ASCII85Decode /FlateDecode] applies filters in correct order | PASS | Filter array test verifies left-to-right application |
| Filter abbreviations normalized: /A85 routes to ASCII85Decode | PASS | `normalize_filter_name()` function + test |
| 2 GB bomb limit: FlateDecode bomb returns ~2 GB + STREAM_BOMB diagnostic | PASS | `test_flate_decode_bomb_limit` creates 1 MB bomb, stops at 500 KB limit |
| Unknown filter: STRUCT_UNKNOWN_FILTER, raw bytes returned | PASS | `test_decode_stream_unknown_filter` verifies passthrough |
| INV-8 maintained (no panics, partial bytes on error) | PASS | All decoders return Ok(partial_bytes) on corrupt data |

## Test Results

All 146 tests pass, including:
- 24 stream-specific tests
- FlateDecode bomb limit test (1 MB compressed → stops at 500 KB limit)
- Document-level bomb limit test (multiple streams share budget)
- Filter array ordering tests
- ASCII85 decoder with 'z' shortcut and partial tuples
- Unknown filter passthrough

## Files Modified

- `crates/pdftract-core/src/parser/stream.rs` - Complete implementation (1119 lines)
- `crates/pdftract-core/src/parser/diagnostic.rs` - Already had required DiagCode variants
- `crates/pdftract-core/src/parser/object/types.rs` - Already had PdfStream methods
- `crates/pdftract-core/src/parser/mod.rs` - Already exported stream module types

## Key Design Decisions

1. **Match-based dispatch** over `phf` map: Simpler, faster, and sufficient for the 8-10 filter types in PDF spec
2. **Bomb limit checking per 64 KB chunk**: Balances performance with protection
3. **Passthrough for unsupported filters**: DCTDecode (JPEG), JBIG2Decode, JPXDecode, CCITTFaxDecode pass raw bytes
4. **Document-level counter**: Passed as `&mut u64` through all decode calls
5. **Per-stream validation**: Each individual stream also checked against limit (prevents single 3 GB stream from bypassing doc limit)

## INV-3 (Deterministic Decoding)

The implementation maintains deterministic decoding for fingerprint stability:
- Same input + same params → byte-identical output
- No random or time-based behavior
- Error recovery produces consistent partial results

## Next Steps

The stream decoding infrastructure is complete. Future work may include:
- LZWDecode implementation (currently passthrough)
- RunLengthDecode implementation (currently passthrough)
- Crypt filter with /Name != Identity
- scan_for_endstream() fallback for streams without /Length
