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
use crate::schema::ExtractionQuality;
use anyhow::{Context, Result};
// serde is an optional capability: JSON call sites gate on the feature so `--no-default-features` (the wasm32 library edge) compiles (pdftract-c1fceb36).
#[cfg(feature = "serde")]
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
#[cfg(feature = "serde")]
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
    let errors = footer_errors(&result)?;

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

/// Assemble the NDJSON footer frame's `errors` array.
///
/// Per `docs/integrations/diagnostics-codes.md` ("Where structured
/// diagnostics appear"): one `page_extraction_error` record per failed page
/// first, then the document's structured diagnostics
/// (`metadata.diagnostics_detailed`) in emission order, serialized
/// identically to the `errors` array of the full JSON output. Factored out of
/// [`extract_streaming`] so the ordering contract is unit-testable without a
/// working extraction path.
#[cfg(feature = "serde")]
pub fn footer_errors(result: &crate::extract::ExtractionResult) -> Result<Vec<serde_json::Value>> {
    let mut errors: Vec<serde_json::Value> = result
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

    for diag in &result.metadata.diagnostics_detailed {
        errors.push(serde_json::to_value(diag).context("Failed to serialize diagnostic")?);
    }

    Ok(errors)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_extract_streaming_smoke() {
        // This is a placeholder test
        // The full implementation will have actual fixture-based tests
    }

    #[test]
    fn test_footer_errors_carry_structured_diagnostics() {
        // The NDJSON footer's `errors` array is the streaming surface of the
        // structured diagnostics documented in
        // docs/integrations/diagnostics-codes.md: document-level diagnostics
        // (here TAGGED_PDF_STRUCT_TREE_DEFERRED from a marked PDF) must reach
        // the footer as objects with code/severity fields, not bare strings.
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("tagged_streaming.pdf");
        // Byte-identical to the inline tagged fixture in extract.rs's tests
        // (xref offsets depend on it).
        std::fs::write(
            &pdf_path,
            br#"%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R/MarkInfo<</Marked true>>>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>>>>endobj

xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000096 00000 n
0000000145 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref
283
%%EOF
"#,
        )
        .unwrap();

        let mut buf: Vec<u8> = Vec::new();
        super::extract_streaming(
            &pdf_path,
            &crate::options::ExtractionOptions::default(),
            &mut buf,
        )
        .expect("streaming extraction should succeed");

        let footer_line = buf
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .last()
            .expect("NDJSON output must contain a footer frame");
        let footer: serde_json::Value =
            serde_json::from_slice(footer_line).expect("footer frame must be valid JSON");

        let errors = footer["errors"].as_array().expect("footer errors array");
        let deferred = errors
            .iter()
            .find(|e| e["code"] == "TAGGED_PDF_STRUCT_TREE_DEFERRED")
            .expect("footer errors must carry the structured diagnostic");
        assert_eq!(deferred["severity"], "info");
        assert!(
            deferred["hint"].is_string(),
            "catalog hint must serialize with the diagnostic"
        );
    }
}
