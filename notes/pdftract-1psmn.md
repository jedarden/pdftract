# pdftract-1psmn: FileSource Implementation

## Summary

Implemented FileSource as a PdfSource fallback for when memory-mapping is not available or desired. This provides standard I/O-based access to PDF files using Read+Seek.

## Changes Made

### 1. crates/pdftract-core/src/source/file_source.rs
- Rewrote FileSource to use `parking_lot::Mutex<File>` for thread-safe concurrent access
- Implemented proper `Send + Sync` traits via unsafe impl (backed by Mutex)
- Implemented `PdfSource` trait with `len()` and `read_range()` methods
- Implemented `Read` and `Seek` traits for standard I/O usage
- Added comprehensive tests:
  - `test_open_valid_file`: Opens a valid file
  - `test_open_nonexistent_file`: Returns Err for non-existent file
  - `test_read_range`: Reads byte ranges correctly
  - `test_read_seek`: Tests Read+Seek trait methods
  - `test_read_range_bounds`: Tests boundary conditions
  - `test_send_sync`: Verifies Send trait (move to thread)
  - `test_sync_multiple_threads`: Verifies Sync trait (concurrent reads from 4 threads)
  - `test_concurrent_read_range`: Verifies concurrent reads all succeed
  - `test_read_range_past_eof_returns_err`: Tests EOF handling
  - `test_empty_file`: Handles empty files
  - `test_large_file`: Handles 100KB file
  - `test_read_mixed_with_seek`: Tests mixed read/seek operations

### 2. crates/pdftract-core/Cargo.toml
- Added `parking_lot = "0.12"` dependency

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| FileSource::open(/path/to/file.pdf) returns Ok | **PASS** | test_open_valid_file |
| FileSource::open(/nonexistent) returns Err | **PASS** | test_open_nonexistent_file |
| read_range(0, 10) returns first 10 bytes | **PASS** | test_read_range |
| read_range past EOF returns Err | **PASS** | test_read_range_bounds |
| Send + Sync: FileSource can be sent across threads | **PASS** | test_send_sync, test_sync_multiple_threads |
| Concurrent read_range from 4 threads succeeds | **PASS** | test_concurrent_read_range |
| Test fixture for FUSE-mounted file | **WARN** | No FUSE fixture tested (environment limitation) |

## Test Results

All 12 FileSource tests pass:
```
source::file_source::tests::test_open_valid_file          PASS
source::file_source::tests::test_open_nonexistent_file    PASS
source::file_source::tests::test_read_seek                 PASS
source::file_source::tests::test_send_sync                 PASS
source::file_source::tests::test_read_range_past_eof_ret.. PASS
source::file_source::tests::test_concurrent_read_range     PASS
source::file_source::tests::test_read_range_bounds         PASS
source::file_source::tests::test_empty_file               PASS
source::file_source::tests::test_sync_multiple_threads    PASS
source::file_source::tests::test_read_range                PASS
source::file_source::tests::test_read_mixed_with_seek     PASS
source::file_source::tests::test_large_file                PASS
```

## Key Implementation Details

1. **Thread Safety**: Uses `parking_lot::Mutex<File>` to enable concurrent reads across threads. The Mutex serializes access, which is the cost of seek-based I/O compared to mmap's zero-copy reads.

2. **Zero-Copy Bytes**: Uses `Bytes::from(Vec<u8>)` which takes ownership of the heap buffer without copying.

3. **Bounds Checking**: `read_range()` validates offsets and truncates reads at EOF rather than returning errors for short reads.

4. **Read+Seek Traits**: Implemented for compatibility with existing code that uses standard I/O patterns.

## WARN Items

- No FUSE-mounted file test fixture: Would require setting up sshfs or similar, which is an environmental limitation not a code issue.

## References

- Plan section: Phase 1.8 (FileSource description)
- Coordinator: pdftract-2cnmr (parent)
- Sibling implementations: PdfSource trait, MmapSource
