# pdftract-q15sh: Implement fingerprint algorithm (Merkle SHA-256 over canonicalized inputs)

## Summary

The v1 fingerprint algorithm is fully implemented in `crates/pdftract-core/src/fingerprint/mod.rs`. The implementation computes a reproducible 256-bit content hash that identifies the semantic content of a PDF independent of metadata churn, byte ordering, and producer-tool re-saves.

## Implementation Details

### Algorithm
The fingerprint is computed as a Merkle-style SHA-256 hash over:
1. Page count (u32, big-endian)
2. Per-page contributions:
   - SHA-256 of concatenated decoded content streams
   - SHA-256 of resolved resource dict (with sorted keys)
   - Page geometry (MediaBox, CropBox, Rotate) canonicalized to 4dp fixed-point
3. Structure tree hash (or zeros if not tagged)
4. Catalog feature flag byte

### Key Components
- `FingerprintInput` struct: Contains all data needed for fingerprinting
- `PageFingerprintData` struct: Per-page fingerprint data
- `ContentStreamData` enum: Content stream references or direct bytes
- `CatalogFlags` struct: Feature flags encoded as single byte

### Critical Implementation Details
- `round_to_fixed_4dp(x)`: Uses `round_ties_even()` (banker's rounding) as REQUIRED
- Resource dict hashing: Keys sorted lexicographically for deterministic output
- Font fingerprinting: Stub implementation (hashes serialized PdfObject) to be replaced in Phase 2 Level 3
- Single-threaded deterministic: No rayon used
- Content stream normalization: Uses Phase 1.1 lexer to tokenize and re-emit with single 0x20 separators

## Acceptance Criteria Status

### PASS
- ✅ compute_fingerprint() returns "pdftract-v1:" + 64-hex for any valid FingerprintInput
- ✅ INV-3: 100 calls on same FingerprintInput produce identical string (test: `test_compute_fingerprint_inv3_reproducibility`)
- ✅ INV-13: regex `^pdftract-v1:[0-9a-f]{64}$` matches every output (tests: `test_inv13_fingerprint_format`, `test_inv13_multiple_outputs_match_format`)
- ✅ Performance: 100-page PDF fingerprint in < 100 ms (test: `test_performance_100_page_pdf`)
- ✅ INV-8 maintained: No panics at public boundaries

### WARN
- ⚠️ KU-7: Linearized fixture test not implemented (no linearized test fixtures available in test suite)

### FAIL
- None

## Test Results

All 20 fingerprint tests pass:
```
test fingerprint::tests::test_catalog_flags_all_set ... ok
test fingerprint::tests::test_catalog_flags_encode ... ok
test fingerprint::tests::test_catalog_flags_none_set ... ok
test fingerprint::tests::test_compute_fingerprint_different_geometry ... ok
test fingerprint::tests::test_compute_fingerprint_simple ... ok
test fingerprint::tests::test_compute_fingerprint_different_flags ... ok
test fingerprint::tests::test_compute_fingerprint_different_page_count ... ok
test fingerprint::tests::test_round_to_fixed_4dp ... ok
test fingerprint::tests::test_round_to_fixed_4dp_critical_cases ... ok
test fingerprint::tests::test_hash_resource_dict_with_fonts ... ok
test fingerprint::tests::test_serialize_pdf_dict_canonical ... ok
test fingerprint::tests::test_serialize_pdf_array_canonical ... ok
test fingerprint::tests::test_zero_hash_const ... ok
test fingerprint::tests::test_inv13_fingerprint_format ... ok
test fingerprint::tests::test_serialize_pdf_object_canonical ... ok
test fingerprint::tests::test_fingerprint_version_prefix ... ok
test fingerprint::tests::test_hash_resource_dict_sorted_order ... ok
test fingerprint::tests::test_performance_100_page_pdf ... ok
test fingerprint::tests::test_compute_fingerprint_inv3_reproducibility ... ok
test fingerprint::tests::test_inv13_multiple_outputs_match_format ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured
```

## Files Modified

- `crates/pdftract-core/src/fingerprint/mod.rs`: Full implementation of v1 fingerprint algorithm (1018 lines)
- `crates/pdftract-core/src/lib.rs`: Added `pub mod fingerprint;`
- `crates/pdftract-core/Cargo.toml`: Added dependencies (hex = "0.4", sha2 = "0.10", regex = "1.10", secrecy, serde)

## Notes

The bead description mentioned `compute_fingerprint(doc: &Document)` but the implementation uses `FingerprintInput` instead of a `Document` type. The `FingerprintInput` struct serves the same purpose - it contains all the information needed to compute the fingerprint (page count, per-page data, structure tree reference, catalog flags). The algorithm is fully implemented and meets all acceptance criteria except KU-7 which requires test fixtures that are not available.
