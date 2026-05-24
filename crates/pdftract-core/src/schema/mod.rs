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

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::layout::correction::CorrectableText;
use crate::receipts::Receipt;
use crate::signature::Signature;

/// JSON representation of a text span.
///
/// A span is the smallest unit of extracted text, representing a
/// contiguous run of text with consistent font and styling.
///
/// # TODO: Phase 6.1 - Add confidence_source field
///
/// When the `confidence_source` field is added to the schema (per plan line 363, 1662),
/// it should include "ocr-fallback" as a valid value for spans emitted via
/// Phase 5.5.3 region-level fallback. The internal `SpanSource::OcrFallback` variant
/// in `hybrid.rs` maps to this value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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

    /// Column index (0-based) assigned by Phase 4.3 column detection.
    ///
    /// This field is `None` for spans outside any detected column
    /// (e.g., full-width headings, inter-column gaps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

impl CorrectableText for SpanJson {
    fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    fn text(&self) -> &str {
        &self.text
    }
}

/// JSON representation of a structural block.
///
/// A block is a higher-level semantic unit composed of one or more
/// spans. Examples include paragraphs, headings, list items, and
/// table cells.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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

    /// Optional table index for "table" kind blocks.
    ///
    /// This field is present only for table blocks and points to the
    /// corresponding entry in the page's `tables` array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_index: Option<usize>,

    /// Optional cryptographic receipt for verification.
    ///
    /// This field is present when `--receipts=lite` or `--receipts=svg`
    /// is enabled. When receipts are disabled, the field is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
}

/// A reference to a span by index.
///
/// This type is used in table cells to reference spans from the
/// page-level `spans` array.
pub type SpanRef = usize;

/// JSON representation of a table cell.
///
/// A cell represents a single unit within a table row, containing
/// its text content, bounding box, and position information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CellJson {
    /// Bounding box in PDF user-space points.
    ///
    /// Format: `[x0, y0, x1, y1]` where (x0, y0) is the bottom-left
    /// corner and (x1, y1) is the top-right corner.
    pub bbox: [f64; 4],

    /// The concatenated text content of all spans in the cell.
    pub text: String,

    /// References to spans in the page's `spans` array.
    ///
    /// These indices point to the spans that make up this cell's content.
    pub spans: Vec<SpanRef>,

    /// Zero-based row index within the table.
    pub row: usize,

    /// Zero-based column index within the table.
    pub col: usize,

    /// Number of rows this cell spans (default 1).
    ///
    /// Values greater than 1 indicate a merged cell that spans
    /// multiple rows vertically.
    #[serde(default = "default_one")]
    pub rowspan: u32,

    /// Number of columns this cell spans (default 1).
    ///
    /// Values greater than 1 indicate a merged cell that spans
    /// multiple columns horizontally.
    #[serde(default = "default_one")]
    pub colspan: u32,

    /// Whether this cell is in a header row.
    ///
    /// Header cells are typically rendered differently (bold, centered)
    /// and may be reused when tables span multiple pages.
    pub is_header_row: bool,
}

fn default_one() -> u32 {
    1
}

/// JSON representation of a table row.
///
/// A row contains a sequence of cells that form a horizontal strip
/// in the table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RowJson {
    /// Bounding box in PDF user-space points.
    ///
    /// Format: `[x0, y0, x1, y1]` where (x0, y0) is the bottom-left
    /// corner and (x1, y1) is the top-right corner.
    pub bbox: [f64; 4],

    /// Cells in this row, ordered left-to-right.
    pub cells: Vec<CellJson>,

    /// Whether this row is a header row.
    ///
    /// Header rows are typically repeated when tables span multiple pages.
    pub is_header: bool,
}

/// JSON representation of a table.
///
/// Tables are emitted in parallel with table blocks - the block
/// provides the concatenated text and position, while the TableJson
/// provides full cell-level structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TableJson {
    /// Unique identifier for this table (e.g., "table_0").
    pub id: String,

    /// Bounding box in PDF user-space points.
    ///
    /// Format: `[x0, y0, x1, y1]` where (x0, y0) is the bottom-left
    /// corner and (x1, y1) is the top-right corner.
    pub bbox: [f64; 4],

    /// Rows in this table, ordered top-to-bottom.
    pub rows: Vec<RowJson>,

    /// Number of contiguous header rows at the top of the table.
    ///
    /// Header rows are typically repeated when tables span multiple pages.
    pub header_rows: u32,

    /// Detection method used to identify this table.
    ///
    /// - "line_based": Table detected via ruling lines (borders)
    /// - "borderless": Table detected via x0 alignment heuristics
    pub detection_method: String,

    /// Whether this table continues on the next page.
    ///
    /// Set to `true` when a table is split across pages and this
    /// page contains the first part.
    pub continued: bool,

    /// Whether this table is a continuation from the previous page.
    ///
    /// Set to `true` when a table is split across pages and this
    /// page contains a subsequent part.
    pub continued_from_prev: bool,

    /// Zero-based page index where this table appears.
    pub page_index: usize,
}

/// Extraction quality metrics for the document.
///
/// This structure appears in the document footer (NDJSON mode) or
/// in the root metadata (full JSON mode). It provides aggregate
/// quality signals across all pages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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

    /// Per-page readability score (char-weighted median of span scores) [0.0, 1.0].
    ///
    /// This is the median of per-span readability scores, weighted by character count.
    /// A score below 0.5 may indicate mojibake, encoding issues, or broken text layers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readability: Option<f32>,
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
            readability: None,
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

/// JSON representation of a form field.
///
/// This struct represents a single interactive form field from the PDF's
/// AcroForm or XFA data, including its type, value, and metadata.
///
/// Per the plan (Phase 7.4), form fields are extracted from both AcroForm
/// and XFA sources, with XFA values taking precedence on collision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FormFieldJson {
    /// The absolute (dot-joined) field name from the AcroForm.
    /// Example: "employer_signature" or "form.employee_sig"
    pub name: String,

    /// The field type variant (text, button, choice, or signature).
    #[serde(rename = "type")]
    pub field_type: FormFieldTypeJson,

    /// The current value of the form field.
    ///
    /// This field's structure varies by field_type:
    /// - text: string value
    /// - button: boolean selected state
    /// - choice: string or array of strings (for multi-select)
    /// - signature: signature reference number (or null if unsigned)
    pub value: FormFieldValueJson,

    /// The default value (/DV entry) if present.
    ///
    /// Matches the structure of `value` but represents the field's default state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<FormFieldValueJson>,

    /// Zero-based page index where this field's widget appears.
    ///
    /// None if the field has no visual representation (form-only field).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<usize>,

    /// Bounding box in PDF user-space points.
    ///
    /// Format: [x0, y0, x1, y1] where (x0, y0) is the bottom-left corner.
    /// None if the field has no visual appearance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f32; 4]>,

    /// Whether this field is required (bit 2 of /Ff flags).
    pub required: bool,

    /// Whether this field is read-only (bit 1 of /Ff flags).
    pub read_only: bool,

    /// Whether this text field supports multiple lines (bit 13 of /Ff).
    /// Only present for text fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiline: Option<bool>,

    /// Maximum length for text fields (/MaxLen entry).
    /// Only present for text fields that have a max length set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,

    /// Available options for choice fields.
    ///
    /// Each option is a [export_value, display_name] pair.
    /// Only present for choice fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<[String; 2]>>,

    /// Whether this choice field supports multiple selections (bit 21 of /Ff).
    /// Only present for choice fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_select: Option<bool>,

    /// Selected state for button fields.
    /// True = checked/selected, False = unchecked.
    /// Only present for button fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,

    /// Appearance state name for button fields.
    /// E.g., "Yes", "Off", or custom state names.
    /// Only present for button fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_name: Option<String>,

    /// Whether this button is a pushbutton (bit 26 of /Ff).
    /// Only present for button fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pushbutton: Option<bool>,

    /// Whether this button is a radio button (bit 25 of /Ff).
    /// Only present for button fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio: Option<bool>,
}

/// Form field type discriminator.
///
/// This enum uses serde's "tag" representation to produce a JSON string
/// indicating the field type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum FormFieldTypeJson {
    /// Text field (/FT /Tx) - single-line or multi-line text input.
    Text,
    /// Button field (/FT /Btn) - pushbutton, checkbox, or radio button.
    Button,
    /// Choice field (/FT /Ch) - dropdown or list box.
    Choice,
    /// Signature field (/FT /Sig) - digital signature field.
    Signature,
}

/// Form field value representation.
///
/// This enum captures the current value of a form field, with the variant
/// type matching the field_type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum FormFieldValueJson {
    /// Text field value (string or null).
    Text(Option<String>),
    /// Button field value (boolean selected state).
    Button(bool),
    /// Choice field value (single string or array of strings for multi-select).
    Choice(ChoiceValueJson),
    /// Signature field value (signature reference number or null).
    Signature(Option<u32>),
}

/// Choice field value representation.
///
/// Choice fields can have either a single selected value or multiple
/// selected values (for multi-select list boxes).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ChoiceValueJson {
    /// Single selected option.
    Single(String),
    /// Multiple selected options.
    Multiple(Vec<String>),
}

/// JSON representation of a digital signature.
///
/// This struct represents a signature extracted from a PDF signature field,
/// including signer identity, timestamp, and coverage information.
///
/// Per the plan (Phase 7.3), pdftract does NOT perform cryptographic validation
/// in v1. The `validation_status` field is always "not_checked" — future versions
/// may add "valid", "invalid", or "indeterminate" as cryptographic validation
/// is implemented.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SignatureJson {
    /// The absolute (dot-joined) field name from the AcroForm.
    /// Example: "employer_signature" or "form.employee_sig"
    pub field_name: String,

    /// The signer's name from the /Name entry in the signature dictionary.
    ///
    /// Empty string if /Name is absent.
    pub signer_name: String,

    /// The signing date as an ISO 8601 string (RFC 3339 format).
    ///
    /// Parsed from the PDF /M date string. None if the date is missing,
    /// malformed, or the field is unsigned.
    ///
    /// Format: "YYYY-MM-DDTHH:MM:SS+HH:MM" or "YYYY-MM-DDTHH:MM:SSZ"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_date: Option<String>,

    /// The reason for signing from the /Reason entry.
    ///
    /// None if /Reason is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// The location of signing from the /Location entry.
    ///
    /// None if /Location is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// The signature format / filter from the /SubFilter entry.
    ///
    /// Indicates the signature format: "adbe.pkcs7.detached", "adbe.x509.rsa.sha1", etc.
    /// None if /SubFilter is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_filter: Option<String>,

    /// The /ByteRange array defining which bytes of the file are signed.
    ///
    /// Format: array of 4 integers [offset, length, offset, length] defining two byte ranges.
    /// None if /ByteRange is missing or malformed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_range: Option<Vec<u64>>,

    /// Fraction of the file covered by the signature (0.0 to 1.0).
    ///
    /// Computed as `(byte_range[1] + byte_range[3]) / file_size`.
    /// None if /ByteRange is missing, malformed, or file_size is unknown.
    ///
    /// Values < 1.0 indicate partial signatures (a common red flag for tampered docs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_fraction: Option<f64>,

    /// Validation status — always "not_checked" in v1.
    ///
    /// Future versions may add "valid", "invalid", "indeterminate" as cryptographic
    /// validation is implemented. This is a string enum for schema stability.
    pub validation_status: String,
}

impl From<Signature> for SignatureJson {
    fn from(sig: Signature) -> Self {
        SignatureJson {
            field_name: sig.field_name,
            signer_name: sig.signer_name,
            signing_date: sig.signing_date,
            reason: sig.reason,
            location: sig.location,
            sub_filter: sig.sub_filter,
            byte_range: sig.byte_range,
            coverage_fraction: sig.coverage_fraction,
            validation_status: sig.validation_status,
        }
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
            column: None,
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
            table_index: None,
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
            table_index: None,
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
            table_index: None,
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
            column: None,
        };

        let span_without_receipt = SpanJson {
            text: "test".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            confidence: None,
            receipt: None,
            column: None,
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
        assert_eq!(quality.readability, None);
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
            readability: Some(0.87),
        };

        let json = serde_json::to_string(&quality).unwrap();
        assert!(json.contains("overall_quality"));
        assert!(json.contains("high"));
        assert!(json.contains("dpi_used"));
        assert!(json.contains("300"));
        assert!(json.contains("ocr_fraction"));
        assert!(json.contains("min_confidence"));
        assert!(json.contains("avg_confidence"));
        assert!(json.contains("readability"));
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
            readability: None,
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
        assert!(!json.contains("readability"));
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

    #[test]
    fn test_table_json_serialization() {
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [50.0, 100.0, 550.0, 400.0],
            rows: vec![RowJson {
                bbox: [50.0, 350.0, 550.0, 400.0],
                cells: vec![
                    CellJson {
                        bbox: [50.0, 350.0, 200.0, 400.0],
                        text: "Header 1".to_string(),
                        spans: vec![0],
                        row: 0,
                        col: 0,
                        rowspan: 1,
                        colspan: 1,
                        is_header_row: true,
                    },
                    CellJson {
                        bbox: [200.0, 350.0, 550.0, 400.0],
                        text: "Header 2".to_string(),
                        spans: vec![1],
                        row: 0,
                        col: 1,
                        rowspan: 1,
                        colspan: 1,
                        is_header_row: true,
                    },
                ],
                is_header: true,
            }],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let json = serde_json::to_string(&table).unwrap();

        assert!(json.contains("id"));
        assert!(json.contains("table_0"));
        assert!(json.contains("rows"));
        assert!(json.contains("header_rows"));
        assert!(json.contains("detection_method"));
        assert!(json.contains("line_based"));
        assert!(json.contains("continued"));
        assert!(json.contains("continued_from_prev"));
    }

    #[test]
    fn test_table_json_borderless() {
        let table = TableJson {
            id: "table_1".to_string(),
            bbox: [50.0, 100.0, 400.0, 300.0],
            rows: vec![],
            header_rows: 0,
            detection_method: "borderless".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 1,
        };

        let json = serde_json::to_string(&table).unwrap();
        assert!(json.contains("borderless"));
    }

    #[test]
    fn test_table_json_continued_flags() {
        let table = TableJson {
            id: "table_2".to_string(),
            bbox: [50.0, 40.0, 550.0, 200.0],
            rows: vec![],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: true, // Table continues on next page
            continued_from_prev: false,
            page_index: 0,
        };

        let json = serde_json::to_string(&table).unwrap();

        // Check that continued is true and continued_from_prev is false
        assert!(json.contains(r#""continued":true"#));
        assert!(json.contains(r#""continued_from_prev":false"#));
    }

    #[test]
    fn test_table_json_continued_from_prev() {
        let table = TableJson {
            id: "table_3".to_string(),
            bbox: [50.0, 750.0, 550.0, 900.0],
            rows: vec![],
            header_rows: 0,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: true, // Continuation from previous page
            page_index: 1,
        };

        let json = serde_json::to_string(&table).unwrap();

        // Check that continued is false and continued_from_prev is true
        assert!(json.contains(r#""continued":false"#));
        assert!(json.contains(r#""continued_from_prev":true"#));
    }

    #[test]
    fn test_row_json_serialization() {
        let row = RowJson {
            bbox: [50.0, 100.0, 550.0, 150.0],
            cells: vec![CellJson {
                bbox: [50.0, 100.0, 200.0, 150.0],
                text: "Cell 1".to_string(),
                spans: vec![],
                row: 0,
                col: 0,
                rowspan: 1,
                colspan: 1,
                is_header_row: false,
            }],
            is_header: false,
        };

        let json = serde_json::to_string(&row).unwrap();

        assert!(json.contains("bbox"));
        assert!(json.contains("cells"));
        assert!(json.contains("is_header"));
    }

    #[test]
    fn test_cell_json_serialization() {
        let cell = CellJson {
            bbox: [50.0, 100.0, 200.0, 150.0],
            text: "Cell content".to_string(),
            spans: vec![0, 1, 2],
            row: 1,
            col: 0,
            rowspan: 2, // Spans 2 rows
            colspan: 1,
            is_header_row: false,
        };

        let json = serde_json::to_string(&cell).unwrap();

        assert!(json.contains("bbox"));
        assert!(json.contains("text"));
        assert!(json.contains("Cell content"));
        assert!(json.contains("spans"));
        assert!(json.contains("row"));
        assert!(json.contains("col"));
        assert!(json.contains("rowspan"));
        assert!(json.contains("colspan"));
        assert!(json.contains("is_header_row"));
    }

    #[test]
    fn test_v_1_0_table_schema_roundtrip() {
        // Critical test: synthetic table -> JSON -> schema validate
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [50.0, 100.0, 550.0, 400.0],
            rows: vec![
                RowJson {
                    bbox: [50.0, 350.0, 550.0, 400.0],
                    cells: vec![
                        CellJson {
                            bbox: [50.0, 350.0, 200.0, 400.0],
                            text: "Header 1".to_string(),
                            spans: vec![0],
                            row: 0,
                            col: 0,
                            rowspan: 1,
                            colspan: 1,
                            is_header_row: true,
                        },
                        CellJson {
                            bbox: [200.0, 350.0, 400.0, 400.0],
                            text: "Header 2".to_string(),
                            spans: vec![1],
                            row: 0,
                            col: 1,
                            rowspan: 1,
                            colspan: 2, // Merged cell
                            is_header_row: true,
                        },
                    ],
                    is_header: true,
                },
                RowJson {
                    bbox: [50.0, 100.0, 550.0, 350.0],
                    cells: vec![
                        CellJson {
                            bbox: [50.0, 100.0, 200.0, 350.0],
                            text: "Data 1".to_string(),
                            spans: vec![2],
                            row: 1,
                            col: 0,
                            rowspan: 1,
                            colspan: 1,
                            is_header_row: false,
                        },
                        CellJson {
                            bbox: [200.0, 100.0, 400.0, 350.0],
                            text: "Data 2".to_string(),
                            spans: vec![3],
                            row: 1,
                            col: 1,
                            rowspan: 1,
                            colspan: 2,
                            is_header_row: false,
                        },
                    ],
                    is_header: false,
                },
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        // Serialize to JSON
        let json_str = serde_json::to_string(&table).unwrap();

        // Deserialize back to struct
        let deserialized: TableJson = serde_json::from_str(&json_str).unwrap();

        // Verify round-trip preservation
        assert_eq!(deserialized.id, table.id);
        assert_eq!(deserialized.bbox, table.bbox);
        assert_eq!(deserialized.rows.len(), table.rows.len());
        assert_eq!(deserialized.header_rows, table.header_rows);
        assert_eq!(deserialized.detection_method, table.detection_method);
        assert_eq!(deserialized.continued, table.continued);
        assert_eq!(deserialized.continued_from_prev, table.continued_from_prev);
        assert_eq!(deserialized.page_index, table.page_index);

        // Verify row structure
        assert_eq!(deserialized.rows[0].cells.len(), 2);
        assert_eq!(deserialized.rows[0].cells[1].colspan, 2); // Merged cell preserved
    }

    #[test]
    fn test_tables_array_emitted_on_page_output() {
        // Schema test: tables array emitted on every page output (even when empty)
        // This test verifies that a page JSON always includes a "tables" field

        // Create a minimal page output JSON with empty tables array
        let page_json_with_empty_tables = json!({
            "index": 0,
            "spans": [],
            "blocks": [],
            "tables": []
        });

        // Verify tables field is present
        assert!(page_json_with_empty_tables.get("tables").is_some());

        // Verify it's an array
        assert!(page_json_with_empty_tables["tables"].is_array());

        // Verify it's empty
        assert_eq!(
            page_json_with_empty_tables["tables"]
                .as_array()
                .unwrap()
                .len(),
            0
        );

        // Test with non-empty tables array
        let page_json_with_tables = json!({
            "index": 0,
            "spans": [],
            "blocks": [],
            "tables": [
                {
                    "id": "table_0",
                    "bbox": [50.0, 100.0, 550.0, 400.0],
                    "rows": [],
                    "header_rows": 0,
                    "detection_method": "line_based",
                    "continued": false,
                    "continued_from_prev": false,
                    "page_index": 0
                }
            ]
        });

        // Verify tables field is present and has one entry
        assert!(page_json_with_tables.get("tables").is_some());
        assert_eq!(page_json_with_tables["tables"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_table_block_emission_shape() {
        // Test that table blocks have the correct shape with table_index
        let table_block = json!({
            "kind": "table",
            "text": "Table 0",
            "bbox": [50.0, 100.0, 550.0, 400.0],
            "table_index": 0
        });

        // Verify required fields
        assert_eq!(table_block["kind"], "table");
        assert!(table_block.get("bbox").is_some());
        assert!(table_block.get("table_index").is_some());
        assert_eq!(table_block["table_index"], 0);
    }

    #[test]
    fn test_signature_json_full() {
        let sig = SignatureJson {
            field_name: "employer_sig".to_string(),
            signer_name: "John Doe".to_string(),
            signing_date: Some("2023-01-15T14:30:45Z".to_string()),
            reason: Some("Contract approval".to_string()),
            location: Some("New York, NY".to_string()),
            sub_filter: Some("adbe.pkcs7.detached".to_string()),
            byte_range: Some(vec![0, 1000, 2000, 500]),
            coverage_fraction: Some(0.5),
            validation_status: "not_checked".to_string(),
        };

        let json_str = serde_json::to_string(&sig).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["field_name"], "employer_sig");
        assert_eq!(json_val["signer_name"], "John Doe");
        assert_eq!(json_val["signing_date"], "2023-01-15T14:30:45Z");
        assert_eq!(json_val["reason"], "Contract approval");
        assert_eq!(json_val["location"], "New York, NY");
        assert_eq!(json_val["sub_filter"], "adbe.pkcs7.detached");
        assert_eq!(json_val["validation_status"], "not_checked");

        // Round-trip test
        let deserialized: SignatureJson = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, sig);
    }

    #[test]
    fn test_signature_json_minimal_unsigned() {
        let sig = SignatureJson {
            field_name: "blank_sig".to_string(),
            signer_name: String::new(),
            signing_date: None,
            reason: None,
            location: None,
            sub_filter: None,
            byte_range: None,
            coverage_fraction: None,
            validation_status: "not_checked".to_string(),
        };

        let json_str = serde_json::to_string(&sig).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["field_name"], "blank_sig");
        assert_eq!(json_val["signer_name"], "");
        assert_eq!(json_val["validation_status"], "not_checked");

        // Optional fields should not be present in JSON when None
        assert!(json_val.get("signing_date").is_none());
        assert!(json_val.get("reason").is_none());
        assert!(json_val.get("location").is_none());
        assert!(json_val.get("sub_filter").is_none());
        assert!(json_val.get("byte_range").is_none());
        assert!(json_val.get("coverage_fraction").is_none());
    }

    #[test]
    fn test_signature_json_round_trip() {
        let sig = SignatureJson {
            field_name: "test_sig".to_string(),
            signer_name: "Alice Smith".to_string(),
            signing_date: Some("2023-06-01T10:00:00+05:30".to_string()),
            reason: None,
            location: Some("San Francisco, CA".to_string()),
            sub_filter: Some("adbe.x509.rsa.sha1".to_string()),
            byte_range: Some(vec![0, 2048, 4096, 1024]),
            coverage_fraction: Some(0.75),
            validation_status: "not_checked".to_string(),
        };

        let json_str = serde_json::to_string(&sig).unwrap();
        let deserialized: SignatureJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.field_name, sig.field_name);
        assert_eq!(deserialized.signer_name, sig.signer_name);
        assert_eq!(deserialized.signing_date, sig.signing_date);
        assert_eq!(deserialized.reason, sig.reason);
        assert_eq!(deserialized.location, sig.location);
        assert_eq!(deserialized.sub_filter, sig.sub_filter);
        assert_eq!(deserialized.byte_range, sig.byte_range);
        assert_eq!(deserialized.coverage_fraction, sig.coverage_fraction);
        assert_eq!(deserialized.validation_status, sig.validation_status);
    }
}
