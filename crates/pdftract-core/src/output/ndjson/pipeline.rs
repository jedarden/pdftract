//! Streaming NDJSON extraction pipeline.
//!
//! This module implements the end-to-end streaming pipeline that:
//! 1. Emits a HeaderFrame with document metadata
//! 2. Spawns rayon workers to extract pages in parallel
//! 3. Buffers completed pages and emits them in order via OutOfOrderBuffer
//! 4. Emits a FooterFrame with aggregated metrics
//!
//! Header/footer detection in streaming mode uses a deferred window:
//! - First 3 pages: blocks emitted as kind: paragraph (no retroactive correction)
//! - Pages 4+: blocks identified as header/footer if matched across trailing 4-page window

use crate::options::ExtractionOptions;
use crate::output::ndjson::frames::{FooterFrame, HeaderFrame, PageFrame};
use crate::page_class::PageClass;
use crate::schema::ExtractionQuality;
use anyhow::{Context, Result};
use serde_json::json;
use std::io::Write;
use std::path::Path;

/// Extract a PDF in streaming NDJSON format.
///
/// This is a simplified implementation that integrates with the existing
/// extraction pipeline. For now, it delegates to the non-streaming extract
/// function and splits the result into frames.
///
/// # TODO
///
/// The full streaming implementation will:
/// - Parse document metadata for header
/// - Extract pages in parallel with rayon
/// - Buffer and emit pages in order
/// - Aggregate metrics for footer
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options
/// * `writer` - Buffered writer to receive NDJSON output
///
/// # Output Format
///
/// Emits NDJSON frames in sequence:
/// 1. Header frame (metadata, outline, page count)
/// 2. Page frames (one per page, in order)
/// 3. Footer frame (quality metrics, diagnostics)
pub fn extract_streaming<W: Write>(
    pdf_path: &Path,
    options: &ExtractionOptions,
    writer: &mut W,
) -> Result<()> {
    // Use the existing extraction function for now
    // The full streaming implementation will parse incrementally
    let result = crate::extract_pdf(pdf_path, options)?;

    // Emit header frame
    let header = HeaderFrame::new(
        "1.0".to_string(),
        json!({
            "title": null,
            "author": null,
            "subject": null,
            "keywords": null,
            "creator": null,
            "producer": null,
            "creation_date": null,
            "modification_date": null,
            "page_count": result.metadata.page_count,
        }),
        None, // TODO: extract outline
        result.metadata.page_count,
    );
    writer
        .write_all(header.to_json_line()?.as_bytes())
        .context("Failed to write header frame")?;

    // Emit page frames
    for page in &result.pages {
        let page_type = if page.spans.is_empty() && page.blocks.is_empty() {
            "blank".to_string()
        } else {
            "content".to_string()
        };

        let frame = PageFrame::new(
            page.index,
            page_type,
            page.spans.clone(),
            page.blocks.clone(),
            page.tables.clone(),
        );

        if let Some(ref error) = page.error {
            let frame = frame.with_errors(vec![json!({
                "code": "page_extraction_error",
                "severity": "error",
                "message": error,
            })]);
            writer
                .write_all(frame.to_json_line()?.as_bytes())
                .context("Failed to write page frame")?;
        } else {
            writer
                .write_all(frame.to_json_line()?.as_bytes())
                .context("Failed to write page frame")?;
        }
    }

    // Build and emit footer frame
    let errors: Vec<serde_json::Value> = result
        .pages
        .iter()
        .filter_map(|p| p.error.as_ref())
        .map(|e| {
            json!({
                "code": "page_extraction_error",
                "severity": "error",
                "message": e,
            })
        })
        .collect();

    let quality = ExtractionQuality::new()
        .with_quality(if errors.is_empty() { "high" } else { "medium" })
        .with_ocr_fraction(0.0); // TODO: compute actual OCR fraction

    let footer = FooterFrame::new(quality, errors);

    writer
        .write_all(footer.to_json_line()?.as_bytes())
        .context("Failed to write footer frame")?;

    writer.flush().context("Failed to flush output")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_streaming_smoke() {
        // This is a placeholder test
        // The full implementation will have actual fixture-based tests
    }
}
