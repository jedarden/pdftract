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
//! use pdftract_core::options::ExtractionOptions;
//!
//! let opts = RemoteOpts::new()
//!     .with_header("Authorization", "Bearer token");
//!
//! // Just open the remote PDF (for custom processing)
//! let (catalog, resolver, source, fingerprint) = open_remote("https://example.com/doc.pdf", &opts)?;
//!
//! // Or extract directly
//! let result = extract_remote("https://example.com/doc.pdf", &opts, &ExtractionOptions::default())?;
//! ```

use crate::document::compute_fingerprint_lazy;
use crate::extract::{extract_pdf_from_source, ExtractionSource};
use crate::options::ExtractionOptions;
use crate::parser::catalog::{parse_catalog, Catalog};
use crate::parser::hint_stream;
use crate::parser::xref::{detect_linearization, load_xref_with_prev_chain, XrefResolver};
use crate::source::{open_remote as open_remote_source, RemoteOpts};
use anyhow::{Context, Result};

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

    // Find the startxref offset (reads last 1 KB of the file)
    let startxref_offset = find_startxref(&*source).context("Failed to find startxref offset")?;

    // Load the xref table (forward-scan is disabled for remote sources)
    let xref_section = load_xref_with_prev_chain(&*source, startxref_offset);

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
    let catalog = parse_catalog(&resolver, root_ref, Some(&*source as &dyn ParserPdfSource)).map_err(
        |diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow::anyhow!("Failed to parse catalog: {}", msg)
        },
    )?;

    // Resolve AcroForm dictionary if present (for XFA detection and fingerprint)
    let acroform = catalog
        .acroform_ref
        .and_then(|r| resolver.resolve(r).ok())
        .and_then(|o| o.as_dict())
        .cloned();

    // Build fingerprint input (without full page tree for lazy extraction)
    let fingerprint = compute_fingerprint_lazy(&catalog, &resolver, &acroform);

    Ok((catalog, resolver, source, fingerprint))
}

/// Extract pages from a remote PDF using the extraction options.
///
/// This is a convenience function that combines `open_remote` with extraction.
/// It performs the HTTP fetch sequence and then extracts the specified pages.
///
/// # Arguments
///
/// * `url` - HTTP/HTTPS URL to the PDF file
/// * `opts` - Remote options (headers, credentials, etc.)
/// * `extraction_opts` - Extraction options (page range, receipts, etc.)
///
/// # Returns
///
/// An `ExtractionResult` containing the extracted pages and metadata.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::remote::{extract_remote, RemoteOpts};
/// use pdftract_core::options::ExtractionOptions;
///
/// let remote_opts = RemoteOpts::new()
///     .with_header("Authorization", "Bearer token");
///
/// let extraction_opts = ExtractionOptions::default();
///
/// let result = extract_remote("https://example.com/doc.pdf", &remote_opts, &extraction_opts)?;
/// ```
pub fn extract_remote(
    url: &str,
    opts: &RemoteOpts,
    extraction_opts: &ExtractionOptions,
) -> Result<crate::extract::ExtractionResult> {
    // Open the remote PDF source
    let source = open_remote_source(url, opts).context("Failed to open remote PDF source")?;

    // Prefetch pages using hint stream if available (optimization for linearized PDFs)
    prefetch_hint_stream(&*source, extraction_opts);

    // Use the extraction pipeline with the remote source
    let extraction_source = ExtractionSource::Remote(source);

    extract_pdf_from_source(extraction_source, extraction_opts)
}

/// Prefetch pages using the hint stream from a linearized PDF.
///
/// This function:
/// 1. Detects if the PDF is linearized
/// 2. Parses the hint stream if present
/// 3. Prefetches the requested page ranges using the hint table predictions
///
/// # Parameters
/// - `source`: The PDF source to read from
/// - `extraction_opts`: Extraction options containing page ranges
///
/// # Returns
/// Nothing; prefetch is a performance optimization that doesn't affect correctness.
pub fn prefetch_hint_stream(
    source: &dyn crate::parser::stream::PdfSource,
    extraction_opts: &ExtractionOptions,
) {
    // Detect linearization
    let lin_info = match detect_linearization(source) {
        Some(info) => info,
        None => return, // Not linearized, no hint stream
    };

    // Check if hint stream info is available
    let (hint_offset, hint_length) = match (lin_info.hint_stream_offset, lin_info.hint_stream_length) {
        (Some(offset), Some(length)) => (offset, length),
        _ => return, // No hint stream, nothing to prefetch
    };

    // Parse the hint stream
    let mut diagnostics = Vec::new();
    let hint_table = match hint_stream::parse_hint_stream_from_linearized(
        source,
        hint_offset,
        hint_length,
        &mut diagnostics,
    ) {
        Some(table) => table,
        None => return, // Failed to parse hint stream, continue without prefetch
    };

    // Get the requested page range (if any)
    let page_ranges = extraction_opts.pages.as_ref();
    let page_indices: Vec<u32> = match page_ranges {
        Some(ranges) => {
            // Convert page ranges to 0-based indices
            ranges
                .iter()
                .flat_map(|r| {
                    let start = r.start.saturating_sub(1) as u32; // Convert to 0-based
                    let end = r.end.saturating_sub(1) as u32;
                    start..=end
                })
                .collect()
        }
        None => {
            // No page range specified, prefetch all pages (up to a limit)
            (0..hint_table.page_count().min(100)).collect()
        }
    };

    // Prefetch each requested page
    for page_idx in page_indices {
        if let Some(range) = hint_table.predict_page_range(page_idx) {
            let length = range.end.saturating_sub(range.start) as usize;
            source.prefetch(range.start, length);
        }
    }

    // Note: Shared object hints are not yet implemented (Phase 2)
    let _shared_ranges = hint_table.predict_shared_objects();
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
