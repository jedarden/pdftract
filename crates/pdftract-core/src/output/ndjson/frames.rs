//! NDJSON frame types for streaming extraction.
//!
//! Defines the three frame types emitted during streaming extraction:
//! - HeaderFrame: Document metadata and outline (emitted first)
//! - PageFrame: Single page extraction result (emitted as pages complete)
//! - FooterFrame: Aggregated quality metrics and diagnostics (emitted last)

use crate::schema::{BlockJson, ExtractionQuality, SpanJson, TableJson};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Frame discriminator field.
///
/// All NDJSON frames include a "frame" field that identifies the frame type.
/// This allows consumers to parse each line and dispatch to the appropriate handler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FrameType {
    /// Header frame containing document metadata.
    Header,
    /// Page frame containing a single page's extraction result.
    Page,
    /// Footer frame containing aggregated metrics and diagnostics.
    Footer,
}

/// Header frame emitted at the start of streaming extraction.
///
/// Contains document-level metadata that is known before page processing begins.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeaderFrame {
    /// Frame discriminator (always "header").
    #[serde(rename = "frame")]
    pub frame_type: FrameType,

    /// Schema version identifier.
    ///
    /// Consumers should check this field to ensure compatibility.
    /// Current version is "1.0".
    pub schema_version: String,

    /// Document metadata.
    ///
    /// Includes title, author, creation date, page count, etc.
    pub metadata: Value,

    /// Document outline (table of contents).
    ///
    /// Null if the document has no outline.
    pub outline: Option<Value>,

    /// Total number of pages in the document.
    ///
    /// Consumers can use this to pre-allocate or show progress.
    pub total_pages: usize,
}

impl HeaderFrame {
    /// Create a new header frame.
    pub fn new(
        schema_version: String,
        metadata: Value,
        outline: Option<Value>,
        total_pages: usize,
    ) -> Self {
        Self {
            frame_type: FrameType::Header,
            schema_version,
            metadata,
            outline,
            total_pages,
        }
    }

    /// Serialize this frame to a JSON string with a trailing newline.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        Ok(s)
    }
}

/// Page frame emitted as each page completes extraction.
///
/// Pages may be emitted out-of-order by rayon, but are buffered
/// and output in page_index order by the streaming pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageFrame {
    /// Frame discriminator (always "page").
    #[serde(rename = "frame")]
    pub frame_type: FrameType,

    /// Zero-based page index.
    ///
    /// Consumers use this to reorder pages if processing concurrently.
    pub page_index: usize,

    /// Page type classification.
    ///
    /// Values include "content", "blank", "figure_only", etc.
    pub page_type: String,

    /// Extracted text spans in reading order.
    ///
    /// Empty array for pages with no extractable text.
    pub spans: Vec<SpanJson>,

    /// Structural blocks (paragraphs, headings, lists, tables).
    ///
    /// Empty array for pages with no structural blocks.
    pub blocks: Vec<BlockJson>,

    /// Tables detected on this page.
    ///
    /// Empty array for pages with no tables.
    pub tables: Vec<TableJson>,

    /// Annotations (highlights, stamps, notes, links).
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Value>,

    /// Optional page-level diagnostics.
    ///
    /// Present only if there were errors or warnings during extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<Value>>,
}

impl PageFrame {
    /// Create a new page frame.
    pub fn new(
        page_index: usize,
        page_type: String,
        spans: Vec<SpanJson>,
        blocks: Vec<BlockJson>,
        tables: Vec<TableJson>,
    ) -> Self {
        Self {
            frame_type: FrameType::Page,
            page_index,
            page_type,
            spans,
            blocks,
            tables,
            annotations: Vec::new(),
            errors: None,
        }
    }

    /// Set page-level diagnostics.
    pub fn with_errors(mut self, errors: Vec<Value>) -> Self {
        self.errors = Some(errors);
        self
    }

    /// Serialize this frame to a JSON string with a trailing newline.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        Ok(s)
    }
}

/// Footer frame emitted at the end of streaming extraction.
///
/// Contains aggregated metrics and diagnostics that are only
/// known after all pages have been processed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FooterFrame {
    /// Frame discriminator (always "footer").
    #[serde(rename = "frame")]
    pub frame_type: FrameType,

    /// Aggregate extraction quality metrics.
    ///
    /// Includes overall quality, confidence statistics, OCR fraction, etc.
    pub extraction_quality: ExtractionQuality,

    /// All diagnostics collected during extraction.
    ///
    /// Includes errors and warnings from all pages.
    pub errors: Vec<Value>,

    /// Thread information (for debugging and profiling).
    ///
    /// Empty in the initial implementation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<Value>,

    /// Attachments extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Value>,

    /// Digital signatures extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<Value>,

    /// Form fields extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub form_fields: Vec<Value>,

    /// Links extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Value>,
}

impl FooterFrame {
    /// Create a new footer frame.
    pub fn new(extraction_quality: ExtractionQuality, errors: Vec<Value>) -> Self {
        Self {
            frame_type: FrameType::Footer,
            extraction_quality,
            errors,
            threads: Vec::new(),
            attachments: Vec::new(),
            signatures: Vec::new(),
            form_fields: Vec::new(),
            links: Vec::new(),
        }
    }

    /// Serialize this frame to a JSON string with a trailing newline.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_frame_serialization() {
        let header = HeaderFrame::new(
            "1.0".to_string(),
            serde_json::json!({"title": "Test", "author": "Test Author"}),
            Some(serde_json::json!([{"title": "Chapter 1", "level": 1}])),
            10,
        );

        let json = header.to_json_line().unwrap();
        assert!(json.contains("\"frame\":\"header\""));
        assert!(json.contains("\"schema_version\":\"1.0\""));
        assert!(json.contains("\"total_pages\":10"));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn test_page_frame_serialization() {
        let page = PageFrame::new(
            0,
            "content".to_string(),
            vec![SpanJson {
                text: "Hello".to_string(),
                bbox: [0.0, 0.0, 100.0, 20.0],
                font: "Helvetica".to_string(),
                size: 12.0,
                confidence: None,
                receipt: None,
                column: None,
            }],
            vec![],
            vec![],
        );

        let json = page.to_json_line().unwrap();
        assert!(json.contains("\"frame\":\"page\""));
        assert!(json.contains("\"page_index\":0"));
        assert!(json.contains("\"page_type\":\"content\""));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn test_footer_frame_serialization() {
        let footer = FooterFrame::new(ExtractionQuality::new().with_quality("high"), vec![]);

        let json = footer.to_json_line().unwrap();
        assert!(json.contains("\"frame\":\"footer\""));
        assert!(json.contains("\"overall_quality\":\"high\""));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn test_page_frame_with_empty_collections() {
        let page = PageFrame::new(5, "blank".to_string(), vec![], vec![], vec![]);

        let json = page.to_json_line().unwrap();
        // Empty spans/blocks/tables should still be present
        assert!(json.contains("\"spans\":[]"));
        assert!(json.contains("\"blocks\":[]"));
        assert!(json.contains("\"tables\":[]"));
        // annotations should not appear when empty
        assert!(!json.contains("\"annotations\""));
    }
}
