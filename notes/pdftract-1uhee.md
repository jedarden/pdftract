# pdftract-1uhee: MmapSource Implementation

## Summary

The MmapSource implementation was already complete in `crates/pdftract-core/src/source/mmap.rs`. This task verified the implementation and fixed two incorrect test assertions.

## Changes Made

### Test Fixes (commit: ba5d101)

1. **test_open_valid_file**: Fixed assertion from 20 to 22 bytes
   - The byte string `b"%PDF-1.4\ntest content\n"` is 22 bytes
   - `%PDF-1.4` (8) + `\n` (1) + `test content` (12) + `\n` (1) = 22

2. **test_seek_from_end**: Fixed expected result from `b"el"` to `b"lo"`
   - Content: `b"Hello"` (indices 0='H', 1='e', 2='l', 3='l', 4='o')
   - `SeekFrom::End(-2)` puts position at index 3
   - Reading 2 bytes from position 3 gives `b"lo"`

## Acceptance Criteria Status

| Criterion | Status | Test |
|-----------|--------|------|
| MmapSource::open(/path/to/file.pdf) returns Ok for valid file | PASS | test_open_valid_file |
| MmapSource::open(/nonexistent) returns Err | PASS | test_open_nonexistent_file |
| read_range(0, 10) returns first 10 bytes | PASS | test_read_range |
| read_range past EOF returns Err | PASS | test_read_range_past_eof |
| len() matches file size | PASS | test_len_matches_file_size |
| Read+Seek trait usage works | PASS | test_read_trait, test_seek_trait |
| Send + Sync: can send across threads | PASS | test_send_sync, test_sync_multiple_threads |
| MADV_SEQUENTIAL compiles and runs | PASS | test_advise_sequential, test_prefetch |

## Implementation Details (Already Complete)

### MmapSource Structure
```rust
pub struct MmapSource {
    mmap: Mmap,
    cursor: Cursor<u64>,
}
```

### Key Methods
- `open(path)`: Creates memory-mapped file using `memmap2::MmapOptions`
- `read_range(offset, length)`: Zero-copy read via `Bytes::copy_from_slice`
- `advise_sequential(offset, length)`: Applies `MADV_SEQUENTIAL` for content streams
- `prefetch(offset, length)`: Wrapper for `advise_sequential`

### Thread Safety
- `unsafe impl Send for MmapSource`
- `unsafe impl Sync for MmapSource`
- Verified by `test_send_sync` and `test_sync_multiple_threads`

### Files
- Implementation: `crates/pdftract-core/src/source/mmap.rs` (460 lines)
- Module: `crates/pdftract-core/src/source/mod.rs` (exports MmapSource)
