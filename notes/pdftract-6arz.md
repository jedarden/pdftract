# pdftract-6arz: Signature metadata extraction (/V dict + ByteRange coverage)

## Summary

Implemented Phase 7.3.2: Digital signature metadata extraction. The implementation resolves /V dictionaries for each discovered signature field, extracts signer identity and timestamps, computes coverage statistics from /ByteRange, and produces a structured `Signature` output.

## Changes Made

### Added Signature struct
- `field_name`: Absolute field name from AcroForm
- `signer_name`: From /Name entry (defaults to "")
- `signing_date`: Option<ISO 8601 string> parsed from PDF /M date format
- `reason`: Option<String> from /Reason
- `location`: Option<String> from /Location
- `sub_filter`: Option<String> from /SubFilter (signature format)
- `byte_range`: Option<Vec<u64>> defining signed byte ranges
- `coverage_fraction`: Option<f64> computed as (br[1] + br[3]) / file_size
- `validation_status`: Hard-coded "not_checked" per plan (v1 has no crypto validation)

### Added PDF date to ISO 8601 parser
- `parse_pdf_date()` function handles PDF date format: `D:YYYYMMDDHHmmSSOHH'mm`
- Tolerates truncated dates (date only, no time, no tz)
- Outputs RFC 3339 ISO 8601 format with "Z" for UTC
- Defaults missing components: 00 for time, Z for timezone

### Added PDF string decoder
- `decode_pdf_string()` handles UTF-16BE BOM, UTF-16BE without BOM (heuristic), and PDFDocEncoding
- Copied from outline.rs (private there) to avoid coupling modules
- Handles both PDFDocEncoding and UTF-16BE encoded strings

### Added metadata extraction functions
- `extract_signature_metadata()`: Extracts all fields from a single signature's /V dict
- `extract_signatures()`: Public API for processing all discovered signature fields

### Test coverage (27 tests, all PASS)

#### Discovery tests (9 tests from 7.3.1)
- All existing discovery tests continue to pass

#### Metadata extraction tests (5 new tests)
- `test_extract_signature_metadata_full`: Full signature with all fields
- `test_extract_signature_metadata_unsigned`: Unsigned field (no /V)
- `test_extract_signature_metadata_missing_optional_fields`: Minimal signature
- `test_extract_signatures_multiple`: Two signatures with different /V dicts
- `test_walk_acroform_fields_reusable`: Verifies walker returns all field types

#### PDF date parsing tests (7 new tests)
- `test_parse_pdf_date_full_with_timezone`: D:20230115143045+05'30'
- `test_parse_pdf_date_utc`: D:20230115143045Z
- `test_parse_pdf_date_negative_timezone`: D:20230115143045-08'00'
- `test_parse_pdf_date_only`: D:20230115 (date only, defaults to 00:00:00Z)
- `test_parse_pdf_date_no_timezone`: D:20230115143045 (no tz, defaults to Z)
- `test_parse_pdf_date_without_d_prefix`: 20230115143045Z
- `test_parse_pdf_date_malformed`: Various malformed inputs return None

#### ByteRange coverage tests (4 new tests)
- `test_coverage_fraction_full_coverage`: 4000/4000 bytes = 1.0
- `test_coverage_fraction_partial`: 1500/3000 bytes = 0.5
- `test_coverage_fraction_no_file_size`: None when file_size unknown
- `test_coverage_fraction_invalid_byte_range`: None when /ByteRange malformed

#### PDF string decoding tests (3 new tests)
- `test_decode_pdf_string_ascii`: ASCII string
- `test_decode_pdf_string_utf16be_bom`: UTF-16BE with BOM
- `test_decode_pdf_string_empty`: Empty string

## Acceptance Criteria Status

- ✅ Critical test: PDF with two signature fields - both extracted with correct signer names and dates
- ✅ Critical test: unsigned signature field - emitted with value: null (modeled as unsigned Signature with empty fields)
- ✅ Critical test: /ByteRange coverage fraction computed correctly
- ✅ Unit tests: malformed date string (returns None), missing /Name (returns ""), missing /ByteRange (returns None coverage)
- ✅ Output: Signature struct with all required fields

## Implementation Notes

1. **Unsigned signature handling**: When /V is absent, we return a `Signature` with:
   - `signer_name`: ""
   - `signing_date`: None
   - `reason`: None
   - `location`: None
   - `sub_filter`: None
   - `byte_range`: None
   - `coverage_fraction`: None
   - `validation_status`: "not_checked"

2. **Date parsing**: The PDF date format is complex and may include:
   - Literal "D:" prefix
   - Truncated values (date only, date+time only)
   - Timezone as "Z", "+HH'mm'", "-HH'mm'", or omitted
   - Our parser handles all these cases and outputs clean ISO 8601

3. **Coverage computation**: Per plan, coverage is (br[1] + br[3]) / file_size
   - br[0] and br[2] are offsets, br[1] and br[3] are lengths
   - The signature value itself is NOT covered (it's between the two ranges)
   - Values < 1.0 indicate partial signatures (red flag for tampered docs)

4. **String decoding**: /Name, /Reason, and /Location are PDF strings that may use:
   - PDFDocEncoding (Latin-1 with overrides)
   - UTF-16BE with BOM (0xFE 0xFF)
   - UTF-16BE without BOM (heuristic detection)
   - Our decoder handles all three cases

5. **SubFilter is a Name**: Unlike other string fields, /SubFilter is a PDF Name object
   - Read via `as_name()` instead of `as_string()`
   - No decoding needed (Names are always ASCII identifiers)

## Known Limitations

1. **page_index**: Still None (deferred from 7.3.1). Requires reverse lookup through page /Annots arrays.

2. **value field**: The actual signature value (PKCS#7 DER blob) is not extracted in v1.
   - This would require resolving the /Contents entry and decoding the signature
   - Deferred to future work when cryptographic validation is implemented

3. **diagnostics not surfaced**: Extraction failures (malformed /V, unresolvable references) return
   default/empty values rather than surfacing diagnostics. This is acceptable for v1 but may
   need improvement for production use.

## Git Commit

- Commit: TBD
- Message: `feat(pdftract-6arz): implement signature metadata extraction`
- Files changed: `crates/pdftract-core/src/signature/mod.rs` (+835 lines)

## Next Steps

- pdftract-j6yd (7.3.3): signatures array output + validation_status enum + schema integration
- Future: Cryptographic validation (ring/openssl integration)
