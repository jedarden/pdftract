//! PDF source abstraction.
//!
//! This module defines the `PdfSource` trait, which abstracts over different
//! sources of PDF byte data (local files, memory-mapped files, remote HTTP sources).
//! The trait provides a uniform API for parsers to read PDF data regardless of
//! the underlying storage mechanism.
//!
//! # Example
//!
//! ```ignore
//! use pdftract_core::source::PdfSource;
//!
//! // Read using Read+Seek adapter (standard IO trait pattern)
//! fn read_header(source: &dyn PdfSource) -> std::io::Result<String> {
//!     let mut buffer = vec![0u8; 1024];
//!     source.read(&mut buffer)?;
//!     Ok(String::from_utf8_lossy(&buffer).to_string())
//! }
//!
//! // Read using direct read_range (zero-copy Bytes)
//! fn read_xref(source: &dyn PdfSource, offset: u64) -> std::io::Result<bytes::Bytes> {
//!     source.read_range(offset, 4096)
//! }
//! ```

use bytes::Bytes;
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;

/// Abstraction over PDF byte sources.
///
/// This trait provides a uniform interface for reading PDF data from different
/// sources: local files (MmapSource, FileSource), memory buffers, and remote
/// HTTP sources (HttpRangeSource in Phase 1.8).
///
/// # Object safety
///
/// The trait is object-safe, allowing `&dyn PdfSource` to be used for dynamic
/// dispatch. This is important for APIs that need to accept any source type
/// at runtime.
///
/// # Thread safety
///
/// All sources must be `Send + Sync` to support rayon page-parallelism in
/// Phase 3+. Multiple threads may read from the same source concurrently.
///
/// # Example: Read+Seek adapter
///
/// ```ignore
/// use pdftract_core::source::PdfSource;
/// use std::io::Read;
///
/// fn parse_trailer(source: &dyn PdfSource) -> std::io::Result<Vec<u8>> {
///     let mut buffer = Vec::new();
///     source.seek(io::SeekFrom::End(-1024))?;
///     source.read_to_end(&mut buffer)?;
///     Ok(buffer)
/// }
/// ```
///
/// # Example: Direct read_range
///
/// ```ignore
/// use pdftract_core::source::PdfSource;
///
/// fn read_xref_section(source: &dyn PdfSource, offset: u64) -> io::Result<bytes::Bytes> {
///     // Zero-copy read using Bytes
///     source.read_range(offset, 4096)
/// }
/// ```
pub trait PdfSource: Read + Seek + Send + Sync {
    /// Total length of the source in bytes.
    ///
    /// This must return the exact byte length of the PDF source. For file-backed
    /// sources, this is the file size. For HTTP sources, this is the Content-Length.
    fn len(&self) -> u64;

    /// Read `length` bytes starting at `offset`.
    ///
    /// Returns a `Bytes` object for zero-copy slicing. The returned Bytes may
    /// be a view into the source's internal buffer (for memory-mapped or cached
    /// sources), so cloning the Bytes is cheap.
    ///
    /// # Bounds
    ///
    /// - `offset + length <= len()`: Returns io::Error with kind `InvalidInput`
    ///   if the range exceeds the source length.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::source::PdfSource;
    ///
    /// let data = source.read_range(100, 512)?;
    /// assert_eq!(data.len(), 512);
    /// ```
    fn read_range(&self, offset: u64, length: usize) -> io::Result<Bytes>;

    /// Optional hint to pre-fetch a range.
    ///
    /// For local sources (MmapSource, FileSource), this is a no-op since the
    /// OS manages paging via the page cache.
    ///
    /// For remote HTTP sources (HttpRangeSource, Phase 1.8), this issues a
    /// speculative Range request to warm the cache for upcoming reads.
    ///
    /// The default implementation is a no-op.
    fn prefetch(&self, _offset: u64, _length: usize) {}
}

mod file_source;
mod mmap;

pub use file_source::FileSource;
pub use mmap::MmapSource;
