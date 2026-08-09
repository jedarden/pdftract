//! Comprehensive error handling for Page extraction operations.
//!
//! This module provides detailed error types for all failure modes that can occur
//! during Page extraction from PDF documents. These errors enable precise error
//! handling, clear user feedback, and targeted recovery strategies.

use std::fmt;

/// Comprehensive error type for Page extraction failures.
///
/// This enum provides specific error types for various failure modes
/// when extracting pages from PDF documents, enabling better error handling,
/// user feedback, and recovery strategies.
#[derive(Debug, Clone, PartialEq)]
pub enum PageExtractionError {
    /// The document has no pages (page count is 0)
    NoPagesInDocument,

    /// Page index is out of bounds for the document
    IndexOutOfBounds {
        /// The requested 0-based page index
        requested: usize,
        /// The actual number of pages in the document
        available: usize,
    },

    /// Page has invalid or missing media box (bounding box)
    InvalidMediaBox {
        /// Page index
        page_index: usize,
        /// The media box values [x0, y0, x1, y1]
        media_box: Option<[f64; 4]>,
    },

    /// Page has invalid dimensions (width or height is zero or negative)
    InvalidDimensions {
        /// Page index
        page_index: usize,
        /// Width value in points
        width: f64,
        /// Height value in points
        height: f64,
    },

    /// Page has an invalid rotation value (not 0, 90, 180, or 270)
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

    /// Content stream exceeds decompression bomb limit
    ContentStreamTooLarge {
        /// Page index
        page_index: usize,
        /// Size in bytes that was attempted
        size_bytes: u64,
        /// Maximum allowed size
        max_bytes: u64,
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

    /// Glyph extraction failed
    GlyphExtractionFailed {
        /// Page index
        page_index: usize,
        /// Underlying error message
        message: String,
    },

    /// Span merging failed
    SpanMergeFailed {
        /// Page index
        page_index: usize,
        /// Number of glyphs attempted
        glyph_count: usize,
        /// Underlying error message
        message: String,
    },

    /// Layout analysis failed
    LayoutAnalysisFailed {
        /// Page index
        page_index: usize,
        /// Analysis stage that failed
        stage: String,
        /// Underlying error message
        message: String,
    },

    /// Table detection failed
    TableDetectionFailed {
        /// Page index
        page_index: usize,
        /// Underlying error message
        message: String,
    },

    /// Receipt generation failed
    ReceiptGenerationFailed {
        /// Page index
        page_index: usize,
        /// Underlying error message
        message: String,
    },

    /// Page data is malformed or corrupted
    MalformedPageData {
        /// Page index
        page_index: usize,
        /// Description of the malformed data
        message: String,
    },

    /// Document structure is malformed (missing or corrupt page tree)
    MalformedDocumentStructure(String),

    /// Page extraction panicked (caught via catch_unwind)
    ExtractionPanicked {
        /// Page index
        page_index: usize,
        /// Panic message or reason
        message: String,
    },

    /// Generic extraction failure with context
    ExtractionFailed {
        /// Page index
        page_index: usize,
        /// Detailed error message
        message: String,
    },
}

impl fmt::Display for PageExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPagesInDocument => {
                write!(f, "Document contains no pages")
            }
            Self::IndexOutOfBounds { requested, available } => {
                write!(
                    f,
                    "Page index {} out of bounds (document has {} pages; valid indices: 0-{})",
                    requested,
                    available,
                    available.saturating_sub(1)
                )
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
            Self::ContentStreamTooLarge { page_index, size_bytes, max_bytes } => {
                write!(
                    f,
                    "Page {} content stream too large: {} bytes (maximum: {} bytes)",
                    page_index, size_bytes, max_bytes
                )
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
            Self::GlyphExtractionFailed { page_index, message } => {
                write!(f, "Glyph extraction failed for page {}: {}", page_index, message)
            }
            Self::SpanMergeFailed { page_index, glyph_count, message } => {
                write!(
                    f,
                    "Span merge failed for page {} ({} glyphs): {}",
                    page_index, glyph_count, message
                )
            }
            Self::LayoutAnalysisFailed { page_index, stage, message } => {
                write!(
                    f,
                    "Layout analysis failed for page {} at stage '{}': {}",
                    page_index, stage, message
                )
            }
            Self::TableDetectionFailed { page_index, message } => {
                write!(f, "Table detection failed for page {}: {}", page_index, message)
            }
            Self::ReceiptGenerationFailed { page_index, message } => {
                write!(f, "Receipt generation failed for page {}: {}", page_index, message)
            }
            Self::MalformedPageData { page_index, message } => {
                write!(f, "Page {} has malformed data: {}", page_index, message)
            }
            Self::MalformedDocumentStructure(msg) => {
                write!(f, "Document has malformed structure: {}", msg)
            }
            Self::ExtractionPanicked { page_index, message } => {
                write!(f, "Page {} extraction panicked: {}", page_index, message)
            }
            Self::ExtractionFailed { page_index, message } => {
                write!(f, "Failed to extract page {}: {}", page_index, message)
            }
        }
    }
}

impl std::error::Error for PageExtractionError {}

// Note: From<PageExtractionError> for anyhow::Error is provided by anyhow's blanket implementation
// since PageExtractionError implements std::error::Error. We don't need a custom implementation.

/// Result type alias for Page extraction operations.
pub type PageResult<T> = std::result::Result<T, PageExtractionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_no_pages() {
        let err = PageExtractionError::NoPagesInDocument;
        assert_eq!(err.to_string(), "Document contains no pages");
    }

    #[test]
    fn test_display_index_out_of_bounds() {
        let err = PageExtractionError::IndexOutOfBounds {
            requested: 10,
            available: 5,
        };
        let msg = err.to_string();
        assert!(msg.contains("Page index 10 out of bounds"));
        assert!(msg.contains("document has 5 pages"));
        assert!(msg.contains("valid indices: 0-4"));
    }

    #[test]
    fn test_display_invalid_media_box() {
        let err = PageExtractionError::InvalidMediaBox {
            page_index: 0,
            media_box: Some([0.0, 0.0, -1.0, 792.0]),
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 0 has invalid media box"));
        assert!(msg.contains("must have x1 > x0"));
    }

    #[test]
    fn test_display_invalid_dimensions() {
        let err = PageExtractionError::InvalidDimensions {
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
        let err = PageExtractionError::InvalidRotation {
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
        let err = PageExtractionError::ContentStreamDecodeFailed {
            page_index: 3,
            message: "Invalid FlateDecode stream".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to decode content stream for page 3"));
        assert!(msg.contains("Invalid FlateDecode stream"));
    }

    #[test]
    fn test_display_missing_content_stream() {
        let err = PageExtractionError::MissingContentStream { page_index: 4 };
        assert_eq!(
            err.to_string(),
            "Page 4 has no content stream (empty or missing)"
        );
    }

    #[test]
    fn test_display_content_stream_too_large() {
        let err = PageExtractionError::ContentStreamTooLarge {
            page_index: 5,
            size_bytes: 500_000_000,
            max_bytes: 100_000_000,
        };
        let msg = err.to_string();
        assert!(msg.contains("Page 5 content stream too large"));
        assert!(msg.contains("500000000 bytes"));
        assert!(msg.contains("maximum: 100000000 bytes"));
    }

    #[test]
    fn test_display_glyph_extraction_failed() {
        let err = PageExtractionError::GlyphExtractionFailed {
            page_index: 6,
            message: "Font encoding not supported".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Glyph extraction failed for page 6"));
        assert!(msg.contains("Font encoding not supported"));
    }

    #[test]
    fn test_display_span_merge_failed() {
        let err = PageExtractionError::SpanMergeFailed {
            page_index: 7,
            glyph_count: 1000,
            message: "Inconsistent font sizes".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Span merge failed for page 7"));
        assert!(msg.contains("1000 glyphs"));
        assert!(msg.contains("Inconsistent font sizes"));
    }

    #[test]
    fn test_display_layout_analysis_failed() {
        let err = PageExtractionError::LayoutAnalysisFailed {
            page_index: 8,
            stage: "XY-cut".to_string(),
            message: "Invalid block geometry".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Layout analysis failed for page 8"));
        assert!(msg.contains("at stage 'XY-cut'"));
        assert!(msg.contains("Invalid block geometry"));
    }

    #[test]
    fn test_error_implements_send_and_sync() {
        // Ensure error type can be sent across threads
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<PageExtractionError>();
        assert_sync::<PageExtractionError>();
    }

    #[test]
    fn test_error_clone() {
        let err1 = PageExtractionError::IndexOutOfBounds {
            requested: 5,
            available: 3,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_conversion_to_anyhow() {
        let page_err = PageExtractionError::NoPagesInDocument;
        let anyhow_err = anyhow::Error::from(page_err.clone());
        assert!(anyhow_err.to_string().contains("Document contains no pages"));
    }
}
