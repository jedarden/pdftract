# pdftract-57np8: Image Filter Passthroughs Verification

## Task
Implement DCTDecode / JBIG2Decode / JPXDecode / CCITTFaxDecode passthroughs with SOI/EOI validation + OCR_*_UNSUPPORTED diagnostics

## Status: COMPLETE

All four image filter passthroughs are implemented in `crates/pdftract-core/src/parser/stream.rs` with proper validation and diagnostic emission.

## Implementation Summary

### 1. DCTDecode (JPEG) Passthrough
- **Location**: `crates/pdftract-core/src/parser/stream.rs` lines 3718-3743
- **SOI Marker Validation**: Checks first 2 bytes are 0xFF 0xD8 (SOI = Start Of Image)
- **EOI Marker Validation**: Checks last 2 bytes are 0xFF 0xD9 (EOI = End Of Image)
- **Diagnostic**: `STREAM_INVALID_JPEG` emitted for missing SOI or EOI markers
- **Passthrough**: Raw JPEG bytes passed through unchanged
- **Tests**:
  - `test_dctdecode_passthrough_valid_jpeg` - verifies bytes unchanged with SOI/EOI
  - `test_dctdecode_passthrough_missing_soi` - verifies warning without SOI
  - `test_dctdecode_passthrough_missing_eoi` - verifies warning without EOI
  - `prop_dct_decode_never_panics` - proptest for random input

### 2. JBIG2Decode Passthrough
- **Location**: `crates/pdftract-core/src/parser/stream.rs` lines 3697-3716
- **Diagnostic**: `OCR_JBIG2_UNSUPPORTED` emitted when full-render feature is disabled
- **Passthrough**: Raw JBIG2 bytes passed through unchanged
- **Globals Recording**: `/JBIG2Globals` reference extracted and stored in StreamMeta
- **Tests**:
  - `test_jbig2_passthrough` - integration test for passthrough
  - `prop_jbig2_decode_never_panics` - proptest for random input
  - `prop_jbig2_passthrough_never_panics` - proptest via get_decoder

### 3. JPXDecode (JPEG2000) Passthrough
- **Location**: `crates/pdftract-core/src/parser/stream.rs` lines 3745-3757
- **JP2 Box Magic Validation**: Checks first 12 bytes match JP2 signature (00 00 00 0C 6A 50 20 20 0D 0A 87 0A)
- **Diagnostics**:
  - `OCR_JPX_UNSUPPORTED` emitted when full-render AND libopenjp2 are unavailable
  - `STREAM_INVALID_JPX` emitted when JP2 box magic doesn't match (raw J2K or corrupt)
- **Passthrough**: Raw JPEG2000 bytes passed through unchanged
- **Tests**:
  - `test_jpxstream_passthrough_valid_jp2` - verifies JP2 passthrough
  - `test_jpxstream_passthrough_raw_j2k` - verifies raw J2K passthrough
  - `test_jpxstream_passthrough_empty` - edge case
  - `prop_jpx_decode_never_panics` - proptest for random input

### 4. CCITTFaxDecode Passthrough
- **Location**: `crates/pdftract-core/src/parser/stream.rs` lines 3667-3695
- **Diagnostic**: `OCR_CCITT_UNSUPPORTED` emitted when full-render AND libtiff are unavailable
- **Parameter Parsing**: Parses /K, /Columns, /Rows, /EncodedByteAlign, /EndOfLine, /BlackIs1
- **Defaults**: Uses DEFAULT_COLUMNS (1728) when /Columns missing
- **Passthrough**: Raw CCITT bytes passed through unchanged
- **Tests**:
  - `test_ccittfax_passthrough_with_columns` - verifies passthrough with params
  - `test_ccittfax_passthrough_missing_columns` - verifies default used
  - `test_ccittfax_parse_params_with_all_fields` - verifies parameter parsing
  - `prop_ccitt_decode_never_panics` - proptest for random input

## Acceptance Criteria Status

### Critical Test
- **PASS**: DCTDecode fixture with known JPEG — bytes unchanged, SOI marker present
  - Test: `test_dctdecode_passthrough_valid_jpeg` (line 1951)

### Diagnostics
- **PASS**: JPEG without EOI marker passes through with STREAM_INVALID_JPEG warning
  - Test: `test_dctdecode_passthrough_missing_eoi` (line 1982)
- **PASS**: JBIG2Decode without full-render emits OCR_JBIG2_UNSUPPORTED
  - Emission at line 3703 (emits when cfg!(feature = "full-render") is false)
- **PASS**: JPXDecode without full-render emits OCR_JPX_UNSUPPORTED
  - Emission at line 3750 (via JpxDecoder::emit_unsupported_diagnostic)
- **PASS**: CCITTFaxDecode without libtiff emits OCR_CCITT_UNSUPPORTED
  - Emission at line 3690 (emits when !has_full_render && !has_libtiff)

### Validation
- **PASS**: JP2 box magic check detects malformed JPX with STREAM_INVALID_JPX
  - Validation at line 3754 (via JpxDecoder::validate_jp2_magic)

### INV-8 Compliance
- **PASS**: Proptest random byte sequences for each filter never panic
  - Tests: `prop_dct_decode_never_panics`, `prop_jbig2_decode_never_panics`,
           `prop_jpx_decode_never_panics`, `prop_ccitt_decode_never_panics`

## Files Modified

### Core Implementation
- `crates/pdftract-core/src/parser/stream.rs`: Diagnostic emissions for all 4 filters
- `crates/pdftract-core/src/decoder/jbig2.rs`: JBIG2Decoder with diagnostic emission
- `crates/pdftract-core/src/decoder/jpx.rs`: JpxDecoder with JP2 validation and diagnostics

### Tests
- `tests/proptest/stream.rs`: Added proptest coverage for all 4 filters
  - 14 new property tests verifying never-panic and passthrough behavior

## Feature Gate Behavior

### With full-render feature
- All diagnostics suppressed
- Image data passed to OCR pipeline for pdfium-render decoding

### Without full-render feature
- OCR_JBIG2_UNSUPPORTED emitted per JBIG2 stream (EC-11)
- OCR_JPX_UNSUPPORTED emitted per JPX stream (EC-12)
- OCR_CCITT_UNSUPPORTED emitted per CCITT stream (EC-13)
- Data still passed through for downstream consumption

## Verification Date
2026-05-28

## Notes
- Diagnostics emitted in `decode_stream_impl` function, not in individual decoder implementations
- This is because `StreamDecoder` trait doesn't provide a way to return diagnostics
- Passthrough pattern preserves all bytes unchanged, including malformed data (INV-8)
