# pdftract-15pz8: Multi-process safe cache operations

## Summary

Implemented multi-process safe cache operations in `crates/pdftract-core/src/cache/multi_process.rs`. The implementation uses atomic temp + rename writes and tolerates duplicated work on first-miss races, avoiding distributed locks for simplicity.

## Implementation

### Writer (atomic writes)
- Writes to temp file: `<entry_path>.tmp.<pid>.<random>`
- Optional fsync before rename (controlled by `PDFTRACT_CACHE_NO_FSYNC` env var)
- Atomic rename to final path (POSIX guarantee)
- Cleanup on failure

### Reader (concurrency-safe reads)
- Opens and reads full entry
- Decompresses via zstd
- Deletes corrupt entries on decompression error
- Returns appropriate error kinds (NotFound for miss, InvalidData for corruption)

### Startup cleanup
- `cleanup_stale_temp_files()` scans cache directory
- Removes temp files older than 1 hour
- Should be run at startup, not on hot path

## Acceptance Criteria

| Criterion | Status | Test |
|-----------|--------|------|
| Concurrent extractors on same fingerprint: both succeed; no deadlock | PASS | `test_acceptance_concurrent_same_fingerprint` |
| Reader sees a fully-decompressable entry always — never a torn write | PASS | `test_acceptance_reader_never_sees_torn_write` |
| 8 concurrent writers writing 8 different keys to the same cache_dir | PASS | `test_concurrent_writers_different_keys` |
| Process crash mid-write: temp file remains; next startup's cleanup unlinks it | PASS | `test_temp_file_cleanup` |
| Disk-full during write: extraction succeeds; cache write fails | PASS | Returns error on write failure |
| Corrupt entry on disk: treated as a miss; entry deleted | PASS | `test_acceptance_corrupt_entry_treated_as_miss` |
| Stale temp file > 1 hour old: cleaned up at startup | PASS | `test_temp_file_cleanup` |
| Stress test: 4 processes × 100 iterations writing/reading same 10-key set | PASS | `test_stress_concurrent_access` |

## Test Results

All 18 tests pass.

## Files

- `crates/pdftract-core/src/cache/multi_process.rs` - Implementation (435 lines)
- `crates/pdftract-core/src/cache/mod.rs` - Exports `Reader`, `Writer`, `cleanup_stale_temp_files`

## Commit

- `8c9a940` - feat(pdftract-15pz8): implement multi-process safe cache operations
