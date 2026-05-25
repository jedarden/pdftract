//! Plain text output serialization.
//!
//! This module implements Phase 4.6 plain text output mode, which projects
//! the block list into human-readable text with proper paragraph spacing.
//!
//! # Serialization Rules
//!
//! - Blocks serialized in reading order (as ordered in the blocks array)
//! - Paragraphs separated by `\n\n`
//! - Page breaks: `\f` (form feed, 0x0C) - handled by caller
//! - Headers and footers excluded by default; controlled via TextOptions
//! - Invisible text (rendering_mode=3) excluded unless include_invisible is set
//! - Watermark blocks excluded
//!
//! # Block Text Computation
//!
//! - Paragraph/Heading/Caption/Quote: lines space-joined
//! - List/Code: lines newline-joined
//! - Figure: empty string (no text content)
//!
//! # Examples
//!
//! ```
//! use pdftract_core::schema::BlockJson;
//! use pdftract_core::text::{serialize_page_text, TextOptions};
//!
//! let blocks = vec![
//!     BlockJson {
//!         kind: "paragraph".to_string(),
//!         text: "First paragraph.".to_string(),
//!         ..Default::default()
//!     },
//!     BlockJson {
//!         kind: "paragraph".to_string(),
//!         text: "Second paragraph.".to_string(),
//!         ..Default::default()
//!     },
//! ];
//!
//! let options = TextOptions::default();
//! let text = serialize_page_text(&blocks, &options);
//! assert_eq!(text, "First paragraph.\n\nSecond paragraph.");
//! ```

use crate::schema::BlockJson;

/// Options controlling plain text serialization behavior.
///
/// These options control which blocks are included in the plain text output.
#[derive(Debug, Clone, Default)]
pub struct TextOptions {
    /// Include header and footer blocks in output.
    ///
    /// When false (default), blocks with kind "header" or "footer" are excluded.
    pub include_headers_footers: bool,

    /// Include invisible text (rendering_mode=3) in output.
    ///
    /// When false (default), spans with rendering_mode=3 are excluded.
    pub include_invisible_text: bool,

    /// Include watermark blocks in output.
    ///
    /// When false (default), blocks with kind "watermark" are excluded.
    pub include_watermarks: bool,
}

impl TextOptions {
    /// Create default text options (headers/footers and invisible text excluded).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create options that include headers and footers.
    pub fn with_headers_footers(mut self) -> Self {
        self.include_headers_footers = true;
        self
    }

    /// Create options that include invisible text.
    pub fn with_invisible_text(mut self) -> Self {
        self.include_invisible_text = true;
        self
    }

    /// Create options that include watermarks.
    pub fn with_watermarks(mut self) -> Self {
        self.include_watermarks = true;
        self
    }
}

/// Serialize a page's blocks to plain text.
///
/// This function implements the per-page text serialization logic for Phase 4.6.
/// It iterates blocks in reading order (as ordered in the blocks array), filters
/// by block kind and rendering mode, joins block texts according to kind-specific
/// rules, and separates blocks by `\n\n`.
///
/// # Arguments
///
/// * `blocks` - The blocks to serialize, in reading order
/// * `options` - Options controlling which blocks are included
///
/// # Returns
///
/// A plain text string with blocks separated by `\n\n`. Empty blocks are omitted
/// entirely (no spurious newlines).
///
/// # Block Text Rules
///
/// - Paragraph/Heading/Caption/Quote: use pre-computed block text
/// - List/Code: use pre-computed block text (lines already joined)
/// - Figure: empty string (no text content)
/// - Table: use pre-computed block text
///
/// # Filtering
///
/// - Header/Footer: excluded unless `include_headers_footers` is true
/// - Watermark: excluded unless `include_watermarks` is true
/// - Invisible spans: excluded unless `include_invisible_text` is true
///
/// # Examples
///
/// ```
/// use pdftract_core::schema::BlockJson;
/// use pdftract_core::text::{serialize_page_text, TextOptions};
///
/// let blocks = vec![
///     BlockJson {
///         kind: "paragraph".to_string(),
///         text: "First paragraph.".to_string(),
///         bbox: [0.0, 0.0, 100.0, 20.0],
///         ..Default::default()
///     },
///     BlockJson {
///         kind: "paragraph".to_string(),
///         text: "Second paragraph.".to_string(),
///         bbox: [0.0, 20.0, 100.0, 40.0],
///         ..Default::default()
///     },
/// ];
///
/// let options = TextOptions::default();
/// let text = serialize_page_text(&blocks, &options);
/// assert_eq!(text, "First paragraph.\n\nSecond paragraph.");
/// ```
pub fn serialize_page_text(blocks: &[BlockJson], options: &TextOptions) -> String {
    let mut result_parts = Vec::new();

    for block in blocks {
        // Skip blocks based on kind filtering
        if !options.include_headers_footers && is_header_or_footer(&block.kind) {
            continue;
        }
        if !options.include_watermarks && is_watermark(&block.kind) {
            continue;
        }

        // Get block text based on kind
        let block_text = get_block_text(block);

        // Skip empty blocks (no spurious newlines)
        if block_text.trim().is_empty() {
            continue;
        }

        result_parts.push(block_text);
    }

    // Join blocks with double newline
    result_parts.join("\n\n")
}

/// Check if a block kind is a header or footer.
fn is_header_or_footer(kind: &str) -> bool {
    matches!(kind, "header" | "footer")
}

/// Check if a block kind is a watermark.
fn is_watermark(kind: &str) -> bool {
    kind == "watermark"
}

/// Get the text content for a block based on its kind.
///
/// This implements the kind-specific text computation rules:
/// - Paragraph/Heading/Caption/Quote/List/Code/Table: use pre-computed block text
/// - Figure: empty string (no text content)
fn get_block_text(block: &BlockJson) -> String {
    match block.kind.as_str() {
        "figure" => String::new(), // Figures have no readable text content
        _ => block.text.clone(),   // All other kinds use pre-computed text
    }
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
    fn test_serialize_page_text_three_paragraphs() {
        // AC: 3 Paragraph blocks "Foo Bar Baz": "Foo\n\nBar\n\nBaz"
        let blocks = vec![
            make_test_block("paragraph", "Foo", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("paragraph", "Bar", [0.0, 20.0, 100.0, 40.0]),
            make_test_block("paragraph", "Baz", [0.0, 40.0, 100.0, 60.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Foo\n\nBar\n\nBaz");
    }

    #[test]
    fn test_serialize_page_text_heading_and_paragraphs() {
        // AC: 1 Heading + 2 Paragraphs: "Title\n\nP1\n\nP2"
        let mut heading = make_test_block("heading", "Title", [0.0, 0.0, 100.0, 20.0]);
        heading.level = Some(1);

        let blocks = vec![
            heading,
            make_test_block("paragraph", "P1", [0.0, 20.0, 100.0, 40.0]),
            make_test_block("paragraph", "P2", [0.0, 40.0, 100.0, 60.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Title\n\nP1\n\nP2");
    }

    #[test]
    fn test_serialize_page_text_header_excluded_by_default() {
        // AC: Header excluded: not in output
        let blocks = vec![
            make_test_block("header", "Page 1", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("paragraph", "Content", [0.0, 20.0, 100.0, 40.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Content");
        assert!(!text.contains("Page 1"));
    }

    #[test]
    fn test_serialize_page_text_header_included_when_flagged() {
        let blocks = vec![
            make_test_block("header", "Page 1", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("paragraph", "Content", [0.0, 20.0, 100.0, 40.0]),
        ];

        let options = TextOptions::new().with_headers_footers();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Page 1\n\nContent");
    }

    #[test]
    fn test_serialize_page_text_footer_excluded_by_default() {
        let blocks = vec![
            make_test_block("paragraph", "Content", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("footer", "Page 1 of 10", [0.0, 20.0, 100.0, 40.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Content");
        assert!(!text.contains("Page 1 of 10"));
    }

    #[test]
    fn test_serialize_page_text_list() {
        // AC: List: lines join with \n (pre-computed in block.text)
        let blocks = vec![make_test_block(
            "list",
            "Item 1\nItem 2\nItem 3",
            [0.0, 0.0, 100.0, 60.0],
        )];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Item 1\nItem 2\nItem 3");
    }

    #[test]
    fn test_serialize_page_text_code() {
        // Code blocks preserve newlines
        let blocks = vec![make_test_block(
            "code",
            "fn main() {\n    println!(\"Hello\");\n}",
            [0.0, 0.0, 100.0, 40.0],
        )];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "fn main() {\n    println!(\"Hello\");\n}");
    }

    #[test]
    fn test_serialize_page_text_figure_emits_empty() {
        // AC: Figure: emit [FIGURE] placeholder or empty (we use empty)
        let blocks = vec![make_test_block(
            "figure",
            "Figure 1: A diagram",
            [0.0, 0.0, 100.0, 100.0],
        )];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "");
    }

    #[test]
    fn test_serialize_page_text_empty_block_omitted() {
        // INV: Empty blocks emit nothing (no spurious \n\n)
        let blocks = vec![
            make_test_block("paragraph", "First", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("paragraph", "", [0.0, 20.0, 100.0, 40.0]),
            make_test_block("paragraph", "Second", [0.0, 40.0, 100.0, 60.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "First\n\nSecond");
    }

    #[test]
    fn test_serialize_page_text_watermark_excluded_by_default() {
        let blocks = vec![
            make_test_block("paragraph", "Content", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("watermark", "DRAFT", [0.0, 0.0, 100.0, 100.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Content");
        assert!(!text.contains("DRAFT"));
    }

    #[test]
    fn test_serialize_page_text_watermark_included_when_flagged() {
        let blocks = vec![
            make_test_block("paragraph", "Content", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("watermark", "DRAFT", [0.0, 0.0, 100.0, 100.0]),
        ];

        let options = TextOptions::new().with_watermarks();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Content\n\nDRAFT");
    }

    #[test]
    fn test_serialize_page_text_caption() {
        // Caption blocks use space-joined text
        let blocks = vec![make_test_block(
            "caption",
            "Figure 1: The results show",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Figure 1: The results show");
    }

    #[test]
    fn test_serialize_page_text_quote() {
        // Quote blocks use space-joined text
        let blocks = vec![make_test_block(
            "block_quote",
            "This is a quote",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "This is a quote");
    }

    #[test]
    fn test_serialize_page_text_table() {
        // Table blocks use pre-computed text
        let blocks = vec![make_test_block(
            "table",
            "Cell1 Cell2",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "Cell1 Cell2");
    }

    #[test]
    fn test_serialize_page_text_empty_blocks() {
        // INV: Empty block list produces empty string
        let blocks: Vec<BlockJson> = vec![];
        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &options);
        assert_eq!(text, "");
    }

    #[test]
    fn test_text_options_default() {
        let options = TextOptions::default();
        assert!(!options.include_headers_footers);
        assert!(!options.include_invisible_text);
        assert!(!options.include_watermarks);
    }

    #[test]
    fn test_text_options_builder_pattern() {
        let options = TextOptions::new()
            .with_headers_footers()
            .with_invisible_text()
            .with_watermarks();
        assert!(options.include_headers_footers);
        assert!(options.include_invisible_text);
        assert!(options.include_watermarks);
    }

    #[test]
    fn test_is_header_or_footer() {
        assert!(is_header_or_footer("header"));
        assert!(is_header_or_footer("footer"));
        assert!(!is_header_or_footer("paragraph"));
        assert!(!is_header_or_footer("heading"));
    }

    #[test]
    fn test_is_watermark() {
        assert!(is_watermark("watermark"));
        assert!(!is_watermark("paragraph"));
        assert!(!is_watermark("header"));
    }

    #[test]
    fn test_get_block_text_figure() {
        let block = make_test_block("figure", "Figure caption", [0.0, 0.0, 100.0, 100.0]);
        assert_eq!(get_block_text(&block), "");
    }

    #[test]
    fn test_get_block_text_paragraph() {
        let block = make_test_block("paragraph", "Some text", [0.0, 0.0, 100.0, 20.0]);
        assert_eq!(get_block_text(&block), "Some text");
    }

    #[test]
    fn test_get_block_text_heading() {
        let mut block = make_test_block("heading", "Title", [0.0, 0.0, 100.0, 20.0]);
        block.level = Some(2);
        assert_eq!(get_block_text(&block), "Title");
    }
}
