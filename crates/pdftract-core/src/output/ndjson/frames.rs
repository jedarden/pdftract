//! NDJSON frame types for streaming extraction.
//!
//! Defines the three frame types emitted during streaming extraction:
//! - HeaderFrame: Document metadata and outline (emitted first)
//! - PageFrame: Single page extraction result (emitted as pages complete)
//! - FooterFrame: Aggregated quality metrics and diagnostics (emitted last)

use crate::schema::{BlockJson, ExtractionQuality, SpanJson, TableJson};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;

/// Unified NDJSON frame enum with internal-tag discriminator.
///
/// This enum uses serde's internal tagging with the "frame" field as the tag.
/// When serialized, the "frame" field appears first with values "header", "page",
/// or "footer", allowing consumers to dispatch to the appropriate handler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "frame", rename_all = "lowercase")]
pub enum NdjsonFrame {
    /// Header frame containing document metadata.
    Header(HeaderFrame),
    /// Page frame containing a single page's extraction result.
    Page(PageFrame),
    /// Footer frame containing aggregated metrics and diagnostics.
    Footer(FooterFrame),
}

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<Value>,

    /// Optional page-level diagnostics.
    ///
    /// Present only if there were errors or warnings during extraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<Value>,

    /// Attachments extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Value>,

    /// Digital signatures extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<Value>,

    /// Form fields extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_fields: Vec<Value>,

    /// Links extracted from the document.
    ///
    /// Empty in Phase 6; populated in Phase 7.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Value>,
}

impl FooterFrame {
    /// Create a new footer frame.
    pub fn new(extraction_quality: ExtractionQuality, errors: Vec<Value>) -> Self {
        Self {
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

/// Write a single frame to a writer as a JSON line with trailing newline and flush.
///
/// This is the primary function for emitting NDJSON frames during streaming extraction.
/// It serializes the frame, appends a newline, writes it to the writer, and flushes
/// to ensure immediate delivery to streaming consumers.
///
/// # Arguments
///
/// * `writer` - Any writer implementing `Write` (e.g., `File`, `BufWriter`, `Stdout`)
/// * `frame` - The frame to write (wrapped in `NdjsonFrame` enum)
///
/// # Returns
///
/// * `Ok(())` if the frame was written and flushed successfully
/// * `Err(io::Error)` if serialization or writing failed
///
/// # Example
///
/// ```ignore
/// use std::io::BufWriter;
/// use pdftract_core::output::ndjson::frames::{write_frame, NdjsonFrame, HeaderFrame};
///
/// let mut writer = BufWriter::new(file);
/// let header = HeaderFrame::new(...);
/// write_frame(&mut writer, &NdjsonFrame::Header(header))?;
/// ```
pub fn write_frame<W: Write>(writer: &mut W, frame: &NdjsonFrame) -> std::io::Result<()> {
    // Serialize the frame to JSON
    let json_string = serde_json::to_string(frame)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Write the JSON line with trailing newline
    writer.write_all(json_string.as_bytes())?;
    writer.write_all(b"\n")?;

    // Flush to ensure immediate delivery to streaming consumers
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_ndjson_frame_header_discriminator() {
        let header = HeaderFrame::new(
            "1.0".to_string(),
            serde_json::json!({"title": "Test", "author": "Test Author"}),
            Some(serde_json::json!([{"title": "Chapter 1", "level": 1}])),
            10,
        );
        let frame = NdjsonFrame::Header(header);

        let json = serde_json::to_string(&frame).unwrap();
        // The "frame" key should appear first (serde internal tag)
        assert!(json.starts_with("{\"frame\":\"header\""));
        assert!(json.contains("\"schema_version\":\"1.0\""));
        assert!(json.contains("\"total_pages\":10"));
    }

    #[test]
    fn test_ndjson_frame_page_discriminator() {
        let page = PageFrame::new(
            0,
            "content".to_string(),
            vec![SpanJson {
                text: "Hello".to_string(),
                bbox: [0.0, 0.0, 100.0, 20.0],
                font: "Helvetica".to_string(),
                size: 12.0,
                color: None,
                rendering_mode: None,
                confidence: None,
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            }],
            vec![],
            vec![],
        );
        let frame = NdjsonFrame::Page(page);

        let json = serde_json::to_string(&frame).unwrap();
        // The "frame" key should appear first
        assert!(json.starts_with("{\"frame\":\"page\""));
        assert!(json.contains("\"page_index\":0"));
        assert!(json.contains("\"page_type\":\"content\""));
    }

    #[test]
    fn test_ndjson_frame_footer_discriminator() {
        let footer = FooterFrame::new(ExtractionQuality::new().with_quality("high"), vec![]);
        let frame = NdjsonFrame::Footer(footer);

        let json = serde_json::to_string(&frame).unwrap();
        // The "frame" key should appear first
        assert!(json.starts_with("{\"frame\":\"footer\""));
        assert!(json.contains("\"overall_quality\":\"high\""));
    }

    #[test]
    fn test_write_frame_includes_newline_and_flush() {
        let header = HeaderFrame::new(
            "1.0".to_string(),
            serde_json::json!({"title": "Test"}),
            None,
            1,
        );
        let frame = NdjsonFrame::Header(header);

        let mut buffer = Vec::new();
        write_frame(&mut buffer, &frame).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        // Should end with newline
        assert!(output.ends_with('\n'));
        // Should contain the frame discriminator
        assert!(output.contains("\"frame\":\"header\""));
    }

    #[test]
    fn test_roundtrip_header_frame() {
        let original = HeaderFrame::new(
            "1.0".to_string(),
            serde_json::json!({"title": "Test", "author": "Test Author"}),
            Some(serde_json::json!([{"title": "Chapter 1", "level": 1}])),
            10,
        );
        let frame = NdjsonFrame::Header(original.clone());

        // Serialize
        let json = serde_json::to_string(&frame).unwrap();

        // Deserialize
        let deserialized: NdjsonFrame = serde_json::from_str(&json).unwrap();

        // Verify equality
        assert_eq!(frame, deserialized);

        // Extract and verify the inner HeaderFrame
        match deserialized {
            NdjsonFrame::Header(header) => {
                assert_eq!(header.schema_version, original.schema_version);
                assert_eq!(header.metadata, original.metadata);
                assert_eq!(header.outline, original.outline);
                assert_eq!(header.total_pages, original.total_pages);
            }
            _ => panic!("Expected Header frame"),
        }
    }

    #[test]
    fn test_roundtrip_page_frame() {
        let original = PageFrame::new(
            0,
            "content".to_string(),
            vec![SpanJson {
                text: "Hello".to_string(),
                bbox: [0.0, 0.0, 100.0, 20.0],
                font: "Helvetica".to_string(),
                size: 12.0,
                color: None,
                rendering_mode: None,
                confidence: None,
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            }],
            vec![],
            vec![],
        );
        let frame = NdjsonFrame::Page(original.clone());

        // Serialize
        let json = serde_json::to_string(&frame).unwrap();

        // Deserialize
        let deserialized: NdjsonFrame = serde_json::from_str(&json).unwrap();

        // Verify equality
        assert_eq!(frame, deserialized);
    }

    #[test]
    fn test_roundtrip_footer_frame() {
        let original = FooterFrame::new(ExtractionQuality::new().with_quality("high"), vec![]);
        let frame = NdjsonFrame::Footer(original.clone());

        // Serialize
        let json = serde_json::to_string(&frame).unwrap();

        // Deserialize
        let deserialized: NdjsonFrame = serde_json::from_str(&json).unwrap();

        // Verify equality
        assert_eq!(frame, deserialized);
    }

    #[test]
    fn test_page_frame_with_empty_collections() {
        let page = PageFrame::new(5, "blank".to_string(), vec![], vec![], vec![]);
        let frame = NdjsonFrame::Page(page);

        let json = serde_json::to_string(&frame).unwrap();
        // Empty spans/blocks/tables should still be present
        assert!(json.contains("\"spans\":[]"));
        assert!(json.contains("\"blocks\":[]"));
        assert!(json.contains("\"tables\":[]"));
        // annotations should not appear when empty
        assert!(!json.contains("\"annotations\""));
    }
}
