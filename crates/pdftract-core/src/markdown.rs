//! Markdown output generation with positional HTML comment anchors.
//!
//! This module provides functions for converting extracted PDF content to
//! Markdown format with optional HTML comment anchors that allow downstream
//! tools to map excerpts back to precise PDF locations.
//!
//! # Anchor Format
//!
//! Each block can be preceded by a single-line HTML comment:
//!
//! ```markdown
//! <!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
//! ## Chapter 3
//! ```
//!
//! The anchor format is a stable schema parseable with one regex:
//!
//! ```text
//! <!-- pdftract: page=(\d+) block=(\d+) bbox=\[([\d.,]+)\] kind=(\w+) -->
//! ```
//!
//! # Parsing Anchors
//!
//! Use [`parse_anchors`] to extract all anchors from markdown text:
//!
//! ```
//! use pdftract_core::markdown::{parse_anchors, Anchor};
//!
//! let md = r#"<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
//! # Title"#;
//!
//! let anchors = parse_anchors(md);
//! assert_eq!(anchors.len(), 1);
//! assert_eq!(anchors[0].page, 0);
//! assert_eq!(anchors[0].block, 0);
//! ```

use crate::schema::{
    BlockJson, ChoiceValueJson, FormFieldJson, FormFieldTypeJson, FormFieldValueJson,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Regex for parsing pdftract HTML comment anchors.
///
/// Format: `<!-- pdftract: page=(\d+) block=(\d+) bbox=\[([\d.,]+)\] kind=(\w+) -->`
fn anchor_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"<!--\s*pdftract:\s*page=(\d+)\s+block=(\d+)\s+bbox=\[([\d.,]+)\]\s+kind=(\w+)\s*-->",
        )
        .expect("invalid ANCHOR_REGEX")
    })
}

/// A parsed HTML comment anchor containing positional metadata.
///
/// Anchors are extracted from markdown output and provide a mapping from
/// markdown text back to precise PDF locations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Anchor {
    /// Zero-based page index.
    pub page: usize,
    /// Zero-based block index within the page.
    pub block: usize,
    /// Bounding box in PDF points: [x0, y0, x1, y1].
    pub bbox: [f32; 4],
    /// Block kind (e.g., "heading", "paragraph", "table").
    pub kind: String,
}

impl Anchor {
    /// Create a new anchor from components.
    pub fn new(page: usize, block: usize, bbox: [f32; 4], kind: String) -> Self {
        Self {
            page,
            block,
            bbox,
            kind,
        }
    }

    /// Format this anchor as an HTML comment.
    ///
    /// Returns a single-line comment suitable for insertion before block content.
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::markdown::Anchor;
    ///
    /// let anchor = Anchor::new(3, 12, [72.0, 640.5, 540.0, 672.0], "heading".to_string());
    /// let comment = anchor.to_comment();
    /// assert_eq!(comment, "<!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->");
    /// ```
    pub fn to_comment(&self) -> String {
        format!(
            "<!-- pdftract: page={} block={} bbox=[{:.1},{:.1},{:.1},{:.1}] kind={} -->",
            self.page,
            self.block,
            self.bbox[0],
            self.bbox[1],
            self.bbox[2],
            self.bbox[3],
            self.kind
        )
    }
}

/// Parse all pdftract anchors from markdown text.
///
/// Returns a vector of [`Anchor`] structs in the order they appear in the text.
/// Invalid anchor formats are silently skipped.
///
/// # Arguments
///
/// * `md` - The markdown text to parse
///
/// # Returns
///
/// A vector of parsed anchors.
///
/// # Example
///
/// ```
/// use pdftract_core::markdown::parse_anchors;
///
/// let md = r#"<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
/// # Title
///
/// <!-- pdftract: page=0 block=1 bbox=[72.0,600.0,540.0,630.0] kind=paragraph -->
/// Some text."#;
///
/// let anchors = parse_anchors(md);
/// assert_eq!(anchors.len(), 2);
/// assert_eq!(anchors[0].page, 0);
/// assert_eq!(anchors[0].block, 0);
/// assert_eq!(anchors[1].page, 0);
/// assert_eq!(anchors[1].block, 1);
/// ```
pub fn parse_anchors(md: &str) -> Vec<Anchor> {
    let mut anchors = Vec::new();

    for captures in anchor_regex().captures_iter(md) {
        // Parse page number
        let page = match captures.get(1).and_then(|m| m.as_str().parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        // Parse block number
        let block = match captures.get(2).and_then(|m| m.as_str().parse().ok()) {
            Some(b) => b,
            None => continue,
        };

        // Parse bbox: "x0,y0,x1,y1" with possible decimal points
        let bbox_str = match captures.get(3) {
            Some(m) => m.as_str(),
            None => continue,
        };

        let bbox: [f32; 4] = match parse_bbox(bbox_str) {
            Some(b) => b,
            None => continue,
        };

        // Parse kind
        let kind = match captures.get(4) {
            Some(m) => m.as_str().to_string(),
            None => continue,
        };

        anchors.push(Anchor::new(page, block, bbox, kind));
    }

    anchors
}

/// Parse a bbox string like "72.0,640.5,540.0,672.0" into [f32; 4].
fn parse_bbox(s: &str) -> Option<[f32; 4]> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return None;
    }

    let mut bbox = [0.0f32; 4];
    for (i, part) in parts.iter().enumerate() {
        bbox[i] = part.trim().parse().ok()?;
    }

    Some(bbox)
}

/// Convert a block to markdown with optional anchor comment.
///
/// If `include_anchor` is true, emits an HTML comment before the block content.
///
/// # Arguments
///
/// * `block` - The block to convert
/// * `page_index` - Zero-based page index
/// * `block_index` - Zero-based block index within the page
/// * `include_anchor` - Whether to include the HTML comment anchor
///
/// # Returns
///
/// A markdown string with optional anchor.
pub fn block_to_markdown(
    block: &BlockJson,
    page_index: usize,
    block_index: usize,
    include_anchor: bool,
) -> String {
    let mut result = String::new();

    // Add anchor comment if requested
    if include_anchor {
        let anchor = Anchor::new(
            page_index,
            block_index,
            [
                block.bbox[0] as f32,
                block.bbox[1] as f32,
                block.bbox[2] as f32,
                block.bbox[3] as f32,
            ],
            block.kind.clone(),
        );
        result.push_str(&anchor.to_comment());
        result.push('\n');
    }

    // Add block content based on kind
    match block.kind.as_str() {
        "heading" => {
            let level = block.level.unwrap_or(1);
            let prefix = "#".repeat(level as usize);
            result.push_str(&format!("{} {}\n", prefix, block.text));
        }
        "paragraph" => {
            result.push_str(&format!("{}\n", block.text));
        }
        "list" => {
            result.push_str(&format!("* {}\n", block.text));
        }
        "table" => {
            result.push_str(&format!("| {}\n", block.text));
        }
        "figure" => {
            result.push_str(&format!("![]()\n\n{}\n", block.text));
        }
        _ => {
            result.push_str(&format!("{}\n", block.text));
        }
    }

    result
}

/// Convert all blocks from a page to markdown with optional anchors.
///
/// If `include_anchor` is true, each block is preceded by an HTML comment.
/// If `include_page_break` is true, adds a horizontal rule between pages.
///
/// # Arguments
///
/// * `blocks` - The blocks to convert
/// * `page_index` - Zero-based page index
/// * `include_anchor` - Whether to include HTML comment anchors
/// * `include_page_break` - Whether to add a page break separator
///
/// # Returns
///
/// A markdown string with all blocks from the page.
pub fn page_to_markdown(
    blocks: &[BlockJson],
    page_index: usize,
    include_anchor: bool,
    include_page_break: bool,
) -> String {
    let mut result = String::new();

    for (block_index, block) in blocks.iter().enumerate() {
        let md = block_to_markdown(block, page_index, block_index, include_anchor);
        result.push_str(&md);
        result.push('\n');
    }

    // Add page break if requested and this isn't the last page
    if include_page_break {
        result.push_str("\n---\n\n");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::BlockJson;

    fn make_test_block(kind: &str, text: &str, bbox: [f64; 4]) -> BlockJson {
        BlockJson {
            kind: kind.to_string(),
            text: text.to_string(),
            bbox,
            level: None,
            table_index: None,
            spans: vec![],
            receipt: None,
        }
    }

    #[test]
    fn test_anchor_to_comment() {
        let anchor = Anchor::new(3, 12, [72.0, 640.5, 540.0, 672.0], "heading".to_string());
        let comment = anchor.to_comment();
        assert_eq!(
            comment,
            "<!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->"
        );
    }

    #[test]
    fn test_anchor_to_comment_round_bbox() {
        let anchor = Anchor::new(
            0,
            0,
            [72.123, 640.567, 540.999, 672.111],
            "paragraph".to_string(),
        );
        let comment = anchor.to_comment();
        // Should be rounded to 1 decimal place
        assert_eq!(
            comment,
            "<!-- pdftract: page=0 block=0 bbox=[72.1,640.6,541.0,672.1] kind=paragraph -->"
        );
    }

    #[test]
    fn test_parse_anchors_single() {
        let md = r#"<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
# Title"#;

        let anchors = parse_anchors(md);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].page, 0);
        assert_eq!(anchors[0].block, 0);
        assert_eq!(anchors[0].bbox, [72.0, 640.5, 540.0, 672.0]);
        assert_eq!(anchors[0].kind, "heading");
    }

    #[test]
    fn test_parse_anchors_multiple() {
        let md = r#"<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
# Title

<!-- pdftract: page=0 block=1 bbox=[72.0,600.0,540.0,630.0] kind=paragraph -->
Some text."#;

        let anchors = parse_anchors(md);
        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[0].page, 0);
        assert_eq!(anchors[0].block, 0);
        assert_eq!(anchors[1].page, 0);
        assert_eq!(anchors[1].block, 1);
    }

    #[test]
    fn test_parse_anchors_invalid_format_skipped() {
        let md = r#"<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
# Title

<!-- malformed anchor -->
Some text."#;

        let anchors = parse_anchors(md);
        assert_eq!(anchors.len(), 1);
    }

    #[test]
    fn test_parse_anchors_whitespace_tolerant() {
        let md =
            r#"<!--  pdftract:  page=0  block=0  bbox=[72.0,640.5,540.0,672.0]  kind=heading  -->"#;
        let anchors = parse_anchors(md);
        assert_eq!(anchors.len(), 1);
    }

    #[test]
    fn test_parse_bbox() {
        assert_eq!(
            parse_bbox("72.0,640.5,540.0,672.0"),
            Some([72.0, 640.5, 540.0, 672.0])
        );
        assert_eq!(parse_bbox("0,0,100,100"), Some([0.0, 0.0, 100.0, 100.0]));
        assert_eq!(
            parse_bbox("72.0, 640.5, 540.0, 672.0"),
            Some([72.0, 640.5, 540.0, 672.0])
        ); // with spaces
        assert_eq!(parse_bbox("invalid"), None);
        assert_eq!(parse_bbox("1,2,3"), None); // too few values
        assert_eq!(parse_bbox("1,2,3,4,5"), None); // too many values
    }

    #[test]
    fn test_block_to_markdown_heading_with_anchor() {
        let block = BlockJson {
            kind: "heading".to_string(),
            text: "Chapter 1".to_string(),
            bbox: [72.0, 640.5, 540.0, 672.0],
            level: Some(2),
            table_index: None,
            spans: vec![],
            receipt: None,
        };

        let md = block_to_markdown(&block, 0, 0, true);
        assert!(md.contains(
            "<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->"
        ));
        assert!(md.contains("## Chapter 1"));
    }

    #[test]
    fn test_block_to_markdown_paragraph_without_anchor() {
        let block = make_test_block("paragraph", "Some text.", [72.0, 600.0, 540.0, 630.0]);
        let md = block_to_markdown(&block, 0, 0, false);
        assert!(!md.contains("<!-- pdftract:"));
        assert!(md.contains("Some text."));
    }

    #[test]
    fn test_block_to_markdown_list() {
        let block = make_test_block("list", "Item 1", [72.0, 500.0, 540.0, 520.0]);
        let md = block_to_markdown(&block, 0, 0, false);
        assert!(md.contains("* Item 1"));
    }

    #[test]
    fn test_block_to_markdown_table() {
        let block = make_test_block("table", "Cell data", [72.0, 400.0, 540.0, 450.0]);
        let md = block_to_markdown(&block, 0, 0, false);
        assert!(md.contains("| Cell data"));
    }

    #[test]
    fn test_block_to_markdown_figure() {
        let block = make_test_block("figure", "Alt text", [72.0, 300.0, 540.0, 350.0]);
        let md = block_to_markdown(&block, 0, 0, false);
        assert!(md.contains("![]()"));
        assert!(md.contains("Alt text"));
    }

    #[test]
    fn test_page_to_markdown_with_page_break() {
        let blocks = vec![
            make_test_block("heading", "Title", [72.0, 640.5, 540.0, 672.0]),
            make_test_block("paragraph", "Text", [72.0, 600.0, 540.0, 630.0]),
        ];

        let md = page_to_markdown(&blocks, 0, false, true);
        assert!(md.contains("---"));
    }

    #[test]
    fn test_page_to_markdown_without_page_break() {
        let blocks = vec![
            make_test_block("heading", "Title", [72.0, 640.5, 540.0, 672.0]),
            make_test_block("paragraph", "Text", [72.0, 600.0, 540.0, 630.0]),
        ];

        let md = page_to_markdown(&blocks, 0, false, false);
        assert!(!md.contains("---"));
    }

    #[test]
    fn test_page_to_markdown_with_anchors() {
        let blocks = vec![
            make_test_block("heading", "Title", [72.0, 640.5, 540.0, 672.0]),
            make_test_block("paragraph", "Text", [72.0, 600.0, 540.0, 630.0]),
        ];

        let md = page_to_markdown(&blocks, 0, true, false);
        assert_eq!(md.matches("<!-- pdftract:").count(), 2);
    }

    #[test]
    fn test_roundtrip_extract_and_parse() {
        let blocks = vec![BlockJson {
            kind: "heading".to_string(),
            text: "Chapter 1".to_string(),
            bbox: [72.0, 640.5, 540.0, 672.0],
            level: Some(2),
            table_index: None,
            spans: vec![],
            receipt: None,
        }];

        let md = page_to_markdown(&blocks, 3, true, false);
        let anchors = parse_anchors(&md);

        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].page, 3);
        assert_eq!(anchors[0].block, 0);
        assert_eq!(anchors[0].kind, "heading");
    }
}

/// Generate a markdown footer section for form fields.
///
/// This function creates a formatted markdown table listing all form fields
/// with their names, types, and current values. Only emits the section when
/// form_fields count > 0.
///
/// # Arguments
///
/// * `form_fields` - The form fields to include in the footer
///
/// # Returns
///
/// A markdown string with a form fields table, or an empty string if no fields.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::markdown::form_fields_to_markdown;
/// use pdftract_core::schema::{FormFieldJson, FormFieldTypeJson, FormFieldValueJson};
///
/// let fields = vec![
///     FormFieldJson {
///         name: "employee_name".to_string(),
///         field_type: FormFieldTypeJson::Text,
///         value: FormFieldValueJson::Text(Some("John Doe".to_string())),
///         // ... other fields
///     },
/// ];
///
/// let md = form_fields_to_markdown(&fields);
/// assert!(md.contains("## Form Fields"));
/// assert!(md.contains("employee_name"));
/// ```
pub fn form_fields_to_markdown(form_fields: &[FormFieldJson]) -> String {
    if form_fields.is_empty() {
        return String::new();
    }

    let mut result = String::from("\n\n## Form Fields\n\n");
    result.push_str("| Name | Type | Value |\n");
    result.push_str("|------|------|-------|\n");

    for field in form_fields {
        let type_str = match field.field_type {
            FormFieldTypeJson::Text => "text",
            FormFieldTypeJson::Button => "button",
            FormFieldTypeJson::Choice => "choice",
            FormFieldTypeJson::Signature => "signature",
        };

        let value_str = format_value_json(&field.value);

        result.push_str(&format!(
            "| {} | {} | {} |\n",
            field.name, type_str, value_str
        ));
    }

    result
}

/// Format a FormFieldValueJson as a string for markdown display.
fn format_value_json(value: &FormFieldValueJson) -> String {
    match value {
        FormFieldValueJson::Text(None) => "*empty*".to_string(),
        FormFieldValueJson::Text(Some(s)) => escape_pipe(s),
        FormFieldValueJson::Button(b) => b.to_string(),
        FormFieldValueJson::Choice(ChoiceValueJson::Single(s)) => escape_pipe(s),
        FormFieldValueJson::Choice(ChoiceValueJson::Multiple(vec)) => {
            let values: Vec<String> = vec.iter().map(|s| escape_pipe(s.as_str())).collect();
            values.join(", ")
        }
        FormFieldValueJson::Signature(None) => "*unsigned*".to_string(),
        FormFieldValueJson::Signature(Some(n)) => format!("ref #{}", n),
    }
}

/// Escape pipe characters for markdown table cells.
fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|")
}
