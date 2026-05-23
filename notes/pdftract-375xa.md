# pdftract-375xa: Cache Key Construction

## Summary

Implemented Phase 6.9.2: Cache key construction for (PDF fingerprint, extraction options) pairs. The key is a tuple (fingerprint, opts_hash) where opts_hash is the SHA-256 of the canonical JSON serialization of ExtractionOptions.

## Changes Made

### File: `crates/pdftract-core/src/cache/key.rs`

1. **Enhanced canonicalization implementation**:
   - Replaced struct-based serialization with `BTreeMap`-based approach
   - Added `canonical_json()` helper for testing sorted-key canonicalization
   - Added `canonical_json_value()` for recursive canonicalization

2. **Key invariants implemented**:
   - Keys are sorted lexicographically using `BTreeMap`
   - Floats have canonical representation (preserves integers, canonicalizes floats)
   - Booleans are always `true`/`false` (handled by serde_json)
   - `extraction_version` is included for cache invalidation on upgrades

3. **Added comprehensive tests**:
   - `test_acceptance_same_effective_values_same_hash` - AC for identical values
   - `test_acceptance_receipts_off_to_lite_changes_hash` - AC for receipts toggle
   - `test_acceptance_different_version_changes_hash` - AC for version pinning
   - `test_acceptance_sorted_key_canonical` - AC for sorted keys
   - `test_acceptance_float_canonical` - AC for float canonicalization
   - `test_acceptance_float_canonical_edge_cases` - Edge cases for floats
   - `test_invariant_documented` - Meta-test documenting the invariant
   - `test_canonical_json_nested_objects` - Nested object sorting
   - `test_canonical_json_arrays` - Array handling
   - `test_canonical_json_float_arrays` - Float array handling
   - `test_canonical_json_mixed` - Mixed nested structures

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Same effective values → same hash | ✅ PASS | `test_acceptance_same_effective_values_same_hash` |
| Toggle receipts off→lite → hash differs | ✅ PASS | `test_acceptance_receipts_off_to_lite_changes_hash` |
| Different version → hash differs | ✅ PASS | `test_acceptance_different_version_changes_hash` |
| Sorted-key canonical | ✅ PASS | `test_acceptance_sorted_key_canonical` |
| Float canonical (0.5 == 0.500) | ✅ PASS | `test_acceptance_float_canonical` |
| Documented invariant | ✅ PASS | `test_invariant_documented` |

## Future Considerations

1. **OCR field** - When `ocr` field is added to ExtractionOptions, it will automatically be included in the canonical JSON
2. **Password field** - When added, should use a stable token (e.g., `password_set: bool`) instead of the literal password to avoid leaking sensitive data in cache directory entries
3. **Option\<T\> fields** - The canonicalization already handles defaults correctly; None and Some(default) will produce the same hash if the default-filling is done before canonicalization

## Implementation Notes

- Used `BTreeMap` for guaranteed lexicographic key ordering
- Integer representation is preserved (not converted to float)
- Float canonicalization is handled by serde_json's default behavior (shortest decimal representation)
- The implementation is forward-compatible with new fields added to ExtractionOptions
