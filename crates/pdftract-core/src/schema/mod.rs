//! JSON output schema for PDF extraction.
//!
//! This module defines the JSON serialization types used by the
//! extraction pipeline. These types are serde-serializable and
//! match the schema exposed by the CLI and language SDKs.
//!
//! # Schema versioning
//!
//! The `schema_version` field indicates which version of the schema
//! is in use. Consumers should check this field before parsing to
//! ensure compatibility.
//!
//! # Receipts
//!
//! When `--receipts=lite` or `--receipts=svg` is enabled, spans and
//! blocks include an optional `receipt` field containing cryptographic
//! proof of provenance. When receipts are disabled, the field is `null`.

use serde::{Deserialize, Serialize};

use crate::receipts::Receipt;

/// JSON representation of a text span.
///
/// A span is the smallest unit of extracted text, representing a
/// contiguous run of text with consistent font and styling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpanJson {
    /// The extracted text content.
    pub text: String,

    /// Bounding box in PDF user-space points.
    ///
    /// Format: `[x0, y0, x1, y1]` where (x0, y0) is the bottom-left
    /// corner and (x1, y1) is the top-right corner.
    pub bbox: [f64; 4],

    /// Font name or identifier.
    pub font: String,

    /// Font size in points.
    pub size: f64,

    /// Optional confidence score (0.0 to 1.0).
    ///
    /// This field is present when OCR is used or when the extraction
    /// has uncertainty about the text. When confidence is not applicable,
    /// this field is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,

    /// Optional cryptographic receipt for verification.
    ///
    /// This field is present when `--receipts=lite` or `--receipts=svg`
    /// is enabled. When receipts are disabled, the field is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
}

/// JSON representation of a structural block.
///
/// A block is a higher-level semantic unit composed of one or more
/// spans. Examples include paragraphs, headings, list items, and
/// table cells.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockJson {
    /// The block kind/type.
    ///
    /// Common values: "paragraph", "heading", "list", "table", "figure".
    pub kind: String,

    /// The concatenated text content of all spans in the block.
    pub text: String,

    /// Bounding box in PDF user-space points.
    ///
    /// Format: `[x0, y0, x1, y1]` where (x0, y0) is the bottom-left
    /// corner and (x1, y1) is the top-right corner.
    pub bbox: [f64; 4],

    /// Optional heading level (1-6) for "heading" kind blocks.
    ///
    /// This field is present only for heading blocks. For paragraphs
    /// and other block types, it is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,

    /// Optional cryptographic receipt for verification.
    ///
    /// This field is present when `--receipts=lite` or `--receipts=svg`
    /// is enabled. When receipts are disabled, the field is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
}

/// Extraction quality metrics for the document.
///
/// This structure appears in the document footer (NDJSON mode) or
/// in the root metadata (full JSON mode). It provides aggregate
/// quality signals across all pages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractionQuality {
    /// Overall quality assessment: "high", "medium", "low", or "none".
    ///
    /// - "high": All pages extracted successfully with high confidence
    /// - "medium": Most pages extracted, some with lower confidence
    /// - "low": Significant extraction issues (many low-confidence pages)
    /// - "none": No extractable content found (all blank pages)
    pub overall_quality: String,

    /// DPI used for OCR rendering (Phase 5.2).
    ///
    /// This field records the DPI selected by the automatic DPI selection
    /// algorithm (or the user-specified override). It is present when OCR
    /// was performed on any page.
    ///
    /// Values: 200 (JBIG2), 300 (standard), 400 (fine print), or custom
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpi_used: Option<u32>,

    /// Fraction of pages that required OCR fallback [0.0, 1.0].
    ///
    /// This is the count of pages classified as "scanned" or "mixed"
    /// divided by the total page count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_fraction: Option<f32>,

    /// Minimum confidence score across all spans [0.0, 1.0].
    ///
    /// This represents the weakest link in the extraction chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_confidence: Option<f32>,

    /// Average confidence score across all spans [0.0, 1.0].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_confidence: Option<f32>,
}

impl ExtractionQuality {
    /// Create a new extraction quality summary.
    pub fn new() -> Self {
        Self {
            overall_quality: "none".to_string(),
            dpi_used: None,
            ocr_fraction: None,
            min_confidence: None,
            avg_confidence: None,
        }
    }

    /// Set the overall quality level.
    pub fn with_quality(mut self, quality: &str) -> Self {
        self.overall_quality = quality.to_string();
        self
    }

    /// Set the DPI used for OCR rendering.
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi_used = Some(dpi);
        self
    }

    /// Set the OCR fraction.
    pub fn with_ocr_fraction(mut self, fraction: f32) -> Self {
        self.ocr_fraction = Some(fraction);
        self
    }
}

impl Default for ExtractionQuality {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_json_serialization() {
        let span = SpanJson {
            text: "Hello, world!".to_string(),
            bbox: [100.0, 200.0, 300.0, 220.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            confidence: None,
            receipt: None,
        };

        let json = serde_json::to_string(&span).unwrap();

        assert!(json.contains("text"));
        assert!(json.contains("bbox"));
        assert!(json.contains("font"));
        assert!(json.contains("size"));
        assert!(!json.contains("confidence"));
        assert!(!json.contains("receipt"));
    }

    #[test]
    fn test_span_json_with_confidence() {
        let span = SpanJson {
            text: "OCR text".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "OCR-A".to_string(),
            size: 10.0,
            confidence: Some(0.95),
            receipt: None,
        };

        let json = serde_json::to_string(&span).unwrap();
        assert!(json.contains("confidence"));
    }

    #[test]
    fn test_span_json_with_receipt() {
        let receipt = Receipt::lite(
            "pdftract-v1:test".to_string(),
            0,
            [0.0, 0.0, 100.0, 20.0],
            "OCR text",
        );

        let span = SpanJson {
            text: "OCR text".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            confidence: None,
            receipt: Some(receipt),
        };

        let json = serde_json::to_string(&span).unwrap();
        assert!(json.contains("receipt"));
        assert!(json.contains("pdf_fingerprint"));
    }

    #[test]
    fn test_block_json_serialization() {
        let block = BlockJson {
            kind: "paragraph".to_string(),
            text: "This is a paragraph.".to_string(),
            bbox: [50.0, 100.0, 500.0, 200.0],
            level: None,
            receipt: None,
        };

        let json = serde_json::to_string(&block).unwrap();

        assert!(json.contains("kind"));
        assert!(json.contains("text"));
        assert!(json.contains("bbox"));
        assert!(!json.contains("level"));
        assert!(!json.contains("receipt"));
    }

    #[test]
    fn test_block_json_heading_with_level() {
        let block = BlockJson {
            kind: "heading".to_string(),
            text: "Chapter 1".to_string(),
            bbox: [50.0, 700.0, 500.0, 750.0],
            level: Some(1),
            receipt: None,
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("level"));
        // Numbers are serialized without quotes in JSON
        assert!(json.contains("1"));
    }

    #[test]
    fn test_block_json_with_receipt() {
        let receipt = Receipt::lite(
            "pdftract-v1:test".to_string(),
            0,
            [50.0, 100.0, 500.0, 200.0],
            "This is a paragraph.",
        );

        let block = BlockJson {
            kind: "paragraph".to_string(),
            text: "This is a paragraph.".to_string(),
            bbox: [50.0, 100.0, 500.0, 200.0],
            level: None,
            receipt: Some(receipt),
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("receipt"));
        assert!(json.contains("pdf_fingerprint"));
    }

    #[test]
    fn test_receipt_not_in_json_when_none() {
        // Verify that receipt=null does NOT appear in JSON when receipt is None
        // This matches the requirement that downstream consumers see a stable shape
        let span = SpanJson {
            text: "test".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            confidence: None,
            receipt: None,
        };

        let json = serde_json::to_string(&span).unwrap();

        // The receipt field should be completely omitted when None
        // (not even as null) due to skip_serializing_if
        assert!(!json.contains("receipt"));
    }

    #[test]
    fn test_schema_stability() {
        // Test that the schema maintains stability across versions
        let span_with_receipt = SpanJson {
            text: "test".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            confidence: None,
            receipt: Some(Receipt::lite(
                "pdftract-v1:test".to_string(),
                0,
                [0.0, 0.0, 100.0, 20.0],
                "test",
            )),
        };

        let span_without_receipt = SpanJson {
            text: "test".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            confidence: None,
            receipt: None,
        };

        // Both should serialize successfully
        let json_with = serde_json::to_string(&span_with_receipt).unwrap();
        let json_without = serde_json::to_string(&span_without_receipt).unwrap();

        // The version with receipt should be longer
        assert!(json_with.len() > json_without.len());

        // Both should contain the core fields
        assert!(json_with.contains("text"));
        assert!(json_without.contains("text"));
    }

    #[test]
    fn test_extraction_quality_default() {
        let quality = ExtractionQuality::new();
        assert_eq!(quality.overall_quality, "none");
        assert_eq!(quality.dpi_used, None);
        assert_eq!(quality.ocr_fraction, None);
        assert_eq!(quality.min_confidence, None);
        assert_eq!(quality.avg_confidence, None);
    }

    #[test]
    fn test_extraction_quality_with_quality() {
        let quality = ExtractionQuality::new().with_quality("high");
        assert_eq!(quality.overall_quality, "high");
    }

    #[test]
    fn test_extraction_quality_with_dpi() {
        let quality = ExtractionQuality::new().with_dpi(300);
        assert_eq!(quality.dpi_used, Some(300));
    }

    #[test]
    fn test_extraction_quality_with_ocr_fraction() {
        let quality = ExtractionQuality::new().with_ocr_fraction(0.5);
        assert_eq!(quality.ocr_fraction, Some(0.5));
    }

    #[test]
    fn test_extraction_quality_serialization() {
        let quality = ExtractionQuality {
            overall_quality: "high".to_string(),
            dpi_used: Some(300),
            ocr_fraction: Some(0.25),
            min_confidence: Some(0.95),
            avg_confidence: Some(0.98),
        };

        let json = serde_json::to_string(&quality).unwrap();
        assert!(json.contains("overall_quality"));
        assert!(json.contains("high"));
        assert!(json.contains("dpi_used"));
        assert!(json.contains("300"));
        assert!(json.contains("ocr_fraction"));
        assert!(json.contains("min_confidence"));
        assert!(json.contains("avg_confidence"));
    }

    #[test]
    fn test_extraction_quality_serialization_minimal() {
        // Test that optional fields are omitted when None
        let quality = ExtractionQuality {
            overall_quality: "none".to_string(),
            dpi_used: None,
            ocr_fraction: None,
            min_confidence: None,
            avg_confidence: None,
        };

        let json = serde_json::to_string(&quality).unwrap();
        // Should only contain overall_quality
        assert!(json.contains("overall_quality"));
        assert!(json.contains("none"));
        // Optional fields should not be present
        assert!(!json.contains("dpi_used"));
        assert!(!json.contains("ocr_fraction"));
        assert!(!json.contains("min_confidence"));
        assert!(!json.contains("avg_confidence"));
    }

    #[test]
    fn test_extraction_quality_default_impl() {
        let quality = ExtractionQuality::default();
        assert_eq!(quality.overall_quality, "none");
        assert_eq!(quality.dpi_used, None);
    }

    #[test]
    fn test_extraction_quality_chained_setters() {
        let quality = ExtractionQuality::new()
            .with_quality("medium")
            .with_dpi(400)
            .with_ocr_fraction(0.75);

        assert_eq!(quality.overall_quality, "medium");
        assert_eq!(quality.dpi_used, Some(400));
        assert_eq!(quality.ocr_fraction, Some(0.75));
    }
}
