//! Argument structs for MCP tools.
//!
//! Each tool has a corresponding argument struct that derives JsonSchema
//! to generate the inputSchema for tools/list.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Common password argument for tools that support encrypted PDFs.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PasswordArg {
    /// PDF password for encrypted documents
    pub password: Option<String>,
}

/// Arguments for the extract tool.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExtractArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// Page range (e.g., "1-5,7")
    #[serde(default)]
    pub pages: Option<String>,

    /// Enable OCR for scanned pages
    #[serde(default)]
    pub ocr: Option<bool>,

    /// Output formats for multi-output (e.g., ["json", "markdown"])
    #[serde(default)]
    pub formats: Option<Vec<String>>,

    /// Enable auto-profiling for font detection
    #[serde(default)]
    pub auto_profile: Option<bool>,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,

    /// Receipt mode: "off", "lite", or "svg"
    #[serde(default)]
    pub receipts: Option<String>,
}

/// Arguments for the extract_text tool.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExtractTextArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// Page range (e.g., "1-5,7")
    #[serde(default)]
    pub pages: Option<String>,

    /// Enable OCR for scanned pages
    #[serde(default)]
    pub ocr: Option<bool>,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,
}

/// Arguments for the extract_markdown tool.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExtractMarkdownArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// Page range (e.g., "1-5,7")
    #[serde(default)]
    pub pages: Option<String>,

    /// Enable OCR for scanned pages
    #[serde(default)]
    pub ocr: Option<bool>,

    /// Include anchor links for headings
    #[serde(default)]
    pub anchors: Option<bool>,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,
}

/// Arguments for the search tool.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// Regular expression pattern to search for
    pub pattern: String,

    /// Case-insensitive search
    #[serde(default)]
    pub case_insensitive: Option<bool>,

    /// Maximum number of matches to return
    #[serde(default)]
    pub max_matches: Option<u32>,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,
}

/// Arguments for the get_metadata tool.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetMetadataArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,
}

/// Arguments for the get_table tool (Phase 7.2 stub).
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetTableArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// Page index (0-based)
    pub page: u32,

    /// Table index on the page (0-based)
    pub table_index: u32,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,
}

/// Arguments for the get_form_fields tool (Phase 7.4 stub).
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetFormFieldsArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,
}

/// Arguments for the get_attachments tool (Phase 7.5 stub).
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetAttachmentsArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// Include base64-encoded file data in the response
    #[serde(default)]
    pub include_data: Option<bool>,
}

/// Arguments for the hash tool.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HashArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,

    /// PDF password for encrypted documents
    #[serde(default)]
    pub password: Option<String>,
}

/// Arguments for the classify tool (Phase 5.6 stub).
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClassifyArgs {
    /// Path to the PDF file (local filesystem path or https:// URL)
    pub path: String,
}
