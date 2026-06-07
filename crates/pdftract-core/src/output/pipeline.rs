//! Multi-sink pipeline for concurrent multi-format output.
//!
//! This module provides the pipeline that orchestrates multiple output sinks,
//! allowing a single extraction pass to populate any subset of output formats.

use crate::output::sink::{
    DocumentFooter, DocumentHeader, JsonSink, MarkdownSink, NdjsonSink, OutputSink, Page, TextSink,
};
use crate::output::multi::{Destination, Format, OutputSpec};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Multi-sink pipeline that coordinates output to multiple sinks.
///
/// The pipeline manages the lifecycle of multiple sinks, ensuring that
/// all sinks are opened before extraction, receive all pages, and are
/// properly closed after extraction completes.
pub struct MultiSinkPipeline {
    /// All sinks being managed by this pipeline
    sinks: Vec<Box<dyn OutputSink>>,
}

impl MultiSinkPipeline {
    /// Create a new multi-sink pipeline from output specifications.
    ///
    /// # Arguments
    ///
    /// * `specs` - Output specifications defining which formats to emit
    ///
    /// # Returns
    ///
    /// A new MultiSinkPipeline instance
    ///
    /// # Errors
    ///
    /// Returns an error if any sink cannot be created.
    pub fn from_specs(specs: &[OutputSpec]) -> Result<Self> {
        let mut sinks = Vec::new();

        for spec in specs {
            let sink: Box<dyn OutputSink> = match spec.format {
                Format::Json => {
                    let path = match &spec.dest {
                        Destination::File(p) => p.clone(),
                        Destination::Stdout => PathBuf::from("-"),
                    };
                    Box::new(JsonSink::new(path)?)
                }
                Format::Markdown => {
                    let path = match &spec.dest {
                        Destination::File(p) => p.clone(),
                        Destination::Stdout => PathBuf::from("-"),
                    };
                    Box::new(MarkdownSink::new(path, Default::default())?)
                }
                Format::Text => {
                    let path = match &spec.dest {
                        Destination::File(p) => p.clone(),
                        Destination::Stdout => PathBuf::from("-"),
                    };
                    Box::new(TextSink::new(path)?)
                }
                Format::Ndjson => {
                    let path = match &spec.dest {
                        Destination::File(p) => p.clone(),
                        Destination::Stdout => PathBuf::from("-"),
                    };
                    Box::new(NdjsonSink::new(path)?)
                }
            };
            sinks.push(sink);
        }

        Ok(Self { sinks })
    }

    /// Open all sinks with the document header.
    ///
    /// # Arguments
    ///
    /// * `header` - Document metadata available at extraction start
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns an error if any sink fails to open.
    pub fn open(&mut self, header: &DocumentHeader) -> Result<()> {
        for sink in &mut self.sinks {
            sink.open(header)
                .with_context(|| format!("failed to open sink"))?;
        }
        Ok(())
    }

    /// Process a single page through all sinks.
    ///
    /// # Arguments
    ///
    /// * `page` - The page data
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns an error if any sink fails to process the page.
    pub fn page(&mut self, page: &Page) -> Result<()> {
        for sink in &mut self.sinks {
            sink.page(page)
                .with_context(|| format!("failed to process page {}", page.page_index))?;
        }
        Ok(())
    }

    /// Close all sinks with the document footer.
    ///
    /// # Arguments
    ///
    /// * `footer` - Aggregated document metadata
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns an error if any sink fails to close or commit.
    pub fn close(&mut self, footer: &DocumentFooter) -> Result<()> {
        for sink in &mut self.sinks {
            sink.close(footer)
                .with_context(|| format!("failed to close sink"))?;
        }
        Ok(())
    }

    /// Run the full pipeline with a header, pages, and footer.
    ///
    /// This is a convenience method that calls open, page (for each page),
    /// and close in sequence.
    ///
    /// # Arguments
    ///
    /// * `header` - Document metadata
    /// * `pages` - All pages to process
    /// * `footer` - Aggregated metadata
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns an error if any step fails.
    pub fn run(&mut self, header: &DocumentHeader, pages: &[Page], footer: &DocumentFooter) -> Result<()> {
        self.open(header)?;
        for page in pages {
            self.page(page)?;
        }
        self.close(footer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::multi::validate_outputs;
    use std::fs;

    fn make_test_page(index: usize) -> Page {
        Page {
            page_index: index,
            page_number: (index + 1) as u32,
            page_label: None,
            width: 612.0,
            height: 792.0,
            rotation: 0,
            page_type: "text".to_string(),
            spans: vec![],
            blocks: vec![],
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
    fn test_multi_sink_pipeline_with_json_and_md() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let specs = vec![
            OutputSpec::file(Format::Json, temp_dir.path().join("output.json")),
            OutputSpec::file(Format::Markdown, temp_dir.path().join("output.md")),
        ];

        validate_outputs(&specs).unwrap();

        let mut pipeline = MultiSinkPipeline::from_specs(&specs).unwrap();
        let header = make_test_header();
        let pages = vec![make_test_page(0), make_test_page(1)];
        let footer = make_test_footer();

        pipeline.run(&header, &pages, &footer).unwrap();

        // Both outputs should exist
        assert!(temp_dir.path().join("output.json").exists());
        assert!(temp_dir.path().join("output.md").exists());

        // Verify JSON output
        let json_output = fs::read_to_string(temp_dir.path().join("output.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_output).unwrap();
        assert_eq!(json["schema_version"], "1.0");

        // Verify Markdown output
        let md_output = fs::read_to_string(temp_dir.path().join("output.md")).unwrap();
        assert!(!md_output.is_empty());
    }

    #[test]
    fn test_multi_sink_pipeline_with_three_formats() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let specs = vec![
            OutputSpec::file(Format::Json, temp_dir.path().join("output.json")),
            OutputSpec::file(Format::Markdown, temp_dir.path().join("output.md")),
            OutputSpec::file(Format::Text, temp_dir.path().join("output.txt")),
        ];

        validate_outputs(&specs).unwrap();

        let mut pipeline = MultiSinkPipeline::from_specs(&specs).unwrap();
        let header = make_test_header();
        let pages = vec![make_test_page(0)];
        let footer = make_test_footer();

        pipeline.run(&header, &pages, &footer).unwrap();

        // All three outputs should exist
        assert!(temp_dir.path().join("output.json").exists());
        assert!(temp_dir.path().join("output.md").exists());
        assert!(temp_dir.path().join("output.txt").exists());
    }

    #[test]
    fn test_multi_sink_pipeline_step_by_step() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let specs = vec![
            OutputSpec::file(Format::Json, temp_dir.path().join("output.json")),
        ];

        let mut pipeline = MultiSinkPipeline::from_specs(&specs).unwrap();
        let header = make_test_header();
        let footer = make_test_footer();

        // Step-by-step execution
        pipeline.open(&header).unwrap();
        pipeline.page(&make_test_page(0)).unwrap();
        pipeline.page(&make_test_page(1)).unwrap();
        pipeline.close(&footer).unwrap();

        // Output should exist
        assert!(temp_dir.path().join("output.json").exists());
    }

    #[test]
    fn test_multi_sink_pipeline_with_ndjson() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let specs = vec![
            OutputSpec::file(Format::Ndjson, temp_dir.path().join("output.ndjson")),
        ];

        validate_outputs(&specs).unwrap();

        let mut pipeline = MultiSinkPipeline::from_specs(&specs).unwrap();
        let header = make_test_header();
        let pages = vec![make_test_page(0), make_test_page(1)];
        let footer = make_test_footer();

        pipeline.run(&header, &pages, &footer).unwrap();

        // NDJSON output should exist
        let output = fs::read_to_string(temp_dir.path().join("output.ndjson")).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        // Should have header + 2 pages + footer = 4 lines
        assert_eq!(lines.len(), 4);

        // Verify frames
        let header_frame: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header_frame["type"], "header");

        let page0_frame: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(page0_frame["type"], "page");
        assert_eq!(page0_frame["page_index"], 0);

        let page1_frame: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(page1_frame["type"], "page");
        assert_eq!(page1_frame["page_index"], 1);

        let footer_frame: serde_json::Value = serde_json::from_str(lines[3]).unwrap();
        assert_eq!(footer_frame["type"], "footer");
    }

    #[test]
    fn test_multi_sink_pipeline_cross_format_consistency() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let specs = vec![
            OutputSpec::file(Format::Json, temp_dir.path().join("output.json")),
            OutputSpec::file(Format::Markdown, temp_dir.path().join("output.md")),
        ];

        validate_outputs(&specs).unwrap();

        let mut pipeline = MultiSinkPipeline::from_specs(&specs).unwrap();

        let header = DocumentHeader {
            document_fingerprint: "consistency-test-fingerprint".to_string(),
            page_count: 1,
            schema_version: "1.0",
        };

        let pages = vec![make_test_page(0)];
        let footer = make_test_footer();

        pipeline.run(&header, &pages, &footer).unwrap();

        // Both outputs should exist with consistent fingerprint
        let json_output = fs::read_to_string(temp_dir.path().join("output.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_output).unwrap();

        let md_output = fs::read_to_string(temp_dir.path().join("output.md")).unwrap();

        // Both should exist and have content
        assert!(json_output.contains("schema_version"));
        assert!(!md_output.is_empty());

        // Verify schema version consistency
        assert_eq!(json["schema_version"], "1.0");
    }

    #[test]
    fn test_multi_sink_pipeline_rejects_ndjson_with_other_formats() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let specs = vec![
            OutputSpec::file(Format::Ndjson, temp_dir.path().join("output.ndjson")),
            OutputSpec::file(Format::Json, temp_dir.path().join("output.json")),
        ];

        // Should fail validation because NDJSON is mutually exclusive
        let result = validate_outputs(&specs);
        assert!(result.is_err());
        match result {
            Err(e) => {
                let err_msg = e.to_string();
                assert!(err_msg.contains("ndjson") || err_msg.contains("cannot be combined"),
                    "Expected NDJSON mutual exclusivity error, got: {}", err_msg);
            }
            Ok(_) => panic!("Expected validation error for NDJSON + other formats"),
        }
    }

    #[test]
    fn test_multi_sink_pipeline_atomicity() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let specs = vec![
            OutputSpec::file(Format::Json, temp_dir.path().join("output.json")),
        ];

        let mut pipeline = MultiSinkPipeline::from_specs(&specs).unwrap();
        let header = make_test_header();
        let footer = make_test_footer();

        // Open and write pages, but drop before close
        pipeline.open(&header).unwrap();
        pipeline.page(&make_test_page(0)).unwrap();

        // Drop pipeline without closing - no output should exist
        drop(pipeline);

        // Output should NOT exist after drop without close
        assert!(!temp_dir.path().join("output.json").exists());

        // Verify no temp files remain
        let entries = fs::read_dir(temp_dir.path()).unwrap();
        for entry in entries {
            let path = entry.unwrap().path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                assert!(
                    !name_str.contains(".tmp."),
                    "Temp file should be cleaned up: {}",
                    name_str
                );
            }
        }
    }
}
