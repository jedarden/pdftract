//! JSON output module for full schema extraction results.
//!
//! This module provides conversion functions from `ExtractionResult` to the
//! full JSON `Output` schema defined in the schema module. This is the canonical
//! output format for pdftract v1.0.
//!
//! # Usage
//!
//! ```rust,no_run
//! use pdftract_core::{extract_pdf, ExtractionOptions, output::json::result_to_output};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let result = extract_pdf(
//!     &std::path::PathBuf::from("document.pdf"),
//!     &ExtractionOptions::default()
//! )?;
//!
//! let output = result_to_output(&result);
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok(())
//! # }
//! ```

use crate::extract::ExtractionResult;
use crate::schema::{
    BlockJson, CellJson, DiagnosticJson, DocumentMetadata, ExtractionQuality, FormFieldJson,
    JavascriptActionJson, LinkJson, Output, OutlineNode, PageJson, RowJson, SignatureJson,
    SpanJson, TableJson, ThreadJson, AttachmentJson, AnnotationJson,
};
use crate::parser::outline::{Outline, DestAnchor};
use serde_json::{json, Value};

/// Convert an `ExtractionResult` to the full JSON `Output` schema.
///
/// This function populates all fields of the `Output` struct according to the
/// schema specification at `docs/research/extraction-output-schema.md`.
///
/// # Arguments
///
/// * `result` - The extraction result from `extract_pdf`
///
/// # Returns
///
/// A fully populated `Output` struct ready for JSON serialization.
///
/// # Document-level fields populated
///
/// - `schema_version`: Always "1.0"
/// - `metadata`: Document metadata (title, author, page count, etc.)
/// - `outline`: Empty until outline extraction is implemented (Phase 7.1)
/// - `threads`: Article thread chains from the extraction result
/// - `attachments`: Embedded file attachments from the extraction result
/// - `signatures`: Digital signature metadata from the extraction result
/// - `form_fields`: AcroForm/XFA fields from the extraction result
/// - `links`: Document-scoped hyperlinks from the extraction result
/// - `pages`: Array of page objects with full schema fields
/// - `extraction_quality`: Aggregate quality metrics
/// - `errors`: All diagnostics converted from string messages
///
/// # Page-level fields populated
///
/// - `page_index`: 0-based index from extraction result
/// - `page_number`: 1-based (page_index + 1)
/// - `page_label`: From /PageLabels if present
/// - `width`, `height`: Page geometry
/// - `rotation`: Page rotation
/// - `page_type`: Classification result
/// - `spans`: Full span array with all fields
/// - `blocks`: Full block array
/// - `tables`: Table structures for table blocks
/// - `annotations`: Empty array until Phase 7.2
pub fn result_to_output(result: &ExtractionResult) -> Output {
    // Convert pages
    let pages: Vec<PageJson> = result
        .pages
        .iter()
        .map(|page| page_result_to_page_json(page))
        .collect();

    // Convert diagnostics strings to DiagnosticJson
    let errors: Vec<DiagnosticJson> = convert_diagnostics(&result.metadata.diagnostics);

    // Compute extraction quality
    let extraction_quality = compute_extraction_quality(result);

    // Build output
    Output {
        schema_version: "1.0",
        metadata: extract_document_metadata(result),
        outline: Vec::new(), // TODO: Extract outline in Phase 7.1
        threads: result.threads.clone(),
        attachments: result.attachments.clone(),
        signatures: result.signatures.clone(),
        form_fields: result.form_fields.clone(),
        links: result.links.clone(),
        pages,
        extraction_quality,
        errors,
    }
}

/// Convert a `PageResult` to a `PageJson` with all schema fields.
fn page_result_to_page_json(page: &crate::extract::PageResult) -> PageJson {
    PageJson {
        page_index: page.index,
        page_number: page.page_number,
        page_label: page.page_label.clone(),
        width: page.width.unwrap_or(0.0),
        height: page.height.unwrap_or(0.0),
        rotation: page.rotation.unwrap_or(0),
        page_type: page.page_type.clone().unwrap_or_else(|| {
            // Determine page_type from content
            if page.spans.is_empty() {
                "blank".to_string()
            } else {
                "text".to_string() // Default to text for now; OCR will set "scanned"
            }
        }),
        spans: page.spans.clone(),
        blocks: page.blocks.clone(),
        tables: convert_tables(&page.tables),
        annotations: Vec::new(), // TODO: Extract annotations in Phase 7.2
    }
}

/// Convert raw table data to `TableJson` schema.
fn convert_tables(raw_tables: &Vec<TableJson>) -> Vec<TableJson> {
    raw_tables
        .iter()
        .map(|table| {
            // Return the table as-is for now
            TableJson {
                id: table.id.clone(),
                bbox: table.bbox,
                rows: Vec::new(), // TODO: Extract rows in Phase 7.4
                header_rows: 0,
                detection_method: "line_based".to_string(),
                continued: false,
                continued_from_prev: false,
                page_index: table.page_index,
            }
        })
        .collect()
}

/// Convert diagnostics strings to `DiagnosticJson` format.
///
/// Since the current extraction stores diagnostics as strings, we parse them
/// to extract code, severity, and page_index when possible.
fn convert_diagnostics(diagnostics: &[String]) -> Vec<DiagnosticJson> {
    diagnostics
        .iter()
        .map(|diag_str| {
            // Try to parse the diagnostic string
            // Format: "CODE: message" or just "message"
            let (code, message) = if let Some(colon_pos) = diag_str.find(':') {
                let code_part = &diag_str[..colon_pos];
                let message_part = &diag_str[colon_pos + 1..].trim();
                (code_part.trim().to_string(), message_part.to_string())
            } else {
                ("UNKNOWN".to_string(), diag_str.clone())
            };

            // Determine severity from code
            let severity = if code.starts_with("ERROR_") || code.contains("ERROR") {
                "error".to_string()
            } else if code.starts_with("WARN_") || code.contains("WARN") {
                "warning".to_string()
            } else {
                "info".to_string()
            };

            DiagnosticJson {
                code,
                message,
                severity,
                page_index: None, // TODO: Extract page_index from diagnostics
                location: None,
                hint: None,
            }
        })
        .collect()
}

/// Compute extraction quality metrics from the extraction result.
fn compute_extraction_quality(result: &ExtractionResult) -> ExtractionQuality {
    // Count pages by type
    let mut scanned_count = 0;
    let mut broken_vector_count = 0;
    let mut total_confidence_sum: f32 = 0.0;
    let mut confidence_span_count = 0;

    for page in &result.pages {
        // Check page type
        if let Some(ref page_type) = page.page_type {
            if page_type == "scanned" {
                scanned_count += 1;
            } else if page_type == "broken_vector" {
                broken_vector_count += 1;
            }
        }

        // Aggregate confidence scores
        for span in &page.spans {
            if let Some(confidence) = span.confidence {
                total_confidence_sum += confidence as f32;
                confidence_span_count += 1;
            }
        }
    }

    // Calculate overall quality
    let page_count = result.pages.len();
    let overall_quality = if page_count == 0 {
        "none".to_string()
    } else {
        let scanned_fraction = scanned_count as f32 / page_count as f32;
        let broken_fraction = broken_vector_count as f32 / page_count as f32;

        if scanned_fraction > 0.5 {
            "medium".to_string()
        } else if broken_fraction > 0.3 {
            "low".to_string()
        } else {
            "high".to_string()
        }
    };

    // Calculate OCR fraction
    let ocr_fraction = if page_count > 0 {
        Some(scanned_count as f32 / page_count as f32)
    } else {
        None
    };

    // Calculate average confidence
    let avg_confidence = if confidence_span_count > 0 {
        Some(total_confidence_sum / confidence_span_count as f32)
    } else {
        None
    };

    // Calculate min confidence
    let mut min_confidence: Option<f32> = None;
    for page in &result.pages {
        for span in &page.spans {
            if let Some(confidence) = span.confidence {
                let conf_f32 = confidence as f32;
                match min_confidence {
                    Some(current_min) => {
                        if conf_f32 < current_min {
                            min_confidence = Some(conf_f32);
                        }
                    }
                    None => min_confidence = Some(conf_f32),
                }
            }
        }
    }

    // Build extraction quality
    let mut quality = ExtractionQuality::new();
    quality.overall_quality = overall_quality;
    quality.ocr_fraction = ocr_fraction;
    quality.avg_confidence = avg_confidence;
    quality.min_confidence = min_confidence;

    quality
}

/// Extract document metadata from the extraction result.
///
/// For now, we use minimal metadata available in ExtractionMetadata.
/// A full implementation would extract title, author, etc. from the PDF's
/// document info dictionary.
fn extract_document_metadata(result: &ExtractionResult) -> DocumentMetadata {
    DocumentMetadata {
        title: None, // TODO: Extract from document info
        author: None, // TODO: Extract from document info
        subject: None, // TODO: Extract from document info
        keywords: None, // TODO: Extract from document info
        creator: None, // TODO: Extract from document info
        producer: None, // TODO: Extract from document info
        creation_date: None, // TODO: Extract from document info
        modification_date: None, // TODO: Extract from document info
        page_count: result.metadata.page_count as u32,
        pdf_version: None, // TODO: Extract from catalog
        is_tagged: false, // TODO: Extract from catalog
        is_encrypted: result.metadata.cache_status.as_ref().map(|s| s.contains("encrypted")).unwrap_or(false),
        conformance: "none".to_string(), // TODO: Detect PDF/A conformance
        contains_javascript: !result.javascript_actions.is_empty(),
        javascript_actions: result.javascript_actions.clone(),
        contains_xfa: false, // TODO: Detect XFA presence
        ocg_present: false, // TODO: Detect OCG presence
        generator: None, // TODO: Heuristic detection
        document_type: "unknown".to_string(), // TODO: Classifier integration (Phase 5.6)
        document_type_confidence: 0.0,
        document_type_reasons: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::{ExtractionMetadata, PageResult};
    use crate::options::{ExtractionOptions, ReceiptsMode};

    #[test]
    fn test_result_to_output_basic() {
        let result = ExtractionResult {
            fingerprint: "test-fingerprint".to_string(),
            pages: vec![],
            metadata: ExtractionMetadata {
                page_count: 0,
                receipts_mode: ReceiptsMode::Off,
                span_count: 0,
                block_count: 0,
                cache_status: None,
                cache_age_seconds: None,
                error_count: 0,
                reading_order_algorithm: None,
                diagnostics: vec![],
                profile_name: None,
                profile_version: None,
                profile_fields: None,
            },
            signatures: vec![],
            form_fields: vec![],
            links: vec![],
            attachments: vec![],
            threads: vec![],
            javascript_actions: vec![],
        };

        let output = result_to_output(&result);

        assert_eq!(output.schema_version, "1.0");
        assert_eq!(output.pages.len(), 0);
        assert_eq!(output.metadata.page_count, 0);
    }

    #[test]
    fn test_page_result_to_page_json() {
        let page = PageResult {
            index: 0,
            page_number: 1,
            page_label: None,
            width: Some(612.0),
            height: Some(792.0),
            rotation: Some(0),
            page_type: Some("text".to_string()),
            spans: vec![],
            blocks: vec![],
            tables: vec![],
            annotations: vec![],
            error: None,
        };

        let page_json = page_result_to_page_json(&page);

        assert_eq!(page_json.page_index, 0);
        assert_eq!(page_json.page_number, 1);
        assert_eq!(page_json.width, 612.0);
        assert_eq!(page_json.height, 792.0);
        assert_eq!(page_json.rotation, 0);
        assert_eq!(page_json.page_type, "text");
    }

    #[test]
    fn test_convert_diagnostics() {
        let diagnostics = vec![
            "FONT_GLYPH_UNMAPPED: Glyph could not be mapped".to_string(),
            "WARN_OCR_LOW_CONFIDENCE: OCR confidence below threshold".to_string(),
            "INFO_FALLBACK_USING_VECTOR: Using vector text".to_string(),
        ];

        let error_json = convert_diagnostics(&diagnostics);

        assert_eq!(error_json.len(), 3);
        assert_eq!(error_json[0].code, "FONT_GLYPH_UNMAPPED");
        assert_eq!(error_json[0].severity, "error");
        assert_eq!(error_json[1].code, "WARN_OCR_LOW_CONFIDENCE");
        assert_eq!(error_json[1].severity, "warning");
        assert_eq!(error_json[2].code, "INFO_FALLBACK_USING_VECTOR");
        assert_eq!(error_json[2].severity, "info");
    }

    #[test]
    fn test_compute_extraction_quality() {
        let result = ExtractionResult {
            fingerprint: "test".to_string(),
            pages: vec![
                PageResult {
                    index: 0,
                    page_number: 1,
                    page_label: None,
                    width: Some(612.0),
                    height: Some(792.0),
                    rotation: Some(0),
                    page_type: Some("text".to_string()),
                    spans: vec![],
                    blocks: vec![],
                    tables: vec![],
                    annotations: vec![],
                    error: None,
                },
                PageResult {
                    index: 1,
                    page_number: 2,
                    page_label: None,
                    width: Some(612.0),
                    height: Some(792.0),
                    rotation: Some(0),
                    page_type: Some("scanned".to_string()),
                    spans: vec![],
                    blocks: vec![],
                    tables: vec![],
                    annotations: vec![],
                    error: None,
                },
            ],
            metadata: ExtractionMetadata {
                page_count: 2,
                receipts_mode: ReceiptsMode::Off,
                span_count: 0,
                block_count: 0,
                cache_status: None,
                cache_age_seconds: None,
                error_count: 0,
                reading_order_algorithm: None,
                diagnostics: vec![],
                profile_name: None,
                profile_version: None,
                profile_fields: None,
            },
            signatures: vec![],
            form_fields: vec![],
            links: vec![],
            attachments: vec![],
            threads: vec![],
            javascript_actions: vec![],
        };

        let quality = compute_extraction_quality(&result);

        assert_eq!(quality.overall_quality, "medium"); // 50% scanned
        assert_eq!(quality.ocr_fraction, Some(0.5));
    }
}
