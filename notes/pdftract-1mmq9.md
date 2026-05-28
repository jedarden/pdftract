# pdftract-1mmq9: PdfSource trait definition verification note

## Summary

Bead: pdftract-1mmq9
Title: PdfSource trait definition + Bytes-based read_range + prefetch + Send/Sync bounds
Date: 2026-05-28

## Completed Work

### 1. PdfSource trait definition (crates/pdftract-core/src/source/mod.rs)

The PdfSource trait is complete with the following features:
- **Supertrait bounds**: Read + Seek + Send + Sync (as required)
- **len()**: Returns total source length as u64
- **read_range()**: Reads arbitrary byte ranges returning io::Result<Bytes> for zero-copy slicing
- **prefetch()**: Optional hint with no-op default implementation (overridden by MmapSource)
- **Object-safe**: Can be used as &dyn PdfSource for dynamic dispatch
- **Well-documented**: Includes examples showing Read+Seek usage and direct read_range usage

### 2. MmapSource implementation (crates/pdftract-core/src/source/mmap.rs)

- Uses memmap2 for memory-mapped file access
- Implements MADV_SEQUENTIAL via `advise_sequential()` method
- Implements `prefetch()` to apply sequential readahead for content streams
- Read+Seek trait implementation with cursor-based position tracking
- Send + Sync unsafe impls (mmap is immutable after mapping)
- Comprehensive test coverage (read_range, bounds checking, Send/Sync, etc.)

### 3. FileSource implementation (crates/pdftract-core/src/source/file_source.rs)

- Standard I/O fallback for when mmap fails (FUSE mounts, /proc, named pipes)
- Read+Seek trait implementation delegating to std::fs::File
- read_range() uses try_clone() to avoid &self mutation issues
- Test coverage for read operations and bounds checking

### 4. Re-exports (crates/pdftract-core/src/lib.rs)

```rust
pub mod source;
pub use source::{FileSource, MmapSource, PdfSource};
```

The trait is properly re-exported from the crate root.

## Current State

### PASS Items

- ✅ Trait compiles in crates/pdftract-core/src/source/mod.rs
- ✅ &dyn PdfSource is object-safe (compiles)
- ✅ Trait re-exported from pdftract-core::source::PdfSource
- ✅ Documented with examples showing Read+Seek usage and direct read_range usage
- ✅ Send + Sync bounds present (required for rayon page-parallelism)
- ✅ Bytes type used for zero-copy slicing
- ✅ prefetch() method with no-op default
- ✅ MmapSource overrides prefetch() for MADV_SEQUENTIAL
- ✅ All implementations compile and have tests

### WARN Items

- ⚠️ **Parser modules NOT yet refactored**: The lexer (Phase 1.1) and other parser modules still take `&'a [u8]` or use the old `PdfSource` trait from `parser/stream.rs`
- ⚠️ **Conflicting PdfSource trait**: There's an older PdfSource trait in `parser/stream.rs` with a different API (`read_at` returning `Vec<u8>`, `len` returning `Result<u64>`)
- ⚠️ **Migration required**: The following modules still import from the old location:
  - `attachment/filespec.rs`
  - `forms/xfa.rs`
  - `document.rs`
  - `parser/xref.rs`
  - `parser/catalog.rs`
  - `parser/objstm.rs`
  - `extract.rs`

### FAIL Items

- ❌ **Acceptance criteria not fully met**: "All Phase 1.1-1.5 parser modules refactored to consume PdfSource" is NOT complete

## Technical Notes

### API Differences: Old vs New PdfSource

**Old trait (parser/stream.rs):**
```rust
pub trait PdfSource {
    fn read_at(&self, offset: u64, len: usize) -> std::io::Result<Vec<u8>>;
    fn len(&self) -> std::io::Result<u64>;
}
```

**New trait (source/mod.rs):**
```rust
pub trait PdfSource: Read + Seek + Send + Sync {
    fn len(&self) -> u64;
    fn read_range(&self, offset: u64, length: usize) -> io::Result<Bytes>;
    fn prefetch(&self, _offset: u64, _length: usize) {}
}
```

Key differences:
1. New trait has Read+Seek+Send+Sync bounds (old has none)
2. New trait's `len()` returns u64 directly (old returns Result<u64>)
3. New trait uses `read_range()` returning Bytes (old uses `read_at` returning Vec<u8>)
4. New trait has `prefetch()` for speculative readahead (old has none)

### Migration Path (per parent coordinator pdftract-2cnmr)

The coordinator describes a 5-step process:
- Step 1: Define PdfSource trait ✅ DONE
- Step 2: Implement MmapSource + FileSource ✅ DONE
- Step 3: Add adapter `Lexer::from_source(source, range)` alongside existing `Lexer::new(bytes)` ⏳ TODO
- Step 4: Migrate callers one by one ⏳ TODO
- Step 5: Deprecate `Lexer::new(bytes)` in favor of `Lexer::from_source` ⏳ TODO

### Why the Migration is Non-Trivial

1. **API incompatibility**: The old and new traits have different method signatures
2. **WIDE blast radius**: The parser module is used throughout the codebase
3. **Test coverage**: Many tests use the old PdfSource trait and would need updating
4. **Backward compatibility**: Need to ensure no regressions during migration

## Files Modified/Created

### Created:
- `crates/pdftract-core/src/source/mod.rs` - PdfSource trait definition
- `crates/pdftract-core/src/source/mmap.rs` - MmapSource implementation
- `crates/pdftract-core/src/source/file_source.rs` - FileSource implementation

### Modified:
- `crates/pdftract-core/src/lib.rs` - Added `pub mod source;` and re-exports

## Recommendations

### Option 1: Close bead with WARN (recommended)

Close this bead with the understanding that:
- The core deliverable (PdfSource trait + 2 implementations) is complete
- Parser migration is deferred to a follow-up bead
- The old PdfSource trait remains for compatibility during transition

### Option 2: Continue with parser migration

Extend this bead to complete Steps 3-5:
1. Add adapter pattern to Lexer
2. Update all parser modules to use new PdfSource
3. Remove old PdfSource trait from parser/stream.rs
4. Update all tests

This would require significant additional work and touches many files.

### Option 3: Create follow-up beads

Create separate beads for:
- Parser module migration (Phase 1.1-1.5)
- Old PdfSource removal
- Test migration

## Conclusion

The PdfSource trait is complete, well-documented, and properly implemented. The trait meets all the core requirements (Read+Seek+Send+Sync bounds, Bytes-based read_range, prefetch). The parser module migration is a significant undertaking that should be tracked separately to maintain clear scope boundaries.

**Recommendation**: Close this bead with WARN for parser migration, create follow-up bead(s) for the migration work.
