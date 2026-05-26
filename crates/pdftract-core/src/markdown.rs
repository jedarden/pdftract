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
    BeadJson, BlockJson, ChoiceValueJson, FormFieldJson, FormFieldTypeJson, FormFieldValueJson,
    SpanJson, ThreadJson,
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

/// Generate a markdown footer section for article threads.
///
/// This function creates a formatted markdown section listing all article
/// threads with their metadata and page ranges. Only emits the section
/// when threads count > 0.
///
/// # Arguments
///
/// * `threads` - The threads to include in the footer
///
/// # Returns
///
/// A markdown string with an article threads section, or an empty string if no threads.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::markdown::threads_to_markdown;
/// use pdftract_core::schema::{ThreadJson, BeadJson};
///
/// let threads = vec![
///     ThreadJson {
///         title: Some("Main Article".to_string()),
///         author: Some("John Doe".to_string()),
///         subject: None,
///         keywords: None,
///         beads: vec![
///             BeadJson { page_index: 0, rect: [100.0, 200.0, 300.0, 220.0] },
///             BeadJson { page_index: 1, rect: [100.0, 500.0, 300.0, 520.0] },
///         ],
///     },
/// ];
///
/// let md = threads_to_markdown(&threads);
/// assert!(md.contains("## Article Threads"));
/// assert!(md.contains("1. *Main Article* (John Doe) - pages 0-1 (2 beads)"));
/// ```
pub fn threads_to_markdown(threads: &[ThreadJson]) -> String {
    if threads.is_empty() {
        return String::new();
    }

    let mut result = String::from("\n\n## Article Threads\n\n");

    for (i, thread) in threads.iter().enumerate() {
        // Build the thread title line
        let title = thread.title.as_deref().unwrap_or("(Untitled)");
        let author = thread.author.as_deref().unwrap_or("");

        // Collapse contiguous page ranges
        let page_ranges = collapse_page_ranges(&thread.beads);

        // Format: "1. *Title* (Author) - pages 0-1, 3-5 (3 beads)"
        result.push_str(&format!(
            "{}. *{}* ({}) - {} ({} beads)\n",
            i + 1,
            title,
            author,
            page_ranges,
            thread.beads.len()
        ));
    }

    result
}

/// Collapse contiguous page indices into ranges.
///
/// Given a list of beads with page indices, this function collapses
/// contiguous sequences into ranges for more compact display.
///
/// # Arguments
///
/// * `beads` - The beads to collapse into page ranges
///
/// # Returns
///
/// A string like "pages 0-1, 3-5" representing the page ranges.
fn collapse_page_ranges(beads: &[BeadJson]) -> String {
    if beads.is_empty() {
        return "no pages".to_string();
    }

    let mut ranges = Vec::new();
    let mut start = beads[0].page_index;
    let mut end = beads[0].page_index;

    for bead in beads.iter().skip(1) {
        // Skip duplicate page indices
        if bead.page_index == end {
            continue;
        }

        if bead.page_index == end + 1 {
            // Contiguous, extend the range
            end = bead.page_index;
        } else {
            // Gap, emit the current range
            ranges.push((start, end));
            start = bead.page_index;
            end = bead.page_index;
        }
    }

    // Emit the last range
    ranges.push((start, end));

    // Format ranges
    let parts: Vec<String> = ranges
        .iter()
        .map(|&(s, e)| {
            if s == e {
                format!("{}", s)
            } else {
                format!("{}-{}", s, e)
            }
        })
        .collect();

    format!("pages {}", parts.join(", "))
}

/// Convert a span to markdown with inline styling based on flags.
///
/// This function implements Phase 6.5 inline span styling, translating
/// span flag bitmask values to Markdown inline syntax.
///
/// # Styling Rules
///
/// - Bold (bit 0) → `**text**`
/// - Italic (bit 1) → `*text*`
/// - Bold + Italic → `***text***`
/// - Subscript (bit 3) → `<sub>text</sub>`
/// - Superscript (bit 4) → `<sup>text</sup>`
/// - Smallcaps (bit 2) → `<span style="font-variant: small-caps">text</span>`
/// - Color-only differences: no styling emitted
///
/// # Arguments
///
/// * `span` - The span to convert
///
/// # Returns
///
/// A markdown string with appropriate inline styling applied.
///
/// # Examples
///
/// ```
/// use pdftract_core::schema::SpanJson;
/// use pdftract_core::markdown::span_to_markdown;
///
/// let mut span = SpanJson {
///     text: "important text".to_string(),
///     flags: vec!["bold".to_string()],
///     ..Default::default()
/// };
///
/// let md = span_to_markdown(&span);
/// assert_eq!(md, "**important text**");
/// ```
///
/// ```
/// // H₂O example: subscript
/// let mut span = SpanJson {
///     text: "2".to_string(),
///     flags: vec!["subscript".to_string()],
///     ..Default::default()
/// };
///
/// let md = span_to_markdown(&span);
/// assert_eq!(md, "<sub>2</sub>");
/// ```
///
/// ```
/// // 4th example: superscript
/// let mut span = SpanJson {
///     text: "th".to_string(),
///     flags: vec!["superscript".to_string()],
///     ..Default::default()
/// };
///
/// let md = span_to_markdown(&span);
/// assert_eq!(md, "<sup>th</sup>");
/// ```
///
/// ```
/// // Bold + italic combination
/// let mut span = SpanJson {
///     text: "emphasized".to_string(),
///     flags: vec!["bold".to_string(), "italic".to_string()],
///     ..Default::default()
/// };
///
/// let md = span_to_markdown(&span);
/// assert_eq!(md, "***emphasized***");
/// ```
///
/// ```
/// // Special character escaping
/// let mut span = SpanJson {
///     text: "1*2".to_string(),
///     flags: vec![],
///     ..Default::default()
/// };
///
/// let md = span_to_markdown(&span);
/// assert_eq!(md, "1\\*2");
/// ```
pub fn span_to_markdown(span: &SpanJson) -> String {
    // Get the text content
    let text = &span.text;

    // Skip whitespace-only spans (no point styling whitespace)
    if text.trim().is_empty() {
        return text.clone();
    }

    // Check for each flag in the flags Vec<String>
    let has_bold = span.flags.contains(&"bold".to_string());
    let has_italic = span.flags.contains(&"italic".to_string());
    let has_subscript = span.flags.contains(&"subscript".to_string());
    let has_superscript = span.flags.contains(&"superscript".to_string());
    let has_smallcaps = span.flags.contains(&"smallcaps".to_string());

    // Color-only differences: emit no styling (just return escaped text)
    // This is checked by seeing if none of the style flags are present
    let has_any_style = has_bold || has_italic || has_subscript || has_superscript || has_smallcaps;

    if !has_any_style {
        // No styling flags, just escape and return
        return escape_markdown_inline(text);
    }

    // Escape the text first (before wrapping in styling)
    let escaped = escape_markdown_inline(text);

    // Build the styled output
    let mut result = String::new();

    // Combination order:
    // - Bold + italic wrapper (***text***) goes outermost
    // - Smallcaps span wraps script tags (<span><sup>text</sup></span>)
    // - Script tags go inside smallcaps (if both present)
    // This order: **<span><sup>text</sup></span>** or **<sub>text</sub>** (if no smallcaps)

    // Bold + italic wrapper (***text***)
    if has_bold && has_italic {
        result.push_str("***");
    } else if has_bold {
        result.push_str("**");
    } else if has_italic {
        result.push_str("*");
    }

    // Smallcaps wrapper (outer relative to scripts)
    if has_smallcaps {
        result.push_str("<span style=\"font-variant: small-caps\">");
    }

    // Script tags (sub/sup) go inside smallcaps
    if has_subscript {
        result.push_str("<sub>");
    } else if has_superscript {
        result.push_str("<sup>");
    }

    // Add the escaped text
    result.push_str(&escaped);

    // Close wrappers in reverse order
    if has_subscript {
        result.push_str("</sub>");
    } else if has_superscript {
        result.push_str("</sup>");
    }

    if has_smallcaps {
        result.push_str("</span>");
    }

    if has_bold && has_italic {
        result.push_str("***");
    } else if has_bold {
        result.push_str("**");
    } else if has_italic {
        result.push_str("*");
    }

    result
}

/// Escape special Markdown characters in inline text.
///
/// This function escapes characters that have special meaning in Markdown
/// to prevent unintended formatting. Per CommonMark spec, these characters
/// are escaped to prevent them from being interpreted as Markdown syntax.
///
/// # Characters Escaped
///
/// The following characters are escaped with a backslash:
/// - `\` (backslash itself - must be escaped to avoid interpretation)
/// - `` ` `` (code span)
/// - `*` (emphasis/strong)
/// - `_` (emphasis)
/// - `[` (link start)
/// - `]` (link end)
/// - `(` (link destination start)
/// - `)` (link destination end)
/// - `#` (ATX heading)
/// - `!` (image)
/// - `+` (list marker)
/// - `<` (HTML tag/auto-link)
/// - `>` (blockquote)
///
/// # Characters NOT Escaped
///
/// - `-` (hyphen) - only special at start of line for lists/HR
/// - `.` (period) - only special as part of list marker like "1."
/// - `=` (equals) - not special in CommonMark
///
/// # Arguments
///
/// * `s` - The string to escape
///
/// # Returns
///
/// A string with special characters escaped.
fn escape_markdown_inline(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);

    for c in s.chars() {
        match c {
            '\\' | '`' | '*' | '_' | '[' | ']' | '(' | ')' | '#' | '!' | '+' | '<' | '>' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }

    result
}

#[cfg(test)]
mod span_tests {
    use super::*;

    /// Helper function to create a test span with the given text and flags.
    /// All other fields are set to reasonable defaults for testing.
    fn make_test_span(text: &str, flags: &[&str]) -> SpanJson {
        SpanJson {
            text: text.to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: flags.iter().map(|s| s.to_string()).collect(),
            receipt: None,
            column: None,
        }
    }

    #[test]
    fn test_span_to_markdown_bold() {
        let span = make_test_span("important", &["bold"]);
        assert_eq!(span_to_markdown(&span), "**important**");
    }

    #[test]
    fn test_span_to_markdown_italic() {
        let span = make_test_span("emphasized", &["italic"]);
        assert_eq!(span_to_markdown(&span), "*emphasized*");
    }

    #[test]
    fn test_span_to_markdown_bold_italic() {
        // Critical test: bold + italic span emitted as ***text***
        let span = make_test_span("very important", &["bold", "italic"]);
        assert_eq!(span_to_markdown(&span), "***very important***");
    }

    #[test]
    fn test_span_to_markdown_subscript() {
        let span = make_test_span("2", &["subscript"]);
        assert_eq!(span_to_markdown(&span), "<sub>2</sub>");
    }

    #[test]
    fn test_span_to_markdown_superscript() {
        let span = make_test_span("th", &["superscript"]);
        assert_eq!(span_to_markdown(&span), "<sup>th</sup>");
    }

    #[test]
    fn test_span_to_markdown_smallcaps() {
        let span = make_test_span("CAPS", &["smallcaps"]);
        assert_eq!(
            span_to_markdown(&span),
            "<span style=\"font-variant: small-caps\">CAPS</span>"
        );
    }

    #[test]
    fn test_span_to_markdown_no_flags() {
        // Color-only difference or no styling: no styling emitted
        let span = make_test_span("plain text", &[]);
        assert_eq!(span_to_markdown(&span), "plain text");
    }

    #[test]
    fn test_span_to_markdown_special_chars_escaped() {
        // Special chars escaped: span text "1*2" -> "1\*2"
        let span = make_test_span("1*2", &[]);
        assert_eq!(span_to_markdown(&span), "1\\*2");
    }

    #[test]
    fn test_span_to_markdown_bold_subscript_combination() {
        // Bold + subscript: **<sub>text</sub>**
        let span = make_test_span("ion", &["bold", "subscript"]);
        assert_eq!(span_to_markdown(&span), "**<sub>ion</sub>**");
    }

    #[test]
    fn test_span_to_markdown_bold_superscript_combination() {
        // Bold + superscript: **<sup>text</sup>**
        let span = make_test_span("st", &["bold", "superscript"]);
        assert_eq!(span_to_markdown(&span), "**<sup>st</sup>**");
    }

    #[test]
    fn test_span_to_markdown_italic_subscript_combination() {
        // Italic + subscript: *<sub>text</sub>*
        let span = make_test_span("ion", &["italic", "subscript"]);
        assert_eq!(span_to_markdown(&span), "*<sub>ion</sub>*");
    }

    #[test]
    fn test_span_to_markdown_all_flags() {
        // All flags: bold + italic + smallcaps + superscript
        let span = make_test_span("X", &["bold", "italic", "smallcaps", "superscript"]);
        assert_eq!(
            span_to_markdown(&span),
            "***<span style=\"font-variant: small-caps\"><sup>X</sup></span>***"
        );
    }

    #[test]
    fn test_span_to_markdown_whitespace_only() {
        // Empty/whitespace-only spans emit unwrapped
        let span = make_test_span("   ", &["bold"]);
        assert_eq!(span_to_markdown(&span), "   ");
    }

    #[test]
    fn test_span_to_markdown_empty_string() {
        let span = make_test_span("", &["bold"]);
        assert_eq!(span_to_markdown(&span), "");
    }

    #[test]
    fn test_escape_markdown_inline_asterisk() {
        assert_eq!(escape_markdown_inline("1*2"), "1\\*2");
    }

    #[test]
    fn test_escape_markdown_inline_underscore() {
        assert_eq!(escape_markdown_inline("hello_world"), "hello\\_world");
    }

    #[test]
    fn test_escape_markdown_inline_backtick() {
        assert_eq!(escape_markdown_inline("code`here"), "code\\`here");
    }

    #[test]
    fn test_escape_markdown_inline_brackets() {
        assert_eq!(escape_markdown_inline("[link]"), "\\[link\\]");
    }

    #[test]
    fn test_escape_markdown_inline_multiple_special() {
        assert_eq!(escape_markdown_inline("*_[link]*"), "\\*\\_\\[link\\]\\*");
    }

    #[test]
    fn test_escape_markdown_inline_backslash() {
        assert_eq!(escape_markdown_inline("C:\\path"), "C:\\\\path");
    }

    #[test]
    fn test_escape_markdown_inline_hash() {
        assert_eq!(escape_markdown_inline("#heading"), "\\#heading");
    }

    #[test]
    fn test_escape_markdown_inline_plus_minus() {
        assert_eq!(escape_markdown_inline("+/-"), "\\+/-");
    }

    #[test]
    fn test_escape_markdown_inline_less_greater() {
        // < and > are escaped (HTML tags/auto-links)
        assert_eq!(escape_markdown_inline("<tag>"), "\\<tag\\>");
    }

    #[test]
    fn test_span_to_markdown_bold_with_asterisk_in_text() {
        // Bold text containing asterisks should be escaped
        let span = make_test_span("2*2=4", &["bold"]);
        assert_eq!(span_to_markdown(&span), "**2\\*2=4**");
    }

    #[test]
    fn test_span_to_markdown_subscript_with_special_chars() {
        // Subscript with special characters
        let span = make_test_span("2+", &["subscript"]);
        assert_eq!(span_to_markdown(&span), "<sub>2\\+</sub>");
    }

    #[test]
    fn test_span_to_markdown_superscript_with_special_chars() {
        // Superscript with special characters
        let span = make_test_span("n-1", &["superscript"]);
        assert_eq!(span_to_markdown(&span), "<sup>n-1</sup>");
    }

    #[test]
    fn test_span_to_markdown_smallcaps_with_special_chars() {
        // Smallcaps with underscore
        let span = make_test_span("HELLO_WORLD", &["smallcaps"]);
        assert_eq!(
            span_to_markdown(&span),
            "<span style=\"font-variant: small-caps\">HELLO\\_WORLD</span>"
        );
    }

    #[test]
    fn test_threads_to_markdown_empty() {
        // Empty threads list returns empty string
        let threads: Vec<ThreadJson> = vec![];
        assert_eq!(threads_to_markdown(&threads), "");
    }

    #[test]
    fn test_threads_to_markdown_single_thread() {
        // Single thread with multiple beads
        let threads = vec![ThreadJson {
            title: Some("Main Article".to_string()),
            author: Some("John Doe".to_string()),
            subject: None,
            keywords: None,
            beads: vec![
                BeadJson {
                    page_index: 0,
                    rect: [100.0, 200.0, 300.0, 220.0],
                },
                BeadJson {
                    page_index: 1,
                    rect: [100.0, 500.0, 300.0, 520.0],
                },
            ],
        }];

        let md = threads_to_markdown(&threads);
        assert!(md.contains("## Article Threads"));
        assert!(md.contains("1. *Main Article* (John Doe) - pages 0-1 (2 beads)"));
    }

    #[test]
    fn test_threads_to_markdown_multiple_threads() {
        // Multiple threads with various metadata
        let threads = vec![
            ThreadJson {
                title: Some("Introduction".to_string()),
                author: Some("Jane Smith".to_string()),
                subject: None,
                keywords: None,
                beads: vec![BeadJson {
                    page_index: 0,
                    rect: [50.0, 100.0, 250.0, 120.0],
                }],
            },
            ThreadJson {
                title: Some("Main Content".to_string()),
                author: None,
                subject: Some("Chapter 1".to_string()),
                keywords: Some("test, example".to_string()),
                beads: vec![
                    BeadJson {
                        page_index: 1,
                        rect: [50.0, 400.0, 250.0, 420.0],
                    },
                    BeadJson {
                        page_index: 2,
                        rect: [50.0, 100.0, 250.0, 120.0],
                    },
                ],
            },
        ];

        let md = threads_to_markdown(&threads);
        assert!(md.contains("1. *Introduction* (Jane Smith) - pages 0 (1 beads)"));
        assert!(md.contains("2. *Main Content* () - pages 1-2 (2 beads)"));
    }

    #[test]
    fn test_threads_to_markdown_untitled_thread() {
        // Thread with no title
        let threads = vec![ThreadJson {
            title: None,
            author: None,
            subject: None,
            keywords: None,
            beads: vec![BeadJson {
                page_index: 5,
                rect: [100.0, 200.0, 300.0, 220.0],
            }],
        }];

        let md = threads_to_markdown(&threads);
        assert!(md.contains("1. *(Untitled)* () - pages 5 (1 beads)"));
    }

    #[test]
    fn test_collapse_page_ranges_single_page() {
        // Single bead
        let beads = vec![BeadJson {
            page_index: 3,
            rect: [0.0, 0.0, 100.0, 20.0],
        }];
        assert_eq!(collapse_page_ranges(&beads), "pages 3");
    }

    #[test]
    fn test_collapse_page_ranges_contiguous() {
        // Contiguous pages
        let beads = vec![
            BeadJson {
                page_index: 0,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 1,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 2,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
        ];
        assert_eq!(collapse_page_ranges(&beads), "pages 0-2");
    }

    #[test]
    fn test_collapse_page_ranges_gaps() {
        // Pages with gaps
        let beads = vec![
            BeadJson {
                page_index: 0,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 2,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 5,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
        ];
        assert_eq!(collapse_page_ranges(&beads), "pages 0, 2, 5");
    }

    #[test]
    fn test_collapse_page_ranges_mixed() {
        // Mixed contiguous and gaps
        let beads = vec![
            BeadJson {
                page_index: 0,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 1,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 3,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 4,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
            BeadJson {
                page_index: 4,
                rect: [0.0, 0.0, 100.0, 20.0],
            },
        ];
        assert_eq!(collapse_page_ranges(&beads), "pages 0-1, 3-4");
    }
}
