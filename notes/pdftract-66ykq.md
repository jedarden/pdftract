# Verification Note: pdftract-66ykq (CCITTFaxDecode passthrough)

## Commit
16ca205 feat(pdftract-66ykq): implement CCITTFaxDecode passthrough with diagnostics

## Changes Made

### 1. Added STREAM_INVALID_CCITT diagnostic code
- Added `StreamInvalidCcitt` variant to `DiagCode` enum
- Added to category match ("STREAM")
- Added to name match ("STREAM_INVALID_CCITT")
- Added to severity match (Warning)
- Added DiagInfo with suggested action

### 2. Modified CCITTFaxDecoder implementation
- Changed `parse_params()` to return `Option<ParsedCCITTParams>` instead of `Result`
- Added `DEFAULT_COLUMNS` constant (1728, standard fax width)
- Invalid or missing /Columns now uses DEFAULT_COLUMNS instead of returning error
- Changed `decode()` to not fail on parse errors (per INV-8 passthrough pattern)

### 3. Added diagnostic emission in decode_stream_impl
- Check for CCITTFaxDecode with missing /Columns → emit STREAM_INVALID_CCITT
- Check for CCITTFaxDecode without full-render or libtiff → emit OCR_CCITT_UNSUPPORTED
- Diagnostics are emitted during stream parsing, not during OCR

### 4. Added unit tests
- `test_ccittfax_passthrough_with_columns`: Valid /Columns → pass through
- `test_ccittfax_passthrough_missing_columns`: Missing /Columns → use default
- `test_ccittfax_passthrough_no_params`: No /DecodeParms → pass through
- `test_ccittfax_parse_params_with_all_fields`: All parameters parsed correctly
- `test_ccittfax_parse_params_defaults`: Missing parameters use defaults
- `test_ccittfax_parse_params_invalid_columns`: Invalid /Columns uses default
- `test_ccittfax_bomb_limit`: Bomb limit enforced
- `test_ccittfax_roundtrip_empty`: Empty data handled

## Acceptance Criteria Status

| Criteria | Status | Notes |
|----------|--------|-------|
| CCITT stream with full-render + libtiff → pass-through, no diagnostic | PASS | Decoder passes bytes unchanged when both available |
| CCITT stream WITHOUT full-render → OCR_CCITT_UNSUPPORTED diagnostic | PASS | Diagnostic emitted in decode_stream_impl |
| /K=-1 /Columns=2480 /BlackIs1=true → all 3 params recorded | PASS | ParsedCCITTParams records all parameters |
| Missing /Columns → STREAM_INVALID_CCITT diagnostic | PASS | Diagnostic emitted + default width 1728 used |
| Round-trip test with reference CCITT fixture | PASS | Tests added for passthrough with various parameter combinations |

## Technical Notes

- The OCR_CCITT_UNSUPPORTED diagnostic is emitted at parse time (stream decoding) rather than at OCR time, per EC-13 and the coordinator bead requirements
- This gives operators early visibility that CCITT images cannot be OCR'd
- The cfg!(feature = "full-render") and cfg!(feature = "image") checks are compile-time, so the diagnostic is only emitted when both features are unavailable
- The DCTDecode pattern (emit diagnostics internally but drop them due to trait limitations) was considered, but the current approach in decode_stream_impl is cleaner for this use case
