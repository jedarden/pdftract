//! PDF text extraction with receipt generation.
//!
//! This module provides the main extraction pipeline that processes PDFs
//! and generates spans and blocks with optional cryptographic receipts.

use crate::document::parse_pdf_file;
use crate::options::{ExtractionOptions, ReceiptsMode};
use crate::receipts::Receipt;
use crate::schema::{BlockJson, SpanJson};
use anyhow::{Context, Result};
use serde_json::json;

#[cfg(feature = "receipts")]
use crate::receipts::svg::GlyphList;

/// Result of a PDF extraction operation.
///
/// Contains the extracted pages, spans, blocks, and metadata.
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// The PDF fingerprint (for receipt generation).
    pub fingerprint: String,
    /// Extracted pages, each containing spans and blocks.
    pub pages: Vec<PageResult>,
    /// Metadata about the extraction.
    pub metadata: ExtractionMetadata,
}

/// Result for a single page.
#[derive(Debug, Clone)]
pub struct PageResult {
    /// 0-based page index.
    pub index: usize,
    /// Extracted spans (text fragments with consistent styling).
    pub spans: Vec<SpanJson>,
    /// Extracted blocks (semantic units like paragraphs, headings).
    pub blocks: Vec<BlockJson>,
}

/// Metadata about the extraction process.
#[derive(Debug, Clone)]
pub struct ExtractionMetadata {
    /// Total number of pages in the document.
    pub page_count: usize,
    /// Receipts mode used for this extraction.
    pub receipts_mode: ReceiptsMode,
    /// Number of spans extracted.
    pub span_count: usize,
    /// Number of blocks extracted.
    pub block_count: usize,
}

/// Extract text and structure from a PDF file.
///
/// This is the main entry point for PDF extraction. It:
/// 1. Parses the PDF and computes its fingerprint
/// 2. Extracts spans and blocks from each page
/// 3. Generates receipts if requested
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options controlling receipt generation
///
/// # Returns
///
/// An `ExtractionResult` containing pages with spans and blocks.
pub fn extract_pdf(
    pdf_path: &std::path::Path,
    options: &ExtractionOptions,
) -> Result<ExtractionResult> {
    // Parse the PDF to get fingerprint and page info
    let (fingerprint, _catalog, pages, _resolver) = parse_pdf_file(pdf_path)
        .context("Failed to parse PDF file")?;

    let page_count = pages.len();

    // Extract each page
    let mut extracted_pages = Vec::new();
    let mut total_spans = 0;
    let mut total_blocks = 0;

    for (page_idx, page) in pages.iter().enumerate() {
        let page_result = extract_page(
            &fingerprint,
            page_idx,
            page,
            options,
        )?;
        total_spans += page_result.spans.len();
        total_blocks += page_result.blocks.len();
        extracted_pages.push(page_result);
    }

    Ok(ExtractionResult {
        fingerprint,
        pages: extracted_pages,
        metadata: ExtractionMetadata {
            page_count,
            receipts_mode: options.receipts,
            span_count: total_spans,
            block_count: total_blocks,
        },
    })
}

/// Extract content from a single page.
///
/// # Arguments
///
/// * `fingerprint` - The PDF fingerprint for receipt generation
/// * `page_index` - 0-based page index
/// * `page` - The page dictionary from the PDF
/// * `options` - Extraction options
fn extract_page(
    fingerprint: &str,
    page_index: usize,
    page: &crate::parser::pages::PageDict,
    options: &ExtractionOptions,
) -> Result<PageResult> {
    // For now, create placeholder spans based on the page media box
    // In a full implementation, this would parse the content streams
    // and extract actual text with positioning information

    let [x0, y0, x1, y1] = page.media_box;

    // Create a placeholder span for the entire page
    // This is a minimal implementation - the full Phase 3 pipeline
    // would extract actual text from content streams
    let span_text = format!("[Page {} text extraction]", page_index);
    let span_bbox = [x0, y0, x1, y1];

    // Generate receipt if requested
    let receipt = generate_receipt(
        fingerprint,
        page_index,
        span_bbox,
        &span_text,
        options.receipts,
        #[cfg(feature = "receipts")] None,
    )?;

    let span = SpanJson {
        text: span_text,
        bbox: span_bbox,
        font: "Unknown".to_string(),
        size: 12.0,
        confidence: None,
        receipt,
    };

    // Create a block containing the span
    let block_text = span.text.clone();
    let block_bbox = span_bbox;
    let block_receipt = generate_receipt(
        fingerprint,
        page_index,
        block_bbox,
        &block_text,
        options.receipts,
        #[cfg(feature = "receipts")] None,
    )?;

    let block = BlockJson {
        kind: "paragraph".to_string(),
        text: block_text,
        bbox: block_bbox,
        level: None,
        receipt: block_receipt,
    };

    Ok(PageResult {
        index: page_index,
        spans: vec![span],
        blocks: vec![block],
    })
}

/// Generate a receipt for a span or block.
///
/// # Arguments
///
/// * `fingerprint` - The PDF fingerprint
/// * `page_index` - 0-based page index
/// * `bbox` - Bounding box in PDF points
/// * `text` - The text content
/// * `mode` - Receipt generation mode
/// * `glyph_list` - Optional glyph list for SVG generation (only used with receipts feature)
fn generate_receipt(
    fingerprint: &str,
    page_index: usize,
    bbox: [f64; 4],
    text: &str,
    mode: ReceiptsMode,
    #[cfg(feature = "receipts")] glyph_list: Option<&GlyphList>,
) -> Result<Option<Receipt>> {
    match mode {
        ReceiptsMode::Off => Ok(None),
        ReceiptsMode::Lite => Ok(Some(Receipt::lite(
            fingerprint.to_string(),
            page_index,
            bbox,
            text,
        ))),
        #[cfg(feature = "receipts")]
        ReceiptsMode::SvgClip => {
            // For SVG mode, we need a glyph list to generate the SVG clip
            // In this minimal implementation, we fall back to lite mode
            // if no glyph list is provided
            if let Some(glyphs) = glyph_list {
                let svg_gen = crate::receipts::svg::SvgGenerator::new(glyphs.clone());
                let svg_clip = svg_gen.generate(bbox);
                Ok(Some(Receipt::with_svg(
                    fingerprint.to_string(),
                    page_index,
                    bbox,
                    text,
                    svg_clip,
                )))
            } else {
                // No glyph data available - fall back to lite mode
                Ok(Some(Receipt::lite(
                    fingerprint.to_string(),
                    page_index,
                    bbox,
                    text,
                )))
            }
        }
        #[cfg(not(feature = "receipts"))]
        ReceiptsMode::SvgClip => {
            // Receipts feature not enabled - fall back to lite mode
            Ok(Some(Receipt::lite(
                fingerprint.to_string(),
                page_index,
                bbox,
                text,
            )))
        }
    }
}

/// Convert an ExtractionResult to JSON format.
///
/// This produces the JSON output format expected by the CLI and API.
pub fn result_to_json(result: &ExtractionResult) -> serde_json::Value {
    let pages: Vec<serde_json::Value> = result
        .pages
        .iter()
        .map(|page| {
            json!({
                "index": page.index,
                "spans": page.spans,
                "blocks": page.blocks,
            })
        })
        .collect();

    json!({
        "fingerprint": result.fingerprint,
        "schema_version": "1.0",
        "pages": pages,
        "metadata": {
            "page_count": result.metadata.page_count,
            "span_count": result.metadata.span_count,
            "block_count": result.metadata.block_count,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Create a minimal valid PDF for testing.
    fn create_minimal_pdf(path: &Path) -> Result<()> {
        let pdf_data = br#"%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>>>>endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000052 00000 n
0000000109 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref
206
%%EOF
"#;
        fs::write(path, pdf_data)?;
        Ok(())
    }

    /// Get a test PDF file path.
    /// Uses one of the classifier fixture PDFs for testing.
    fn get_test_pdf_path() -> std::path::PathBuf {
        // For now, use the temp-based minimal PDF to ensure tests are self-contained
        // This avoids dependency on external fixture files that may be malformed
        std::path::PathBuf::from("__test__.pdf")
    }

    /// Get or create the test PDF file.
    fn ensure_test_pdf() -> std::path::PathBuf {
        let path = get_test_pdf_path();
        if !path.exists() {
            create_minimal_pdf(&path).unwrap();
        }
        path
    }

    #[test]
    fn test_extract_pdf_with_receipts_off() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert!(result.pages.len() >= 1);
        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::Off);

        let page = &result.pages[0];
        assert!(!page.spans.is_empty());

        // Receipts should be None when mode is Off
        for span in &page.spans {
            assert!(span.receipt.is_none());
        }
        for block in &page.blocks {
            assert!(block.receipt.is_none());
        }
    }

    #[test]
    fn test_extract_pdf_with_receipts_lite() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::Lite);

        let page = &result.pages[0];
        assert!(!page.spans.is_empty());

        // Receipts should be present in lite mode
        for span in &page.spans {
            assert!(span.receipt.is_some());
            let receipt = span.receipt.as_ref().unwrap();
            assert_eq!(receipt.pdf_fingerprint, result.fingerprint);
            assert!(receipt.svg_clip.is_none());
        }

        for block in &page.blocks {
            assert!(block.receipt.is_some());
            let receipt = block.receipt.as_ref().unwrap();
            assert_eq!(receipt.pdf_fingerprint, result.fingerprint);
            assert!(receipt.svg_clip.is_none());
        }
    }

    #[test]
    fn test_extract_pdf_with_receipts_svg() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::SvgClip);
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::SvgClip);

        let page = &result.pages[0];
        assert!(!page.spans.is_empty());

        // Receipts should be present
        // Note: In this minimal implementation without glyph data,
        // SVG mode falls back to lite mode (svg_clip is None)
        for span in &page.spans {
            assert!(span.receipt.is_some());
            let receipt = span.receipt.as_ref().unwrap();
            assert_eq!(receipt.pdf_fingerprint, result.fingerprint);
        }
    }

    #[test]
    fn test_result_to_json_format() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::default();
        let result = extract_pdf(&pdf_path, &options).unwrap();
        let json = result_to_json(&result);

        assert!(json.is_object());
        assert!(json.get("fingerprint").is_some());
        assert!(json.get("schema_version").is_some());
        assert!(json.get("pages").is_some());
        assert!(json.get("metadata").is_some());

        let pages = json.get("pages").and_then(|v| v.as_array()).unwrap();
        assert_eq!(pages.len(), 1);

        let page = &pages[0];
        assert!(page.get("index").is_some());
        assert!(page.get("spans").is_some());
        assert!(page.get("blocks").is_some());
    }

    #[test]
    fn test_result_to_json_with_receipts() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let result = extract_pdf(&pdf_path, &options).unwrap();
        let json = result_to_json(&result);

        let pages = json.get("pages").and_then(|v| v.as_array()).unwrap();
        let page = &pages[0];
        let spans = page.get("spans").and_then(|v| v.as_array()).unwrap();
        let span = &spans[0];

        // Span should have receipt field
        assert!(span.get("receipt").is_some());

        let receipt = span.get("receipt").unwrap();
        assert!(receipt.get("pdf_fingerprint").is_some());
        assert!(receipt.get("page_index").is_some());
        assert!(receipt.get("bbox").is_some());
        assert!(receipt.get("content_hash").is_some());
        assert!(receipt.get("extraction_version").is_some());

        // svg_clip should not be present in lite mode
        assert!(receipt.get("svg_clip").is_none());
    }

    #[test]
    fn test_extraction_metadata() {
        let pdf_path = ensure_test_pdf();

        let options = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let result = extract_pdf(&pdf_path, &options).unwrap();

        assert!(result.metadata.page_count >= 1);
        assert!(result.metadata.span_count > 0);
        assert!(result.metadata.block_count > 0);
        assert_eq!(result.metadata.receipts_mode, ReceiptsMode::Lite);
    }
}
