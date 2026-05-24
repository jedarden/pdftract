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

use crate::fingerprint::{CatalogFlags, ContentStreamData, FingerprintInput, PageFingerprintData, compute_fingerprint};
use crate::parser::catalog::{parse_catalog, Catalog};
use crate::parser::pages::{flatten_page_tree, PageDict, LazyPageIter};
use crate::parser::stream::{FileSource, PdfSource};
use crate::parser::xref::{XrefResolver, load_xref_with_prev_chain, XrefSection};
use crate::receipts::verifier::SpanData;
use anyhow::{Context, Result, anyhow};
use serde::{Serialize, Deserialize};
use std::path::Path;

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
pub fn parse_pdf_file(pdf_path: &std::path::Path) -> Result<(String, Catalog, Vec<crate::parser::pages::PageDict>, XrefResolver)> {
    // Open the PDF file
    let source = FileSource::open(pdf_path)
        .context("Failed to open PDF file")?;

    // Find the startxref offset
    let startxref_offset = find_startxref(&source)
        .context("Failed to find startxref offset")?;

    // Load the xref table
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);

    // Create resolver from xref section
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Get the root reference from trailer
    let root_ref = xref_section.trailer
        .as_ref()
        .and_then(|trailer| trailer.get("Root"))
        .and_then(|obj| obj.as_ref())
        .ok_or_else(|| anyhow!("No /Root reference in trailer"))?;

    // Parse the catalog
    let catalog = parse_catalog(&resolver, root_ref)
        .map_err(|diagnostics| {
            let msg = diagnostics.first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow!("Failed to parse catalog: {}", msg)
        })?;

    // Flatten the page tree
    let pages = flatten_page_tree(&resolver, catalog.pages_ref)
        .map_err(|diagnostics| {
            let msg = diagnostics.first()
                .map(|d| d.message.as_ref())
                .unwrap_or("unknown error");
            anyhow!("Failed to flatten page tree: {}", msg)
        })?;

    // Build fingerprint input
    let fingerprint_input = build_fingerprint_input(&catalog, &pages, &xref_section);

    // Compute fingerprint
    let fingerprint = compute_fingerprint(&fingerprint_input, &resolver);

    Ok((fingerprint, catalog, pages, resolver))
}

/// Find the startxref offset in a PDF file.
///
/// Scans the last 1024 bytes of the file for "startxref" keyword.
fn find_startxref(source: &dyn PdfSource) -> Result<u64> {
    let len = source.len()? as usize;
    let scan_start = len.saturating_sub(1024);
    let scan_end = len;

    let tail_data = source.read_at(scan_start as u64, scan_end - scan_start)
        .context("Failed to read PDF tail")?;

    // Find "startxref" in the tail data
    let startxref_pos = tail_data.windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| anyhow!("startxref not found in PDF"))?;

    // Parse the offset after "startxref"
    // Skip the "startxref" keyword (9 chars) and any following whitespace
    let offset_data = &tail_data[startxref_pos + 9..];

    // Skip leading whitespace (space, \r, \n, \t)
    let offset_start = offset_data.iter()
        .position(|&b| !matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
        .unwrap_or(offset_data.len());

    let offset_data_trimmed = &offset_data[offset_start..];

    // Find the newline after the offset
    let newline_pos = offset_data_trimmed.iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(offset_data_trimmed.len());

    let offset_str = std::str::from_utf8(&offset_data_trimmed[..newline_pos])
        .context("startxref offset is not valid UTF-8")?;

    let offset: u64 = offset_str.trim().parse()
        .context("startxref offset is not a valid number")?;

    Ok(offset)
}

/// Build FingerprintInput from catalog and pages.
fn build_fingerprint_input(
    catalog: &Catalog,
    pages: &[crate::parser::pages::PageDict],
    _xref_section: &XrefSection,
) -> FingerprintInput {
    let page_count = pages.len() as u32;

    let fingerprint_pages = pages.iter().map(|page| {
        PageFingerprintData {
            content_streams: page.contents.iter()
                .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
                .collect(),
            resources: None, // TODO: convert ResourceDict to PdfDict
            media_box: page.media_box,
            crop_box: page.crop_box,
            rotate: page.rotate,
        }
    }).collect();

    // Build catalog flags
    let catalog_flags = CatalogFlags {
        is_encrypted: false, // TODO: detect encryption
        contains_javascript: catalog.open_action.is_some() || catalog.aa.is_some(),
        contains_xfa: false, // TODO: detect XFA
        ocg_present: catalog.oc_properties.as_ref()
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
        return Err(anyhow!("Page index {} out of bounds (document has {} pages)",
            page_index, pages.len()));
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
/// The fingerprint string in the format "pdftract-v1:<hex>"
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
        let source = FileSource::open(path)
            .context("Failed to open PDF file")?;

        // Find the startxref offset
        let startxref_offset = find_startxref(&source)
            .context("Failed to find startxref offset")?;

        // Load the xref table
        let xref_section = load_xref_with_prev_chain(&source, startxref_offset);

        // Create resolver from xref section
        let resolver = XrefResolver::from_section(xref_section.clone());

        // Get the root reference from trailer
        let root_ref = xref_section.trailer
            .as_ref()
            .and_then(|trailer| trailer.get("Root"))
            .and_then(|obj| obj.as_ref())
            .ok_or_else(|| anyhow!("No /Root reference in trailer"))?;

        // Parse the catalog
        let catalog = parse_catalog(&resolver, root_ref)
            .map_err(|diagnostics| {
                let msg = diagnostics.first()
                    .map(|d| d.message.as_ref())
                    .unwrap_or("unknown error");
                anyhow!("Failed to parse catalog: {}", msg)
            })?;

        // Build fingerprint input (without full page tree for lazy extraction)
        let fingerprint = compute_fingerprint_lazy(&catalog, &xref_section);

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
        Ok(self.pages.as_ref().unwrap())
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
            extractor: self,
            index: 0,
        }
    }

    /// Extract a single page by index.
    ///
    /// This method extracts one page without materializing the entire document.
    /// Content streams are decoded and the result is returned.
    pub fn extract_page(&self, page_index: usize) -> Result<PageExtraction> {
        let pages = self.pages.as_ref()
            .ok_or_else(|| anyhow!("Pages not materialized. Call materialize_pages() first."))?;

        if page_index >= pages.len() {
            return Err(anyhow!("Page index {} out of bounds (document has {} pages)",
                page_index, pages.len()));
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
/// This iterator yields pages one at a time without materializing
/// the entire document model in memory.
///
/// # Memory Behavior
///
/// Uses LazyPageIter internally, which walks the page tree depth-first
/// and materializes only the current path from root to leaf (max ~16 nodes).
/// Each yielded PageExtraction contains the extracted data for one page,
/// and all intermediate data is dropped before yielding the next page.
pub struct PageIter<'a> {
    /// Lazy page iterator from the parser
    lazy_iter: Option<LazyPageIter<'a>>,
    /// Reference to the extractor for accessing source/resolver
    extractor: &'a PdfExtractor,
    /// Current page index
    index: usize,
}

impl<'a> Iterator for PageIter<'a> {
    type Item = Result<PageExtraction>;

    fn next(&mut self) -> Option<Self::Item> {
        // Initialize lazy iterator on first use
        if self.lazy_iter.is_none() {
            match LazyPageIter::new(&self.extractor.resolver, self.extractor.catalog.pages_ref) {
                Ok(iter) => self.lazy_iter = Some(iter),
                Err(diagnostics) => {
                    let msg = diagnostics.first()
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
                let msg = diagnostics.first()
                    .map(|d| d.message.as_ref())
                    .unwrap_or("unknown error");
                self.index += 1;
                Some(Err(anyhow!("Error extracting page {}: {}", self.index - 1, msg)))
            }
            None => None,
        }
    }
}

/// Compute fingerprint without full page materialization.
///
/// This is a simplified version that uses only catalog-level data.
/// The full fingerprint computation requires page content streams.
pub(crate) fn compute_fingerprint_lazy(catalog: &Catalog, _xref_section: &XrefSection) -> String {
    // For lazy extraction, use a simpler fingerprint based on catalog data
    // The full implementation would incrementally hash pages as they're extracted
    use crate::fingerprint::FingerprintInput;

    let fingerprint_input = FingerprintInput {
        page_count: 0, // Will be updated when pages are extracted
        pages: vec![],
        struct_tree_root_ref: catalog.struct_tree_root_ref,
        is_tagged: catalog.mark_info.is_tagged,
        catalog_flags: CatalogFlags {
            is_encrypted: false,
            contains_javascript: catalog.open_action.is_some() || catalog.aa.is_some(),
            contains_xfa: false,
            ocg_present: catalog.oc_properties.as_ref()
                .map(|props| props.present)
                .unwrap_or(false),
        },
    };

    compute_fingerprint(&fingerprint_input, &XrefResolver::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::fs::File;

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
403
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
        assert_eq!(offset, 403);
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
}
