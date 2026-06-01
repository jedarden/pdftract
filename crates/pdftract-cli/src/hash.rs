//! PDF structural fingerprint (hash) subcommand.
//!
//! Implements the `pdftract hash` command that computes the PDF fingerprint
//! and outputs it to stdout with appropriate exit codes.

use anyhow::{anyhow, Context, Result};
use pdftract_core::fingerprint::{compute_fingerprint, CatalogFlags, ContentStreamData, FingerprintInput, PageFingerprintData};
use pdftract_core::parser::catalog::parse_catalog;
use pdftract_core::parser::pages::{flatten_page_tree, PageDict};
use pdftract_core::parser::stream::{FileSource, PdfSource};
use pdftract_core::parser::xref::{load_xref_with_prev_chain, XrefResolver};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Exit codes for the hash subcommand.
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_CORRUPT: i32 = 2;
pub const EXIT_ENCRYPTED: i32 = 3;
pub const EXIT_NOT_FOUND: i32 = 4;
pub const EXIT_NETWORK_FAILURE: i32 = 5;
pub const EXIT_TLS_FAILURE: i32 = 6;

/// Arguments for the hash subcommand.
pub struct HashArgs {
    /// Input path or URL
    pub input: String,
    /// Optional password
    pub password: Option<String>,
    /// Custom HTTP headers (for remote sources)
    pub headers: Vec<(String, String)>,
}

/// Map an error to the appropriate exit code.
pub fn map_error_to_exit_code(err: &anyhow::Error) -> i32 {
    let err_msg = err.to_string().to_lowercase();

    // Check for encryption-related errors
    if err_msg.contains("encryption") || err_msg.contains("password") || err_msg.contains("decrypt") {
        return EXIT_ENCRYPTED;
    }

    // Check for network-related errors (remote sources only)
    if err_msg.contains("tls") || err_msg.contains("certificate") || err_msg.contains("handshake") {
        return EXIT_TLS_FAILURE;
    }

    if err_msg.contains("network") || err_msg.contains("timeout") || err_msg.contains("connection") {
        return EXIT_NETWORK_FAILURE;
    }

    if err_msg.contains("dns") || err_msg.contains("hostname") || err_msg.contains("resolution") {
        return EXIT_NOT_FOUND;
    }

    // Check for file not found / permission errors
    if err_msg.contains("not found") || err_msg.contains("no such file") {
        return EXIT_NOT_FOUND;
    }

    // Check for io::ErrorKind::PermissionDenied for file permission errors (NOT TLS)
    if err_msg.contains("permission denied") && !err_msg.contains("tls") {
        return EXIT_NOT_FOUND;
    }

    // Default to corrupt for unrecognised errors
    EXIT_CORRUPT
}

/// Check if a string is a URL (http:// or https://).
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Compute the fingerprint for a PDF from a local file.
fn compute_fingerprint_from_file(
    path: &Path,
    _password: Option<&str>,
) -> Result<String> {
    // Open the PDF file
    let source = FileSource::open(path).context("Failed to open PDF file")?;

    // Find the startxref offset
    let startxref_offset = find_startxref(&source).context("Failed to find startxref offset")?;

    // Load the xref table
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);

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
    let catalog = parse_catalog(&resolver, root_ref, Some(&source as &dyn PdfSource))
        .map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow::anyhow!("Failed to parse catalog: {}", msg)
        })?;

    // Flatten the page tree
    let pages = flatten_page_tree(&resolver, catalog.pages_ref).map_err(|diagnostics| {
        let msg = diagnostics
            .first()
            .map(|d| d.message.as_ref())
            .unwrap_or("unknown error");
        anyhow::anyhow!("Failed to flatten page tree: {}", msg)
    })?;

    // Build fingerprint input
    let fingerprint_input = build_fingerprint_input(&catalog, &pages, &xref_section);

    // Compute fingerprint
    let fingerprint = compute_fingerprint(&fingerprint_input, &resolver);

    Ok(fingerprint)
}

/// Compute the fingerprint for a PDF from a remote URL.
#[cfg(feature = "remote")]
fn compute_fingerprint_from_url(
    url: &str,
    headers: &[(String, String)],
) -> Result<String> {
    use pdftract_core::source::HttpRangeSource;

    // Open the remote PDF
    let source = HttpRangeSource::with_headers(url, headers.to_vec())
        .context("Failed to open remote PDF")?;

    // Find the startxref offset
    let startxref_offset = find_startxref(&source).context("Failed to find startxref offset")?;

    // Load the xref table
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);

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
    let catalog = parse_catalog(&resolver, root_ref, Some(&source as &dyn PdfSource))
        .map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow::anyhow!("Failed to parse catalog: {}", msg)
        })?;

    // Flatten the page tree
    let pages = flatten_page_tree(&resolver, catalog.pages_ref).map_err(|diagnostics| {
        let msg = diagnostics
            .first()
            .map(|d| d.message.as_ref())
            .unwrap_or("unknown error");
        anyhow::anyhow!("Failed to flatten page tree: {}", msg)
    })?;

    // Build fingerprint input
    let fingerprint_input = build_fingerprint_input(&catalog, &pages, &xref_section);

    // Compute fingerprint
    let fingerprint = compute_fingerprint(&fingerprint_input, &resolver);

    Ok(fingerprint)
}

/// Find the startxref offset in a PDF source.
fn find_startxref(source: &dyn PdfSource) -> Result<u64> {
    let len = source.len();
    let scan_size = 1024.min(len) as usize;
    let scan_start = (len - scan_size as u64) as u64;

    let tail_data = source
        .read_range(scan_start, scan_size)
        .context("Failed to read PDF tail")?;

    // Find "startxref" in the tail data
    let startxref_pos = tail_data
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| anyhow!("startxref not found in PDF"))?;

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

    Ok(offset)
}

/// Build FingerprintInput from catalog and pages.
fn build_fingerprint_input(
    catalog: &pdftract_core::parser::catalog::Catalog,
    pages: &[PageDict],
    _xref_section: &pdftract_core::parser::xref::XrefSection,
) -> FingerprintInput {
    let page_count = pages.len() as u32;

    let fingerprint_pages = pages
        .iter()
        .map(|page| PageFingerprintData {
            content_streams: page
                .contents
                .iter()
                .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
                .collect(),
            resources: None,
            media_box: page.media_box,
            crop_box: page.crop_box,
            rotate: page.rotate,
        })
        .collect();

    // Build catalog flags
    let catalog_flags = CatalogFlags {
        is_encrypted: catalog.is_encrypted,
        contains_javascript: catalog.open_action.is_some() || catalog.aa.is_some(),
        contains_xfa: catalog.xfa.is_some(),
        ocg_present: catalog
            .oc_properties
            .as_ref()
            .map(|props| props.present)
            .unwrap_or(false),
    };

    FingerprintInput {
        page_count,
        pages: fingerprint_pages,
        struct_tree_root_ref: catalog.struct_tree_root_ref,
        is_tagged: catalog.mark_info.is_tagged,
        catalog_flags,
    }
}

/// Run the hash subcommand.
pub fn run_hash(args: HashArgs) -> Result<()> {
    let input = &args.input;

    if is_url(input) {
        #[cfg(feature = "remote")]
        {
            let fingerprint = compute_fingerprint_from_url(input, &args.headers)
                .context("Failed to compute fingerprint from URL")?;
            println!("{}", fingerprint);
            return Ok(());
        }

        #[cfg(not(feature = "remote"))]
        {
            return Err(anyhow::anyhow!(
                "Remote sources are not supported; rebuild with --features remote"
            ));
        }
    } else {
        // Local file
        let path = Path::new(input);
        let fingerprint = compute_fingerprint_from_file(path, args.password.as_deref())
            .context("Failed to compute fingerprint from file")?;
        println!("{}", fingerprint);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url() {
        assert!(is_url("http://example.com/file.pdf"));
        assert!(is_url("https://example.com/file.pdf"));
        assert!(!is_url("file.pdf"));
        assert!(!is_url("/path/to/file.pdf"));
        assert!(!is_url("ftp://example.com/file.pdf"));
    }

    #[test]
    fn test_exit_code_constants() {
        assert_eq!(EXIT_SUCCESS, 0);
        assert_eq!(EXIT_CORRUPT, 2);
        assert_eq!(EXIT_ENCRYPTED, 3);
        assert_eq!(EXIT_NOT_FOUND, 4);
        assert_eq!(EXIT_NETWORK_FAILURE, 5);
        assert_eq!(EXIT_TLS_FAILURE, 6);
    }
}
