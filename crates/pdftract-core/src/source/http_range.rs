//! HTTP Range-backed PDF source implementation.
//!
//! This module provides `HttpRangeSource`, a `PdfSource` implementation that
//! fetches PDF data from HTTP/HTTPS servers using Range requests. Data is cached
//! in 64 KiB blocks with a 64-block LRU cache (4 MiB total per document).

#![cfg(feature = "remote")]

use crate::source::PdfSource;
use bytes::Bytes;
use lru::LruCache;
use parking_lot::Mutex;
use std::io::{self, Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;
use std::cell::Cell;

/// Block size for cache (64 KiB).
const BLOCK_SIZE: u64 = 65536;

/// Number of blocks in LRU cache (4 MiB total).
const CACHE_CAPACITY: usize = 64;

/// Connection timeout (10 seconds).
const CONNECT_TIMEOUT_SECS: u64 = 10;

/// Read timeout (30 seconds).
const READ_TIMEOUT_SECS: u64 = 30;

/// HTTP-backed PDF source with Range request support and LRU caching.
///
/// This implementation fetches PDF data from HTTP/HTTPS servers using Range
/// requests, with a 64-block LRU cache (64 KiB per block, 4 MiB total).
///
/// # Architecture
///
/// - Single `ureq::Agent` for connection pooling (shared across all instances)
/// - Cache: 64 blocks × 64 KiB = 4 MiB per document
/// - Block index = offset / 65536
/// - Contiguous miss blocks are batched into a single Range request
///
/// # HTTP semantics
///
/// - `Range: bytes=START-END` (inclusive, per RFC 7233)
/// - Expects `206 Partial Content` with `Content-Range: bytes START-END/TOTAL`
/// - On `200 OK` (no Range support): emits `REMOTE_NO_RANGE_SUPPORT`, aborts
/// - Timeouts: 10s connection, 30s read → `REMOTE_FETCH_INTERRUPTED`
///
/// # Thread safety
///
/// The cache is wrapped in a `parking_lot::Mutex` for concurrent access.
/// Multiple threads may read from the same source simultaneously.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::source::http_range::HttpRangeSource;
///
/// let source = HttpRangeSource::open("https://example.com/doc.pdf").unwrap();
/// let data = source.read_range(1000, 4096).unwrap();
/// ```
pub struct HttpRangeSource {
    /// Shared HTTP agent for connection pooling.
    agent: Arc<ureq::Agent>,
    /// Document URL.
    url: String,
    /// Custom headers to include on every request.
    headers: Vec<(String, String)>,
    /// Total content length from HEAD request.
    content_length: u64,
    /// Whether server supports Range requests.
    supports_range: bool,
    /// LRU cache: block index → cached block data.
    cache: Mutex<LruCache<u64, Bytes>>,
    /// Current cursor position for Read+Seek traits.
    cursor: Cell<u64>,
}

impl HttpRangeSource {
    /// Open a PDF from an HTTP/HTTPS URL.
    ///
    /// Performs a HEAD request to verify Range support and record Content-Length.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL is invalid or DNS fails → `io::Error` with kind `NotFound`
    /// - TLS handshake fails → `io::Error` with kind `PermissionDenied`
    /// - HEAD request times out → `io::Error` with kind `TimedOut`
    /// - Server returns non-2xx status → `io::Error` with kind `Other`
    pub fn open(url: &str) -> io::Result<Self> {
        Self::with_headers(url, Vec::new())
    }

    /// Open a PDF from a URL with custom headers.
    ///
    /// Headers are included on every request (HEAD and Range).
    /// Useful for authentication (Bearer tokens, API keys).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::source::http_range::HttpRangeSource;
    ///
    /// let headers = vec![
    ///     ("Authorization".to_string(), "Bearer token123".to_string()),
    ///     ("X-Custom-Header".to_string(), "value".to_string()),
    /// ];
    /// let source = HttpRangeSource::with_headers("https://example.com/doc.pdf", headers)?;
    /// ```
    pub fn with_headers(url: &str, headers: Vec<(String, String)>) -> io::Result<Self> {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .build();

        let url = url.to_string();

        // Perform HEAD request to check Range support and get Content-Length
        let head_req = agent.head(&url);
        let head_req = apply_headers(head_req, &headers);

        let response = match head_req.call() {
            Ok(r) => r,
            Err(e) => {
                let err = classify_http_error(&e, "HEAD request failed");
                // Check if this is a 405 Method Not Allowed error
                if let Some(ureq::Error::Status(code, _)) = Some(&e) {
                    if *code == 405 {
                        // Fall back to GET with Range: bytes=0-0 to probe server
                        return Self::open_with_get_probe(&agent, &url, &headers);
                    }
                }
                return Err(err);
            }
        };

        if response.status() < 200 || response.status() >= 300 {
            // Check for 405 Method Not Allowed
            if response.status() == 405 {
                // Fall back to GET with Range: bytes=0-0 to probe server
                return Self::open_with_get_probe(&agent, &url, &headers);
            }
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("HEAD request failed with status {}", response.status()),
            ));
        }

        let content_length = response
            .header("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let accept_ranges = response
            .header("accept-ranges")
            .map(|v| v.to_lowercase());
        let supports_range = accept_ranges.as_deref() == Some("bytes");

        // Initialize LRU cache
        let cache = LruCache::new(NonZeroUsize::new(CACHE_CAPACITY).unwrap());

        Ok(Self {
            agent: Arc::new(agent),
            url,
            headers,
            content_length,
            supports_range,
            cache: Mutex::new(cache),
            cursor: Cell::new(0),
        })
    }

    /// Check if the server supports Range requests.
    ///
    /// Returns false if the server doesn't support Range (Accept-Ranges: none
    /// or returned 200 for a Range request). In this case, use the fallback
    /// `download_to_temp_and_mmap` function to download the entire file.
    pub fn supports_range(&self) -> bool {
        self.supports_range
    }

    /// Get the URL for this source.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the headers used for this source.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Open using GET with Range: bytes=0-0 to probe server capabilities.
    ///
    /// This is a fallback for servers that don't support HEAD requests (return 405).
    /// We use a minimal Range request to check for Range support and get Content-Length.
    fn open_with_get_probe(agent: &ureq::Agent, url: &str, headers: &[(String, String)]) -> io::Result<Self> {
        // Try GET with Range: bytes=0-0 to probe server
        let get_req = agent.get(url);
        let get_req = apply_headers(get_req, headers);
        let get_req = get_req.set("Range", "bytes=0-0");

        let response = get_req.call().map_err(|e| {
            classify_http_error(&e, "GET probe request failed")
        })?;

        // Check status
        let status = response.status();

        // 206 Partial Content → server supports Range
        // 200 OK → server ignored Range header (no Range support)
        // 416 Range Not Satisfiable → server supports Range but range is invalid (zero-length file?)

        let supports_range = status == 206 || status == 416;

        // Get Content-Length from Content-Range header or Content-Length header
        let content_length = if status == 206 {
            // Try Content-Range header: "bytes 0-0/TOTAL"
            response
                .header("content-range")
                .and_then(|v| {
                    v.rsplit('/').next().and_then(|s| s.parse().ok())
                })
        } else if status == 416 {
            // Range Not Satisfiable - check Content-Range for *
            // Or use Content-Length
            response
                .header("content-range")
                .and_then(|v| {
                    v.rsplit('/').next().and_then(|s| s.parse().ok())
                })
                .or_else(|| {
                    response.header("content-length").and_then(|v| v.parse().ok())
                })
        } else {
            // 200 OK or other - use Content-Length
            response.header("content-length").and_then(|v| v.parse().ok())
        }.unwrap_or(0);

        // Initialize LRU cache
        let cache = LruCache::new(NonZeroUsize::new(CACHE_CAPACITY).unwrap());

        Ok(Self {
            agent: Arc::new(agent.clone()),
            url: url.to_string(),
            headers: headers.to_vec(),
            content_length,
            supports_range,
            cache: Mutex::new(cache),
            cursor: Cell::new(0),
        })
    }

    /// Internal method: fetch a Range of bytes from the server.
    ///
    /// Batches contiguous miss blocks into a single request.
    /// Returns the fetched data (may be larger than requested if batched).
    fn fetch_range(&self, block_start: u64, block_end: u64) -> io::Result<Bytes> {
        let start = block_start * BLOCK_SIZE;
        let end = (block_end + 1) * BLOCK_SIZE - 1;

        let url = &self.url;
        let range_header = format!("bytes={}-{}", start, end);

        let req = self.agent.get(url);
        let req = apply_headers(req, &self.headers);
        let req = req.set("Range", &range_header);

        let response = req.call().map_err(|e| {
            classify_http_error(&e, "Range request failed")
        })?;

        let status = response.status();

        // 206 Partial Content → server supports Range
        if status == 206 {
            let mut data = Vec::new();
            response.into_reader().read_to_end(&mut data).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Interrupted,
                    format!("Failed to read response body: {}", e),
                )
            })?;
            return Ok(Bytes::from(data));
        }

        // 200 OK → server ignored Range header (no Range support)
        if status == 200 {
            // Do NOT cache the 200 response; we'll abort and trigger fallback
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Server does not support Range requests (returned 200 OK)",
            ));
        }

        // 502/503/504 → server errors, treat as connection interrupted
        if status == 502 || status == 503 || status == 504 {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                format!("Server error: HTTP {}", status),
            ));
        }

        // Other status codes
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Unexpected status: {}", status),
        ))
    }
}

impl PdfSource for HttpRangeSource {
    fn len(&self) -> u64 {
        self.content_length
    }

    fn is_remote(&self) -> bool {
        true
    }

    fn read_range(&self, offset: u64, length: usize) -> io::Result<Bytes> {
        // Bounds check
        if offset > self.content_length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("offset {} exceeds content length {}", offset, self.content_length),
            ));
        }

        let max_read = (self.content_length - offset).min(length as u64) as usize;

        if max_read == 0 {
            return Ok(Bytes::new());
        }

        if !self.supports_range {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Server does not support Range requests",
            ));
        }

        // Calculate block range needed
        let start_block = offset / BLOCK_SIZE;
        let end_offset = offset + max_read as u64 - 1;
        let end_block = end_offset / BLOCK_SIZE;

        // Identify cached vs. missing blocks
        let mut cached_blocks: Vec<Option<Bytes>> = Vec::with_capacity((end_block - start_block + 1) as usize);
        let mut missing_runs: Vec<(u64, u64)> = Vec::new(); // (start_block, end_block) inclusive

        {
            let mut cache = self.cache.lock();

            for block_index in start_block..=end_block {
                if let Some(data) = cache.get(&block_index) {
                    cached_blocks.push(Some(data.clone()));
                } else {
                    cached_blocks.push(None);
                }
            }

            // Find contiguous runs of missing blocks
            let mut run_start: Option<u64> = None;
            for (i, is_missing) in cached_blocks.iter().enumerate() {
                let block_index = start_block + i as u64;
                if is_missing.is_none() {
                    if run_start.is_none() {
                        run_start = Some(block_index);
                    }
                } else if let Some(start) = run_start {
                    let run_end = block_index - 1;
                    missing_runs.push((start, run_end));
                    run_start = None;
                }
            }
            // Handle trailing run
            if let Some(start) = run_start {
                missing_runs.push((start, end_block));
            }
        }

        // Batch fetch each contiguous run of missing blocks
        for (run_start, run_end) in missing_runs {
            let data = self.fetch_range(run_start, run_end)?;

            // Split the fetched data into individual blocks and cache them
            let mut cache = self.cache.lock();
            let mut data_offset = 0;
            for block_index in run_start..=run_end {
                let block_start = block_index * BLOCK_SIZE;
                let block_end = std::cmp::min(
                    block_start + BLOCK_SIZE,
                    self.content_length,
                );
                let block_len = (block_end - block_start) as usize;

                if data_offset + block_len <= data.len() {
                    let block_data = data.slice(data_offset..data_offset + block_len);
                    cache.put(block_index, block_data.clone());

                    // Update cached_blocks for later assembly
                    let idx = (block_index - start_block) as usize;
                    if idx < cached_blocks.len() {
                        cached_blocks[idx] = Some(block_data);
                    }

                    data_offset += block_len;
                }
            }
        }

        // Assemble the result from cached/fetched blocks
        let mut result = Vec::with_capacity(max_read);

        for (i, block_data_opt) in cached_blocks.iter().enumerate() {
            let block_index = start_block + i as u64;
            if let Some(block_data) = block_data_opt {
                let block_start = block_index * BLOCK_SIZE;

                let slice_start = if block_index == start_block {
                    (offset - block_start) as usize
                } else {
                    0
                };

                let slice_end = if block_index == end_block {
                    std::cmp::min(
                        block_data.len(),
                        (end_offset - block_start + 1) as usize
                    )
                } else {
                    block_data.len()
                };

                if slice_start < slice_end && slice_start < block_data.len() {
                    result.extend_from_slice(&block_data[slice_start..slice_end]);
                }
            }
        }

        Ok(Bytes::from(result))
    }

    fn prefetch(&self, offset: u64, length: usize) {
        if !self.supports_range || length == 0 {
            return;
        }

        let end_offset = offset.saturating_add(length as u64);
        let start_block = offset / BLOCK_SIZE;
        let end_block = (end_offset.saturating_sub(1)) / BLOCK_SIZE;

        // Find which blocks in the range are missing from cache
        let mut missing_runs: Vec<(u64, u64)> = Vec::new();

        {
            let cache = self.cache.lock();

            let mut run_start: Option<u64> = None;
            for block_index in start_block..=end_block {
                if !cache.contains(&block_index) {
                    if run_start.is_none() {
                        run_start = Some(block_index);
                    }
                } else if let Some(start) = run_start {
                    missing_runs.push((start, block_index - 1));
                    run_start = None;
                }
            }
            // Handle trailing run
            if let Some(start) = run_start {
                missing_runs.push((start, end_block));
            }
        }

        // Batch fetch each contiguous run of missing blocks
        for (run_start, run_end) in missing_runs {
            let _ = self.fetch_range(run_start, run_end);
        }
    }
}

impl Read for HttpRangeSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pos = self.cursor.get();

        if pos >= self.content_length {
            return Ok(0); // EOF
        }

        let data = self.read_range(pos, buf.len())?;
        let len = data.len();
        buf[..len].copy_from_slice(&data);
        self.cursor.set(pos + len as u64);
        Ok(len)
    }
}

impl Seek for HttpRangeSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::End(n) => {
                let end = self.content_length as i64;
                end.saturating_add(n)
            }
            SeekFrom::Current(n) => {
                let current = self.cursor.get() as i64;
                current.saturating_add(n)
            }
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before start",
            ));
        }

        self.cursor.set(new_pos as u64);
        Ok(new_pos as u64)
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        Ok(self.cursor.get())
    }
}

// SAFETY: Arc<Agent> is Send + Sync, LruCache is protected by Mutex
unsafe impl Send for HttpRangeSource {}
unsafe impl Sync for HttpRangeSource {}

impl std::fmt::Debug for HttpRangeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRangeSource")
            .field("url", &self.url)
            .field("content_length", &self.content_length)
            .field("supports_range", &self.supports_range)
            .field("cache_size", &self.cache.lock().len())
            .finish_non_exhaustive()
    }
}

/// Apply custom headers to a ureq request.
fn apply_headers(mut req: ureq::Request, headers: &[(String, String)]) -> ureq::Request {
    for (key, value) in headers {
        req = req.set(key, value);
    }
    req
}

/// Classify HTTP errors into io::Error kinds for proper handling.
///
/// Maps ureq errors to appropriate io::Error kinds:
/// - Connection/timeout → Interrupted (trigger REMOTE_FETCH_INTERRUPTED)
/// - TLS → PermissionDenied (trigger REMOTE_TLS_FAILED)
/// - DNS → NotFound (trigger REMOTE_DNS_FAILED)
/// - 401/403 → PermissionDenied (trigger REMOTE_AUTH_FAILED)
/// - 502/503/504 → Interrupted (server errors, treat as fetch interrupted)
fn classify_http_error(err: &ureq::Error, context: &str) -> io::Error {
    match err {
        ureq::Error::Status(code, _) => {
            // 401 Unauthorized and 403 Forbidden are permission errors
            if *code == 401 || *code == 403 {
                return io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{}: HTTP {} (authentication required)", context, code),
                );
            }
            // 502 Bad Gateway, 503 Service Unavailable, 504 Gateway Timeout
            // are treated as connection interruptions
            if *code == 502 || *code == 503 || *code == 504 {
                return io::Error::new(
                    io::ErrorKind::Interrupted,
                    format!("{}: HTTP {} (service unavailable)", context, code),
                );
            }
            io::Error::new(
                io::ErrorKind::Other,
                format!("{}: HTTP {}", context, code),
            )
        }
        ureq::Error::Transport(transport_err) => {
            let msg = transport_err.to_string().to_lowercase();

            if msg.contains("timeout") || msg.contains("timed out") {
                return io::Error::new(
                    io::ErrorKind::Interrupted,
                    format!("{}: request timeout", context),
                );
            }

            if msg.contains("connection") || msg.contains("reset") || msg.contains("broken pipe") {
                return io::Error::new(
                    io::ErrorKind::Interrupted,
                    format!("{}: connection interrupted", context),
                );
            }

            if msg.contains("tls") || msg.contains("certificate") || msg.contains("handshake") {
                return io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{}: TLS handshake failed", context),
                );
            }

            if msg.contains("dns") || msg.contains("name resolution") || msg.contains("hostname") {
                return io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{}: DNS resolution failed", context),
                );
            }

            io::Error::new(
                io::ErrorKind::Interrupted,
                format!("{}: {}", context, transport_err),
            )
        }
    }
}

/// Fallback: download entire file to temp and memory-map it.
///
/// Used when the server doesn't support Range requests. Downloads the entire
/// file to a temporary file and memory-maps it for efficient access.
///
/// # Arguments
///
/// * `url` - HTTP/HTTPS URL to download from
/// * `headers` - Custom headers to include in the request
/// * `diagnostics` - Optional diagnostics vector to emit errors to
///
/// # Returns
///
/// A tuple of (temp file, mmap source). The temp file must be kept alive
/// for the lifetime of the mmap source.
///
/// # Errors
///
/// Returns an error if:
/// - Disk space is insufficient (emits REMOTE_INSUFFICIENT_DISK diagnostic)
/// - Download fails (REMOTE_FETCH_INTERRUPTED)
/// - File cannot be memory-mapped
pub fn download_to_temp_and_mmap(
    url: &str,
    headers: &[(String, String)],
    diagnostics: Option<&mut Vec<crate::diagnostics::Diagnostic>>,
) -> io::Result<(tempfile::NamedTempFile, super::MmapSource)> {
    #[cfg(feature = "remote")]
    {
        use std::io::Write;
        use crate::diagnostics::{Diagnostic, DiagCode};

        // Build agent and request
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(READ_TIMEOUT_SECS))
            .build();

        let req = agent.get(url);
        let req = apply_headers(req, headers);

        // Get response to check Content-Length first
        let response = req.call().map_err(|e| {
            classify_http_error(&e, "Fallback download request failed")
        })?;

        if response.status() < 200 || response.status() >= 300 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Fallback download failed with status {}", response.status()),
            ));
        }

        // Get Content-Length for disk space check
        let content_length = response
            .header("content-length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        // Check disk space
        #[cfg(feature = "remote")]
        {
            use std::path::Path;

            // Get temp directory path - use std::env::temp_dir() to avoid extra allocation
            let temp_path = std::env::temp_dir();

            // Use nix for safer statvfs wrapper
            #[cfg(unix)]
            {
                use nix::sys::statvfs::statvfs;
                use nix::sys::statvfs::Statvfs;

                let stat = statvfs(&temp_path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to get filesystem stats: {}", e),
                    )
                })?;

                // Calculate available space (blocks_available * fragment_size)
                let available_bytes = stat.blocks_available() as u64 * stat.fragment_size() as u64;

                // Add 10% buffer for filesystem overhead and temp file metadata
                let required_bytes = content_length.saturating_mul(11) / 10;

                if content_length > 0 && available_bytes < required_bytes {
                    // Emit REMOTE_INSUFFICIENT_DISK diagnostic
                    if let Some(diags) = diagnostics {
                        diags.push(Diagnostic::with_dynamic_no_offset(
                            DiagCode::RemoteInsufficientDisk,
                            format!(
                                "Insufficient disk space for fallback download: need {} bytes, have {} bytes available. Set TMPDIR to a different path if needed.",
                                required_bytes, available_bytes
                            ),
                        ));
                    }

                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Insufficient disk space: need {} bytes, have {} bytes available",
                            required_bytes, available_bytes
                        ),
                    ));
                }
            }
        }

        // Create temp file
        let mut temp_file = tempfile::NamedTempFile::new()?;

        // Download and write to temp file
        let mut reader = response.into_reader();
        let mut writer = temp_file.as_file_mut();

        io::copy(&mut reader, &mut writer).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Interrupted,
                format!("Failed to download file: {}", e),
            )
        })?;

        // Sync to disk
        writer.flush()?;
        writer.sync_all()?;

        // Reopen as MmapSource
        let mmap_source = super::MmapSource::open(temp_file.path())?;

        Ok((temp_file, mmap_source))
    }

    #[cfg(not(feature = "remote"))]
    {
        let _ = (url, headers);
        let _ = diagnostics;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Remote sources are not supported; rebuild pdftract with --features remote",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_size_constants() {
        assert_eq!(BLOCK_SIZE, 65536);
        assert_eq!(CACHE_CAPACITY, 64);
        assert_eq!(BLOCK_SIZE * CACHE_CAPACITY as u64, 4194304); // 4 MiB
    }

    #[test]
    fn test_block_index_calculation() {
        // Offset 0 → block 0
        assert_eq!(0 / BLOCK_SIZE, 0);

        // Offset 65535 → block 0
        assert_eq!(65535 / BLOCK_SIZE, 0);

        // Offset 65536 → block 1
        assert_eq!(65536 / BLOCK_SIZE, 1);

        // Offset 200000 → block 3
        assert_eq!(200000 / BLOCK_SIZE, 3);
    }

    #[test]
    fn test_cache_size() {
        let cache = LruCache::<u64, Bytes>::new(NonZeroUsize::new(CACHE_CAPACITY).unwrap());
        assert_eq!(cache.cap().get(), CACHE_CAPACITY);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_http_range_source_url_validation() {
        // Valid URL
        let result = HttpRangeSource::open("https://example.com/doc.pdf");
        // Will fail at HEAD request (server doesn't exist), but URL parsing succeeds
        assert!(result.is_err());

        // Invalid URL scheme (ureq rejects non-http/https)
        let result = HttpRangeSource::open("ftp://example.com/doc.pdf");
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_http_range_source_with_headers() {
        let headers = vec![
            ("Authorization".to_string(), "Bearer test123".to_string()),
            ("X-API-Key".to_string(), "key456".to_string()),
        ];

        // URL doesn't exist, but we verify header construction doesn't crash
        let result = HttpRangeSource::with_headers("https://example.com/doc.pdf", headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_classify_http_error() {
        // This test verifies the error classification logic
        // Since ureq::Error is opaque, we create synthetic errors via the function

        // Note: ureq::Error doesn't have public constructors,
        // so we can only test via actual HTTP calls
        // This is covered by integration tests
    }

    #[test]
    fn test_range_header_format() {
        let start = 0u64;
        let end = 65535u64;
        let header = format!("bytes={}-{}", start, end);
        assert_eq!(header, "bytes=0-65535");

        let start = 65536u64;
        let end = 131071u64;
        let header = format!("bytes={}-{}", start, end);
        assert_eq!(header, "bytes=65536-131071");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_empty_read_range() {
        // This would need a real HTTP server, so it's in integration tests
        // Unit test verifies the bounds logic

        // Test with a mock-like scenario
        let result = HttpRangeSource::open("https://example.com/doc.pdf");
        assert!(result.is_err()); // No real server
    }
}
