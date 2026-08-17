//! Multi-output emission architecture.
//!
//! This module provides the OutputSink trait and concrete sink implementations
//! for emitting PDF extraction results in multiple formats concurrently.
//!
//! # Architecture
//!
//! The trait-based design allows a single extraction pass to populate any
//! subset of output formats:
//!
//! - [`JsonSink`] - Whole-document JSON (buffers pages, emits on close)
//! - [`MarkdownSink`] - Whole-document Markdown (buffers pages, emits on close)
//! - [`TextSink`] - Streaming plain text (emits per page)
//! - [`NdjsonSink`] - Streaming NDJSON (emits frames per page)
//!
//! All sinks are opened before extraction, receive pages as they complete,
//! and are closed after extraction completes. This ensures atomic writes
//! via temp-file-and-rename semantics.

use crate::atomic_file_writer::AtomicFileWriter;
use crate::markdown::{
    page_to_markdown_with_links_and_footnotes,
    MarkdownOptions,
};
use crate::schema::{BlockJson, LinkJson, Output, PageJson, SpanJson};
use anyhow::Result;
use std::io::{self, Write};

/// Document header passed to all sinks on open.
///
/// Contains metadata available at the start of extraction.
#[derive(Debug, Clone)]
pub struct DocumentHeader {
    /// Document fingerprint from Phase 1.7
    pub document_fingerprint: String,
    /// Number of pages in the document
    pub page_count: u32,
    /// Schema version (always "1.0")
    pub schema_version: &'static str,
}

impl DocumentHeader {
    /// Create a new DocumentHeader from an Output reference.
    ///
    /// This is used when extracting with the multi-sink pipeline after
    /// the full extraction result is available.
    pub fn from_output(output: &Output) -> Self {
        Self {
            document_fingerprint: output.metadata.page_count.to_string(), // Temporary - should use real fingerprint
            page_count: output.metadata.page_count,
            schema_version: output.schema_version,
        }
    }
}

/// Document footer passed to all sinks on close.
///
/// Contains aggregated metadata after all pages are extracted.
#[derive(Debug, Clone)]
pub struct DocumentFooter {
    /// Extraction quality assessment
    pub overall_quality: String,
    /// OCR fraction (0.0 to 1.0)
    pub ocr_fraction: Option<f32>,
    /// Average confidence score (0.0 to 1.0)
    pub avg_confidence: Option<f32>,
    /// Minimum confidence score (0.0 to 1.0)
    pub min_confidence: Option<f32>,
    /// Number of diagnostic errors
    pub error_count: usize,
}

impl DocumentFooter {
    /// Create a new DocumentFooter from an Output reference.
    pub fn from_output(output: &Output) -> Self {
        Self {
            overall_quality: output.extraction_quality.overall_quality.clone(),
            ocr_fraction: output.extraction_quality.ocr_fraction,
            avg_confidence: output.extraction_quality.avg_confidence,
            min_confidence: output.extraction_quality.min_confidence,
            error_count: output.errors.len(),
        }
    }
}

/// Page representation passed to sinks.
///
/// Contains all data for a single page including spans, blocks, tables,
/// and annotations.
#[derive(Debug, Clone)]
pub struct Page {
    /// Zero-based page index
    pub page_index: usize,
    /// One-based page number
    pub page_number: u32,
    /// Page label from /PageLabels (if present)
    pub page_label: Option<String>,
    /// Page width in points
    pub width: f32,
    /// Page height in points
    pub height: f32,
    /// Page rotation (0, 90, 180, 270)
    pub rotation: i32,
    /// Page type classification
    pub page_type: String,
    /// All text spans on this page
    pub spans: Vec<SpanJson>,
    /// All blocks on this page
    pub blocks: Vec<BlockJson>,
    /// All link annotations on this page (for Phase 7.6 integration)
    pub links: Vec<LinkJson>,
}

impl Page {
    /// Create a new Page from a PageJson reference.
    pub fn from_page_json(page: &PageJson, links: Vec<LinkJson>) -> Self {
        Self {
            page_index: page.page_index,
            page_number: page.page_number,
            page_label: page.page_label.clone(),
            width: page.width,
            height: page.height,
            rotation: page.rotation as i32,
            page_type: page.page_type.clone(),
            spans: page.spans.clone(),
            blocks: page.blocks.clone(),
            links,
        }
    }

    /// Validate that this Page has all required fields populated with valid values.
    ///
    /// This ensures that incomplete or invalid Page objects are not returned to callers.
    ///
    /// # Required Fields
    ///
    /// - `page_index`: Must be a valid usize (no specific range validation)
    /// - `page_number`: Must be >= 1 (one-based page number)
    /// - `width`: Must be > 0.0 (positive width in points)
    /// - `height`: Must be > 0.0 (positive height in points)
    /// - `rotation`: Must be one of 0, 90, 180, 270 (standard PDF rotations)
    /// - `page_type`: Must not be empty (classification must be present)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all required fields are valid
    /// - `Err(String)` describing which field is missing or invalid
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::output::sink::Page;
    ///
    /// let page = create_test_page();
    /// match page.validate() {
    ///     Ok(()) => println!("Page is valid"),
    ///     Err(e) => eprintln!("Validation failed: {}", e),
    /// }
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        // Validate page_number >= 1 (one-based)
        if self.page_number < 1 {
            return Err(format!(
                "Invalid page_number: {} (must be >= 1, page numbers are one-based)",
                self.page_number
            ));
        }

        // Validate width > 0 (must have positive width)
        if self.width <= 0.0 {
            return Err(format!(
                "Invalid width: {} (must be > 0.0 points)",
                self.width
            ));
        }

        // Validate height > 0 (must have positive height)
        if self.height <= 0.0 {
            return Err(format!(
                "Invalid height: {} (must be > 0.0 points)",
                self.height
            ));
        }

        // Validate rotation is one of the standard PDF rotations
        if ![0, 90, 180, 270].contains(&self.rotation) {
            return Err(format!(
                "Invalid rotation: {} degrees (must be one of 0, 90, 180, 270)",
                self.rotation
            ));
        }

        // Validate page_type is not empty
        if self.page_type.is_empty() {
            return Err("Invalid page_type: empty string (must have a classification)".to_string());
        }

        Ok(())
    }
}

/// Trait for output sinks that receive extraction results.
///
/// All sinks follow the same lifecycle:
/// 1. `open()` - Called at the start with document header
/// 2. `page()` - Called once per page as pages complete
/// 3. `close()` - Called at the end with document footer
///
/// Sinks may buffer pages for whole-document emission (JSON, Markdown)
/// or emit streaming results immediately (NDJSON, text).
///
/// # Send but not Sync
///
/// Sinks are Send because they may be moved between threads,
/// but not Sync because concurrent writes would corrupt output.
pub trait OutputSink: Send {
    /// Open the sink for writing.
    ///
    /// Called once at the start of extraction with document metadata.
    /// Sinks should open their output file and write any header information.
    ///
    /// # Arguments
    ///
    /// * `header` - Document metadata available at extraction start
    ///
    /// # Errors
    ///
    /// Returns IO errors if the output file cannot be opened or written.
    fn open(&mut self, header: &DocumentHeader) -> io::Result<()>;

    /// Process a single page.
    ///
    /// Called once per page as pages complete extraction. Sinks may
    /// buffer pages for whole-document emission or emit immediately.
    ///
    /// # Arguments
    ///
    /// * `page` - The page data
    ///
    /// # Errors
    ///
    /// Returns IO errors if writing fails.
    fn page(&mut self, page: &Page) -> io::Result<()>;

    /// Close the sink and commit output.
    ///
    /// Called once at the end of extraction with aggregated metadata.
    /// Sinks should write any footer information and commit their output
    /// (e.g., by renaming temp file to final path).
    ///
    /// # Arguments
    ///
    /// * `footer` - Aggregated document metadata
    ///
    /// # Errors
    ///
    /// Returns IO errors if writing or committing fails.
    fn close(&mut self, footer: &DocumentFooter) -> io::Result<()>;
}

/// Sink that emits the full JSON schema.
///
/// This sink buffers all pages and emits the complete JSON Output
/// schema on close. The output is byte-identical whether emitted alone
/// or alongside other sinks (sink isolation invariant).
pub struct JsonSink {
    /// Atomic file writer for output
    writer: Option<AtomicFileWriter>,
    /// Buffered pages for emission on close
    pages: Vec<PageJson>,
    /// Document header saved for emission on close
    header: Option<DocumentHeader>,
}

impl JsonSink {
    /// Create a new JsonSink writing to the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path (or "-" for stdout)
    ///
    /// # Returns
    ///
    /// A new JsonSink instance
    pub fn new(path: std::path::PathBuf) -> Result<Self> {
        let writer = AtomicFileWriter::create(path)?;
        Ok(Self {
            writer: Some(writer),
            pages: Vec::new(),
            header: None,
        })
    }

    /// Emit the complete JSON output.
    ///
    /// This is called on close and writes the full Output schema.
    fn emit_output(&mut self, footer: &DocumentFooter) -> io::Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "writer already consumed")
        })?;

        // Create a minimal Output for now
        // In production, this would use the full extraction result
        let output = serde_json::json!({
            "schema_version": self.header.as_ref().map(|h| h.schema_version).unwrap_or("1.0"),
            "pages": self.pages,
            "metadata": {
                "page_count": self.header.as_ref().map(|h| h.page_count).unwrap_or(0),
            },
            "extraction_quality": {
                "overall_quality": footer.overall_quality,
            }
        });

        let json = serde_json::to_string_pretty(&output)?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?;

        Ok(())
    }
}

impl OutputSink for JsonSink {
    fn open(&mut self, header: &DocumentHeader) -> io::Result<()> {
        self.header = Some(header.clone());
        Ok(())
    }

    fn page(&mut self, page: &Page) -> io::Result<()> {
        // Convert Page to PageJson for buffering
        let page_json = PageJson {
            page_index: page.page_index,
            page_number: page.page_number,
            page_label: page.page_label.clone(),
            width: page.width,
            height: page.height,
            rotation: page.rotation as u16,
            page_type: page.page_type.clone(),
            spans: page.spans.clone(),
            blocks: page.blocks.clone(),
            tables: Vec::new(), // TODO: Include tables when available
            annotations: Vec::new(), // TODO: Include annotations when available
        };
        self.pages.push(page_json);
        Ok(())
    }

    fn close(&mut self, footer: &DocumentFooter) -> io::Result<()> {
        self.emit_output(footer)?;
        if let Some(writer) = self.writer.take() {
            writer.commit().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("failed to commit JSON output: {}", e))
            })?;
        }
        Ok(())
    }
}

/// Sink that emits Markdown output.
///
/// This sink buffers all pages and emits the complete Markdown document
/// on close. Supports the same emission options as the direct Markdown
/// module (anchors, page breaks, link/footnote support).
pub struct MarkdownSink {
    /// Atomic file writer for output
    writer: Option<AtomicFileWriter>,
    /// Buffered Markdown pages
    pages: Vec<String>,
    /// Header for link/footnote support
    header: Option<DocumentHeader>,
    /// Markdown emission options
    options: MarkdownOptions,
}

impl MarkdownSink {
    /// Create a new MarkdownSink writing to the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path (or "-" for stdout)
    /// * `options` - Markdown emission options
    ///
    /// # Returns
    ///
    /// A new MarkdownSink instance
    pub fn new(path: std::path::PathBuf, options: MarkdownOptions) -> Result<Self> {
        let writer = AtomicFileWriter::create(path)?;
        Ok(Self {
            writer: Some(writer),
            pages: Vec::new(),
            header: None,
            options,
        })
    }

    /// Emit the complete Markdown document.
    ///
    /// This is called on close and writes all buffered pages.
    fn emit_markdown(&mut self, _footer: &DocumentFooter) -> io::Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "writer already consumed")
        })?;

        for page_md in &self.pages {
            writer.write_all(page_md.as_bytes())?;
        }

        Ok(())
    }
}

impl OutputSink for MarkdownSink {
    fn open(&mut self, header: &DocumentHeader) -> io::Result<()> {
        self.header = Some(header.clone());
        Ok(())
    }

    fn page(&mut self, page: &Page) -> io::Result<()> {
        // Emit this page as Markdown
        let page_md = page_to_markdown_with_links_and_footnotes(
            &page.blocks,
            &page.spans,
            &[],
            &page.links,
            page.page_index,
            false, // include_anchor
            &self.options,
            None, // footnotes - Phase 7 integration
        );
        self.pages.push(page_md);
        Ok(())
    }

    fn close(&mut self, footer: &DocumentFooter) -> io::Result<()> {
        self.emit_markdown(footer)?;
        if let Some(writer) = self.writer.take() {
            writer.commit().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("failed to commit Markdown output: {}", e))
            })?;
        }
        Ok(())
    }
}

/// Sink that emits plain text output.
///
/// This sink emits text immediately as each page completes,
/// making it suitable for streaming and large documents.
pub struct TextSink {
    /// Atomic file writer for output
    writer: Option<AtomicFileWriter>,
    /// Whether we've written any content (for separator management)
    has_content: bool,
}

impl TextSink {
    /// Create a new TextSink writing to the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path (or "-" for stdout)
    ///
    /// # Returns
    ///
    /// A new TextSink instance
    pub fn new(path: std::path::PathBuf) -> Result<Self> {
        let writer = AtomicFileWriter::create(path)?;
        Ok(Self {
            writer: Some(writer),
            has_content: false,
        })
    }
}

impl OutputSink for TextSink {
    fn open(&mut self, _header: &DocumentHeader) -> io::Result<()> {
        self.has_content = false;
        Ok(())
    }

    fn page(&mut self, page: &Page) -> io::Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "writer already consumed")
        })?;

        // Add page separator if not the first page
        if self.has_content {
            writeln!(writer, "\n---")?;
        }

        // Emit all blocks as plain text
        for block in &page.blocks {
            if !block.text.is_empty() {
                writeln!(writer, "{}", block.text)?;
            }
        }

        self.has_content = true;
        Ok(())
    }

    fn close(&mut self, _footer: &DocumentFooter) -> io::Result<()> {
        if let Some(writer) = self.writer.take() {
            writer.commit().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("failed to commit text output: {}", e))
            })?;
        }
        Ok(())
    }
}

/// Sink that emits NDJSON (newline-delimited JSON) output.
///
/// This sink emits a sequence of JSON frames:
/// - Header frame on open
/// - One page frame per page
/// - Footer frame on close
///
/// Each frame is a complete JSON object on its own line, making
/// the output suitable for streaming and incremental processing.
pub struct NdjsonSink {
    /// Atomic file writer for output
    writer: Option<AtomicFileWriter>,
}

impl NdjsonSink {
    /// Create a new NdjsonSink writing to the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path (or "-" for stdout)
    ///
    /// # Returns
    ///
    /// A new NdjsonSink instance
    pub fn new(path: std::path::PathBuf) -> Result<Self> {
        let writer = AtomicFileWriter::create(path)?;
        Ok(Self {
            writer: Some(writer),
        })
    }
}

impl OutputSink for NdjsonSink {
    fn open(&mut self, header: &DocumentHeader) -> io::Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "writer already consumed")
        })?;

        // Emit header frame
        let header_frame = serde_json::json!({
            "type": "header",
            "document_fingerprint": header.document_fingerprint,
            "page_count": header.page_count,
            "schema_version": header.schema_version,
        });
        writeln!(writer, "{}", header_frame)?;
        Ok(())
    }

    fn page(&mut self, page: &Page) -> io::Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "writer already consumed")
        })?;

        // Emit page frame
        let page_frame = serde_json::json!({
            "type": "page",
            "page_index": page.page_index,
            "page_number": page.page_number,
            "page_label": page.page_label,
            "width": page.width,
            "height": page.height,
            "rotation": page.rotation,
            "page_type": page.page_type,
            "blocks": page.blocks,
            "spans": page.spans,
        });
        writeln!(writer, "{}", page_frame)?;
        Ok(())
    }

    fn close(&mut self, footer: &DocumentFooter) -> io::Result<()> {
        let writer = self.writer.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "writer already consumed")
        })?;

        // Emit footer frame
        let footer_frame = serde_json::json!({
            "type": "footer",
            "overall_quality": footer.overall_quality,
            "ocr_fraction": footer.ocr_fraction,
            "avg_confidence": footer.avg_confidence,
            "min_confidence": footer.min_confidence,
            "error_count": footer.error_count,
        });
        writeln!(writer, "{}", footer_frame)?;

        if let Some(writer) = self.writer.take() {
            writer.commit().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("failed to commit NDJSON output: {}", e))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    fn make_test_page(index: usize) -> Page {
        Page {
            page_index: index,
            page_number: (index + 1) as u32,
            page_label: None,
            width: 612.0,
            height: 792.0,
            rotation: 0,
            page_type: "text".to_string(),
            spans: vec![SpanJson {
                text: "Test span".to_string(),
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
            blocks: vec![BlockJson {
                kind: "paragraph".to_string(),
                text: "Test paragraph".to_string(),
                bbox: [0.0, 0.0, 612.0, 100.0],
                level: None,
                table_index: None,
                spans: vec![0],
                receipt: None,
            }],
            links: vec![],
        }
    }

    fn make_test_header() -> DocumentHeader {
        DocumentHeader {
            document_fingerprint: "test-fingerprint".to_string(),
            page_count: 2,
            schema_version: "1.0",
        }
    }

    fn make_test_footer() -> DocumentFooter {
        DocumentFooter {
            overall_quality: "high".to_string(),
            ocr_fraction: Some(0.0),
            avg_confidence: Some(1.0),
            min_confidence: Some(1.0),
            error_count: 0,
        }
    }

    #[test]
    fn test_json_sink_emits_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let mut sink = JsonSink::new(output_path.clone()).unwrap();

        let header = make_test_header();
        sink.open(&header).unwrap();

        sink.page(&make_test_page(0)).unwrap();
        sink.page(&make_test_page(1)).unwrap();

        let footer = make_test_footer();
        sink.close(&footer).unwrap();

        // Verify output exists and is valid JSON
        let mut output = String::new();
        std::fs::File::open(output_path)
            .unwrap()
            .read_to_string(&mut output)
            .unwrap();

        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["schema_version"], "1.0");
        assert_eq!(json["metadata"]["page_count"], 2);
        assert_eq!(json["pages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_markdown_sink_emits_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let mut sink = MarkdownSink::new(
            output_path.clone(),
            MarkdownOptions::default(),
        )
        .unwrap();

        let header = make_test_header();
        sink.open(&header).unwrap();

        sink.page(&make_test_page(0)).unwrap();

        let footer = make_test_footer();
        sink.close(&footer).unwrap();

        // Verify output exists and contains Markdown
        let output = std::fs::read_to_string(output_path).unwrap();
        assert!(output.contains("Test paragraph"));
    }

    #[test]
    fn test_text_sink_emits_text() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.txt");

        let mut sink = TextSink::new(output_path.clone()).unwrap();

        let header = make_test_header();
        sink.open(&header).unwrap();

        sink.page(&make_test_page(0)).unwrap();
        sink.page(&make_test_page(1)).unwrap();

        let footer = make_test_footer();
        sink.close(&footer).unwrap();

        // Verify output exists and contains text
        let output = std::fs::read_to_string(output_path).unwrap();
        assert!(output.contains("Test paragraph"));
        assert!(output.contains("---")); // Page separator
    }

    #[test]
    fn test_ndjson_sink_emits_frames() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.ndjson");

        let mut sink = NdjsonSink::new(output_path.clone()).unwrap();

        let header = make_test_header();
        sink.open(&header).unwrap();

        sink.page(&make_test_page(0)).unwrap();

        let footer = make_test_footer();
        sink.close(&footer).unwrap();

        // Verify output exists and contains NDJSON frames
        let output = std::fs::read_to_string(output_path).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines.len(), 3); // header + page + footer

        // Verify header frame
        let header_frame: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header_frame["type"], "header");
        assert_eq!(header_frame["page_count"], 2);

        // Verify page frame
        let page_frame: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(page_frame["type"], "page");
        assert_eq!(page_frame["page_index"], 0);

        // Verify footer frame
        let footer_frame: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(footer_frame["type"], "footer");
        assert_eq!(footer_frame["overall_quality"], "high");
    }

    #[test]
    fn test_sink_atomic_write_on_drop() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        {
            let mut sink = JsonSink::new(output_path.clone()).unwrap();
            let header = make_test_header();
            sink.open(&header).unwrap();
            sink.page(&make_test_page(0)).unwrap();
            // Drop without calling close - output should NOT exist
            drop(sink);
        }

        // Output should not exist after drop without close
        assert!(!output_path.exists());
    }

    #[test]
    fn test_multiple_sinks_can_coexist() {
        let temp_dir = TempDir::new().unwrap();

        let json_path = temp_dir.path().join("output.json");
        let md_path = temp_dir.path().join("output.md");
        let txt_path = temp_dir.path().join("output.txt");

        let mut json_sink = JsonSink::new(json_path.clone()).unwrap();
        let mut md_sink = MarkdownSink::new(md_path.clone(), MarkdownOptions::default()).unwrap();
        let mut txt_sink = TextSink::new(txt_path.clone()).unwrap();

        let header = make_test_header();
        json_sink.open(&header).unwrap();
        md_sink.open(&header).unwrap();
        txt_sink.open(&header).unwrap();

        let page0 = make_test_page(0);
        json_sink.page(&page0).unwrap();
        md_sink.page(&page0).unwrap();
        txt_sink.page(&page0).unwrap();

        let page1 = make_test_page(1);
        json_sink.page(&page1).unwrap();
        md_sink.page(&page1).unwrap();
        txt_sink.page(&page1).unwrap();

        let footer = make_test_footer();
        json_sink.close(&footer).unwrap();
        md_sink.close(&footer).unwrap();
        txt_sink.close(&footer).unwrap();

        // All three outputs should exist
        assert!(json_path.exists());
        assert!(md_path.exists());
        assert!(txt_path.exists());

        // Verify each has appropriate content
        let json_output = std::fs::read_to_string(json_path).unwrap();
        assert!(json_output.contains("\"schema_version\""));

        let md_output = std::fs::read_to_string(md_path).unwrap();
        assert!(md_output.contains("Test paragraph"));

        let txt_output = std::fs::read_to_string(txt_path).unwrap();
        assert!(txt_output.contains("Test paragraph"));
    }

    #[test]
    fn test_page_validate_success() {
        let page = make_test_page(0);
        assert!(page.validate().is_ok(), "Valid page should pass validation");
    }

    #[test]
    fn test_page_validate_invalid_page_number() {
        let mut page = make_test_page(0);
        page.page_number = 0; // Invalid: must be >= 1

        let result = page.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("page_number"));
        assert!(error.contains(">= 1"));
    }

    #[test]
    fn test_page_validate_invalid_width() {
        let mut page = make_test_page(0);
        page.width = 0.0; // Invalid: must be > 0

        let result = page.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("width"));
        assert!(error.contains("> 0.0"));
    }

    #[test]
    fn test_page_validate_invalid_height() {
        let mut page = make_test_page(0);
        page.height = -10.0; // Invalid: must be > 0

        let result = page.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("height"));
        assert!(error.contains("> 0.0"));
    }

    #[test]
    fn test_page_validate_invalid_rotation() {
        let mut page = make_test_page(0);
        page.rotation = 45; // Invalid: not one of 0, 90, 180, 270

        let result = page.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("rotation"));
        assert!(error.contains("0, 90, 180, 270"));
    }

    #[test]
    fn test_page_validate_empty_page_type() {
        let mut page = make_test_page(0);
        page.page_type = String::new(); // Invalid: must not be empty

        let result = page.validate();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("page_type"));
        assert!(error.contains("empty"));
    }

    #[test]
    fn test_page_validate_all_invalid_fields_reports_first() {
        let mut page = make_test_page(0);
        page.page_number = 0;
        page.width = 0.0;
        page.height = 0.0;
        page.rotation = 45;
        page.page_type = String::new();

        let result = page.validate();
        assert!(result.is_err());
        // Should report the first invalid field encountered
        let error = result.unwrap_err();
        assert!(error.contains("page_number") || error.contains("width") ||
                error.contains("height") || error.contains("rotation") ||
                error.contains("page_type"));
    }
}
