# pdftract-2wyd: Signature field discovery

## Summary

Implemented Phase 7.3.1: AcroForm signature field discovery. The implementation walks the AcroForm /Fields array recursively, filters to /FT /Sig fields, and extracts field metadata including absolute names, signature value references, bounding rectangles, and page indices.

## Changes Made

### Created signature module
- `crates/pdftract-core/src/signature/mod.rs` (709 lines)
- Added to `crates/pdftract-core/src/lib.rs`

### Key components
1. **SigFieldRef struct** - Public type representing a discovered signature field
   - `full_name`: Absolute dot-joined field name
   - `v_ref`: Optional reference to /V dictionary (signature value)
   - `rect`: Optional bounding rectangle [x0, y0, x1, y1]
   - `page_index`: Optional page index (None for form-only signatures)
   - `field_ref`: The field's indirect reference

2. **walk_acroform_fields helper** - Reusable field walker for 7.4
   - DFS traversal of /Kids hierarchy
   - Resolves /FT inheritance from parent to child
   - Constructs absolute field names via dot-joined /T values
   - Returns Vec<FieldRef> for all field types

3. **sig::discover public API** - Main entry point
   - Takes XrefResolver and Catalog
   - Returns Vec<SigFieldRef> filtered to /FT /Sig fields
   - Returns empty vec if no AcroForm or no signature fields

### Test coverage (9 tests, all PASS)
- `test_discover_no_acroform` - Returns empty vec when no AcroForm
- `test_discover_no_fields` - Returns empty vec when /Fields absent/empty
- `test_discover_two_flat_signatures` - Finds two flat signature fields
- `test_discover_non_signature_fields_excluded` - Filters out Tx/Btn/Ch fields
- `test_discover_nested_signature_inherits_ft` - Handles /FT inheritance from parent
- `test_discover_nested_mixed_field_types` - Child can override parent /FT
- `test_discover_with_rect` - Extracts bounding rectangle
- `test_discover_with_v_ref` - Extracts /V reference
- `test_walk_acroform_fields_reusable` - Verifies walker returns all field types

## Acceptance Criteria Status

- ✅ Discovery returns all /FT /Sig fields, including nested ones
- ✅ Unit tests: flat 2 sigs, nested 1 sig under parent, no AcroForm, AcroForm with no Fields, kids inheriting /FT from parent
- ✅ Public sig::discover(&Document) -> Vec<SigFieldRef> (via Catalog)
- ✅ Reusable walk_acroform_fields helper available for 7.4

## Known Limitations

1. **page_index resolution** - Currently always None. Per bead description, resolving page_index requires reverse lookup through page /Annots arrays to find which page contains the field's widget annotation. This requires access to the page tree which is not available in the current scope. Deferred to future work when 7.3.2 integrates with the extraction pipeline.

2. **diagnostics not returned** - The walk_acroform_fields function accumulates diagnostics but they are currently discarded. This is acceptable for discovery (missing/malformed fields are simply skipped), but may need to be surfaced for debugging in production use.

## Git Commit

- Commit: `fe15c81`
- Message: `feat(pdftract-2wyd): implement signature field discovery`
- Files changed: 2 files, 709 insertions(+)

## Next Steps

- pdftract-6arz (7.3.2): Signature metadata extraction (/V dict + ByteRange coverage)
- pdftract-j6yd (7.3.3): signatures array output + validation_status enum + schema integration
- pdftract-* (7.4): Form field extraction (reuses walk_acroform_fields helper)
