//! Page extraction helper functions.
//!
//! This module provides high-level helper functions for extracting
//! Page data from PDF documents. It serves as a convenient API surface
//! for common page extraction operations.

use pdftract_core::document::{Document, PageExtraction};

/// Extract a single page from a Document by index.
///
/// This helper function provides a simple way to extract a single page
/// from a parsed Document without dealing with the iterator directly.
///
/// # Arguments
///
/// * `document` - A reference to a parsed Document
/// * `page_index` - 0-based page index to extract
///
/// # Returns
///
/// A `Result` containing the `PageExtraction` for the requested page,
/// or an error if the index is out of bounds.
///
/// # Example
///
/// ```ignore
/// use pdftract::{page_helper, Document};
///
/// let doc = Document::open("document.pdf")?;
/// let page = page_helper::extract_page(&doc, 0)?;
/// println!("Page dimensions: {}x{}", page.width, page.height);
/// ```
pub fn extract_page(document: &Document, page_index: usize) -> anyhow::Result<PageExtraction> {
    // Get total page count to validate bounds
    let page_count = document.page_count()
        .map_err(|e| anyhow::anyhow!("Failed to get page count: {:?}", e))?;

    if page_index >= page_count {
        return Err(anyhow::anyhow!(
            "Page index {} out of bounds (document has {} pages)",
            page_index,
            page_count
        ));
    }

    // Iterate to the requested page
    let mut page_iter = document.pages();
    for (idx, page_result) in page_iter.enumerate() {
        if idx == page_index {
            return page_result;
        }
    }

    // This should not be reached if the bounds check passed
    Err(anyhow::anyhow!("Failed to extract page at index {}", page_index))
}

/// Extract all pages from a Document.
///
/// This helper function collects all pages from a Document into a Vec.
/// For large documents, consider using the Document's iterator directly
/// to avoid materializing all pages in memory.
///
/// # Arguments
///
/// * `document` - A reference to a parsed Document
///
/// # Returns
///
/// A `Result` containing a `Vec<PageExtraction>` with all pages in order,
/// or an error if extraction fails.
///
/// # Example
///
/// ```ignore
/// use pdftract::{page_helper, Document};
///
/// let doc = Document::open("document.pdf")?;
/// let pages = page_helper::extract_all_pages(&doc)?;
/// for page in pages {
///     println!("Page {}: {}x{}", page.index, page.width, page.height);
/// }
/// ```
///
/// # Memory Warning
///
/// This function materializes ALL pages in memory. For large documents
/// (1000+ pages), prefer iterating directly with `document.pages()`
/// to process pages one at a time with bounded memory usage.
pub fn extract_all_pages(document: &Document) -> anyhow::Result<Vec<PageExtraction>> {
    let mut pages = Vec::new();

    for page_result in document.pages() {
        let page = page_result
            .map_err(|e| anyhow::anyhow!("Failed to extract page: {:?}", e))?;
        pages.push(page);
    }

    Ok(pages)
}

/// Get page count from a Document with error handling.
///
/// This is a convenience wrapper around `Document::page_count()` that
/// converts the error to an `anyhow::Error`.
///
/// # Arguments
///
/// * `document` - A reference to a parsed Document
///
/// # Returns
///
/// The total number of pages in the document.
///
/// # Example
///
/// ```ignore
/// use pdftract::{page_helper, Document};
///
/// let doc = Document::open("document.pdf")?;
/// let count = page_helper::page_count(&doc)?;
/// println!("Document has {} pages", count);
/// ```
pub fn page_count(document: &Document) -> anyhow::Result<usize> {
    document.page_count()
        .map_err(|e| anyhow::anyhow!("Failed to get page count: {:?}", e))
}
