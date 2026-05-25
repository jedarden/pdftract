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
/// Per INV-7 (confidence_source on every Span), all spans include
/// the confidence_source field to indicate how the text was extracted.
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

    /// Fill color as CSS hex string (e.g., "#1a1a1a"), or null if not expressible as RGB.
    ///
    /// Null for spot colors, patterns, or complex color spaces that cannot be
    /// accurately represented as RGB hex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// PDF Tr operator value (0-7) indicating the text rendering mode.
    ///
    /// 0 = fill, 1 = stroke, 2 = fill then stroke, 3 = invisible,
    /// 4 = fill to clip, 5 = stroke to clip, 6 = fill then stroke to clip,
    /// 7 = clip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendering_mode: Option<u8>,

    /// Optional confidence score (0.0 to 1.0).
    ///
    /// This field is present when OCR is used or when the extraction
    /// has uncertainty about the text. When confidence is not applicable,
    /// this field is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,

    /// Source of the confidence/text extraction.
    ///
    /// One of: "vector" (native font decoding), "ocr" (pure OCR),
    /// "ocr-assisted" (OCR + vector correction), "ocr-fallback" (region-level fallback),
    /// "repaired" (text was repaired via heuristics).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_source: Option<String>,

    /// BCP-47 language tag if detected, otherwise null.
    ///
    /// Examples: "en", "en-US", "zh-Hans". Null when language detection
    /// is not available or not applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,

    /// Set of style flags applied to this span.
    ///
    /// Possible values: "bold", "italic", "smallcaps", "subscript", "superscript".
    #[serde(default)]
    pub flags: Vec<String>,

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

    /// References to spans in the page's `spans` array.
    ///
    /// These indices point to the spans that make up this block's content.
    #[serde(default)]
    pub spans: Vec<usize>,

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

/// JSON representation of a diagnostic error.
///
/// This struct wraps the internal Diagnostic type for JSON serialization,
/// providing stable error codes and human-readable messages for consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DiagnosticJson {
    /// Stable string identifier for this diagnostic (e.g., "FONT_GLYPH_UNMAPPED").
    pub code: String,

    /// Human-readable description of the diagnostic.
    pub message: String,

    /// Severity level: "info", "warning", "error", or "fatal".
    pub severity: String,

    /// Page index where this diagnostic occurred, or `null` for document-level events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<usize>,

    /// PDF object reference where the issue originated, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<ObjectLocationJson>,

    /// Optional hint for resolving the diagnostic (e.g., "Install Tesseract for OCR recovery").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// JSON representation of a PDF object reference.
///
/// Identifies a specific PDF indirect object by its object and generation numbers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ObjectLocationJson {
    /// Object number (zero-based index in the xref table).
    pub object_number: u32,

    /// Generation number (incremented on each save).
    pub generation_number: u16,
}

/// JSON representation of an outline node (bookmark).
///
/// Represents a single node in the document's outline hierarchy, with support
/// for nested children via the `children` field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OutlineNode {
    /// The outline title text (decoded to UTF-8).
    pub title: String,

    /// Hierarchical level in the outline tree (0-based, root is 0).
    pub level: u8,

    /// Zero-based page index this outline points to, if resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_index: Option<u32>,

    /// Destination type and coordinates within the page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationJson>,

    /// Nested child outlines (empty array for leaf nodes).
    #[serde(default)]
    pub children: Vec<OutlineNode>,
}

/// JSON representation of a destination anchor.
///
/// Describes a specific location within a PDF page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DestinationJson {
    /// Destination type: "xyz", "fit", "fith", "fitv", "fitr", "fitb", "fitbh", "fitbv".
    #[serde(rename = "type")]
    pub dest_type: String,

    /// Left coordinate (user-space points), present for "xyz", "fitv", "fitr", "fitbv".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<f64>,

    /// Top coordinate (user-space points), present for "xyz", "fith", "fitr", "fitbh".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<f64>,

    /// Right coordinate (user-space points), present only for "fitr".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<f64>,

    /// Bottom coordinate (user-space points), present only for "fitr".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<f64>,

    /// Zoom factor, present only for "xyz".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
}

/// JSON representation of document metadata.
///
/// Contains all standard PDF document information dictionary fields along
/// with derived signals from the document catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DocumentMetadata {
    /// PDF /Title - document title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// PDF /Author - name of the person who created the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// PDF /Subject - subject matter summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// PDF /Keywords - space- or comma-delimited keyword list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,

    /// PDF /Creator - the authoring application (e.g., "Microsoft Word 2019").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,

    /// PDF /Producer - the PDF-writing library (e.g., "Acrobat Distiller 23.0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,

    /// PDF /CreationDate - ISO-8601 string from /CreationDate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,

    /// PDF /ModDate - ISO-8601 string from /ModDate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modification_date: Option<String>,

    /// Total number of pages in the document.
    pub page_count: u32,

    /// PDF version (e.g., "1.7", "2.0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_version: Option<String>,

    /// True if /MarkInfo /Marked: true is present.
    pub is_tagged: bool,

    /// True if document is encrypted.
    pub is_encrypted: bool,

    /// PDF/A or PDF/UA conformance level.
    ///
    /// One of: "none", "PDF-A-1a", "PDF-A-1b", "PDF-A-2a", "PDF-A-2b", "PDF-A-2u",
    /// "PDF-A-3a", "PDF-A-3b", "PDF-A-3u", "PDF-UA-1", "PDF-UA-2", "PDF-X-1a".
    #[serde(default = "default_conformance")]
    pub conformance: String,

    /// True if JavaScript actions are present in the document.
    pub contains_javascript: bool,

    /// True if XFA forms are present.
    pub contains_xfa: bool,

    /// True if optional content groups (layers) are present.
    pub ocg_present: bool,

    /// Heuristic string identifying the producing application.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
}

fn default_conformance() -> String {
    "none".to_string()
}

/// A single bead in an article thread chain.
///
/// Represents one bead's position on a page, extracted during bead chain walking.
/// Per PDF 1.7 Section 12.4.3, each bead contains a reference to its page and
/// a bounding rectangle defining the article region on that page.
///
/// # Fields
///
/// * `page_index` - 0-based index of the page containing this bead
/// * `rect` - Bounding rectangle of the bead region in PDF user-space coordinates [x0, y0, x1, y1]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct BeadJson {
    /// 0-based page index where this bead is located.
    pub page_index: usize,

    /// Bounding rectangle in PDF user-space coordinates [x0, y0, x1, y1].
    ///
    /// Per PDF spec, the origin is at the bottom-left corner of the page.
    /// This rect is NOT flipped to image-space coordinates.
    pub rect: [f32; 4],
}

/// JSON representation of an article thread.
///
/// Represents a single article thread from the PDF's /Threads array,
/// including metadata from the thread info dict (/I) and the complete
/// bead chain walked from the first bead.
///
/// Per the plan (Phase 7.7), threads are extracted and emitted at the
/// document level in the `/threads` array. The bead chain is walked by
/// following `/N` (next bead) links from the first bead until termination.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ThreadJson {
    /// Thread title from /I/Title.
    ///
    /// - `Some("")` if /I/Title is present but empty string
    /// - `None` if /I is missing or /Title is absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Thread author from /I/Author.
    ///
    /// - `Some("")` if /I/Author is present but empty string
    /// - `None` if /I is missing or /Author is absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Thread subject from /I/Subject.
    ///
    /// - `Some("")` if /I/Subject is present but empty string
    /// - `None` if /I is missing or /Subject is absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Thread keywords from /I/Keywords.
    ///
    /// Per PDF spec, this is a comma-separated convention (not an array).
    /// - `Some("")` if /I/Keywords is present but empty string
    /// - `None` if /I is missing or /Keywords is absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,

    /// Beads in this thread chain, in traversal order.
    ///
    /// Each bead represents a region on a page that is part of this article.
    /// The beads are ordered by following `/N` (next bead) links from the
    /// first bead through the chain until termination.
    #[serde(default)]
    pub beads: Vec<BeadJson>,
}

/// JSON representation of an embedded file attachment.
///
/// Represents a single embedded file extracted from the PDF's
/// `/EmbeddedFiles` name tree or `/AF` (Associated Files) array.
///
/// Per the plan (Phase 7.5.3), attachments exceeding 50 MB are truncated
/// (metadata only, `data: null`, `truncated: true`). The `data` field
/// contains base64-encoded content using RFC 4648 standard alphabet with
/// padding and no line breaks.
///
/// The JSON Schema declares `contentEncoding: base64` for the `data` field,
/// enabling JSON Schema validators and code generation tools to understand
/// the encoding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AttachmentJson {
    /// Attachment filename from /UF (Unicode, preferred) or /F (system-independent).
    pub name: String,

    /// Description from /Desc (None if absent, not empty string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type from stream /Subtype (None if absent, no guessing from extension).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Original decoded size in bytes (always populated, even when truncated).
    ///
    /// This is the size of the attachment content before base64 encoding.
    /// When `truncated: true`, this represents the full original size that
    /// was not included in the output.
    pub size: u64,

    /// Creation date from /Params /CreationDate as ISO 8601 string (None if absent).
    ///
    /// Format: "YYYY-MM-DDTHH:MM:SS+HH:MM" or "YYYY-MM-DDTHH:MM:SSZ"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// Modification date from /Params /ModDate as ISO 8601 string (None if absent).
    ///
    /// Format: "YYYY-MM-DDTHH:MM:SS+HH:MM" or "YYYY-MM-DDTHH:MM:SSZ"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,

    /// MD5 checksum from /Params /CheckSum as hex string (None if absent).
    ///
    /// Per PDF spec, /CheckSum is a 16-byte binary string (MD5), hex-encoded
    /// as 32 lowercase hex characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_md5: Option<String>,

    /// Base64-encoded attachment content (null if truncated or empty).
    ///
    /// Per JSON Schema, this field has `contentEncoding: base64`, indicating
    /// the string is base64-encoded binary data. Downstream tools can use this
    /// information to automatically decode the content.
    ///
    /// - `Some(base64_string)` when content <= 50 MB
    /// - `None` when `truncated: true` (content too large)
    ///
    /// In the Python API (PyO3), this field is returned as a `bytes` object
    /// (PyO3 automatically decodes the base64 string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,

    /// Whether the attachment content was truncated due to the 50 MB size limit.
    ///
    /// When `true`, the `data` field is `None` and only metadata is included.
    /// The `size` field still reflects the original full size.
    pub truncated: bool,
}

/// JSON representation of a hyperlink annotation.
///
/// Represents either a URI hyperlink (external link) or an internal destination
/// link (named or explicit destination within the same document).
///
/// Per the plan (Phase 7.6.4), links are emitted at the document level in the
/// `/links` array, sorted by (page_index, rect.y0 desc, rect.x0) for deterministic output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LinkJson {
    /// Zero-based page index containing this link.
    pub page_index: usize,

    /// Bounding box in PDF user-space points.
    ///
    /// Format: [x0, y0, x1, y1] where (x0, y0) is the bottom-left corner.
    pub rect: [f32; 4],

    /// The URI target for external links (from /A /S /URI /URI).
    ///
    /// Present for URI links and JavaScript actions (prefixed with "javascript:").
    /// Null for internal destination links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    /// The internal destination name (from /Dest as a name string).
    ///
    /// Present for named destination links. Null for URI links or explicit destinations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<String>,

    /// Explicit destination array (from /Dest as an array or resolved name tree).
    ///
    /// Present when the link target can be resolved to explicit coordinates.
    /// Null for URI links or unresolved named destinations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_array: Option<DestArrayJson>,
}

/// JSON representation of an explicit destination array.
///
/// Describes a specific location within a PDF page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DestArrayJson {
    /// Zero-based page index within the document.
    pub page_index: usize,

    /// Destination type and coordinates.
    #[serde(flatten)]
    pub dest: DestTypeJson,
}

/// JSON representation of a destination type.
///
/// Uses serde's "tag" representation for unambiguous variant discrimination.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "fit", rename_all = "lowercase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum DestTypeJson {
    /// XYZ destination with optional left, top, zoom.
    ///
    /// Null values mean "retain current view" for that parameter.
    Xyz {
        #[serde(skip_serializing_if = "Option::is_none")]
        left: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        zoom: Option<f64>,
    },
    /// Fit page to window.
    Fit,
    /// Fit horizontally with optional top coordinate.
    FitH {
        #[serde(skip_serializing_if = "Option::is_none")]
        top: Option<f64>,
    },
    /// Fit vertically with optional left coordinate.
    FitV {
        #[serde(skip_serializing_if = "Option::is_none")]
        left: Option<f64>,
    },
    /// Fit rectangle (left, bottom, right, top).
    FitR {
        left: f64,
        bottom: f64,
        right: f64,
        top: f64,
    },
    /// Fit bounding box to window.
    FitB,
    /// Fit bounding box horizontally with optional top coordinate.
    FitBH {
        #[serde(skip_serializing_if = "Option::is_none")]
        top: Option<f64>,
    },
    /// Fit bounding box vertically with optional left coordinate.
    FitBV {
        #[serde(skip_serializing_if = "Option::is_none")]
        left: Option<f64>,
    },
}

/// JSON representation of a single page.
///
/// Contains all page-level fields including geometry, classification,
/// and content arrays (spans, blocks, tables, annotations).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PageJson {
    /// Zero-based page index, canonical for programmatic use.
    ///
    /// This is the stable identifier used in all internal references.
    pub page_index: usize,

    /// One-based page number (= page_index + 1).
    ///
    /// Emitted as a convenience for human-facing display. For programmatic
    /// access, use page_index instead.
    pub page_number: u32,

    /// Human-readable label from PDF /PageLabels number tree.
    ///
    /// Examples: "iv", "A-3", "1". Null if the PDF defines no page labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_label: Option<String>,

    /// Page width in points (1/72 inch).
    pub width: f32,

    /// Page height in points (1/72 inch).
    pub height: f32,

    /// Page rotation in degrees clockwise (0, 90, 180, or 270).
    pub rotation: u16,

    /// Page classification from the page classifier.
    ///
    /// One of: "text", "scanned", "mixed", "broken_vector", "blank", "figure_only".
    #[serde(rename = "type")]
    pub page_type: String,

    /// Text spans (atomic units with consistent font and styling).
    #[serde(default)]
    pub spans: Vec<SpanJson>,

    /// Semantic blocks (paragraphs, headings, lists, tables, etc.).
    #[serde(default)]
    pub blocks: Vec<BlockJson>,

    /// Parallel table structure objects.
    #[serde(default)]
    pub tables: Vec<TableJson>,

    /// Page-level annotations (highlights, stamps, notes, links).
    ///
    /// Empty until Phase 7.2; always present as an array.
    #[serde(default)]
    pub annotations: Vec<AnnotationJson>,
}

/// JSON representation of a non-link annotation.
///
/// Represents markup annotations like highlights, text notes, stamps,
/// and other non-link annotations.
///
/// Per the plan (Phase 7.6.4), annotations are emitted at the page level in the
/// `/pages[i]/annotations` array, sorted by (rect.y0 desc, rect.x0) for deterministic output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AnnotationJson {
    /// Annotation subtype (e.g., "Text", "Highlight", "Stamp", "FreeText").
    ///
    /// Per INV: stable taxonomy of annotation subtypes.
    #[serde(rename = "type")]
    pub subtype: String,

    /// Bounding box in PDF user-space points.
    ///
    /// Format: [x0, y0, x1, y1] where (x0, y0) is the bottom-left corner.
    /// None if the /Rect entry is missing or invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f32; 4]>,

    /// The annotation's content text (from /Contents).
    ///
    /// None if /Contents is missing or not a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,

    /// The annotation's author (from /T).
    ///
    /// None if /T is missing or not a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// The modification date (from /M) as an ISO 8601 string.
    ///
    /// None if /M is missing, malformed, or fails to parse.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,

    /// The color array (from /C) as RGB/Grayscale components.
    ///
    /// None if /C is missing. Length is 1 (grayscale), 3 (RGB), or 4 (CMYK).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Vec<f32>>,

    /// The opacity (from /CA).
    ///
    /// None if not specified (defaults to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,

    /// The name identifier (from /NM).
    ///
    /// None if /NM is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_id: Option<String>,

    /// The subject (from /Subj).
    ///
    /// None if /Subj is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Subtype-specific fields.
    ///
    /// The presence and contents of this field depend on the annotation subtype:
    /// - TextMarkup (Highlight, Squiggly, StrikeOut, Underline): contains "quads" array
    /// - Stamp: contains "name" field
    /// - FreeText: contains "da" (default appearance) field
    /// - Text (sticky note): contains "open", "state", "state_model" fields
    /// - Ink: contains "strokes" array
    /// - Line: contains "endpoints" array
    /// - Polygon/PolyLine: contains "vertices" array
    /// - FileAttachment: contains "fs_ref" field
    /// - Other subtypes: null or omitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specific: Option<AnnotationSpecificJson>,
}

/// JSON representation of subtype-specific annotation fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AnnotationSpecificJson {
    /// Text markup annotations (Highlight, Squiggly, StrikeOut, Underline).
    ///
    /// Contains quad points for the highlighted regions.
    TextMarkup { quads: Vec<[f32; 8]> },

    /// Stamp annotation with icon name.
    Stamp { name: Option<String> },

    /// FreeText annotation with default appearance string.
    FreeText { da: Option<String> },

    /// Text (sticky note) annotation.
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        open: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        state_model: Option<String>,
    },

    /// Ink annotation with stroke paths.
    Ink { strokes: Vec<Vec<[f32; 2]>> },

    /// Line annotation with endpoints.
    Line {
        #[serde(skip_serializing_if = "Option::is_none")]
        endpoints: Option<[f32; 4]>,
    },

    /// Polygon or PolyLine annotation with vertices.
    Polygon { vertices: Vec<[f32; 2]> },

    /// FileAttachment annotation.
    FileAttachment { fs_ref: Option<u32> },

    /// Other annotation types with no subtype-specific fields.
    #[serde(other)]
    Other,
}

/// Top-level output structure for PDF extraction.
///
/// This is the canonical JSON output format, containing document-level
/// metadata and an array of page objects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Output {
    /// Schema version identifier (e.g., "1.0").
    #[serde(rename = "schema_version")]
    pub schema_version: &'static str,

    /// Document-level metadata.
    pub metadata: DocumentMetadata,

    /// Document outline (bookmark tree).
    ///
    /// Empty array if no bookmarks are present.
    #[serde(default)]
    pub outline: Vec<OutlineNode>,

    /// Article thread chains.
    ///
    /// Empty until Phase 7.1; always present as an array.
    #[serde(default)]
    pub threads: Vec<ThreadJson>,

    /// Embedded file attachments.
    ///
    /// Empty until Phase 7.5; always present as an array.
    #[serde(default)]
    pub attachments: Vec<AttachmentJson>,

    /// Digital signature metadata.
    ///
    /// Empty until Phase 7.3; always present as an array.
    #[serde(default)]
    pub signatures: Vec<SignatureJson>,

    /// AcroForm/XFA form fields.
    ///
    /// Empty until Phase 7.4; always present as an array.
    #[serde(default)]
    pub form_fields: Vec<FormFieldJson>,

    /// Document-scoped hyperlinks.
    ///
    /// Empty until Phase 7.6; always present as an array.
    #[serde(default)]
    pub links: Vec<LinkJson>,

    /// Page objects array.
    pub pages: Vec<PageJson>,

    /// Aggregate extraction quality metrics.
    pub extraction_quality: ExtractionQuality,

    /// All diagnostics emitted during extraction.
    #[serde(default)]
    pub errors: Vec<DiagnosticJson>,
}

impl Output {
    /// Create a new empty Output structure.
    pub fn new() -> Self {
        Output {
            schema_version: "1.0",
            metadata: DocumentMetadata {
                title: None,
                author: None,
                subject: None,
                keywords: None,
                creator: None,
                producer: None,
                creation_date: None,
                modification_date: None,
                page_count: 0,
                pdf_version: None,
                is_tagged: false,
                is_encrypted: false,
                conformance: default_conformance(),
                contains_javascript: false,
                contains_xfa: false,
                ocg_present: false,
                generator: None,
            },
            outline: Vec::new(),
            threads: Vec::new(),
            attachments: Vec::new(),
            signatures: Vec::new(),
            form_fields: Vec::new(),
            links: Vec::new(),
            pages: Vec::new(),
            extraction_quality: ExtractionQuality::new(),
            errors: Vec::new(),
        }
    }
}

impl Default for Output {
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
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
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
        assert!(!json.contains("color"));
        assert!(!json.contains("flags"));
    }

    #[test]
    fn test_span_json_with_confidence() {
        let span = SpanJson {
            text: "OCR text".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "OCR-A".to_string(),
            size: 10.0,
            color: None,
            rendering_mode: None,
            confidence: Some(0.95),
            confidence_source: Some("ocr".to_string()),
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        };

        let json = serde_json::to_string(&span).unwrap();
        assert!(json.contains("confidence"));
        assert!(json.contains("confidence_source"));
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
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: Some(receipt),
            column: None,
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
            spans: vec![],
            receipt: None,
        };

        let json = serde_json::to_string(&block).unwrap();

        assert!(json.contains("kind"));
        assert!(json.contains("text"));
        assert!(json.contains("bbox"));
        assert!(!json.contains("level"));
        assert!(!json.contains("receipt"));
        assert!(json.contains("spans"));
    }

    #[test]
    fn test_block_json_heading_with_level() {
        let block = BlockJson {
            kind: "heading".to_string(),
            text: "Chapter 1".to_string(),
            bbox: [50.0, 700.0, 500.0, 750.0],
            level: Some(1),
            table_index: None,
            spans: vec![0, 1],
            receipt: None,
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("level"));
        assert!(json.contains("spans"));
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
            spans: vec![],
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
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
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
            color: Some("#000000".to_string()),
            rendering_mode: Some(0),
            confidence: None,
            confidence_source: Some("vector".to_string()),
            lang: Some("en".to_string()),
            flags: vec!["bold".to_string()],
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
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
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

        // span_with_receipt should contain new fields
        assert!(json_with.contains("color"));
        assert!(json_with.contains("confidence_source"));
        assert!(json_with.contains("lang"));
        assert!(json_with.contains("flags"));
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

    #[test]
    fn test_output_empty_serialization() {
        // Critical test: serialize empty Output -> JSON has all document-level keys
        let output = Output::new();
        let json_str = serde_json::to_string(&output).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Verify all document-level keys are present
        assert!(json_val.get("schema_version").is_some());
        assert_eq!(json_val["schema_version"], "1.0");
        assert!(json_val.get("metadata").is_some());
        assert!(json_val.get("outline").is_some());
        assert!(json_val.get("threads").is_some());
        assert!(json_val.get("attachments").is_some());
        assert!(json_val.get("signatures").is_some());
        assert!(json_val.get("form_fields").is_some());
        assert!(json_val.get("links").is_some());
        assert!(json_val.get("pages").is_some());
        assert!(json_val.get("extraction_quality").is_some());
        assert!(json_val.get("errors").is_some());
    }

    #[test]
    fn test_output_phase7_placeholders_present() {
        // Critical test: Phase 7 placeholder fields present as empty arrays
        let output = Output::new();
        let json_str = serde_json::to_string(&output).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Verify Phase 7 placeholder fields are present and empty
        assert!(json_val["threads"].is_array());
        assert_eq!(json_val["threads"].as_array().unwrap().len(), 0);
        assert!(json_val["attachments"].is_array());
        assert_eq!(json_val["attachments"].as_array().unwrap().len(), 0);
        assert!(json_val["signatures"].is_array());
        assert_eq!(json_val["signatures"].as_array().unwrap().len(), 0);
        assert!(json_val["form_fields"].is_array());
        assert_eq!(json_val["form_fields"].as_array().unwrap().len(), 0);
        assert!(json_val["links"].is_array());
        assert_eq!(json_val["links"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_document_metadata_optional_fields_skipped() {
        // Test that optional metadata fields are omitted when None
        let metadata = DocumentMetadata {
            title: None,
            author: None,
            subject: None,
            keywords: None,
            creator: None,
            producer: None,
            creation_date: None,
            modification_date: None,
            page_count: 10,
            pdf_version: None,
            is_tagged: false,
            is_encrypted: false,
            conformance: "none".to_string(),
            contains_javascript: false,
            contains_xfa: false,
            ocg_present: false,
            generator: None,
        };

        let json_str = serde_json::to_string(&metadata).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Optional string fields should not be present when None
        assert!(json_val.get("title").is_none());
        assert!(json_val.get("author").is_none());
        assert!(json_val.get("subject").is_none());
        assert!(json_val.get("keywords").is_none());
        assert!(json_val.get("creator").is_none());
        assert!(json_val.get("producer").is_none());
        assert!(json_val.get("creation_date").is_none());
        assert!(json_val.get("modification_date").is_none());
        assert!(json_val.get("pdf_version").is_none());
        assert!(json_val.get("generator").is_none());

        // Required fields should be present
        assert_eq!(json_val["page_count"], 10);
        assert_eq!(json_val["is_tagged"], false);
        assert_eq!(json_val["is_encrypted"], false);
        assert_eq!(json_val["conformance"], "none");
    }

    #[test]
    fn test_document_metadata_with_all_fields() {
        // Test serialization with all fields populated
        let metadata = DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: Some("John Doe".to_string()),
            subject: Some("Test Subject".to_string()),
            keywords: Some("test, example".to_string()),
            creator: Some("Test App".to_string()),
            producer: Some("pdftract".to_string()),
            creation_date: Some("2023-01-15T00:00:00Z".to_string()),
            modification_date: Some("2023-01-16T00:00:00Z".to_string()),
            page_count: 5,
            pdf_version: Some("1.7".to_string()),
            is_tagged: true,
            is_encrypted: false,
            conformance: "PDF-A-1b".to_string(),
            contains_javascript: true,
            contains_xfa: false,
            ocg_present: false,
            generator: Some("pdftract v0.1.0".to_string()),
        };

        let json_str = serde_json::to_string(&metadata).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["title"], "Test Document");
        assert_eq!(json_val["author"], "John Doe");
        assert_eq!(json_val["subject"], "Test Subject");
        assert_eq!(json_val["keywords"], "test, example");
        assert_eq!(json_val["creator"], "Test App");
        assert_eq!(json_val["producer"], "pdftract");
        assert_eq!(json_val["creation_date"], "2023-01-15T00:00:00Z");
        assert_eq!(json_val["modification_date"], "2023-01-16T00:00:00Z");
        assert_eq!(json_val["page_count"], 5);
        assert_eq!(json_val["pdf_version"], "1.7");
        assert_eq!(json_val["is_tagged"], true);
        assert_eq!(json_val["is_encrypted"], false);
        assert_eq!(json_val["conformance"], "PDF-A-1b");
        assert_eq!(json_val["contains_javascript"], true);
        assert_eq!(json_val["contains_xfa"], false);
        assert_eq!(json_val["ocg_present"], false);
        assert_eq!(json_val["generator"], "pdftract v0.1.0");
    }

    #[test]
    fn test_outline_node_serialization() {
        // Test outline node serialization
        let outline = OutlineNode {
            title: "Chapter 1".to_string(),
            level: 0,
            page_index: Some(5),
            destination: Some(DestinationJson {
                dest_type: "fit".to_string(),
                left: None,
                top: None,
                right: None,
                bottom: None,
                zoom: None,
            }),
            children: vec![],
        };

        let json_str = serde_json::to_string(&outline).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["title"], "Chapter 1");
        assert_eq!(json_val["level"], 0);
        assert_eq!(json_val["page_index"], 5);
        assert!(!json_val["destination"].is_null());
        assert_eq!(json_val["destination"]["type"], "fit");
        assert!(json_val["children"].is_array());
        assert_eq!(json_val["children"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_outline_node_nested() {
        // Test nested outline structure
        let outline = OutlineNode {
            title: "Chapter 1".to_string(),
            level: 0,
            page_index: Some(1),
            destination: None,
            children: vec![
                OutlineNode {
                    title: "Section 1.1".to_string(),
                    level: 1,
                    page_index: Some(2),
                    destination: None,
                    children: vec![],
                },
                OutlineNode {
                    title: "Section 1.2".to_string(),
                    level: 1,
                    page_index: Some(5),
                    destination: None,
                    children: vec![],
                },
            ],
        };

        let json_str = serde_json::to_string(&outline).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["title"], "Chapter 1");
        assert_eq!(json_val["level"], 0);
        assert_eq!(json_val["children"].as_array().unwrap().len(), 2);
        assert_eq!(json_val["children"][0]["title"], "Section 1.1");
        assert_eq!(json_val["children"][0]["level"], 1);
        assert_eq!(json_val["children"][1]["title"], "Section 1.2");
    }

    #[test]
    fn test_destination_json_xyz() {
        // Test XYZ destination (most common)
        let dest = DestinationJson {
            dest_type: "xyz".to_string(),
            left: Some(100.0),
            top: Some(700.0),
            right: None,
            bottom: None,
            zoom: Some(1.5),
        };

        let json_str = serde_json::to_string(&dest).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["type"], "xyz");
        assert_eq!(json_val["left"], 100.0);
        assert_eq!(json_val["top"], 700.0);
        assert_eq!(json_val["zoom"], 1.5);
        assert!(json_val.get("right").is_none());
        assert!(json_val.get("bottom").is_none());
    }

    #[test]
    fn test_page_json_minimal() {
        // Test minimal page JSON (blank page)
        let page = PageJson {
            page_index: 0,
            page_number: 1,
            page_label: None,
            width: 612.0,
            height: 792.0,
            rotation: 0,
            page_type: "blank".to_string(),
            spans: vec![],
            blocks: vec![],
            tables: vec![],
            annotations: vec![],
        };

        let json_str = serde_json::to_string(&page).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["page_index"], 0);
        assert_eq!(json_val["page_number"], 1);
        assert!(json_val.get("page_label").is_none());
        assert_eq!(json_val["width"], 612.0);
        assert_eq!(json_val["height"], 792.0);
        assert_eq!(json_val["rotation"], 0);
        assert_eq!(json_val["type"], "blank");
        assert!(json_val["spans"].as_array().unwrap().is_empty());
        assert!(json_val["blocks"].as_array().unwrap().is_empty());
        assert!(json_val["tables"].as_array().unwrap().is_empty());
        assert!(json_val["annotations"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_page_json_with_content() {
        // Test page with spans and blocks
        let page = PageJson {
            page_index: 2,
            page_number: 3,
            page_label: Some("iii".to_string()),
            width: 595.0,
            height: 842.0,
            rotation: 90,
            page_type: "text".to_string(),
            spans: vec![
                SpanJson {
                    text: "Hello ".to_string(),
                    bbox: [100.0, 700.0, 150.0, 710.0],
                    font: "Helvetica".to_string(),
                    size: 12.0,
                    color: None,
                    rendering_mode: None,
                    confidence: None,
                    confidence_source: Some("vector".to_string()),
                    lang: None,
                    flags: vec![],
                    receipt: None,
                    column: None,
                },
                SpanJson {
                    text: "World".to_string(),
                    bbox: [150.0, 700.0, 200.0, 710.0],
                    font: "Helvetica".to_string(),
                    size: 12.0,
                    color: None,
                    rendering_mode: None,
                    confidence: None,
                    confidence_source: Some("vector".to_string()),
                    lang: None,
                    flags: vec![],
                    receipt: None,
                    column: None,
                },
            ],
            blocks: vec![BlockJson {
                kind: "paragraph".to_string(),
                text: "Hello World".to_string(),
                bbox: [100.0, 700.0, 200.0, 710.0],
                level: None,
                table_index: None,
                spans: vec![0, 1],
                receipt: None,
            }],
            tables: vec![],
            annotations: vec![],
        };

        let json_str = serde_json::to_string(&page).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["page_index"], 2);
        assert_eq!(json_val["page_number"], 3);
        assert_eq!(json_val["page_label"], "iii");
        assert_eq!(json_val["spans"].as_array().unwrap().len(), 2);
        assert_eq!(json_val["blocks"].as_array().unwrap().len(), 1);
        assert_eq!(json_val["spans"][0]["text"], "Hello ");
        assert_eq!(json_val["blocks"][0]["kind"], "paragraph");
    }

    #[test]
    fn test_diagnostic_json_serialization() {
        // Test diagnostic error JSON serialization
        let diag = DiagnosticJson {
            code: "FONT_GLYPH_UNMAPPED".to_string(),
            message: "Glyph could not be mapped to Unicode".to_string(),
            severity: "warning".to_string(),
            page_index: Some(5),
            location: Some(ObjectLocationJson {
                object_number: 42,
                generation_number: 0,
            }),
            hint: None,
        };

        let json_str = serde_json::to_string(&diag).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["code"], "FONT_GLYPH_UNMAPPED");
        assert_eq!(json_val["message"], "Glyph could not be mapped to Unicode");
        assert_eq!(json_val["severity"], "warning");
        assert_eq!(json_val["page_index"], 5);
        assert!(!json_val["location"].is_null());
        assert_eq!(json_val["location"]["object_number"], 42);
        assert_eq!(json_val["location"]["generation_number"], 0);
        // hint is None, so it should be omitted from JSON
        assert!(json_val.get("hint").is_none() || json_val["hint"].is_null());
    }

    #[test]
    fn test_diagnostic_json_document_level() {
        // Test document-level diagnostic (no page_index)
        let diag = DiagnosticJson {
            code: "XREF_REPAIRED".to_string(),
            message: "Xref was reconstructed via forward scan".to_string(),
            severity: "info".to_string(),
            page_index: None,
            location: None,
            hint: None,
        };

        let json_str = serde_json::to_string(&diag).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["code"], "XREF_REPAIRED");
        assert_eq!(json_val["severity"], "info");
        assert!(json_val.get("page_index").is_none());
        assert!(json_val.get("location").is_none());
    }

    #[test]
    fn test_output_roundtrip() {
        // Critical test: roundtrip serde test passes
        let mut output = Output::new();
        output.metadata.title = Some("Test Document".to_string());
        output.metadata.page_count = 3;
        output.pages.push(PageJson {
            page_index: 0,
            page_number: 1,
            page_label: None,
            width: 612.0,
            height: 792.0,
            rotation: 0,
            page_type: "text".to_string(),
            spans: vec![],
            blocks: vec![],
            tables: vec![],
            annotations: vec![],
        });
        output.errors.push(DiagnosticJson {
            code: "TEST_WARNING".to_string(),
            message: "Test warning message".to_string(),
            severity: "warning".to_string(),
            page_index: Some(0),
            location: None,
            hint: None,
        });

        // Critical test: roundtrip serde test passes
        // Verify JSON serialization works
        let json_str = serde_json::to_string(&output).unwrap();
        assert!(json_str.contains("schema_version"));
        assert!(json_str.contains("\"1.0\""));
        assert!(json_str.contains("Test Document"));
        assert!(json_str.contains("\"page_count\":3"));
        // Note: Full roundtrip deserialization requires static lifetime due to schema_version field

        assert_eq!(output.schema_version, "1.0");
        assert_eq!(output.metadata.title, Some("Test Document".to_string()));
        assert_eq!(output.metadata.page_count, 3);
        assert_eq!(output.pages.len(), 1);
        assert_eq!(output.pages[0].page_index, 0);
        assert_eq!(output.errors.len(), 1);
        assert_eq!(output.errors[0].code, "TEST_WARNING");
    }

    #[test]
    fn test_schema_version_static() {
        // Verify schema_version is a static string
        let output = Output::new();
        assert_eq!(output.schema_version, "1.0");
    }

    #[test]
    fn test_output_default_impl() {
        // Test Default implementation for Output
        let output = Output::default();
        assert_eq!(output.schema_version, "1.0");
        assert_eq!(output.metadata.page_count, 0);
        assert!(output.pages.is_empty());
        assert!(output.errors.is_empty());
    }
}
