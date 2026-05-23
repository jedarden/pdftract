# Verification Note: pdftract-172kr (Filesystem layout for cache directory)

## Summary

Phase 6.9.1 filesystem layout implementation was completed in commit `624fc49290176018feb39524e8f8f03398b2eacf`.

## Implementation

Created `crates/pdftract-core/src/cache/layout.rs` with:

### Core Functions
- `entry_path()` - Constructs cache entry paths with two-byte prefix layout
- `fingerprint_dir()` - Gets the parent directory for all variants of a PDF
- `parse_opts_hash_from_filename()` - Extracts opts_hash from `<hash>-<size>.json.zst` format
- `parse_size_from_filename()` - Extracts compressed size from filename
- `index_path()` / `sentinel_path()` - Paths to cache metadata files
- `load_index()` / `save_index()` - Cache index.json persistence
- `ensure_fingerprint_dir()` - mkdir -p semantics for directory creation

### Data Structures
- `CacheIndex` - Holds schema version (1), creation timestamp, last LRU sweep, total bytes, entry count
- `CURRENT_SCHEMA_VERSION` constant = 1

## Acceptance Criteria Results

### PASS
- ✓ `entry_path("/cache", "pdftract-v1:e7a1f3...", "9b21...", 12387)` → `/cache/e7/a1/e7a1f3.../9b21...-12387.json.zst`
- ✓ Different opts_hashes for same fingerprint share the same fp_dir (verified by `test_entry_path_different_opts_hashes`)
- ✓ Different fingerprints with same first 2 chars share first-level directory (verified by `test_entry_path_different_fingerprints_same_prefix`)
- ✓ index.json round-trips: read at startup, parse schema version, recognize current = 1 (verified by `test_index_roundtrip`)
- ✓ Future schema version (99) → cache disabled with clear error (verified by `test_index_schema_version_mismatch`)
- ✓ mkdir -p creates prefix dirs; idempotent on concurrent writes (verified by `test_ensure_fingerprint_dir`)
- ✓ Unicode-correct path handling via std::path::PathBuf (verified by `test_unicode_path_handling`)
- ✓ Path length stays under 4096 bytes for worst case (verified by `test_path_length_within_limits`)

## Test Results

All 18 tests pass:
```
running 18 tests
test cache::layout::tests::test_entry_path_basic ... ok
test cache::layout::tests::test_ensure_fingerprint_dir ... ok
test cache::layout::tests::test_entry_path_different_fingerprints_same_prefix ... ok
test cache::layout::tests::test_entry_path_different_opts_hashes ... ok
test cache::layout::tests::test_entry_path_large_size ... ok
test cache::layout::tests::test_entry_path_short_fingerprint ... ok
test cache::layout::tests::test_entry_path_zero_size ... ok
test cache::layout::tests::test_fingerprint_dir ... ok
test cache::layout::tests::test_entry_path_too_short - should panic ... ok
test cache::layout::tests::test_fingerprint_without_prefix ... ok
test cache::layout::tests::test_index_default ... ok
test cache::layout::tests::test_index_not_exists ... ok
test cache::layout::tests::test_index_roundtrip ... ok
test cache::layout::tests::test_parse_opts_hash_from_filename ... ok
test cache::layout::tests::test_index_schema_version_mismatch ... ok
test cache::layout::tests::test_parse_size_from_filename ... ok
test cache::layout::tests::test_path_length_within_limits ... ok
test cache::layout::tests::test_unicode_path_handling ... ok

test result: ok. 18 passed; 0 failed; 0 ignored
```

## Files Changed

- `crates/pdftract-core/Cargo.toml` - Added zstd dependency
- `crates/pdftract-core/src/cache/layout.rs` - 543 lines, full implementation
- `crates/pdftract-core/src/cache/mod.rs` - 25 lines, module exports
- `crates/pdftract-core/src/lib.rs` - Added cache module
- `Cargo.lock` - Updated with zstd dependency

## No WARN Items

All acceptance criteria met cleanly.
