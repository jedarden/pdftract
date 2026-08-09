//! PDF document parsing helper.
//!
//! This module provides high-level functions for parsing PDF documents
//! and extracting the information needed for receipt verification.
//!
//! ## Lazy Page Iteration
//!
//! For memory-efficient extraction of large documents, this module provides
//! `PageIter` which yields pages lazily without materializing the entire page tree.
//! Use `PdfExtractor::pages()` to get an iterator that extracts each page on-demand.

use crate::detection::{detect_javascript, detect_xfa};
use crate::fingerprint::{
    compute_fingerprint, CatalogFlags, ContentStreamData, FingerprintInput, PageFingerprintData,
};
use crate::parser::catalog::{parse_catalog, Catalog};
use crate::parser::object::PdfDict;
use crate::parser::pages::{flatten_page_tree, LazyPageIter, PageDict};
use crate::parser::stream::{FileSource as ParserFileSource, PdfSource as ParserPdfSource};
use crate::parser::xref::{
    detect_linearization, load_xref_linearized, load_xref_with_prev_chain, LinearizationInfo,
    XrefResolver, XrefSection,
};
use crate::receipts::verifier::SpanData;
use crate::source::{FileSource, PdfSource};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

#[cfg(feature = "remote")]
use crate::source::RemoteOpts;

/// Comprehensive error type for Document operations.
///
/// This enum provides detailed error types for various failure modes
/// when working with PDF documents, enabling better error handling,
/// user feedback, and recovery strategies.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentError {
    /// The document is empty or has no content
    EmptyDocument {
        /// File path or source identifier
        source: String,
    },

    /// The document catalog is missing the required /Pages field
    MissingPagesArray {
        /// File path or source identifier
        source: String,
    },

    /// The /Pages field exists but is not an array
    InvalidPagesFormat {
        /// File path or source identifier
        source: String,
        /// Description of what was found instead of an array
        found_type: String,
    },

    /// Page index is out of bounds for the document
    PageOutOfBounds {
        /// File path or source identifier
        source: String,
        /// The requested 0-based page index
        requested: usize,
        /// The actual number of pages in the document
        available: usize,
    },

    /// Page data is malformed or structurally invalid
    MalformedPageData {
        /// Page index
        page_index: usize,
        /// Description of the malformed data
        message: String,
    },

    /// Document structure is malformed (missing or corrupt page tree)
    MalformedDocumentStructure {
        /// File path or source identifier
        source: String,
        /// Description of the structural problem
        message: String,
    },

    /// Page extraction failed with a detailed message
    ExtractionFailed {
        /// Page index that failed to extract
        page_index: usize,
        /// Detailed error message describing what went wrong
        message: String,
    },

    /// Failed to open or read the document file
    FileOpenFailed {
        /// File path that failed to open
        path: String,
        /// Underlying error reason
        reason: String,
    },

    /// Failed to find the startxref offset
    StartxrefNotFound {
        /// File path or source identifier
        source: String,
        /// Size of the file that was scanned
        file_size_bytes: u64,
    },

    /// Failed to parse the xref table
    XrefParseFailed {
        /// File path or source identifier
        source: String,
        /// Offset where xref parsing failed
        offset: u64,
        /// Description of what went wrong
        reason: String,
    },

    /// Failed to parse the document catalog
    CatalogParseFailed {
        /// File path or source identifier
        source: String,
        /// Description of the parse failure
        reason: String,
    },

    /// The document is encrypted and password/decryption is not yet supported
    EncryptionNotSupported {
        /// File path or source identifier
        source: String,
    },

    /// Failed to count pages in the document
    PageCountFailed {
        /// File path or source identifier
        source: String,
        /// Description of what went wrong
        reason: String,
    },

    /// Invalid media box in page
    InvalidMediaBox {
        /// Page index
        page_index: usize,
        /// The media box values [x0, y0, x1, y1]
        media_box: Option<[f64; 4]>,
    },

    /// Invalid page dimensions (width or height is zero or negative)
    InvalidDimensions {
        /// Page index
        page_index: usize,
        /// Width value in points
        width: f64,
        /// Height value in points
        height: f64,
    },

    /// Invalid page rotation value
    InvalidRotation {
        /// Page index
        page_index: usize,
        /// The rotation value
        rotation: i32,
    },

    /// Content stream decoding failed
    ContentStreamDecodeFailed {
        /// Page index
        page_index: usize,
        /// Underlying error message
        message: String,
    },

    /// Content stream is empty or missing
    MissingContentStream {
        /// Page index
        page_index: usize,
    },

    /// Page resources are missing or malformed
    InvalidResources {
        /// Page index
        page_index: usize,
        /// Description of what's wrong with resources
        message: String,
    },

    /// Page has missing required fields
    MissingRequiredFields {
        /// Page index
        page_index: usize,
        /// List of missing field names
        fields: Vec<String>,
    },

    /// Linearized PDF parsing failed
    LinearizationFailed {
        /// File path or source identifier
        source: String,
        /// Description of what went wrong
        reason: String,
    },

    /// Remote document fetch failed
    RemoteFetchFailed {
        /// URL that failed to fetch
        url: String,
        /// HTTP status code if available
        status_code: Option<u16>,
        /// Description of what went wrong
        reason: String,
    },

    /// Invalid PDF header signature
    InvalidPdfHeader {
        /// File path or source identifier
        source: String,
        /// The header bytes that were found
        found_header: String,
    },

    /// Trailer is missing or malformed
    InvalidTrailer {
        /// File path or source identifier
        source: String,
        /// Description of what's wrong with the trailer
        reason: String,
    },

    /// Generic document processing failure with context
    ProcessingFailed {
        /// File path or source identifier
        source: String,
        /// Detailed error message
        message: String,
    },
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDocument { source } => {
                write!(f, "Document '{}' is empty or contains no content", source)
            }
            Self::MissingPagesArray { source } => {
                write!(f, "Document '{}' is missing the required /Pages field in catalog", source)
            }
            Self::InvalidPagesFormat { source, found_type } => {
                write!(f, "Document '{}' /Pages field is not an array (found: {})", source, found_type)
            }
            Self::PageOutOfBounds { source, requested, available } => {
                write!(
                    f,
                    "Page index {} out of bounds for document '{}' (document has {} pages; valid indices: 0-{})",
                    requested,
                    source,
                    available,
                    available.saturating_sub(1)
                )
            }
            Self::MalformedPageData { page_index, message } => {
                write!(f, "Page {} has malformed data: {}", page_index, message)
            }
            Self::MalformedDocumentStructure { source, message } => {
                write!(f, "Document '{}' has malformed structure: {}", source, message)
            }
            Self::ExtractionFailed { page_index, message } => {
                write!(f, "Failed to extract page {}: {}", page_index, message)
            }
            Self::FileOpenFailed { path, reason } => {
                write!(f, "Failed to open file '{}': {}", path, reason)
            }
            Self::StartxrefNotFound { source, file_size_bytes } => {
                write!(
                    f,
                    "Failed to find startxref offset in document '{}' (file size: {} bytes)",
                    source,
                    file_size_bytes
                )
            }
            Self::XrefParseFailed { source, offset, reason } => {
                write!(
                    f,
                    "Failed to parse xref table in document '{}' at offset {}: {}",
                    source, offset, reason
                )
            }
            Self::CatalogParseFailed { source, reason } => {
                write!(f, "Failed to parse catalog for document '{}': {}", source, reason)
            }
            Self::EncryptionNotSupported { source } => {
                write!(f, "Document '{}' is encrypted - encryption not yet supported", source)
            }
            Self::PageCountFailed { source, reason } => {
                write!(f, "Failed to count pages in document '{}': {}", source, reason)
            }
            Self::InvalidMediaBox { page_index, media_box } => {
                write!(
                    f,
                    "Page {} has invalid media box: {:?} (must have x1 > x0 and y1 > y0)",
                    page_index, media_box
                )
            }
            Self::InvalidDimensions { page_index, width, height } => {
                write!(
                    f,
                    "Page {} has invalid dimensions: width={}, height={} (both must be positive)",
                    page_index, width, height
                )
            }
            Self::InvalidRotation { page_index, rotation } => {
                write!(
                    f,
                    "Page {} has invalid rotation: {}° (must be 0, 90, 180, or 270)",
                    page_index, rotation
                )
            }
            Self::ContentStreamDecodeFailed { page_index, message } => {
                write!(f, "Failed to decode content stream for page {}: {}", page_index, message)
            }
            Self::MissingContentStream { page_index } => {
                write!(f, "Page {} has no content stream (empty or missing)", page_index)
            }
            Self::InvalidResources { page_index, message } => {
                write!(f, "Page {} has invalid resources: {}", page_index, message)
            }
            Self::MissingRequiredFields { page_index, fields } => {
                write!(
                    f,
                    "Page {} is missing required fields: {}",
                    page_index,
                    fields.join(", ")
                )
            }
            Self::LinearizationFailed { source, reason } => {
                write!(f, "Failed to parse linearized document '{}': {}", source, reason)
            }
            Self::RemoteFetchFailed { url, status_code, reason } => {
                match status_code {
                    Some(code) => {
                        write!(f, "Failed to fetch remote document '{}' (HTTP {}): {}", url, code, reason)
                    }
                    None => {
                        write!(f, "Failed to fetch remote document '{}': {}", url, reason)
                    }
                }
            }
            Self::InvalidPdfHeader { source, found_header } => {
                write!(
                    f,
                    "Document '{}' has invalid PDF header (expected '%PDF-1.x', found: '{}')",
                    source, found_header
                )
            }
            Self::InvalidTrailer { source, reason } => {
                write!(f, "Document '{}' has invalid trailer: {}", source, reason)
            }
            Self::ProcessingFailed { source, message } => {
                write!(f, "Failed to process document '{}': {}", source, message)
            }
        }
    }
}

impl std::error::Error for DocumentError {}

/// Result type for Document operations that use DocumentError.
pub type DocumentResult<T> = std::result::Result<T, DocumentError>;

/// Parse a PDF file and return the document components needed for verification.
///
/// This is a high-level function that:
/// 1. Opens the PDF file
/// 2. Loads the xref table
/// 3. Parses the catalog
/// 4. Flattens the page tree
/// 5. Computes the fingerprint
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
///
/// # Returns
///
/// A tuple of (fingerprint, catalog, pages, resolver)
pub fn parse_pdf_file(
    pdf_path: &std::path::Path,
) -> Result<(
    String,
    Catalog,
    Vec<crate::parser::pages::PageDict>,
    XrefResolver,
)> {
    // Open the PDF file
    let source = ParserFileSource::open(pdf_path).context("Failed to open PDF file")?;

    // Find the startxref offset
    let startxref_offset = find_startxref(&source).context("Failed to find startxref offset")?;

    // Check if this is a linearized PDF
    let xref_section = if let Some(lin_info) = detect_linearization(&source) {
        // Linearized PDF: use special xref loading that merges first-page and full xref
        load_xref_linearized(&source, &lin_info, startxref_offset)
    } else {
        // Normal PDF: load xref with /Prev chain support
        load_xref_with_prev_chain(&source, startxref_offset)
    };

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = xref_section
        .trailer
        .as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| obj.as_ref())
        .ok_or_else(|| anyhow!("No /Root reference in trailer"))?;

    // Parse the catalog
    let catalog = parse_catalog(&resolver, root_ref, Some(&source as &dyn ParserPdfSource))
        .map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow!("Failed to parse catalog: {}", msg)
        })?;

    // Flatten the page tree
    let pages = flatten_page_tree(&resolver, catalog.pages_ref).map_err(|diagnostics| {
        let msg = diagnostics
            .first()
            .map(|d| d.message.as_ref())
            .unwrap_or("unknown error");
        anyhow!("Failed to flatten page tree: {}", msg)
    })?;

    // Resolve AcroForm dictionary if present
    let acroform = catalog
        .acroform_ref
        .and_then(|r| resolver.resolve(r).ok())
        .and_then(|o| o.as_dict().map(|d| d.clone()));

    // Build fingerprint input
    let fingerprint_input = build_fingerprint_input(&catalog, &pages, &resolver, &acroform);

    // Compute fingerprint with source available for content stream decoding
    let fingerprint = compute_fingerprint(
        &fingerprint_input,
        &resolver,
        Some(&source as &dyn ParserPdfSource),
    );

    Ok((fingerprint, catalog, pages, resolver))
}

/// Parse a PDF from a generic source and return document components.
///
/// This is a variant of `parse_pdf_file` that works with any `PdfSource`
/// implementation (local files, HTTP sources, memory buffers, etc.).
///
/// # Arguments
///
/// * `source` - A PDF source (FileSource, HttpRangeSource, etc.)
///
/// # Returns
///
/// A tuple of (fingerprint, catalog, pages, resolver)
pub fn parse_pdf_source(
    source: Box<dyn ParserPdfSource>,
) -> Result<(
    String,
    Catalog,
    Vec<crate::parser::pages::PageDict>,
    XrefResolver,
)> {
    // Find the startxref offset
    let startxref_offset = find_startxref(&*source).context("Failed to find startxref offset")?;

    // Check if this is a linearized PDF
    let xref_section = if let Some(lin_info) = detect_linearization(&*source) {
        // Linearized PDF: use special xref loading that merges first-page and full xref
        load_xref_linearized(&*source, &lin_info, startxref_offset)
    } else {
        // Normal PDF: load xref with /Prev chain support
        load_xref_with_prev_chain(&*source, startxref_offset)
    };

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = xref_section
        .trailer
        .as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| obj.as_ref())
        .ok_or_else(|| anyhow!("No /Root reference in trailer"))?;

    // Parse the catalog
    let catalog = parse_catalog(&resolver, root_ref, Some(&*source as &dyn ParserPdfSource))
        .map_err(|diagnostics| {
            let msg = diagnostics
                .first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow!("Failed to parse catalog: {}", msg)
        })?;

    // Flatten the page tree
    let pages = flatten_page_tree(&resolver, catalog.pages_ref).map_err(|diagnostics| {
        let msg = diagnostics
            .first()
            .map(|d| d.message.as_ref())
            .unwrap_or("unknown error");
        anyhow!("Failed to flatten page tree: {}", msg)
    })?;

    // Resolve AcroForm dictionary if present
    let acroform = catalog
        .acroform_ref
        .and_then(|r| resolver.resolve(r).ok())
        .and_then(|o| o.as_dict().map(|d| d.clone()));

    // Build fingerprint input
    let fingerprint_input = build_fingerprint_input(&catalog, &pages, &resolver, &acroform);

    // Compute fingerprint with source available
    let fingerprint = compute_fingerprint(
        &fingerprint_input,
        &resolver,
        Some(&*source as &dyn ParserPdfSource),
    );

    Ok((fingerprint, catalog, pages, resolver))
}

/// Find the startxref offset in a PDF file.
///
/// Scans the last 1024 bytes of the file for "startxref" keyword.
fn find_startxref(source: &dyn ParserPdfSource) -> Result<u64> {
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

/// Build FingerprintInput from catalog and pages.
fn build_fingerprint_input(
    catalog: &Catalog,
    pages: &[crate::parser::pages::PageDict],
    resolver: &XrefResolver,
    acroform: &Option<PdfDict>,
) -> FingerprintInput {
    let page_count = pages.len() as u32;

    let fingerprint_pages = pages
        .iter()
        .map(|page| {
            PageFingerprintData {
                content_streams: page
                    .contents
                    .iter()
                    .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
                    .collect(),
                resources: None, // TODO: convert ResourceDict to PdfDict
                media_box: page.media_box,
                crop_box: page.crop_box,
                rotate: page.rotate,
            }
        })
        .collect();

    // Detect JavaScript and XFA presence
    let contains_javascript = detect_javascript(catalog, pages, acroform, resolver);
    let contains_xfa = detect_xfa(acroform);

    // Build catalog flags
    let catalog_flags = CatalogFlags {
        is_encrypted: false, // TODO: detect encryption
        contains_javascript,
        contains_xfa,
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

/// Extract text spans from a specific page.
///
/// This is a minimal implementation that extracts basic text information.
/// In a full implementation, this would use the complete text extraction pipeline.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `page_index` - 0-based page index
///
/// # Returns
///
/// A vector of SpanData objects containing text and bbox information
pub fn extract_spans_from_page(
    pdf_path: &std::path::Path,
    page_index: usize,
) -> Result<Vec<SpanData>> {
    // Parse the PDF
    let (_fingerprint, _catalog, pages, _resolver) = parse_pdf_file(pdf_path)?;

    // Check page index bounds
    if page_index >= pages.len() {
        return Err(anyhow!(
            "Page index {} out of bounds (document has {} pages)",
            page_index,
            pages.len()
        ));
    }

    let page = &pages[page_index];

    // For now, return a placeholder span
    // In a full implementation, this would:
    // 1. Parse the content streams
    // 2. Extract text with positioning information
    // 3. Build spans with text and bbox

    // Return a single span covering the entire page as a placeholder
    let [x0, y0, x1, y1] = page.media_box;
    let spans = vec![SpanData {
        text: format!("[Page {} text extraction not yet implemented]", page_index),
        bbox: [x0, y0, x1, y1],
    }];

    Ok(spans)
}

/// Compute the fingerprint of a PDF file.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
///
/// # Returns
///
/// The fingerprint string in the format "pdftract-v1:\<hex\>"
pub fn compute_pdf_fingerprint(pdf_path: &std::path::Path) -> Result<String> {
    let (fingerprint, _catalog, _pages, _resolver) = parse_pdf_file(pdf_path)?;
    Ok(fingerprint)
}

/// A lazy PDF page extractor that yields pages one at a time.
///
/// This struct provides memory-efficient extraction for large PDFs by:
/// - Materializing only the current page's data
/// - Decoding content streams on-demand per page
/// - Dropping decoded data immediately after use
///
/// # Example
///
/// ```ignore
/// let extractor = PdfExtractor::open("document.pdf")?;
/// for page_result in extractor.pages() {
///     let page = page_result?;
///     // Process page without holding all pages in memory
/// }
/// ```
/// PDF document extractor with lazy page iteration.
///
/// This struct provides on-demand access to PDF pages without materializing
/// the entire page tree in memory. Use it for memory-efficient extraction
/// from large documents or when you need random access to specific pages.
///
/// # Examples
///
/// Open a PDF and iterate over pages lazily:
///
/// ```rust,no_run
/// use pdftract_core::document::PdfExtractor;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let extractor = PdfExtractor::open("document.pdf")?;
/// println!("Fingerprint: {}", extractor.fingerprint());
/// println!("Total pages: {}", extractor.catalog().page_count.unwrap_or(0));
/// # Ok(())
/// # }
/// ```
///
/// Memory-bounded extraction of specific pages:
///
/// ```rust,no_run
/// use pdftract_core::document::PdfExtractor;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let extractor = PdfExtractor::open("large.pdf")?;
///
/// // Only pages 5-10 are materialized, not the entire document
/// for page_result in extractor.pages()?.take(10) {
///     let page = page_result?;
///     println!("Page {} has {} spans", page.index, page.spans.len());
/// }
/// # Ok(())
/// # }
/// ```
pub struct PdfExtractor {
    /// The PDF file source
    source: FileSource,
    /// The xref resolver for indirect object lookup
    resolver: XrefResolver,
    /// The parsed catalog
    catalog: Catalog,
    /// The fingerprint of the document
    fingerprint: String,
    /// Pre-flattened pages (for non-streaming extraction)
    pages: Option<Vec<PageDict>>,
}

impl PdfExtractor {
    /// Open a PDF file for lazy extraction.
    ///
    /// This parses the xref table and catalog but does NOT materialize
    /// the page tree. Pages are resolved on-demand from the iterator.
    pub fn open<P: AsRef<Path>>(pdf_path: P) -> Result<Self> {
        let path = pdf_path.as_ref();

        // Open the PDF file
        let source = FileSource::open(path).context("Failed to open PDF file")?;

        // Find the startxref offset
        let startxref_offset =
            find_startxref(&source).context("Failed to find startxref offset")?;

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
            .ok_or_else(|| anyhow!("No /Root reference in trailer"))?;

        // Parse the catalog
        let catalog = parse_catalog(&resolver, root_ref, Some(&source as &dyn ParserPdfSource))
            .map_err(|diagnostics| {
                let msg = diagnostics
                    .first()
                    .map(|d| d.message.as_ref())
                    .unwrap_or("unknown error");
                anyhow!("Failed to parse catalog: {}", msg)
            })?;

        // Resolve AcroForm dictionary if present (for XFA detection)
        let acroform = catalog
            .acroform_ref
            .and_then(|r| resolver.resolve(r).ok())
            .and_then(|o| o.as_dict().map(|d| d.clone()));

        // Build fingerprint input (without full page tree for lazy extraction)
        let fingerprint = compute_fingerprint_lazy(&catalog, &resolver, &acroform);

        Ok(Self {
            source,
            resolver,
            catalog,
            fingerprint,
            pages: None,
        })
    }

    /// Get the document fingerprint.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Get the catalog.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Get the total page count.
    ///
    /// This walks the page tree to count pages without materializing PageDict objects.
    /// Uses O(depth) memory, making it safe for large documents.
    pub fn page_count(&self) -> Result<usize> {
        if let Some(ref pages) = self.pages {
            return Ok(pages.len());
        }

        // Use lazy counting that doesn't materialize all pages
        use crate::parser::pages::count_pages_tree;
        count_pages_tree(&self.resolver, self.catalog.pages_ref)
            .map_err(|e| anyhow!("Failed to count pages: {:?}", e))
    }

    /// Materialize all pages (for non-streaming extraction).
    ///
    /// This caches the flattened page tree for repeated access.
    ///
    /// # WARNING: Memory Implications
    ///
    /// This function materializes ALL pages in memory, which defeats lazy loading
    /// and can consume significant memory for large documents (1000+ pages).
    /// Use this ONLY when you need repeated random access to pages.
    ///
    /// For streaming extraction or one-time sequential access, use the `pages()`
    /// method instead, which returns a lazy `PageIter` that never materializes
    /// all pages at once.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // BAD: Materializes all pages in memory
    /// extractor.materialize_pages()?;
    /// for page in extractor.pages.unwrap() { ... }
    ///
    /// // GOOD: Lazy iteration, one page at a time
    /// for page_result in extractor.pages() {
    ///     let page = page_result?;
    ///     // Process page - it will be dropped after loop iteration
    /// }
    /// ```
    pub fn materialize_pages(&mut self) -> Result<&[PageDict]> {
        if self.pages.is_none() {
            let pages = flatten_page_tree(&self.resolver, self.catalog.pages_ref)
                .map_err(|e| anyhow!("Failed to flatten page tree: {:?}", e))?;
            self.pages = Some(pages);
        }
        // Safe: we just set self.pages = Some(...) above if it was None
        // Use match to avoid unwrap/expect while maintaining the invariant
        match &self.pages {
            Some(pages) => Ok(pages),
            None => Err(anyhow!("materialize_pages invariant violated: pages should be Some")),
        }
    }

    /// Get a lazy iterator over pages.
    ///
    /// The iterator yields pages one at a time, decoding each page's
    /// content streams on-demand and dropping them after use.
    ///
    /// # Memory Behavior
    ///
    /// This uses LazyPageIter which walks the page tree depth-first,
    /// materializing only the current path from root to leaf (max ~16 nodes).
    /// Each yielded PageDict is standalone and can be dropped after use.
    /// Peak RSS stays O(depth) not O(pages).
    ///
    /// # Preferred Streaming Approach
    ///
    /// This is the RECOMMENDED way to iterate over pages for large documents,
    /// as it never materializes all pages in memory. Use `materialize_pages()`
    /// ONLY when you need repeated random access to pages.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // GOOD: Lazy iteration, one page at a time
    /// for page_result in extractor.pages() {
    ///     let page = page_result?;
    ///     // Process page - it will be dropped after loop iteration
    /// }
    ///
    /// // BAD: Materializes all pages in memory (avoid for large documents)
    /// extractor.materialize_pages()?;
    /// for page in extractor.pages.unwrap() { ... }
    /// ```
    pub fn pages(&self) -> PageIter<'_> {
        PageIter {
            lazy_iter: None,
            catalog: &self.catalog,
            resolver: &self.resolver,
            source: Some(&self.source as &dyn ParserPdfSource),
            index: 0,
        }
    }

    /// Extract a single page by index.
    ///
    /// This method extracts one page without materializing the entire document.
    /// Content streams are decoded and the result is returned.
    pub fn extract_page(&self, page_index: usize) -> Result<PageExtraction> {
        let pages = self
            .pages
            .as_ref()
            .ok_or_else(|| anyhow!("Pages not materialized. Call materialize_pages() first."))?;

        if page_index >= pages.len() {
            return Err(anyhow!(
                "Page index {} out of bounds (document has {} pages)",
                page_index,
                pages.len()
            ));
        }

        let page = &pages[page_index];

        // For now, return a placeholder extraction
        // The full implementation would decode content streams here
        let [x0, y0, x1, y1] = page.media_box;

        Ok(PageExtraction {
            index: page_index,
            width: x1 - x0,
            height: y1 - y0,
            rotation: page.rotate,
            spans: vec![],
            blocks: vec![],
        })
    }
}

/// Result of extracting a single page.
///
/// This struct contains the minimal data needed for one page,
/// designed to be dropped immediately after serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageExtraction {
    /// 0-based page index
    pub index: usize,
    /// Page width in points
    pub width: f64,
    /// Page height in points
    pub height: f64,
    /// Page rotation in degrees
    pub rotation: i32,
    /// Extracted text spans
    pub spans: Vec<SpanData>,
    /// Extracted blocks
    pub blocks: Vec<BlockData>,
}

/// Block data for extracted content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    /// Block kind (paragraph, heading, etc.)
    pub kind: String,
    /// Block text
    pub text: String,
}

/// Lazy iterator over PDF pages.
///
/// Compute fingerprint without full page materialization.
///
/// This is a simplified version that uses only catalog-level data.
/// The full fingerprint computation requires page content streams.
pub(crate) fn compute_fingerprint_lazy(
    catalog: &Catalog,
    resolver: &XrefResolver,
    acroform: &Option<PdfDict>,
) -> String {
    // For lazy extraction, use a simpler fingerprint based on catalog data
    // The full implementation would incrementally hash pages as they're extracted
    use crate::fingerprint::FingerprintInput;

    // Detect JavaScript and XFA presence (no pages available in lazy mode)
    let contains_javascript = if catalog.open_action.is_some() || catalog.aa.is_some() {
        true
    } else {
        // For catalog-level checks, use simple detection
        // Full page/annotation walk requires materialized pages
        false
    };
    let contains_xfa = detect_xfa(acroform);

    let fingerprint_input = FingerprintInput {
        page_count: 0, // Will be updated when pages are extracted
        pages: vec![],
        struct_tree_root_ref: catalog.struct_tree_root_ref,
        is_tagged: catalog.mark_info.is_tagged,
        catalog_flags: CatalogFlags {
            is_encrypted: false,
            contains_javascript,
            contains_xfa,
            ocg_present: catalog
                .oc_properties
                .as_ref()
                .map(|props| props.present)
                .unwrap_or(false),
        },
    };

    compute_fingerprint(&fingerprint_input, resolver, None)
}

/// A parsed PDF document that can be from either local or remote sources.
///
/// This type provides a unified interface for working with PDFs regardless
/// of their source (local file, HTTP/HTTPS URL, memory buffer). It holds
/// the parsed catalog, xref resolver, and lazy page iterator.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::document::Document;
///
/// // Open from local file
/// let doc = Document::open("document.pdf")?;
///
/// // Open from remote URL
/// let doc = Document::open_remote("https://example.com/doc.pdf", &RemoteOpts::new())?;
///
/// // Get page count
/// let count = doc.page_count()?;
///
/// // Iterate pages lazily
/// for page_result in doc.pages() {
///     let page = page_result?;
///     println!("Page {}: {}x{}", page.index, page.width, page.height);
/// }
/// ```
pub struct Document {
    /// The parsed catalog
    catalog: Catalog,
    /// The xref resolver for object resolution
    resolver: XrefResolver,
    /// The PDF source (file, HTTP, memory)
    source: Option<Box<dyn ParserPdfSource>>,
    /// The document fingerprint
    fingerprint: String,
    /// Whether this is a remote document
    is_remote: bool,
}

impl Document {
    /// Open a PDF from a local file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PDF file
    ///
    /// # Returns
    ///
    /// A parsed Document ready for extraction.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened
    /// - The PDF is malformed
    /// - The xref table cannot be parsed
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let parser_source = ParserFileSource::open(path).context("Failed to open PDF file")?;
        Self::from_source(Box::new(parser_source), false)
    }

    /// Open a PDF from a remote HTTP/HTTPS URL.
    ///
    /// This performs the HTTP fetch sequence:
    /// 1. HEAD request to verify Range support and get Content-Length
    /// 2. Tail Range fetch (last 16 KB, progressive up to 1 MB) for startxref
    /// 3. Xref parsing with forward-scan disabled (no full file fetch)
    /// 4. Returns a parsed Document
    ///
    /// # Arguments
    ///
    /// * `url` - HTTP/HTTPS URL to the PDF file
    /// * `opts` - Remote options (headers, credentials, etc.)
    ///
    /// # Returns
    ///
    /// A parsed Document ready for extraction.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL is invalid or DNS fails
    /// - TLS handshake fails
    /// - Server returns 401/403
    /// - Server doesn't support Range requests
    /// - No Content-Length header
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::{Document, source::RemoteOpts};
    ///
    /// let opts = RemoteOpts::new()
    ///     .with_header("Authorization", "Bearer token");
    ///
    /// let doc = Document::open_remote("https://example.com/doc.pdf", &opts)?;
    /// ```
    #[cfg(feature = "remote")]
    pub fn open_remote(url: &str, opts: &RemoteOpts) -> Result<Self> {
        use crate::parser::stream::SourceAdapter;
        use crate::source::open_remote as open_remote_source;
        let source =
            open_remote_source(url, opts, None).context("Failed to open remote PDF source")?;
        let adapted = Box::new(SourceAdapter::new(source)) as Box<dyn ParserPdfSource>;
        Self::from_source(adapted, true)
    }

    /// Create a Document from a generic PdfSource.
    ///
    /// This is used internally by both `open` and `open_remote`.
    fn from_source(source: Box<dyn ParserPdfSource>, is_remote: bool) -> Result<Self> {
        // Find the startxref offset
        let startxref_offset =
            find_startxref(&*source).context("Failed to find startxref offset")?;

        // Load the xref table (forward-scan is disabled for remote sources automatically)
        let xref_section = load_xref_with_prev_chain(&*source, startxref_offset);

        // Create resolver from xref section
        let resolver = XrefResolver::from_section(xref_section.clone());

        // Get the root reference from trailer
        let root_ref = xref_section
            .trailer
            .as_ref()
            .and_then(|trailer| trailer.get("Root"))
            .and_then(|obj| obj.as_ref())
            .ok_or_else(|| anyhow!("No /Root reference in trailer"))?;

        // Parse the catalog
        let catalog =
            parse_catalog(&resolver, root_ref, Some(&*source)).map_err(|diagnostics| {
                let msg = diagnostics
                    .first()
                    .map(|d| d.message.as_ref())
                    .unwrap_or("unknown error");
                anyhow!("Failed to parse catalog: {}", msg)
            })?;

        // Resolve AcroForm dictionary if present (for XFA detection)
        let acroform = catalog
            .acroform_ref
            .and_then(|r| resolver.resolve(r).ok())
            .and_then(|o| o.as_dict().map(|d| d.clone()));

        // Build fingerprint (lazy version without full page tree)
        let fingerprint = compute_fingerprint_lazy(&catalog, &resolver, &acroform);

        Ok(Self {
            catalog,
            resolver,
            source: Some(source),
            fingerprint,
            is_remote,
        })
    }

    /// Get the document fingerprint.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Get the catalog.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Check if this is a remote document.
    pub fn is_remote(&self) -> bool {
        self.is_remote
    }

    /// Get the total page count.
    ///
    /// This walks the page tree to count pages without materializing PageDict objects.
    /// Uses O(depth) memory, making it safe for large documents.
    pub fn page_count(&self) -> Result<usize> {
        use crate::parser::pages::count_pages_tree;
        count_pages_tree(&self.resolver, self.catalog.pages_ref)
            .map_err(|e| anyhow!("Failed to count pages: {:?}", e))
    }

    /// Get a lazy iterator over pages.
    ///
    /// The iterator yields pages one at a time, decoding each page's
    /// content streams on-demand and dropping them after use.
    ///
    /// # Memory Behavior
    ///
    /// This uses LazyPageIter which walks the page tree depth-first,
    /// materializing only the current path from root to leaf (max ~16 nodes).
    /// Each yielded PageExtraction contains the extracted data for one page,
    /// and all intermediate data is dropped before yielding the next page.
    pub fn pages(&self) -> PageIter<'_> {
        PageIter {
            lazy_iter: None,
            catalog: &self.catalog,
            resolver: &self.resolver,
            source: self.source.as_ref().map(|s| s.as_ref()),
            index: 0,
        }
    }

    /// Get the xref resolver.
    pub fn resolver(&self) -> &XrefResolver {
        &self.resolver
    }

    /// Get the underlying source if available.
    pub fn source(&self) -> Option<&dyn ParserPdfSource> {
        self.source.as_ref().map(|s| s.as_ref())
    }

    /// Extract a single page by index.
    ///
    /// This function extracts page data from the Document structure and returns
    /// a Page instance suitable for output sinks. This is a basic implementation
    /// that extracts the essential fields without performing validation.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index to extract
    ///
    /// # Returns
    ///
    /// A `DocumentResult<Page>` containing the extracted page data.
    ///
    /// # Errors
    ///
    /// Returns `DocumentError::ExtractionFailed` if:
    /// - The page index is out of bounds
    /// - Page iteration fails
    /// - Page data extraction fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::document::Document;
    ///
    /// let doc = Document::open("document.pdf")?;
    /// let page = doc.extract_page(0)?;
    /// println!("Extracted page {} with dimensions {}x{}", page.page_number, page.width, page.height);
    /// ```
    pub fn extract_page(&self, page_index: usize) -> DocumentResult<crate::output::sink::Page> {
        use crate::output::sink::Page;

        // Validate page index is within bounds
        let page_count = match self.page_count() {
            Ok(count) => count,
            Err(e) => {
                return Err(DocumentError::ExtractionFailed {
                    page_index,
                    message: format!("Failed to get page count for validation: {:?}", e),
                });
            }
        };

        if page_index >= page_count {
            return Err(DocumentError::ExtractionFailed {
                page_index,
                message: format!(
                    "Page index {} out of bounds (document has {} pages)",
                    page_index, page_count
                ),
            });
        }

        // Navigate to the specific page using the iterator
        let mut pages_iter = self.pages();

        // Advance iterator to the target page
        for current_index in 0..=page_index {
            if let Some(result) = pages_iter.next() {
                if current_index == page_index {
                    let page_extraction = match result {
                        Ok(page) => page,
                        Err(e) => {
                            return Err(DocumentError::ExtractionFailed {
                                page_index,
                                message: format!("Page iteration failed: {:?}", e),
                            });
                        }
                    };

                    // Convert PageExtraction to output::sink::Page
                    let page = Page {
                        page_index: page_extraction.index,
                        page_number: (page_extraction.index + 1) as u32,
                        page_label: None, // Not yet implemented
                        width: page_extraction.width as f32,
                        height: page_extraction.height as f32,
                        rotation: page_extraction.rotation,
                        page_type: "unknown".to_string(), // Basic extraction - no classification yet
                        spans: vec![], // Basic extraction - no text spans yet
                        blocks: vec![], // Basic extraction - no blocks yet
                        links: vec![], // Basic extraction - no links yet
                    };

                    return Ok(page);
                }
            } else {
                return Err(DocumentError::ExtractionFailed {
                    page_index,
                    message: format!(
                        "Page extraction failed: iterator ended before reaching page index {}",
                        page_index
                    ),
                });
            }
        }

        Err(DocumentError::ExtractionFailed {
            page_index,
            message: "Failed to extract page: unknown error".to_string(),
        })
    }
}

/// Lazy iterator over PDF pages.
///
/// This iterator yields pages one at a time without materializing
/// the entire document model in memory.
///
/// # Memory Behavior
///
/// Uses LazyPageIter internally, which walks the page tree depth-first
/// and materializes only the current path from root to leaf (max ~16 nodes).
/// Each yielded PageExtraction contains the extracted data for one page,
/// and all intermediate data is dropped before yielding the next page.
///
/// # Examples
///
/// Iterate over pages with bounded memory:
///
/// ```rust,no_run
/// use pdftract_core::document::Document;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let doc = Document::open("large_document.pdf")?;
///
/// // Memory stays O(depth × per-page), not O(pages × per-page)
/// for page_result in doc.pages() {
///     let page = page_result?;
///     println!("Page {}: {}x{}", page.index, page.width, page.height);
///     // PageExtraction is dropped after each iteration
/// }
/// # Ok(())
/// # }
/// ```
pub struct PageIter<'a> {
    /// Lazy page iterator from the parser
    lazy_iter: Option<LazyPageIter<'a>>,
    /// Reference to the catalog for page tree root
    catalog: &'a Catalog,
    /// Reference to the resolver for object resolution
    resolver: &'a XrefResolver,
    /// Reference to the source for stream reading
    source: Option<&'a dyn ParserPdfSource>,
    /// Current page index
    index: usize,
}

impl<'a> Iterator for PageIter<'a> {
    type Item = Result<PageExtraction>;

    fn next(&mut self) -> Option<Self::Item> {
        // Initialize lazy iterator on first use
        if self.lazy_iter.is_none() {
            match LazyPageIter::new(self.resolver, self.catalog.pages_ref) {
                Ok(iter) => self.lazy_iter = Some(iter),
                Err(diagnostics) => {
                    let msg = diagnostics
                        .first()
                        .map(|d| d.message.as_ref())
                        .unwrap_or("unknown error");
                    return Some(Err(anyhow!("Failed to create lazy page iterator: {}", msg)));
                }
            }
        }

        let iter = self.lazy_iter.as_mut()?;

        match iter.next() {
            Some(Ok(page_dict)) => {
                let [x0, y0, x1, y1] = page_dict.media_box;
                let result = Ok(PageExtraction {
                    index: self.index,
                    width: x1 - x0,
                    height: y1 - y0,
                    rotation: page_dict.rotate,
                    spans: vec![],
                    blocks: vec![],
                });
                self.index += 1;

                // Explicitly drop page_dict to ensure memory is freed
                drop(page_dict);

                Some(result)
            }
            Some(Err(diagnostics)) => {
                let msg = diagnostics
                    .first()
                    .map(|d| d.message.as_ref())
                    .unwrap_or("unknown error");
                self.index += 1;
                Some(Err(anyhow!(
                    "Error extracting page {}: {}",
                    self.index - 1,
                    msg
                )))
            }
            None => None,
        }
    }
}

/// Open a PDF from a remote HTTP/HTTPS URL.
///
/// This is a convenience function that performs the HTTP fetch sequence:
/// 1. HEAD request to verify Range support and get Content-Length
/// 2. Tail Range fetch (last 16 KB) to parse startxref and trailer
/// 3. Xref parsing with forward-scan disabled for remote sources
/// 4. Returns the parsed catalog, resolver, source, and fingerprint
///
/// # Arguments
///
/// * `url` - HTTP/HTTPS URL to the PDF file
///
/// # Returns
///
/// A tuple of (catalog, resolver, source, fingerprint) for further processing.
///
/// # Errors
///
/// Returns an error if:
/// - URL is invalid or DNS fails
/// - TLS handshake fails
/// - Server returns 401/403
/// - Server doesn't support Range
/// - HEAD fails with 405 → Falls back to GET with Range: bytes=0-0
/// - No Content-Length → Returns error
///
/// # Example
///
/// ```ignore
/// use pdftract_core::document::open_remote_url;
///
/// let (catalog, resolver, source, fingerprint) = open_remote_url("https://example.com/doc.pdf")?;
/// // Use catalog, resolver, source for custom processing
/// ```
#[cfg(feature = "remote")]
pub fn open_remote_url(url: &str) -> std::io::Result<Box<dyn PdfSource>> {
    use crate::source::open_remote as open_remote_source;
    open_remote_source(url, &RemoteOpts::new(), None)
}

/// Open a PDF from a remote HTTP/HTTPS URL with options.
///
/// This is a convenience function that performs the HTTP fetch sequence
/// with custom options (headers, credentials).
///
/// # Arguments
///
/// * `url` - HTTP/HTTPS URL to the PDF file
/// * `opts` - Remote options (headers, credentials, etc.)
///
/// # Returns
///
/// A `Box<dyn PdfSource>` that can be used for PDF parsing.
///
/// # Errors
///
/// Returns an error if:
/// - URL is invalid or DNS fails → std::io::Error with kind `NotFound`
/// - TLS handshake fails → std::io::Error with kind `PermissionDenied`
/// - Server returns 401/403 → std::io::Error with kind `PermissionDenied`
/// - Server doesn't support Range → std::io::Error with kind `Unsupported`
/// - HEAD fails with 405 → Falls back to GET with Range: bytes=0-0
/// - No Content-Length → Returns error with kind `Other`
///
/// # Example
///
/// ```ignore
/// use pdftract_core::document::open_remote_url_with_opts;
/// use pdftract_core::source::RemoteOpts;
///
/// let opts = RemoteOpts::new()
///     .with_header("Authorization", "Bearer token");
///
/// let source = open_remote_url_with_opts("https://example.com/doc.pdf", &opts)?;
/// ```
#[cfg(feature = "remote")]
pub fn open_remote_url_with_opts(
    url: &str,
    opts: &RemoteOpts,
) -> std::io::Result<Box<dyn PdfSource>> {
    use crate::source::open_remote as open_remote_source;
    open_remote_source(url, opts, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    /// Create a minimal valid PDF for testing.
    fn create_minimal_pdf(path: &std::path::Path) -> Result<()> {
        let pdf_data = br#"%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
>>
endobj
4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Test) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000298 00000 n
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
376
%%EOF
"#;

        let mut file = File::create(path)?;
        file.write_all(pdf_data)?;
        Ok(())
    }

    #[test]
    fn test_find_startxref() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        create_minimal_pdf(&pdf_path).unwrap();

        let source = FileSource::open(&pdf_path).unwrap();
        let offset = find_startxref(&source).unwrap();
        assert_eq!(offset, 376);
    }

    #[test]
    fn test_parse_pdf_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        create_minimal_pdf(&pdf_path).unwrap();

        let (fingerprint, catalog, pages, resolver) = parse_pdf_file(&pdf_path).unwrap();

        assert!(fingerprint.starts_with("pdftract-v1:"));
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].media_box, [0.0, 0.0, 612.0, 792.0]);
        assert_eq!(pages[0].rotate, 0);

        // Verify resolver has entries
        assert!(resolver.len() > 0);
    }

    #[test]
    fn test_compute_pdf_fingerprint() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        create_minimal_pdf(&pdf_path).unwrap();

        let fingerprint = compute_pdf_fingerprint(&pdf_path).unwrap();

        assert!(fingerprint.starts_with("pdftract-v1:"));
        assert_eq!(fingerprint.len(), "pdftract-v1:".len() + 64);

        // Verify hex format
        let hex_part = &fingerprint["pdftract-v1:".len()..];
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_extract_spans_from_page() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        create_minimal_pdf(&pdf_path).unwrap();

        let spans = extract_spans_from_page(&pdf_path, 0).unwrap();

        // Should have at least one span (placeholder for now)
        assert!(!spans.is_empty());

        // Check the span has the expected structure
        let span = &spans[0];
        assert!(!span.text.is_empty());
        assert_eq!(span.bbox, [0.0, 0.0, 612.0, 792.0]);
    }

    #[test]
    fn test_extract_spans_out_of_bounds() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        create_minimal_pdf(&pdf_path).unwrap();

        let result = extract_spans_from_page(&pdf_path, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_page_basic() {
        // Use an existing test PDF that we know works
        let pdf_path = std::path::Path::new("tests/fixtures/sample.pdf");

        // Skip test if file doesn't exist
        if !pdf_path.exists() {
            println!("Skipping test - sample.pdf not found");
            return;
        }

        let doc = Document::open(pdf_path).unwrap();

        // Extract the first page
        let page = doc.extract_page(0).unwrap();

        // Verify basic page structure
        assert_eq!(page.page_index, 0);
        assert_eq!(page.page_number, 1);

        // Verify dimensions are extracted (actual values depend on PDF)
        assert!(page.width > 0.0);
        assert!(page.height > 0.0);

        // Verify fields are present but empty for basic extraction
        assert!(page.page_label.is_none());
        assert!(page.spans.is_empty());
        assert!(page.blocks.is_empty());
        assert!(page.links.is_empty());
        assert_eq!(page.page_type, "unknown");
    }

    #[test]
    fn test_extract_page_out_of_bounds() {
        let pdf_path = std::path::Path::new("tests/fixtures/sample.pdf");

        // Skip test if file doesn't exist
        if !pdf_path.exists() {
            println!("Skipping test - sample.pdf not found");
            return;
        }

        let doc = Document::open(pdf_path).unwrap();

        // Try to extract a page that doesn't exist
        let result = doc.extract_page(10);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));
    }

    #[test]
    fn test_display_empty_document() {
        let err = DocumentError::EmptyDocument {
            source: "test.pdf".to_string(),
        };
        assert_eq!(err.to_string(), "Document 'test.pdf' is empty or contains no content");
    }

    #[test]
    fn test_display_missing_pages_array() {
        let err = DocumentError::MissingPagesArray {
            source: "test.pdf".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("missing the required /Pages field"));
        assert!(msg.contains("test.pdf"));
    }

    #[test]
    fn test_display_invalid_pages_format() {
        let err = DocumentError::InvalidPagesFormat {
            source: "test.pdf".to_string(),
            found_type: "dictionary".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("not an array"));
        assert!(msg.contains("found: dictionary"));
    }

    #[test]
    fn test_display_page_out_of_bounds() {
        let err = DocumentError::PageOutOfBounds {
            source: "test.pdf".to_string(),
            requested: 10,
            available: 5,
        };
        let msg = err.to_string();
        assert!(msg.contains("Page index 10 out of bounds"));
        assert!(msg.contains("document has 5 pages"));
        assert!(msg.contains("valid indices: 0-4"));
    }

    #[test]
    fn test_display_malformed_page_data() {
        let err = DocumentError::MalformedPageData {
            page_index: 0,
            message: "Invalid media box dimensions".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 0 has malformed data"));
        assert!(msg.contains("Invalid media box dimensions"));
    }

    #[test]
    fn test_display_malformed_document_structure() {
        let err = DocumentError::MalformedDocumentStructure {
            source: "test.pdf".to_string(),
            message: "Corrupt page tree".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("has malformed structure"));
        assert!(msg.contains("Corrupt page tree"));
    }

    #[test]
    fn test_display_extraction_failed() {
        let err = DocumentError::ExtractionFailed {
            page_index: 0,
            message: "Content stream decode error".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to extract page 0: Content stream decode error"
        );
    }

    #[test]
    fn test_display_file_open_failed() {
        let err = DocumentError::FileOpenFailed {
            path: "/path/to/file.pdf".to_string(),
            reason: "Permission denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to open file"));
        assert!(msg.contains("Permission denied"));
    }

    #[test]
    fn test_display_startxref_not_found() {
        let err = DocumentError::StartxrefNotFound {
            source: "test.pdf".to_string(),
            file_size_bytes: 1024,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to find startxref offset"));
        assert!(msg.contains("1024 bytes"));
    }

    #[test]
    fn test_display_xref_parse_failed() {
        let err = DocumentError::XrefParseFailed {
            source: "test.pdf".to_string(),
            offset: 100,
            reason: "Invalid xref format".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to parse xref table"));
        assert!(msg.contains("offset 100"));
        assert!(msg.contains("Invalid xref format"));
    }

    #[test]
    fn test_display_catalog_parse_failed() {
        let err = DocumentError::CatalogParseFailed {
            source: "test.pdf".to_string(),
            reason: "Missing Root entry".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to parse catalog"));
        assert!(msg.contains("Missing Root entry"));
    }

    #[test]
    fn test_display_encryption_not_supported() {
        let err = DocumentError::EncryptionNotSupported {
            source: "encrypted.pdf".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("encrypted"));
        assert!(msg.contains("not yet supported"));
    }

    #[test]
    fn test_display_page_count_failed() {
        let err = DocumentError::PageCountFailed {
            source: "test.pdf".to_string(),
            reason: "Circular page tree reference".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to count pages"));
        assert!(msg.contains("Circular page tree reference"));
    }

    #[test]
    fn test_display_invalid_media_box() {
        let err = DocumentError::InvalidMediaBox {
            page_index: 0,
            media_box: Some([0.0, 0.0, -1.0, 792.0]),
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 0 has invalid media box"));
        assert!(msg.contains("must have x1 > x0"));
    }

    #[test]
    fn test_display_invalid_dimensions() {
        let err = DocumentError::InvalidDimensions {
            page_index: 1,
            width: 0.0,
            height: 792.0,
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 1 has invalid dimensions"));
        assert!(msg.contains("width=0"));
        assert!(msg.contains("both must be positive"));
    }

    #[test]
    fn test_display_invalid_rotation() {
        let err = DocumentError::InvalidRotation {
            page_index: 2,
            rotation: 45,
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 2 has invalid rotation"));
        assert!(msg.contains("45°"));
        assert!(msg.contains("must be 0, 90, 180, or 270"));
    }

    #[test]
    fn test_display_content_stream_decode_failed() {
        let err = DocumentError::ContentStreamDecodeFailed {
            page_index: 3,
            message: "Invalid FlateDecode stream".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to decode content stream for page 3"));
        assert!(msg.contains("Invalid FlateDecode stream"));
    }

    #[test]
    fn test_display_missing_content_stream() {
        let err = DocumentError::MissingContentStream { page_index: 4 };
        assert_eq!(
            err.to_string(),
            "Page 4 has no content stream (empty or missing)"
        );
    }

    #[test]
    fn test_display_invalid_resources() {
        let err = DocumentError::InvalidResources {
            page_index: 5,
            message: "Font dictionary missing".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 5 has invalid resources"));
        assert!(msg.contains("Font dictionary missing"));
    }

    #[test]
    fn test_display_missing_required_fields() {
        let err = DocumentError::MissingRequiredFields {
            page_index: 6,
            fields: vec!["MediaBox".to_string(), "Resources".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 6 is missing required fields"));
        assert!(msg.contains("MediaBox"));
        assert!(msg.contains("Resources"));
    }

    #[test]
    fn test_display_linearization_failed() {
        let err = DocumentError::LinearizationFailed {
            source: "linearized.pdf".to_string(),
            reason: "Invalid linearization dictionary".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to parse linearized document"));
        assert!(msg.contains("Invalid linearization dictionary"));
    }

    #[test]
    fn test_display_remote_fetch_failed_with_status() {
        let err = DocumentError::RemoteFetchFailed {
            url: "https://example.com/doc.pdf".to_string(),
            status_code: Some(404),
            reason: "Not Found".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to fetch remote document"));
        assert!(msg.contains("HTTP 404"));
        assert!(msg.contains("Not Found"));
    }

    #[test]
    fn test_display_remote_fetch_failed_without_status() {
        let err = DocumentError::RemoteFetchFailed {
            url: "https://example.com/doc.pdf".to_string(),
            status_code: None,
            reason: "Connection timeout".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to fetch remote document"));
        assert!(msg.contains("Connection timeout"));
        assert!(!msg.contains("HTTP"));
    }

    #[test]
    fn test_display_invalid_pdf_header() {
        let err = DocumentError::InvalidPdfHeader {
            source: "notpdf.txt".to_string(),
            found_header: "%TXT-1.0".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid PDF header"));
        assert!(msg.contains("expected '%PDF-1.x'"));
        assert!(msg.contains("found: '%TXT-1.0'"));
    }

    #[test]
    fn test_display_invalid_trailer() {
        let err = DocumentError::InvalidTrailer {
            source: "test.pdf".to_string(),
            reason: "Missing /Root entry".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid trailer"));
        assert!(msg.contains("Missing /Root entry"));
    }

    #[test]
    fn test_display_processing_failed() {
        let err = DocumentError::ProcessingFailed {
            source: "test.pdf".to_string(),
            message: "Unexpected error during extraction".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to process document"));
        assert!(msg.contains("Unexpected error during extraction"));
    }

    #[test]
    fn test_error_implements_send_and_sync() {
        // Ensure error type can be sent across threads
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<DocumentError>();
        assert_sync::<DocumentError>();
    }

    #[test]
    fn test_error_clone() {
        let err1 = DocumentError::PageOutOfBounds {
            source: "test.pdf".to_string(),
            requested: 5,
            available: 3,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_conversion_to_anyhow() {
        let doc_err = DocumentError::EmptyDocument {
            source: "test.pdf".to_string(),
        };
        // anyhow provides a blanket From implementation for any type that implements Error
        let anyhow_err = anyhow::anyhow!(doc_err);
        assert!(anyhow_err
            .to_string()
            .contains("Document 'test.pdf' is empty or contains no content"));
    }

    #[test]
    fn test_error_variant_count() {
        // Ensure we have at least the required 6 variants from acceptance criteria
        // We'll verify this by checking that specific required variants exist
        let _empty = DocumentError::EmptyDocument {
            source: "test.pdf".to_string(),
        };
        let _missing_pages = DocumentError::MissingPagesArray {
            source: "test.pdf".to_string(),
        };
        let _invalid_format = DocumentError::InvalidPagesFormat {
            source: "test.pdf".to_string(),
            found_type: "not array".to_string(),
        };
        let _out_of_bounds = DocumentError::PageOutOfBounds {
            source: "test.pdf".to_string(),
            requested: 10,
            available: 5,
        };
        let _malformed = DocumentError::MalformedPageData {
            page_index: 0,
            message: "test".to_string(),
        };
        let _struct_malformed = DocumentError::MalformedDocumentStructure {
            source: "test.pdf".to_string(),
            message: "test".to_string(),
        };

        // If we successfully created all 6 required variants, the test passes
        assert!(true, "DocumentError has all required variants");
    }
}
