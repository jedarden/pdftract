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

use crate::schema::{BlockJson, SpanJson};

/// Check if a span should be included based on rendering mode and include_invisible option.
///
/// Per PDF spec, rendering_mode values:
/// - 0-2: visible (fill, stroke, or both)
/// - 3: invisible (no rendering)
/// - 4-7: invisible variants (clip modes, no visible rendering)
///
/// Returns false if the span should be excluded, true if it should be included.
fn should_include_span(span: &SpanJson, include_invisible: bool) -> bool {
    // If include_invisible is true, include all spans regardless of rendering_mode
    if include_invisible {
        return true;
    }

    // Filter out invisible text based on rendering_mode
    if let Some(mode) = span.rendering_mode {
        // Tr=3 is invisible, Tr=4-7 are invisible variants (clip modes with no visible rendering)
        if mode >= 3 {
            return false;
        }
    }

    true
}

/// Compute block text from spans with invisible text filtering.
///
/// This function joins span texts while filtering out invisible spans
/// based on the include_invisible option. Operates at SPAN level as required.
///
/// # Arguments
///
/// * `spans` - All spans on the page
/// * `block_spans` - Indices of spans that belong to this block
/// * `include_invisible` - Whether to include invisible text (Tr=3)
///
/// # Returns
///
/// The concatenated text of visible spans in the block, or empty string
/// if all spans are filtered out.
fn compute_block_text_from_spans(
    spans: &[SpanJson],
    block_spans: &[usize],
    include_invisible: bool,
) -> String {
    let mut result = String::new();
    let mut is_first = true;

    for &span_idx in block_spans {
        if let Some(span) = spans.get(span_idx) {
            if should_include_span(span, include_invisible) {
                if !is_first {
                    // Add space between spans from different parts of the block
                    result.push(' ');
                }
                result.push_str(&span.text);
                is_first = false;
            }
        }
    }

    result
}

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
/// * `spans` - All spans on the page (for span-level invisible text filtering)
/// * `options` - Options controlling which blocks are included
///
/// # Returns
///
/// A plain text string with blocks separated by `\n\n`. Empty blocks are omitted
/// entirely (no spurious newlines).
///
/// # Block Text Rules
///
/// - Paragraph/Heading/Caption/Quote: computed from spans with invisible filtering
/// - List/Code: computed from spans with invisible filtering
/// - Figure: empty string (no text content)
/// - Table: computed from spans with invisible filtering
///
/// # Filtering
///
/// - Header/Footer: excluded unless `include_headers_footers` is true
/// - Watermark: excluded unless `include_watermarks` is true
/// - Invisible spans: excluded unless `include_invisible_text` is true (SPAN-level filter)
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
/// let text = serialize_page_text(&blocks, &[], &options);
/// assert_eq!(text, "First paragraph.\n\nSecond paragraph.");
/// ```
pub fn serialize_page_text(
    blocks: &[BlockJson],
    spans: &[SpanJson],
    options: &TextOptions,
) -> String {
    let mut result_parts = Vec::new();

    for block in blocks {
        // Skip blocks based on kind filtering
        if !options.include_headers_footers && is_header_or_footer(&block.kind) {
            continue;
        }
        if !options.include_watermarks && is_watermark(&block.kind) {
            continue;
        }

        // Get block text by filtering spans at SPAN level (not block level)
        // This recomputes block.text from its constituent spans with invisible filtering.
        // If span data is not available (empty block.spans), fall back to pre-computed text.
        // Figures always emit empty text (no readable text content).
        let block_text = if block.kind == "figure" {
            String::new() // Figures have no readable text content
        } else if block.spans.is_empty() {
            // No span data available - use pre-computed text (backward compatibility)
            block.text.clone()
        } else {
            compute_block_text_from_spans(spans, &block.spans, options.include_invisible_text)
        };

        // Skip empty blocks (no spurious newlines) - includes all-invisible blocks
        if block_text.trim().is_empty() {
            continue;
        }

        result_parts.push(block_text);
    }

    // Join blocks with double newline
    result_parts.join("\n\n")
}

/// Serialize document text from multiple pages.
///
/// This function implements the document-level text serialization for Phase 4.6.
/// It calls `serialize_page_text` for each page and joins the results with form
/// feed characters (`\f`, U+000C, 0x0C) BETWEEN pages, with NO trailing form feed.
///
/// # Arguments
///
/// * `pages` - Slice of tuples containing (blocks, spans) for each page
/// * `options` - Options controlling which blocks are included
///
/// # Returns
///
/// A plain text string with pages separated by `\f`. Empty pages contribute empty
/// strings but still receive form feeds between them (except after the last page).
///
/// # Form Feed Invariant
///
/// - N pages → N-1 form feeds (e.g., 10 pages = 9 form feeds)
/// - No leading form feed
/// - No trailing form feed
/// - Empty page in middle: form feed before AND after
///
/// # Examples
///
/// ```
/// use pdftract_core::schema::BlockJson;
/// use pdftract_core::text::{serialize_document_text, TextOptions};
///
/// let pages = vec![
///     // Page 0: one paragraph
///     (vec![block("P1")], vec![]),
///     // Page 1: one paragraph
///     (vec![block("P2")], vec![]),
/// ];
///
/// let options = TextOptions::default();
/// let text = serialize_document_text(&pages, &options);
/// assert_eq!(text, "P1\fP2");  // One form feed between two pages
/// ```
pub fn serialize_document_text<'a>(
    pages: &[(&'a [BlockJson], &'a [SpanJson])],
    options: &TextOptions,
) -> String {
    if pages.is_empty() {
        return String::new();
    }

    let mut result_parts = Vec::with_capacity(pages.len());

    for (blocks, spans) in pages {
        let page_text = serialize_page_text(blocks, spans, options);
        result_parts.push(page_text);
    }

    // Join pages with form feed (U+000C, 0x0C)
    // This produces exactly N-1 form feeds for N pages
    result_parts.join("\u{000C}")
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
        assert_eq!(text, "Page 1\n\nContent");
    }

    #[test]
    fn test_serialize_page_text_footer_excluded_by_default() {
        let blocks = vec![
            make_test_block("paragraph", "Content", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("footer", "Page 1 of 10", [0.0, 20.0, 100.0, 40.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
        assert_eq!(text, "First\n\nSecond");
    }

    #[test]
    fn test_serialize_page_text_watermark_excluded_by_default() {
        let blocks = vec![
            make_test_block("paragraph", "Content", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("watermark", "DRAFT", [0.0, 0.0, 100.0, 100.0]),
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
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
        let text = serialize_page_text(&blocks, &[], &options);
        assert_eq!(text, "Cell1 Cell2");
    }

    #[test]
    fn test_serialize_page_text_empty_blocks() {
        // INV: Empty block list produces empty string
        let blocks: Vec<BlockJson> = vec![];
        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &[], &options);
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

    // Invisible text filtering tests (pdftract-38p8h)

    fn make_test_span(text: &str, bbox: [f64; 4], rendering_mode: Option<u8>) -> SpanJson {
        SpanJson {
            text: text.to_string(),
            bbox,
            font: "Helvetica".to_string(),
            size: 12.0,
            color: None,
            rendering_mode,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }
    }

    #[test]
    fn test_should_include_span_visible_mode_0() {
        // AC: rendering_mode 0 (fill) is always included
        let span = make_test_span("visible", [0.0, 0.0, 100.0, 20.0], Some(0));
        assert!(should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_visible_mode_1() {
        // AC: rendering_mode 1 (stroke) is always included
        let span = make_test_span("visible", [0.0, 0.0, 100.0, 20.0], Some(1));
        assert!(should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_visible_mode_2() {
        // AC: rendering_mode 2 (fill then stroke) is always included
        let span = make_test_span("visible", [0.0, 0.0, 100.0, 20.0], Some(2));
        assert!(should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_invisible_mode_3_excluded_by_default() {
        // AC: rendering_mode 3 (invisible) excluded when include_invisible=false
        let span = make_test_span("invisible", [0.0, 0.0, 100.0, 20.0], Some(3));
        assert!(!should_include_span(&span, false));
    }

    #[test]
    fn test_should_include_span_invisible_mode_3_included_when_flagged() {
        // AC: rendering_mode 3 (invisible) included when include_invisible=true
        let span = make_test_span("invisible", [0.0, 0.0, 100.0, 20.0], Some(3));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_invisible_mode_4_excluded_by_default() {
        // AC: rendering_mode 4 (fill to clip) treated same as mode 3
        let span = make_test_span("clip", [0.0, 0.0, 100.0, 20.0], Some(4));
        assert!(!should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_invisible_mode_5_excluded_by_default() {
        // AC: rendering_mode 5 (stroke to clip) treated same as mode 3
        let span = make_test_span("clip", [0.0, 0.0, 100.0, 20.0], Some(5));
        assert!(!should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_invisible_mode_6_excluded_by_default() {
        // AC: rendering_mode 6 (fill then stroke to clip) treated same as mode 3
        let span = make_test_span("clip", [0.0, 0.0, 100.0, 20.0], Some(6));
        assert!(!should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_invisible_mode_7_excluded_by_default() {
        // AC: rendering_mode 7 (clip) treated same as mode 3
        let span = make_test_span("clip", [0.0, 0.0, 100.0, 20.0], Some(7));
        assert!(!should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_should_include_span_no_rendering_mode() {
        // AC: spans without rendering_mode are included (default visible)
        let span = make_test_span("default", [0.0, 0.0, 100.0, 20.0], None);
        assert!(should_include_span(&span, false));
        assert!(should_include_span(&span, true));
    }

    #[test]
    fn test_compute_block_text_from_spans_mixed_visibility() {
        // AC: Mixed block with visible and invisible spans - only visible emitted
        let spans = vec![
            make_test_span("visible", [0.0, 0.0, 50.0, 20.0], Some(0)),
            make_test_span("invisible", [50.0, 0.0, 100.0, 20.0], Some(3)),
            make_test_span("visible2", [100.0, 0.0, 150.0, 20.0], Some(0)),
        ];

        let block_spans = vec![0, 1, 2];
        let text = compute_block_text_from_spans(&spans, &block_spans, false);
        assert_eq!(text, "visible visible2");
    }

    #[test]
    fn test_compute_block_text_from_spans_all_invisible_excluded() {
        // AC: All-invisible block produces empty text (no spurious \n\n)
        let spans = vec![
            make_test_span("hidden1", [0.0, 0.0, 50.0, 20.0], Some(3)),
            make_test_span("hidden2", [50.0, 0.0, 100.0, 20.0], Some(4)),
        ];

        let block_spans = vec![0, 1];
        let text = compute_block_text_from_spans(&spans, &block_spans, false);
        assert_eq!(text, "");
    }

    #[test]
    fn test_compute_block_text_from_spans_include_invisible_true() {
        // AC: With include_invisible=true, invisible spans are included
        let spans = vec![
            make_test_span("visible", [0.0, 0.0, 50.0, 20.0], Some(0)),
            make_test_span("invisible", [50.0, 0.0, 100.0, 20.0], Some(3)),
        ];

        let block_spans = vec![0, 1];
        let text = compute_block_text_from_spans(&spans, &block_spans, true);
        assert_eq!(text, "visible invisible");
    }

    #[test]
    fn test_serialize_page_text_invisible_span_filtered() {
        // AC: Invisible text span excluded from --text output by default
        let spans = vec![
            make_test_span("visible", [0.0, 0.0, 50.0, 20.0], Some(0)),
            make_test_span("invisible", [50.0, 0.0, 100.0, 20.0], Some(3)),
        ];

        let blocks = vec![BlockJson {
            kind: "paragraph".to_string(),
            text: "visible invisible".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            level: None,
            table_index: None,
            spans: vec![0, 1],
            receipt: None,
        }];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &spans, &options);
        assert_eq!(text, "visible");
        assert!(!text.contains("invisible"));
    }

    #[test]
    fn test_serialize_page_text_invisible_span_included_when_flagged() {
        // AC: Invisible text span included when include_invisible_text=true
        let spans = vec![
            make_test_span("visible", [0.0, 0.0, 50.0, 20.0], Some(0)),
            make_test_span("invisible", [50.0, 0.0, 100.0, 20.0], Some(3)),
        ];

        let blocks = vec![BlockJson {
            kind: "paragraph".to_string(),
            text: "visible invisible".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            level: None,
            table_index: None,
            spans: vec![0, 1],
            receipt: None,
        }];

        let options = TextOptions::new().with_invisible_text();
        let text = serialize_page_text(&blocks, &spans, &options);
        assert_eq!(text, "visible invisible");
    }

    #[test]
    fn test_serialize_page_text_all_invisible_block_omitted() {
        // AC: All-invisible block omitted from output (no spurious \n\n)
        let spans = vec![make_test_span("hidden", [0.0, 0.0, 100.0, 20.0], Some(3))];

        let blocks = vec![
            BlockJson {
                kind: "paragraph".to_string(),
                text: "visible".to_string(),
                bbox: [0.0, 0.0, 100.0, 20.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "hidden".to_string(),
                bbox: [0.0, 20.0, 100.0, 40.0],
                level: None,
                table_index: None,
                spans: vec![0],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "visible2".to_string(),
                bbox: [0.0, 40.0, 100.0, 60.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
        ];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &spans, &options);
        // Empty block should be skipped, resulting in no double newline between visible blocks
        assert!(text.contains("visible"));
        assert!(!text.contains("hidden"));
        // Count the number of double newlines - should be exactly 1 (between the two visible blocks)
        let double_newline_count = text.matches("\n\n").count();
        assert_eq!(double_newline_count, 1);
    }

    #[test]
    fn test_serialize_page_text_mixed_blocks_with_invisible() {
        // AC: Mixed visibility blocks - visible emitted, invisible filtered
        let spans = vec![
            make_test_span("visible1", [0.0, 0.0, 50.0, 20.0], Some(0)),
            make_test_span("invisible", [50.0, 0.0, 100.0, 20.0], Some(3)),
            make_test_span("visible2", [100.0, 0.0, 150.0, 20.0], Some(0)),
        ];

        let blocks = vec![BlockJson {
            kind: "paragraph".to_string(),
            text: "visible1 invisible visible2".to_string(),
            bbox: [0.0, 0.0, 150.0, 20.0],
            level: None,
            table_index: None,
            spans: vec![0, 1, 2],
            receipt: None,
        }];

        let options = TextOptions::default();
        let text = serialize_page_text(&blocks, &spans, &options);
        assert_eq!(text, "visible1 visible2");
        assert!(!text.contains("invisible"));
    }

    // Document-level serializer tests (pdftract-3bgxq)

    #[test]
    fn test_serialize_document_text_one_page() {
        // AC: 1 page: 0 form feeds
        let blocks = vec![make_test_block("paragraph", "P1", [0.0, 0.0, 100.0, 20.0])];
        let spans: Vec<SpanJson> = vec![];
        let pages = vec![(&blocks[..], &spans[..])];

        let options = TextOptions::default();
        let text = serialize_document_text(&pages, &options);

        assert_eq!(text, "P1");
        assert_eq!(
            text.matches('\x0c').count(),
            0,
            "1 page should have 0 form feeds"
        );
    }

    #[test]
    fn test_serialize_document_text_two_pages() {
        // AC: 2 pages: 1 form feed
        let blocks1 = vec![make_test_block("paragraph", "P1", [0.0, 0.0, 100.0, 20.0])];
        let blocks2 = vec![make_test_block("paragraph", "P2", [0.0, 0.0, 100.0, 20.0])];
        let spans: Vec<SpanJson> = vec![];
        let pages = vec![(&blocks1[..], &spans[..]), (&blocks2[..], &spans[..])];

        let options = TextOptions::default();
        let text = serialize_document_text(&pages, &options);

        assert_eq!(text, "P1\x0cP2");
        assert_eq!(
            text.matches('\x0c').count(),
            1,
            "2 pages should have 1 form feed"
        );
    }

    #[test]
    fn test_serialize_document_text_ten_pages() {
        // AC: 10 pages: 9 form feeds (critical test from plan)
        // Store all blocks to keep them alive for the duration of the test
        let blocks_vec: Vec<Vec<BlockJson>> = (1..=10)
            .map(|i| {
                vec![make_test_block(
                    "paragraph",
                    &format!("P{}", i),
                    [0.0, 0.0, 100.0, 20.0],
                )]
            })
            .collect();
        let spans: Vec<SpanJson> = vec![];

        let pages: Vec<(&[BlockJson], &[SpanJson])> = blocks_vec
            .iter()
            .map(|blocks| (blocks.as_slice(), spans.as_slice()))
            .collect();

        let options = TextOptions::default();
        let text = serialize_document_text(&pages, &options);

        assert_eq!(
            text.matches('\x0c').count(),
            9,
            "10 pages should have exactly 9 form feeds"
        );
        // Verify no leading form feed
        assert!(
            !text.starts_with('\x0c'),
            "Should not have leading form feed"
        );
        // Verify no trailing form feed
        assert!(
            !text.ends_with('\x0c'),
            "Should not have trailing form feed"
        );
    }

    #[test]
    fn test_serialize_document_text_empty_page_in_middle() {
        // AC: Empty page in middle: form feed before AND after
        let blocks1 = vec![make_test_block("paragraph", "P1", [0.0, 0.0, 100.0, 20.0])];
        let blocks2: Vec<BlockJson> = vec![]; // Empty page
        let blocks3 = vec![make_test_block("paragraph", "P3", [0.0, 0.0, 100.0, 20.0])];
        let spans: Vec<SpanJson> = vec![];
        let pages = vec![
            (&blocks1[..], &spans[..]),
            (&blocks2[..], &spans[..]),
            (&blocks3[..], &spans[..]),
        ];

        let options = TextOptions::default();
        let text = serialize_document_text(&pages, &options);

        // Should be: "P1\x0c\x0cP3" (two form feeds for the empty page)
        assert_eq!(
            text.matches('\x0c').count(),
            2,
            "3 pages with empty middle should have 2 form feeds"
        );
        assert!(text.contains("P1\x0c\x0cP3"));
    }

    #[test]
    fn test_serialize_document_text_empty_document() {
        // AC: Empty document: empty string
        let pages: Vec<(&[BlockJson], &[SpanJson])> = vec![];
        let options = TextOptions::default();
        let text = serialize_document_text(&pages, &options);

        assert_eq!(text, "", "Empty document should produce empty string");
    }

    #[test]
    fn test_serialize_document_text_filters_headers() {
        // AC: Header excluded by default across all pages
        let blocks1 = vec![
            make_test_block("header", "Header", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("paragraph", "P1", [0.0, 20.0, 100.0, 40.0]),
        ];
        let blocks2 = vec![
            make_test_block("header", "Header", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("paragraph", "P2", [0.0, 20.0, 100.0, 40.0]),
        ];
        let spans: Vec<SpanJson> = vec![];
        let pages = vec![(&blocks1[..], &spans[..]), (&blocks2[..], &spans[..])];

        let options = TextOptions::default();
        let text = serialize_document_text(&pages, &options);

        assert!(
            !text.contains("Header"),
            "Headers should be excluded by default"
        );
        assert!(text.contains("P1"));
        assert!(text.contains("P2"));
    }

    #[test]
    fn test_serialize_document_text_includes_headers_when_flagged() {
        // AC: Header included when flag is set
        let blocks1 = vec![
            make_test_block("header", "Header1", [0.0, 0.0, 100.0, 20.0]),
            make_test_block("paragraph", "P1", [0.0, 20.0, 100.0, 40.0]),
        ];
        let spans: Vec<SpanJson> = vec![];
        let pages = vec![(&blocks1[..], &spans[..])];

        let options = TextOptions::new().with_headers_footers();
        let text = serialize_document_text(&pages, &options);

        assert!(
            text.contains("Header1"),
            "Headers should be included when flag is set"
        );
        assert!(text.contains("P1"));
    }
}
