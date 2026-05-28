//! Remote PDF loading and extraction.
//!
//! This module provides the HTTP fetch sequence for remote PDFs:
//! 1. HEAD probe to verify Range support and get Content-Length
//! 2. Tail Range fetch to parse startxref, trailer, and root xref subsection
//! 3. Xref parsing with forward-scan disabled for remote sources
//! 4. Page-by-page on-demand fetch as the document model dereferences each page
//! 5. Resource lazy load (fonts and XObjects fetched on first reference)
//!
//! # Example
//!
//! ```ignore
//! use pdftract_core::remote::{open_remote, RemoteOpts};
//!
//! let opts = RemoteOpts::new()
//!     .with_header("Authorization", "Bearer token");
//!
//! // Open the remote PDF (for custom processing)
//! let (catalog, resolver, source, fingerprint) = open_remote("https://example.com/doc.pdf", &opts)?;
//! ```

use crate::document::compute_fingerprint_lazy;
use crate::parser::catalog::{parse_catalog, Catalog};
use crate::parser::xref::{load_xref_with_prev_chain, XrefResolver};
use crate::source::{open_remote as open_remote_source, RemoteOpts};
use anyhow::{anyhow, Context, Result};

/// Open a PDF from a remote HTTP/HTTPS URL.
///
/// This function performs the HTTP fetch sequence:
/// 1. HEAD request to verify Range support and get Content-Length
/// 2. Tail Range fetch (last 16 KB) to parse startxref and trailer
/// 3. Xref parsing with forward-scan disabled for remote sources
/// 4. Returns the parsed catalog, resolver, source, and fingerprint
///
/// # Arguments
///
/// * `url` - HTTP/HTTPS URL to the PDF file
/// * `opts` - Remote options (headers, credentials, etc.)
///
/// # Returns
///
/// A tuple of (catalog, resolver, source, fingerprint) for further processing.
///
/// # Errors
///
/// Returns an error if:
/// - URL is invalid or DNS fails → Error kind "NotFound"
/// - TLS handshake fails → Error kind "PermissionDenied"
/// - Server returns 401/403 → Error kind "PermissionDenied"
/// - Server doesn't support Range → Error kind "Unsupported"
/// - HEAD fails with 405 → Falls back to GET with Range: bytes=0-0
/// - No Content-Length → Returns error with REMOTE_NO_CONTENT_LENGTH diagnostic
///
/// # Example
///
/// ```ignore
/// use pdftract_core::remote::{open_remote, RemoteOpts};
///
/// let opts = RemoteOpts::new()
///     .with_header("Authorization", "Bearer token");
///
/// let (catalog, resolver, source, fingerprint) = open_remote("https://example.com/doc.pdf", &opts)?;
/// // Use catalog, resolver, source for custom processing
/// ```
pub fn open_remote(
    url: &str,
    opts: &RemoteOpts,
) -> Result<(Catalog, XrefResolver, Box<dyn crate::parser::stream::PdfSource>, String)> {
    use crate::parser::stream::PdfSource as ParserPdfSource;

    // Open the remote PDF source
    let source = open_remote_source(url, opts).context("Failed to open remote PDF source")?;

    // Convert source to parser PdfSource
    // The blanket impl in parser/stream.rs converts any source::PdfSource to parser::stream::PdfSource
    let parser_source: Box<dyn ParserPdfSource> = source;

    // Find the startxref offset using progressive tail fetch for remote sources
    // This starts with 16 KB and progressively fetches larger tails if needed
    let startxref_offset = find_startxref_progressive(&*parser_source)
        .context("Failed to find startxref offset")?;

    // Load the xref table (forward-scan is disabled for remote sources)
    let xref_section = load_xref_with_prev_chain(&*parser_source, startxref_offset);

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = xref_section
        .trailer
        .as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| obj.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No /Root reference in trailer"))?;

    // Parse the catalog
    let catalog = parse_catalog(&resolver, root_ref, Some(&*parser_source as &dyn ParserPdfSource))
        .map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow::anyhow!("Failed to parse catalog: {}", msg)
        })?;

    // Resolve AcroForm dictionary if present (for XFA detection and fingerprint)
    let acroform = catalog
        .acroform_ref
        .and_then(|r| resolver.resolve(r).ok())
        .and_then(|o| o.as_dict())
        .cloned();

    // Build fingerprint input (without full page tree for lazy extraction)
    let fingerprint = compute_fingerprint_lazy(&catalog, &resolver, &acroform);

    Ok((catalog, resolver, parser_source, fingerprint))
}

/// Find the startxref offset in a PDF file.
///
/// Scans the last 1024 bytes of the file for "startxref" keyword.
fn find_startxref(source: &dyn crate::parser::stream::PdfSource) -> Result<u64> {
    let len = source.len()? as usize;
    let scan_start = len.saturating_sub(1024);
    let scan_end = len;

    let tail_data = source
        .read_at(scan_start as u64, scan_end - scan_start)
        .context("Failed to read PDF tail")?;

    // Find "startxref" in the tail data
    let startxref_pos = tail_data
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| anyhow!("startxref not found in PDF"))?;

    // Parse the offset after "startxref"
    // Skip the "startxref" keyword (9 chars) and any following whitespace
    let offset_data = &tail_data[startxref_pos + 9..];

    // Skip leading whitespace (space, \r, \n, \t)
    let offset_start = offset_data
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
        .unwrap_or(offset_data.len());

    let offset_data_trimmed = &offset_data[offset_start..];

    // Find the newline after the offset
    let newline_pos = offset_data_trimmed
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(offset_data_trimmed.len());

    let offset_str = std::str::from_utf8(&offset_data_trimmed[..newline_pos])
        .context("startxref offset is not valid UTF-8")?;

    let offset: u64 = offset_str
        .trim()
        .parse()
        .context("startxref offset is not a valid number")?;

    Ok(offset)
}

/// Find the startxref offset with progressive tail fetching for remote PDFs.
///
/// For remote sources, we start with a 16 KB tail fetch. If the startxref offset
/// points before the tail, we progressively fetch larger tails (32, 64, ..., 1024 KB)
/// until we capture the startxref.
///
/// # Parameters
/// - `source`: The PDF source to read from
///
/// # Returns
/// The startxref offset, or an error if not found after progressive fetching
fn find_startxref_progressive(source: &dyn crate::parser::stream::PdfSource) -> Result<u64> {
    const INITIAL_TAIL: u64 = 16 * 1024; // 16 KB
    const MAX_TAIL: u64 = 1024 * 1024;  // 1 MB maximum

    let file_len = source.len()?;

    // Try with progressively larger tails
    let mut tail_size = INITIAL_TAIL;
    while tail_size <= MAX_TAIL {
        let scan_start = file_len.saturating_sub(tail_size) as usize;
        let scan_end = file_len as usize;

        let tail_data = source
            .read_at(scan_start as u64, scan_end - scan_start)
            .context("Failed to read PDF tail")?;

        // Find "startxref" in the tail data
        if let Some(startxref_pos) = tail_data.windows(9).rposition(|w| w == b"startxref") {
            // Parse the offset after "startxref"
            let offset_data = &tail_data[startxref_pos + 9..];

            // Skip leading whitespace
            let offset_start = offset_data
                .iter()
                .position(|&b| !matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
                .unwrap_or(offset_data.len());

            let offset_data_trimmed = &offset_data[offset_start..];

            // Find the newline after the offset
            let newline_pos = offset_data_trimmed
                .iter()
                .position(|&b| b == b'\n' || b == b'\r')
                .unwrap_or(offset_data_trimmed.len());

            let offset_str = std::str::from_utf8(&offset_data_trimmed[..newline_pos])
                .context("startxref offset is not valid UTF-8")?;

            let offset: u64 = offset_str
                .trim()
                .parse()
                .context("startxref offset is not a valid number")?;

            // Check if startxref points before the tail (meaning the xref is not in this tail)
            let startxref_absolute = scan_start as u64 + startxref_pos as u64;
            if offset >= startxref_absolute as u64 {
                // The xref is within the tail we just read
                return Ok(offset);
            }

            // startxref points before our tail - need larger tail
            tail_size *= 2;
        } else {
            // No startxref found - try larger tail
            tail_size *= 2;
        }
    }

    Err(anyhow!(
        "startxref not found after progressive tail fetch up to {} KB",
        MAX_TAIL / 1024
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_startxref() {
        // Test data with startxref at the end
        let test_data = b"Some PDF content...%%EOF\nstartxref\n12345\n%%EOF";
        let source = crate::parser::stream::MemorySource::new(test_data.to_vec());

        let offset = find_startxref(&source).unwrap();
        assert_eq!(offset, 12345);
    }

    #[test]
    fn test_find_startxref_with_crlf() {
        // Test data with CRLF line endings
        let test_data = b"Some PDF content...%%EOF\r\nstartxref\r\n67890\r\n%%EOF";
        let source = crate::parser::stream::MemorySource::new(test_data.to_vec());

        let offset = find_startxref(&source).unwrap();
        assert_eq!(offset, 67890);
    }

    #[test]
    fn test_find_startxref_with_extra_whitespace() {
        // Test data with extra whitespace
        let test_data = b"Some PDF content...%%EOF\nstartxref\t   \n99999\n%%EOF";
        let source = crate::parser::stream::MemorySource::new(test_data.to_vec());

        let offset = find_startxref(&source).unwrap();
        assert_eq!(offset, 99999);
    }

    #[test]
    fn test_find_startxref_not_found() {
        // Test data without startxref
        let test_data = b"Some PDF content...%%EOF\n%%EOF";
        let source = crate::parser::stream::MemorySource::new(test_data.to_vec());

        let result = find_startxref(&source);
        assert!(result.is_err());
    }
}
