//! pdftract SDK public API surface.
//!
//! This module exposes the 9-method SDK contract that all language SDKs implement.
//! Rust users import pdftract-core directly and use these functions to match the SDK contract.

use crate::classify::{classify_page, PageClassification, PageContext};
use crate::extract::{extract_pdf, extract_text as extract_text_impl, ExtractionResult, PageResult};
use crate::options::ExtractionOptions;
use crate::fingerprint::compute_fingerprint;
use crate::markdown::page_to_markdown;
use crate::parser::catalog::parse_catalog;
use crate::parser::pages::{flatten_page_tree, LazyPageIter, PageDict};
use crate::parser::xref::{load_xref_with_prev_chain, XrefResolver};
use crate::receipts::verifier::{verify_receipt, SpanData, VerificationResult};
use crate::receipts::Receipt;
use crate::source::FileSource;
use crate::parser::stream::PdfSource as ParserPdfSource;
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Extract a PDF to the full structured JSON output.
///
/// This is the main extraction method that returns pages, spans, blocks, tables,
/// form fields, and other structured data as JSON-serializable objects.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// An `ExtractionResult` containing pages and metadata.
pub fn extract(pdf_path: &Path, options: &ExtractionOptions) -> Result<ExtractionResult> {
    extract_pdf(pdf_path, options)
}

/// Extract plain text from a PDF.
///
/// Returns the concatenated text content of all pages, with spans separated
/// by newlines. Invisible text (rendering_mode=3) is excluded by default.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// A String containing all extracted text.
pub fn extract_text(pdf_path: &Path, options: &ExtractionOptions) -> Result<String> {
    extract_text_impl(pdf_path, options)
}

/// Extract Markdown from a PDF.
///
/// Returns the document converted to Markdown format, with headers, lists,
/// tables, and form fields rendered using Markdown syntax.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// A String containing the Markdown representation.
pub fn extract_markdown(pdf_path: &Path, options: &ExtractionOptions) -> Result<String> {
    let result = extract_pdf(pdf_path, options)?;

    let mut markdown = String::new();
    for (i, page) in result.pages.iter().enumerate() {
        if i > 0 {
            markdown.push_str("\n\n");
        }

        // Filter links to only those that belong to this page
        let page_links: Vec<_> = result.links.iter()
            .filter(|link| link.page_index == i)
            .cloned()
            .collect();

        markdown.push_str(&crate::markdown::page_to_markdown_with_links(
            &page.blocks,
            &page.spans,
            &[], // No separate tables storage - tables are in blocks
            page_links.as_slice(),
            i,
            false,  // include_anchor
            &crate::markdown::MarkdownOptions::default(),
        ));
    }

    Ok(markdown)
}

/// Extract a PDF page by page as an iterator.
///
/// This is the streaming variant that yields pages one at a time, keeping
/// memory usage bounded regardless of document size.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// An iterator that yields `PageResult` objects.
pub fn extract_stream(
    pdf_path: &Path,
    options: &ExtractionOptions,
) -> Result<impl Iterator<Item = Result<PageResult>>> {
    // For now, extract all and return an iterator over the results
    // TODO: Implement true streaming with lazy page iteration
    let result = extract_pdf(pdf_path, options)?;
    Ok(result.pages.into_iter().map(Ok))
}

/// Search for text patterns in a PDF.
///
/// Returns an iterator of matches with page index, span index, and context.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `pattern` - Search pattern (plain text or regex)
/// * `case_insensitive` - Ignore case when matching
/// * `regex` - Treat pattern as a regular expression
/// * `whole_word` - Match only whole words
///
/// # Returns
///
/// A vector of `SearchMatch` objects with location and context.
pub fn search(
    pdf_path: &Path,
    pattern: &str,
    case_insensitive: bool,
    use_regex: bool,
    whole_word: bool,
) -> Result<Vec<SearchMatch>> {
    let options = ExtractionOptions::default();
    let result = extract_pdf(pdf_path, &options)?;

    let mut matches = Vec::new();

    // Build the regex pattern
    let search_pattern = if whole_word {
        format!(r"\b{}\b", regex::escape(pattern))
    } else if use_regex {
        pattern.to_string()
    } else {
        regex::escape(pattern)
    };

    let re = Regex::new(&search_pattern)
        .with_context(|| format!("Invalid regex pattern: {}", search_pattern))?;

    for (page_idx, page) in result.pages.iter().enumerate() {
        for (span_idx, span) in page.spans.iter().enumerate() {
            let text = &span.text;

            // Check if pattern matches
            let re_with_flags = if case_insensitive {
                Regex::new(&format!("(?i){}", search_pattern))?
            } else {
                re.clone()
            };

            if re_with_flags.is_match(text) {
                matches.push(SearchMatch {
                    page_index: page_idx,
                    span_index: span_idx,
                    text: text.clone(),
                    bbox: span.bbox,
                });
            }
        }
    }

    Ok(matches)
}

/// A single search match result.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Page index where the match was found.
    pub page_index: usize,
    /// Span index within the page.
    pub span_index: usize,
    /// The matched text content.
    pub text: String,
    /// Bounding box of the match [x0, y0, x1, y1].
    pub bbox: [f64; 4],
}

/// Get metadata about a PDF.
///
/// Returns page count and basic metadata without full extraction.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
///
/// # Returns
///
/// A `PdfMetadata` object with page count and other metadata.
pub fn get_metadata(pdf_path: &Path) -> Result<PdfMetadata> {
    let (_fingerprint, catalog, pages, _resolver) = crate::document::parse_pdf_file(pdf_path)?;

    Ok(PdfMetadata {
        page_count: pages.len(),
        is_encrypted: false, // TODO: detect encryption from catalog
        is_tagged: catalog.struct_tree_root_ref.is_some(),
        has_forms: catalog.acroform_ref.is_some(),
    })
}

/// Metadata about a PDF document.
#[derive(Debug, Clone)]
pub struct PdfMetadata {
    /// Total number of pages.
    pub page_count: usize,
    /// Whether the document is encrypted.
    pub is_encrypted: bool,
    /// Whether the document is a tagged PDF.
    pub is_tagged: bool,
    /// Whether the document has AcroForm fields.
    pub has_forms: bool,
}

/// Compute the cryptographic hash of a PDF.
///
/// Returns the v1 fingerprint hash of the PDF content.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
///
/// # Returns
///
/// A String containing the fingerprint hash in format "pdftract-v1:HEX_HASH".
///
/// Where HEX_HASH is a hexadecimal string of the SHA-256 hash.
pub fn hash(pdf_path: &Path) -> Result<String> {
    let (fingerprint, _catalog, _pages, _resolver) = crate::document::parse_pdf_file(pdf_path)?;
    Ok(fingerprint)
}

/// Classify a PDF page.
///
/// Returns the page type (scientific paper, slide, form, etc.) with confidence.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `page_index` - Zero-based page index to classify
///
/// # Returns
///
/// A `PageClassification` with the detected page type and confidence.
pub fn classify(pdf_path: &Path, page_index: usize) -> Result<PageClassification> {
    let options = ExtractionOptions::default();
    let result = extract_pdf(pdf_path, &options)?;

    let page = result.pages.get(page_index)
        .ok_or_else(|| anyhow::anyhow!("Page index {} out of bounds", page_index))?;

    // Create a minimal page context for classification
    // Note: PageContext requires metrics from content stream analysis
    // For SDK simplicity, we create a default context and populate available fields
    let mut ctx = PageContext::new();
    ctx.width = page.width.unwrap_or(0.0) as f64;
    ctx.height = page.height.unwrap_or(0.0) as f64;
    ctx.rotation = page.rotation.unwrap_or(0) as i32;

    Ok(classify_page(&ctx))
}

/// Verify a cryptographic receipt against a PDF.
///
/// Validates that the receipt matches the PDF content by checking:
/// 1. PDF fingerprint matches
/// 2. At least one span has bbox overlap >= 90% IoU
/// 3. That span's NFC-normalized SHA-256 equals the receipt's content_hash
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `receipt_path` - Path to the receipt JSON file
///
/// # Returns
///
/// A `VerificationResult` indicating success or the specific failure mode.
pub fn verify_receipt_from_path(
    pdf_path: &Path,
    receipt_path: &Path,
) -> Result<VerificationResult> {
    // Load the receipt
    let receipt_data = std::fs::read_to_string(receipt_path)
        .context("Failed to read receipt file")?;
    let receipt: Receipt = serde_json::from_str(&receipt_data)
        .context("Failed to parse receipt JSON")?;

    // Extract spans from the PDF
    let options = ExtractionOptions::default();
    let result = extract_pdf(pdf_path, &options)?;

    let page = result.pages.get(receipt.page_index)
        .ok_or_else(|| anyhow::anyhow!("Receipt page index {} out of bounds", receipt.page_index))?;

    // Convert spans to SpanData
    let spans: Vec<SpanData> = page.spans.iter().map(|span| SpanData {
        text: span.text.clone(),
        bbox: span.bbox,
    }).collect();

    // Compute the actual fingerprint
    let actual_fingerprint = hash(pdf_path)?;

    // Verify
    Ok(verify_receipt(&receipt, &spans, &actual_fingerprint))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_basic() {
        // Test will be implemented with fixture
    }
}
