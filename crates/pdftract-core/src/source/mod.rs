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
use std::io::{self, Read, Seek};
#[cfg(feature = "remote")]
use std::io::SeekFrom;

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

    /// Advisory hint to pre-fetch a range.
    ///
    /// For local sources (MmapSource, FileSource), this is a no-op since the
    /// OS manages paging via the page cache.
    ///
    /// For remote HTTP sources (HttpRangeSource, Phase 1.8), this issues a
    /// speculative Range request to warm the cache for upcoming reads.
    ///
    /// Prefetch must never fail the caller or change read correctness. A failed
    /// prefetch degrades readahead performance only; the subsequent read must
    /// still obtain the requested bytes normally. Failures SHOULD remain
    /// observable through a structured `tracing` event: use `trace` for local
    /// kernel hints such as `madvise`, and `debug` for network fetches. The
    /// reference implementations are `MmapSource::prefetch` and
    /// `HttpRangeSource::prefetch`.
    ///
    /// The default implementation is a deliberate no-op for sources with
    /// nothing to warm.
    fn prefetch(&self, _offset: u64, _length: usize) {}

    /// Check if this is a remote source (HTTP/HTTPS).
    ///
    /// Returns true for HttpRangeSource, false for local sources (MmapSource, FileSource).
    /// This is used to disable forward-scan xref recovery for remote sources, which would
    /// require fetching the entire file.
    ///
    /// The default implementation returns false (local source).
    fn is_remote(&self) -> bool {
        false
    }

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

/// Observation-only callback for remote fetch instrumentation.
///
/// Invoked with the number of body bytes successfully read from a remote
/// fetch (HTTP range reads and full fallback downloads). Callers above
/// `pdftract-core` (the cli's metrics registry) register one of these
/// because core cannot depend on cli types. A hook must never influence
/// fetch behavior and must not panic.
#[cfg(feature = "remote")]
pub type BytesDownloadedHook = std::sync::Arc<dyn Fn(u64) + Send + Sync>;

/// Options for opening a remote PDF source.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::source::RemoteOpts;
///
/// let opts = RemoteOpts::new()
///     .with_header("Authorization", "Bearer token")
///     .with_header("X-API-Key", "key123");
/// ```
#[cfg(feature = "remote")]
#[derive(Clone, Default)]
pub struct RemoteOpts {
    /// Custom HTTP headers to include on every request.
    headers: Vec<(String, String)>,
    /// Observation-only bytes-downloaded hook (metrics seam).
    download_hook: Option<BytesDownloadedHook>,
}

#[cfg(feature = "remote")]
impl std::fmt::Debug for RemoteOpts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The hook is deliberately omitted: it is not part of the
        // options' identity and callbacks do not implement Debug.
        f.debug_struct("RemoteOpts")
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "remote")]
impl RemoteOpts {
    /// Create a new RemoteOpts with default settings (no custom headers).
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom header to the request.
    ///
    /// Headers are included on every HEAD and Range request.
    /// Useful for authentication (Bearer tokens, API keys).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::source::RemoteOpts;
    ///
    /// let opts = RemoteOpts::new()
    ///     .with_header("Authorization", "Bearer token123")
    ///     .with_header("X-Custom", "value");
    /// ```
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    /// Add Basic Authentication credentials.
    ///
    /// This adds an `Authorization` header with Basic authentication
    /// (base64-encoded username:password).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::source::RemoteOpts;
    ///
    /// let opts = RemoteOpts::new()
    ///     .with_credentials("user", "pass");
    /// ```
    pub fn with_credentials(self, username: &str, password: &str) -> Self {
        use base64::prelude::*;
        let creds = format!("{}:{}", username, password);
        let encoded = BASE64_STANDARD.encode(creds);
        self.with_header("Authorization", &format!("Basic {}", encoded))
    }

    /// Get the headers as a vector.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Attach an observation-only bytes-downloaded hook.
    ///
    /// See [`BytesDownloadedHook`]. The hook is fired from the remote
    /// fetch path; attaching one never changes fetch behavior.
    pub fn with_bytes_downloaded_hook(mut self, hook: BytesDownloadedHook) -> Self {
        self.download_hook = Some(hook);
        self
    }

    /// The registered bytes-downloaded hook, if any.
    pub fn bytes_downloaded_hook(&self) -> Option<BytesDownloadedHook> {
        self.download_hook.clone()
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
#[cfg(feature = "remote")]
pub fn open_source(
    path_or_url: &str,
    headers: Option<Vec<(String, String)>>,
) -> io::Result<Box<dyn PdfSource>> {
    // Check if this is an HTTP/HTTPS URL
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        // Use HttpRangeSource for URLs
        let headers_vec = headers.unwrap_or_default();
        let source = HttpRangeSource::with_headers(path_or_url, headers_vec)?;

        // Check if Range is supported; if not, trigger fallback
        if !source.supports_range() {
            // Download to temp file and memory-map
            let (temp_file, mmap_source) =
                http_range::download_to_temp_and_mmap(source.url(), source.headers(), None)?;

            // Wrap in TempMmapSource to keep temp file alive
            return Ok(Box::new(TempMmapSource::new(temp_file, mmap_source)));
        }

        Ok(Box::new(source))
    } else {
        // Use FileSource for local paths
        let source = FileSource::open(path_or_url)?;
        Ok(Box::new(source))
    }
}

/// Open a PDF source from a remote HTTP/HTTPS URL.
///
/// This function performs a HEAD request to verify Range support and get Content-Length.
/// If the server doesn't support Range requests, it falls back to downloading the entire
/// file to a temporary file and memory-mapping it.
///
/// # Arguments
///
/// * `url` - HTTP/HTTPS URL to the PDF file
/// * `opts` - Remote options (headers, credentials, etc.)
/// * `diagnostics` - Optional diagnostics vector to emit warnings to
///
/// # Returns
///
/// A `Box<dyn PdfSource>` that can be used for PDF parsing.
///
/// # Errors
///
/// Returns an error if:
/// - The URL is invalid or DNS fails → io::Error with kind `NotFound`
/// - TLS handshake fails → io::Error with kind `PermissionDenied`
/// - Server returns 401/403 → io::Error with kind `PermissionDenied`
/// - Disk space is insufficient for fallback download → io::Error with kind `Other`
/// - HEAD fails with 405 → Falls back to GET with Range: bytes=0-0
///
/// # Behavior when Range is not supported
///
/// If the server doesn't support Range requests (Accept-Ranges: none or returns 200 for Range),
/// this function:
/// 1. Emits a REMOTE_NO_RANGE_SUPPORT diagnostic (if diagnostics vector provided)
/// 2. Downloads the entire file to a temporary file
/// 3. Memory-maps the temporary file
/// 4. Returns the memory-mapped source
///
/// # Example
///
/// ```ignore
/// use pdftract_core::source::{open_remote, RemoteOpts};
///
/// let opts = RemoteOpts::new()
///     .with_header("Authorization", "Bearer token");
///
/// let source = open_remote("https://example.com/doc.pdf", &opts, None)?;
/// ```
#[cfg(feature = "remote")]
pub fn open_remote(
    url: &str,
    opts: &RemoteOpts,
    mut diagnostics: Option<&mut Vec<crate::diagnostics::Diagnostic>>,
) -> io::Result<Box<dyn PdfSource>> {
    let download_hook = opts.bytes_downloaded_hook();
    let source = HttpRangeSource::with_headers_and_hook(
        url,
        opts.headers().to_vec(),
        download_hook.clone(),
    )?;

    // Check if Range is supported; if not, trigger fallback
    if !source.supports_range() {
        // Emit REMOTE_NO_RANGE_SUPPORT diagnostic
        if let Some(diags) = diagnostics.as_mut() {
            use crate::diagnostics::{DiagCode, Diagnostic};
            diags.push(Diagnostic::with_static_no_offset(
                DiagCode::RemoteNoRangeSupport,
                "Server does not support Range requests; falling back to full file download",
            ));
        }

        // Download to temp file and memory-map
        let (temp_file, mmap_source) = http_range::download_to_temp_and_mmap_with_hook(
            source.url(),
            source.headers(),
            download_hook,
            diagnostics,
        )?;

        // Wrap in TempMmapSource to keep temp file alive
        return Ok(Box::new(TempMmapSource::new(temp_file, mmap_source)));
    }

    Ok(Box::new(source))
}

/// Open a PDF source from a local file path.
///
/// This function only supports local file paths when the remote feature is disabled.
/// For URL support, enable the `remote` feature.
///
/// # Arguments
///
/// * `path_or_url` - Path to a local PDF file
///
/// # Returns
///
/// A `Box<dyn PdfSource>` that can be used for PDF parsing.
///
/// # Errors
///
/// Returns an error if:
/// - The path is invalid
/// - The file cannot be opened
#[cfg(not(feature = "remote"))]
pub fn open_source(
    path_or_url: &str,
    _headers: Option<Vec<(String, String)>>,
) -> io::Result<Box<dyn PdfSource>> {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Remote sources are not supported; rebuild pdftract with --features remote",
        ));
    }
    // Use FileSource for local paths
    let source = FileSource::open(path_or_url)?;
    Ok(Box::new(source))
}

mod file_source;
#[cfg(feature = "remote")]
mod http_range;
mod memory;
mod mmap;

pub use file_source::FileSource;
#[cfg(feature = "remote")]
pub use http_range::HttpRangeSource;
pub use memory::MemorySource;
pub use mmap::MmapSource;

/// Wrapper that keeps a temp file alive for the lifetime of a MmapSource.
///
/// When HTTP Range requests aren't supported, we fall back to downloading
/// the entire file to a temp file and memory-mapping it. This wrapper ensures
/// the temp file isn't deleted before the mmap is done using it.
#[cfg(feature = "remote")]
pub struct TempMmapSource {
    /// The temp file (kept alive to prevent deletion)
    _temp_file: tempfile::NamedTempFile,
    /// The memory-mapped source
    mmap: MmapSource,
}

#[cfg(feature = "remote")]
impl TempMmapSource {
    /// Create a new TempMmapSource from a temp file and its mmap.
    pub fn new(temp_file: tempfile::NamedTempFile, mmap: MmapSource) -> Self {
        Self {
            _temp_file: temp_file,
            mmap,
        }
    }
}

#[cfg(feature = "remote")]
impl PdfSource for TempMmapSource {
    fn len(&self) -> u64 {
        self.mmap.len()
    }

    fn read_range(&self, offset: u64, length: usize) -> io::Result<Bytes> {
        self.mmap.read_range(offset, length)
    }

    fn prefetch(&self, offset: u64, length: usize) {
        self.mmap.prefetch(offset, length)
    }
}

#[cfg(feature = "remote")]
impl Read for TempMmapSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.mmap.read(buf)
    }
}

#[cfg(feature = "remote")]
impl Seek for TempMmapSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.mmap.seek(pos)
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        self.mmap.stream_position()
    }
}

// SAFETY: MmapSource is Send + Sync, and tempfile::NamedTempFile is Send
#[cfg(feature = "remote")]
unsafe impl Send for TempMmapSource {}
#[cfg(feature = "remote")]
unsafe impl Sync for TempMmapSource {}

#[cfg(all(test, feature = "remote"))]
mod remote_opts_hook_tests {
    use super::*;

    /// The builder round-trips the hook; the default has none. This is
    /// the registration contract the cli's metrics module relies on.
    #[test]
    fn test_remote_opts_bytes_downloaded_hook_round_trip() {
        assert!(RemoteOpts::default().bytes_downloaded_hook().is_none());

        let opts = RemoteOpts::new()
            .with_header("X-Probe", "1")
            .with_bytes_downloaded_hook(std::sync::Arc::new(|_| {}));
        let hook = opts
            .bytes_downloaded_hook()
            .expect("hook survives the builder round trip");

        // The hook is invocable and observation-only: calling it is a
        // no-op on the options.
        hook(42);
        assert_eq!(opts.headers(), &[("X-Probe".to_string(), "1".to_string())]);
    }
}
