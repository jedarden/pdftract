//! Page extraction helper functions.
//!
//! This module provides high-level helper functions for extracting
//! Page data from PDF documents. It serves as a convenient API surface
//! for common page extraction operations.

use crate::document::{Document, PageExtraction};
use anyhow::{anyhow, Result};
use std::fmt;

/// Errors that can occur during page extraction.
///
/// This enum provides specific error types for various failure modes
/// when extracting pages from PDF documents, enabling better error handling
/// and user feedback.
#[derive(Debug, Clone, PartialEq)]
pub enum PageError {
    /// Document has no pages (page count is 0)
    NoPages,

    /// Page index is out of bounds for the document
    IndexOutOfBounds {
        /// The requested index
        requested: usize,
        /// The actual page count
        available: usize,
    },

    /// Page has invalid dimensions (width or height is zero or negative)
    InvalidDimensions {
        /// Page index
        index: usize,
        /// Width value
        width: f64,
        /// Height value
        height: f64,
    },

    /// Page has an invalid rotation value (not 0, 90, 180, or 270)
    InvalidRotation {
        /// Page index
        index: usize,
        /// The rotation value
        rotation: i32,
    },

    /// Failed to get page count from document
    PageCountFailed(String),

    /// Failed to extract page data
    ExtractionFailed {
        /// Page index
        index: usize,
        /// Underlying error message
        message: String,
    },

    /// Document structure is malformed (missing or corrupt page tree)
    MalformedStructure(String),

    /// Page data is missing required fields
    MissingFields {
        /// Page index
        index: usize,
        /// List of missing field names
        fields: Vec<String>,
    },
}

impl fmt::Display for PageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPages => write!(f, "Document contains no pages"),
            Self::IndexOutOfBounds { requested, available } => {
                write!(
                    f,
                    "Page index {} out of bounds (document has {} pages)",
                    requested, available
                )
            }
            Self::InvalidDimensions { index, width, height } => {
                write!(
                    f,
                    "Page {} has invalid dimensions: width={}, height={} (both must be positive)",
                    index, width, height
                )
            }
            Self::InvalidRotation { index, rotation } => {
                write!(
                    f,
                    "Page {} has invalid rotation: {} (must be 0, 90, 180, or 270)",
                    index, rotation
                )
            }
            Self::PageCountFailed(msg) => {
                write!(f, "Failed to get page count: {}", msg)
            }
            Self::ExtractionFailed { index, message } => {
                write!(f, "Failed to extract page at index {}: {}", index, message)
            }
            Self::MalformedStructure(msg) => {
                write!(f, "Document has malformed page structure: {}", msg)
            }
            Self::MissingFields { index, fields } => {
                write!(
                    f,
                    "Page {} is missing required fields: {}",
                    index,
                    fields.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for PageError {}

/// Validate page extraction data.
///
/// Checks that the extracted page has valid dimensions and rotation values.
/// Returns an error if any validation fails.
fn validate_page_extraction(page: &PageExtraction) -> Result<(), PageError> {
    // Check dimensions are positive
    if page.width <= 0.0 || page.height <= 0.0 {
        return Err(PageError::InvalidDimensions {
            index: page.index,
            width: page.width,
            height: page.height,
        });
    }

    // Check rotation is valid (must be 0, 90, 180, or 270)
    if page.rotation != 0 && page.rotation != 90 && page.rotation != 180 && page.rotation != 270 {
        return Err(PageError::InvalidRotation {
            index: page.index,
            rotation: page.rotation,
        });
    }

    Ok(())
}

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
/// or an error if the index is out of bounds or the page has invalid data.
///
/// # Errors
///
/// Returns `PageError` if:
/// - The document has no pages
/// - The page index is out of bounds
/// - The page has invalid dimensions (width/height <= 0)
/// - The page has an invalid rotation value (not 0, 90, 180, or 270)
/// - The page extraction fails for any other reason
///
/// # Example
///
/// ```ignore
/// use pdftract_core::{page_helper, Document};
///
/// let doc = Document::open("document.pdf")?;
/// let page = page_helper::extract_page(&doc, 0)?;
/// println!("Page dimensions: {}x{}", page.width, page.height);
/// ```
pub fn extract_page(document: &Document, page_index: usize) -> Result<PageExtraction> {
    // Get total page count to validate bounds
    let page_count = document.page_count().map_err(|e| {
        anyhow::Error::from(PageError::PageCountFailed(format!("{:?}", e)))
    })?;

    // Check for empty document
    if page_count == 0 {
        return Err(anyhow::Error::from(PageError::NoPages));
    }

    if page_index >= page_count {
        return Err(anyhow::Error::from(PageError::IndexOutOfBounds {
            requested: page_index,
            available: page_count,
        }));
    }

    // Iterate to the requested page
    let mut page_iter = document.pages();
    for (idx, page_result) in page_iter.enumerate() {
        if idx == page_index {
            let page = page_result.map_err(|e| {
                anyhow::Error::from(PageError::ExtractionFailed {
                    index: page_index,
                    message: format!("{:?}", e),
                })
            })?;

            // Validate the extracted page data
            validate_page_extraction(&page)?;

            return Ok(page);
        }
    }

    // This should not be reached if the bounds check passed
    Err(anyhow::Error::from(PageError::ExtractionFailed {
        index: page_index,
        message: "Page not found in iterator".to_string(),
    }))
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
/// # Errors
///
/// Returns `PageError` if:
/// - The document has no pages
/// - Any page has invalid dimensions or rotation
/// - Page extraction fails for any page
///
/// # Example
///
/// ```ignore
/// use pdftract_core::{page_helper, Document};
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
pub fn extract_all_pages(document: &Document) -> Result<Vec<PageExtraction>> {
    let mut pages = Vec::new();

    for page_result in document.pages() {
        let page = page_result.map_err(|e| {
            anyhow::Error::from(PageError::ExtractionFailed {
                index: pages.len(),
                message: format!("{:?}", e),
            })
        })?;

        // Validate the extracted page data
        validate_page_extraction(&page)?;

        pages.push(page);
    }

    // Return empty collection gracefully when no pages present
    // (not an error - handles documents with 0 pages)
    Ok(pages)
}

/// Extract a range of pages from a Document.
///
/// This helper function extracts a contiguous range of pages from a Document.
/// The range is inclusive of both start and end indices (0-based).
///
/// # Arguments
///
/// * `document` - A reference to a parsed Document
/// * `start` - Start page index (0-based, inclusive)
/// * `end` - End page index (0-based, inclusive)
///
/// # Returns
///
/// A `Result` containing a `Vec<PageExtraction>` with pages in the specified range,
/// or an error if the range is invalid or extraction fails.
///
/// # Errors
///
/// Returns `PageError` if:
/// - The document has no pages
/// - The range bounds are out of bounds
/// - The start index is greater than the end index
/// - Any page has invalid dimensions or rotation
/// - Page extraction fails for any page in the range
///
/// # Example
///
/// ```ignore
/// use pdftract_core::{page_helper, Document};
///
/// let doc = Document::open("document.pdf")?;
/// // Extract pages 5 through 10 (inclusive)
/// let pages = page_helper::extract_page_range(&doc, 5, 10)?;
/// for page in pages {
///     println!("Page {}: {}x{}", page.index, page.width, page.height);
/// }
/// ```
pub fn extract_page_range(document: &Document, start: usize, end: usize) -> Result<Vec<PageExtraction>> {
    // Get total page count to validate bounds
    let page_count = document.page_count().map_err(|e| {
        anyhow::Error::from(PageError::PageCountFailed(format!("{:?}", e)))
    })?;

    // Check for empty document
    if page_count == 0 {
        return Ok(Vec::new()); // Return empty collection gracefully
    }

    // Validate range
    if start > end {
        return Err(anyhow::Error::from(PageError::IndexOutOfBounds {
            requested: start,
            available: page_count,
        }));
    }

    if start >= page_count {
        return Err(anyhow::Error::from(PageError::IndexOutOfBounds {
            requested: start,
            available: page_count,
        }));
    }

    if end >= page_count {
        return Err(anyhow::Error::from(PageError::IndexOutOfBounds {
            requested: end,
            available: page_count,
        }));
    }

    let mut pages = Vec::new();

    // Iterate to the start index
    let mut page_iter = document.pages();
    for (idx, page_result) in page_iter.enumerate() {
        if idx < start {
            continue; // Skip pages before start
        }

        if idx > end {
            break; // Stop after end
        }

        let page = page_result.map_err(|e| {
            anyhow::Error::from(PageError::ExtractionFailed {
                index: idx,
                message: format!("{:?}", e),
            })
        })?;

        // Validate the extracted page data
        validate_page_extraction(&page)?;

        pages.push(page);
    }

    Ok(pages)
}

/// Get page count from a Document with error handling.
///
/// This is a convenience wrapper around `Document::page_count()` that
/// converts the error to a `PageError`.
///
/// # Arguments
///
/// * `document` - A reference to a parsed Document
///
/// # Returns
///
/// The total number of pages in the document.
///
/// # Errors
///
/// Returns `PageError::PageCountFailed` if the page count cannot be determined.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::{page_helper, Document};
///
/// let doc = Document::open("document.pdf")?;
/// let count = page_helper::page_count(&doc)?;
/// println!("Document has {} pages", count);
/// ```
pub fn page_count(document: &Document) -> Result<usize> {
    document
        .page_count()
        .map_err(|e| anyhow::Error::from(PageError::PageCountFailed(format!("{:?}", e))))
}
