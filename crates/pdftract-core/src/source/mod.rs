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

    /// Get the underlying source as a `dyn PdfSource` trait object.
    ///
    /// This is used when you need to erase the concrete type and work with
    /// the trait object (e.g., when passing to functions that accept `&dyn PdfSource`).
    fn as_source(&self) -> &dyn PdfSource
    where
        Self: Sized,
    {
        self
    }
}

/// Open a PDF source from a path or URL string.
///
/// This function detects whether the input is:
/// - An HTTP/HTTPS URL → creates HttpRangeSource with optional headers
/// - A local file path → creates FileSource
///
/// # Arguments
///
/// * `path_or_url` - Path to a local PDF file or HTTP/HTTPS URL
/// * `headers` - Optional custom HTTP headers (only used for HTTP/HTTPS URLs)
///
/// # Returns
///
/// A `Box<dyn PdfSource>` that can be used for PDF parsing.
///
/// # Errors
///
/// Returns an error if:
/// - The path/URL is invalid
/// - The file cannot be opened
/// - The HTTP HEAD request fails (for URLs)
/// - TLS handshake fails
///
/// # Example
///
/// ```ignore
/// use pdftract_core::source::open_source;
///
/// // Local file
/// let source = open_source("document.pdf", None)?;
///
/// // HTTP URL with headers
/// let headers = vec![
///     ("Authorization".to_string(), "Bearer token".to_string()),
///     ("X-API-Key".to_string(), "key123".to_string()),
/// ];
/// let source = open_source("https://example.com/doc.pdf", Some(headers))?;
/// ```
pub fn open_source(
    path_or_url: &str,
    headers: Option<Vec<(String, String)>>,
) -> io::Result<Box<dyn PdfSource>> {
    // Check if this is an HTTP/HTTPS URL
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        // Use HttpRangeSource for URLs
        let headers_vec = headers.unwrap_or_default();
        let source = HttpRangeSource::with_headers(path_or_url, headers_vec)?;
        Ok(Box::new(source))
    } else {
        // Use FileSource for local paths
        let source = FileSource::open(path_or_url)?;
        Ok(Box::new(source))
    }
}

mod file_source;
mod http_range;
mod mmap;

pub use file_source::FileSource;
pub use http_range::HttpRangeSource;
pub use mmap::MmapSource;
