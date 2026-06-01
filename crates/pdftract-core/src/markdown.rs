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
    SpanJson, TableJson, ThreadJson,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Markdown emission options for controlling block inclusion.
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkdownOptions {
    /// Include header and footer blocks in output.
    pub include_headers_footers: bool,
    /// Include watermark blocks in output.
    pub include_watermarks: bool,
    /// Include page break separators between pages.
    pub include_page_breaks: bool,
}

impl MarkdownOptions {
    /// Create a new MarkdownOptions with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to include headers and footers.
    pub fn with_headers_footers(mut self, include: bool) -> Self {
        self.include_headers_footers = include;
        self
    }

    /// Set whether to include watermarks.
    pub fn with_watermarks(mut self, include: bool) -> Self {
        self.include_watermarks = include;
        self
    }

    /// Set whether to include page breaks.
    pub fn with_page_breaks(mut self, include: bool) -> Self {
        self.include_page_breaks = include;
        self
    }
}

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

/// Emit a page anchor for internal link targets.
///
/// This function emits an HTML anchor tag that can be referenced by internal
/// links of the form `[text](#page-N)`. The anchor is formatted as a markdown
/// HTML reference: `<a name="page-N"></a>` where N is the 1-based page number.
///
/// # Arguments
///
/// * `page_index` - Zero-based page index
///
/// # Returns
///
/// A markdown string containing the HTML anchor tag.
///
/// # Example
///
/// ```
/// use pdftract_core::markdown::emit_page_anchor;
///
/// let anchor = emit_page_anchor(0);
/// assert_eq!(anchor, r#"<a name="page-1"></a>"#);
///
/// let anchor = emit_page_anchor(4);
/// assert_eq!(anchor, r#"<a name="page-5"></a>"#);
/// ```
pub fn emit_page_anchor(page_index: usize) -> String {
    format!(r#"<a name="page-{}"></a>"#, page_index + 1)
}

/// Emit a block as Markdown based on its kind.
///
/// This function implements the Phase 6.5 block-kind dispatch table, mapping
/// each block type to its appropriate Markdown representation.
///
/// # Block Kind Dispatch Table
///
/// | Block kind | Markdown emission |
/// |---|---|
/// | `heading` (level N) | `#` × N + space + text + `\n\n` |
/// | `paragraph` | text + `\n\n`; soft line breaks as `  \n` |
/// | `list` (bulleted) | `- item\n` per item |
/// | `list` (numbered) | `1. item\n` (preserves source numbering) |
/// | `code` | Fenced block with language detection |
/// | `formula` (inline) | `$expr$` |
/// | `formula` (display) | `$$\nexpr\n$$\n\n` |
/// | `table` | GFM pipe table or HTML fallback |
/// | `caption` | `*text*\n\n` |
/// | `figure` | `![alt](#)\n\n` |
/// | `header` / `footer` | Skipped unless `include_headers_footers` |
/// | `watermark` | Skipped unless `include_watermarks` |
/// | `block_quote` | `> line\n` per line |
/// | `toc` | Emitted as plain text |
/// | `note` / `footnote` | Emitted as inline text |
/// | `reference` | Emitted as plain text |
///
/// # Arguments
///
/// * `block` - The block to convert
/// * `tables` - The tables array for looking up table structures
/// * `options` - Markdown emission options
///
/// # Returns
///
/// A markdown string representing the block.
fn emit_block_kind(block: &BlockJson, tables: &[TableJson], options: &MarkdownOptions) -> String {
    match block.kind.as_str() {
        "heading" => emit_heading(block),

        "paragraph" => emit_paragraph(block),

        "list" | "list_item" => emit_list_item(block),

        "code" => emit_code_block(block),

        "formula" => emit_formula(block),

        "table" => emit_table_block(block, tables),

        "caption" => emit_caption(block),

        "figure" => emit_figure(block),

        "header" | "footer" => {
            if options.include_headers_footers {
                emit_header_footer(block)
            } else {
                String::new()
            }
        }

        "watermark" => {
            if options.include_watermarks {
                emit_watermark(block)
            } else {
                String::new()
            }
        }

        "block_quote" => emit_block_quote(block),

        "toc" => emit_toc(block),

        "note" | "footnote" => emit_note(block),

        "reference" => emit_reference(block),

        "list_label" | "list_body" => {
            // These are internal structural elements, emit as plain text
            format!("{}\n", block.text)
        }

        _ => {
            // Unknown block kinds fall back to plain text
            format!("{}\n", block.text)
        }
    }
}

/// Emit a heading block with level from block.level or default to 1.
fn emit_heading(block: &BlockJson) -> String {
    let level = block.level.unwrap_or(1).clamp(1, 6);
    let prefix = "#".repeat(level as usize);
    format!("{} {}\n\n", prefix, block.text)
}

/// Emit a paragraph block with soft line breaks preserved.
fn emit_paragraph(block: &BlockJson) -> String {
    // Soft line breaks within a paragraph are encoded as trailing "  \n"
    // (CommonMark hard break syntax). Internal newlines in block.text
    // become soft breaks, while the paragraph ends with "\n\n".
    let text = block.text.replace('\n', "  \n");
    format!("{}\n\n", text)
}

/// Emit a list item (bulleted or numbered).
/// This is used for isolated list items without nesting context.
fn emit_list_item(block: &BlockJson) -> String {
    // Try to detect if this is a numbered list by checking if text starts with a number
    let is_numbered = block
        .text
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false);

    if is_numbered {
        // Numbered list item - preserve source numbering
        format!("{}\n", block.text)
    } else {
        // Bulleted list item
        format!("* {}\n", block.text)
    }
}

/// Emit a sequence of list blocks with proper nesting support.
///
/// This function groups consecutive list items and emits them with proper
/// indentation based on their bbox x0 (left margin) values. Nested sublists
/// are indented by 2 spaces per level per CommonMark convention.
///
/// # Arguments
///
/// * `list_blocks` - A slice of consecutive list blocks
///
/// # Returns
///
/// A markdown string with properly indented list items.
///
/// # Nesting Detection
///
/// Nesting level is inferred from the bbox x0 (left margin) value:
/// - All items at the same x0 are at the same nesting level
/// - Items with greater x0 are nested under the previous item
/// - Each nesting level adds 2 spaces of indentation
fn emit_list_blocks(list_blocks: &[BlockJson]) -> String {
    if list_blocks.is_empty() {
        return String::new();
    }

    // Group by x0 value to detect nesting levels
    let mut result = String::new();
    let mut indent_levels: Vec<f64> = Vec::new(); // Track x0 values for each nesting level

    for block in list_blocks {
        let x0 = block.bbox[0];

        // Determine nesting level by comparing x0 to known levels
        let mut level = 0;
        for (i, &indent) in indent_levels.iter().enumerate() {
            if (x0 - indent).abs() < 5.0 {
                // x0 matches this level (within 5 point tolerance)
                level = i;
                break;
            }
        }

        // If x0 doesn't match any known level, it's a new level
        if level == 0 && indent_levels.iter().all(|&v| (x0 - v).abs() >= 5.0) {
            level = indent_levels.len();
            indent_levels.push(x0);
        } else if level < indent_levels.len() && indent_levels.iter().enumerate().all(|(i, &v)| i != level || (x0 - v).abs() >= 5.0) {
            // x0 is a new level beyond current ones
            level = indent_levels.len();
            indent_levels.push(x0);
        }

        // Detect if this is a numbered list item
        let is_numbered = block
            .text
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false);

        // Emit with proper indentation
        let indent = "  ".repeat(level);
        if is_numbered {
            // Numbered list item - preserve source numbering
            result.push_str(&format!("{}{}\n", indent, block.text));
        } else {
            // Bulleted list item
            result.push_str(&format!("{}* {}\n", indent, block.text));
        }
    }

    result
}

/// Emit a code block with language detection.
fn emit_code_block(block: &BlockJson) -> String {
    // Detect language from monospace font hint + optional shebang/keyword sniff
    let lang = detect_code_language(&block.text);
    format!("```{}\n{}\n```\n\n", lang, block.text)
}

/// Detect the programming language from code content.
///
/// This is a best-effort heuristic based on:
/// - Shebang lines (e.g., `#!/usr/bin/env python`)
/// - Common language keywords/patterns
/// Falls back to empty string (no language specified)
fn detect_code_language(code: &str) -> &str {
    let first_line = code.lines().next().unwrap_or("");

    // Check for shebang
    if first_line.starts_with("#!") {
        if first_line.contains("python") || first_line.contains("python3") {
            return "python";
        }
        if first_line.contains("bash") || first_line.contains("sh") {
            return "bash";
        }
        if first_line.contains("node") || first_line.contains("javascript") {
            return "javascript";
        }
        if first_line.contains("perl") {
            return "perl";
        }
        if first_line.contains("ruby") {
            return "ruby";
        }
    }

    // Check for common language patterns
    let lower = code.to_lowercase();

    // Rust patterns
    if lower.contains("fn main()") || lower.contains("use std::") || lower.contains("let mut ") {
        return "rust";
    }

    // Python patterns
    if lower.contains("def ") || lower.contains("import ") || lower.contains("from ") {
        return "python";
    }

    // JavaScript patterns
    if lower.contains("function ") || lower.contains("const ") || lower.contains("let ") {
        return "javascript";
    }

    // C/C++ patterns
    if lower.contains("#include <") || lower.contains("#include \"") {
        return "c";
    }

    // Java patterns
    if lower.contains("public class") || lower.contains("public static void main") {
        return "java";
    }

    // Go patterns
    if lower.contains("func ") && lower.contains("package ") {
        return "go";
    }

    // Default: no language specified
    ""
}

/// Emit a formula (inline or display).
fn emit_formula(block: &BlockJson) -> String {
    // Distinguish inline vs display mode by checking if the formula
    // contains newlines. Single-line formulas are inline ($...$),
    // multi-line formulas are display ($$\n...\n$$).
    if block.text.contains('\n') {
        // Display mode: multi-line formula
        format!("$$\n{}\n$$\n\n", block.text)
    } else {
        // Inline mode: single-line formula
        format!("${}$", block.text)
    }
}

/// Emit a table block with lookup from tables array.
fn emit_table_block(block: &BlockJson, tables: &[TableJson]) -> String {
    // Look up the table structure from the tables array
    if let Some(table_idx) = block.table_index {
        if let Some(table) = tables.get(table_idx) {
            emit_table(table)
        } else {
            // Fallback to text if table index is invalid
            format!("| {}\n", block.text)
        }
    } else {
        // Fallback to text if no table index
        format!("| {}\n", block.text)
    }
}

/// Emit a caption block (italic text).
fn emit_caption(block: &BlockJson) -> String {
    format!("*{}*\n\n", block.text)
}

/// Emit a figure block with alt text placeholder.
fn emit_figure(block: &BlockJson) -> String {
    // Use block.text as alt text, with placeholder path
    format!("![{}]()\n\n", block.text)
}

/// Emit a header or footer block.
fn emit_header_footer(block: &BlockJson) -> String {
    format!("{}\n", block.text)
}

/// Emit a watermark block.
fn emit_watermark(block: &BlockJson) -> String {
    format!("{}\n", block.text)
}

/// Emit a block quote (prefixed lines).
fn emit_block_quote(block: &BlockJson) -> String {
    // Prefix each line with "> "
    block
        .text
        .lines()
        .map(|line| format!("> {}\n", line))
        .collect()
}

/// Emit a table of contents block.
fn emit_toc(block: &BlockJson) -> String {
    format!("{}\n", block.text)
}

/// Emit a note or footnote block.
fn emit_note(block: &BlockJson) -> String {
    format!("{}\n", block.text)
}

/// Emit a reference block.
fn emit_reference(block: &BlockJson) -> String {
    format!("{}\n", block.text)
}

/// Convert a block to markdown with optional anchor comment.
///
/// If `include_anchor` is true, emits an HTML comment before the block content.
///
/// # Arguments
///
/// * `block` - The block to convert
/// * `tables` - The tables array for looking up table structures by table_index
/// * `page_index` - Zero-based page index
/// * `block_index` - Zero-based block index within the page
/// * `include_anchor` - Whether to include the HTML comment anchor
///
/// # Returns
///
/// A markdown string with optional anchor.
pub fn block_to_markdown(
    block: &BlockJson,
    tables: &[TableJson],
    page_index: usize,
    block_index: usize,
    include_anchor: bool,
) -> String {
    block_to_markdown_with_options(
        block,
        tables,
        page_index,
        block_index,
        include_anchor,
        &MarkdownOptions::default(),
    )
}

/// Convert a block to markdown with optional anchor comment and custom options.
///
/// # Arguments
///
/// * `block` - The block to convert
/// * `tables` - The tables array for looking up table structures by table_index
/// * `page_index` - Zero-based page index
/// * `block_index` - Zero-based block index within the page
/// * `include_anchor` - Whether to include the HTML comment anchor
/// * `options` - Markdown emission options
///
/// # Returns
///
/// A markdown string with optional anchor.
pub fn block_to_markdown_with_options(
    block: &BlockJson,
    tables: &[TableJson],
    page_index: usize,
    block_index: usize,
    include_anchor: bool,
    options: &MarkdownOptions,
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

    // Add block content based on kind using the dispatch table
    result.push_str(&emit_block_kind(block, tables, options));

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
/// * `tables` - The tables array for looking up table structures
/// * `page_index` - Zero-based page index
/// * `include_anchor` - Whether to include HTML comment anchors
/// * `include_page_break` - Whether to add a page break separator
///
/// # Returns
///
/// A markdown string with all blocks from the page.
/// Convert all blocks from a page to markdown with optional anchors.
///
/// If `include_anchor` is true, each block is preceded by an HTML comment.
/// If `include_page_break` is true, adds a horizontal rule between pages.
///
/// # Arguments
///
/// * `blocks` - The blocks to convert
/// * `tables` - The tables array for looking up table structures
/// * `page_index` - Zero-based page index
/// * `include_anchor` - Whether to include HTML comment anchors
/// * `include_page_break` - Whether to add a page break separator
///
/// # Returns
///
/// A markdown string with all blocks from the page.
pub fn page_to_markdown(
    blocks: &[BlockJson],
    tables: &[TableJson],
    page_index: usize,
    include_anchor: bool,
    include_page_break: bool,
) -> String {
    let options = MarkdownOptions {
        include_page_breaks: include_page_break,
        ..Default::default()
    };
    page_to_markdown_with_options(blocks, tables, page_index, include_anchor, &options)
}

/// Convert all blocks from a page to markdown with full options control.
///
/// # Arguments
///
/// * `blocks` - The blocks to convert
/// * `tables` - The tables array for looking up table structures
/// * `page_index` - Zero-based page index
/// * `include_anchor` - Whether to include HTML comment anchors
/// * `options` - Markdown emission options
///
/// # Returns
///
/// A markdown string with all blocks from the page.
pub fn page_to_markdown_with_options(
    blocks: &[BlockJson],
    tables: &[TableJson],
    page_index: usize,
    include_anchor: bool,
    options: &MarkdownOptions,
) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < blocks.len() {
        let block = &blocks[i];

        // Check if this is a list item and if there are consecutive list items
        if block.kind == "list" || block.kind == "list_item" {
            // Find the end of the consecutive list sequence
            let mut list_end = i + 1;
            while list_end < blocks.len()
                && (blocks[list_end].kind == "list" || blocks[list_end].kind == "list_item")
            {
                list_end += 1;
            }

            // Emit the entire list sequence as a group
            let list_blocks = &blocks[i..list_end];
            let list_md = emit_list_blocks(list_blocks);
            result.push_str(&list_md);
            result.push('\n');

            i = list_end;
        } else {
            // Non-list block - emit individually
            let md = block_to_markdown_with_options(
                block,
                tables,
                page_index,
                i,
                include_anchor,
                options,
            );
            result.push_str(&md);
            result.push('\n');
            i += 1;
        }
    }

    // Add page break if requested and this isn't the last page
    if options.include_page_breaks {
        result.push_str("\n---\n\n");
    }

    result
}

/// Emit spans with inline link support.
///
/// This function processes spans and emits them as markdown, with spans that
/// are part of link annotations emitted as inline links `[anchor text](URL)`
/// instead of plain styled text.
///
/// This implements Phase 6.5.5b: inline-link emission from Phase 7.6 link annotations.
///
/// # Arguments
///
/// * `spans` - The spans to emit
/// * `page_links` - Link annotations for this page (from Phase 7.6)
///
/// # Returns
///
/// A markdown string with spans emitted, including inline links where applicable.
///
/// # Example
///
/// ```
/// use pdftract_core::markdown::spans_to_markdown_with_links;
/// use pdftract_core::schema::SpanJson;
///
/// let spans = vec![
///     SpanJson { text: "Click ".to_string(), ..Default::default() },
///     SpanJson { text: "here".to_string(), ..Default::default() },
///     SpanJson { text: " for more".to_string(), ..Default::default() },
/// ];
///
/// // If "here" is part of a link, it will be emitted as [here](https://example.com)
/// let md = spans_to_markdown_with_links(&spans, &[]);
/// ```
pub fn spans_to_markdown_with_links(spans: &[SpanJson], page_links: &[crate::schema::LinkJson]) -> String {
    use crate::output::markdown::links;

    if page_links.is_empty() {
        // No links - emit spans normally with inline styling
        return spans.iter().map(span_to_markdown).collect::<String>();
    }

    // Process links to find which spans are covered
    let link_data = links::emit_page_links_from_json(spans, page_links);

    // Build a map of span index -> link markdown, but only for the FIRST span in each link
    // Other spans in the link are skipped because their text is already included in the anchor text
    let mut span_to_link: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    let mut span_is_in_link: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (span_indices, link_markdown) in &link_data {
        if let Some(&first_idx) = span_indices.first() {
            span_to_link.insert(first_idx, link_markdown.clone());
        }
        // Mark all spans in this link as "used" so we skip them
        for &idx in span_indices {
            span_is_in_link.insert(idx);
        }
    }

    // Emit spans: if a span is part of a link, use the link markdown; otherwise use normal styling
    let mut result = String::new();
    for (idx, span) in spans.iter().enumerate() {
        if let Some(link_md) = span_to_link.get(&idx) {
            // This span is the FIRST span in a link - emit the link markdown
            result.push_str(link_md);
        } else if span_is_in_link.contains(&idx) {
            // This span is part of a link but not the first - skip it
            // (its text is already included in the anchor text from the first span)
        } else {
            // Not part of a link - emit normal styled span
            result.push_str(&span_to_markdown(span));
        }
    }

    result
}

/// Emit a block's text with inline link support.
///
/// This function emits a block's text content, replacing portions that correspond
/// to link annotations with inline markdown links. This is useful for paragraphs
/// and other text blocks that may contain hyperlinks.
///
/// # Arguments
///
/// * `block` - The block to emit
/// * `spans` - All spans on the page (for link detection)
/// * `page_links` - Link annotations for this page (from Phase 7.6)
///
/// # Returns
///
/// A markdown string with the block's text, including inline links where applicable.
///
/// # Example
///
/// ```
/// use pdftract_core::markdown::block_to_markdown_with_links;
/// use pdftract_core::schema::{BlockJson, SpanJson};
///
/// let block = BlockJson {
///     kind: "paragraph".to_string(),
///     text: "See our website for details.".to_string(),
///     // ... other fields
/// };
///
/// let md = block_to_markdown_with_links(&block, &spans, &links);
/// // Result might be: "See our [website](https://example.com) for details."
/// ```
pub fn block_to_markdown_with_links(
    block: &BlockJson,
    spans: &[SpanJson],
    page_links: &[crate::schema::LinkJson],
) -> String {
    if page_links.is_empty() {
        // No links - return the block text as-is (paragraph emission will wrap it)
        return block.text.clone();
    }

    use crate::output::markdown::links;

    // Find which spans belong to this block
    let block_span_indices: Vec<usize> = block.spans.iter().filter_map(|&idx| {
        if idx < spans.len() { Some(idx) } else { None }
    }).collect();

    if block_span_indices.is_empty() {
        // No spans for this block - return text as-is
        return block.text.clone();
    }

    // Filter links to only those that intersect this block's spans
    let block_links: Vec<&crate::schema::LinkJson> = page_links
        .iter()
        .filter(|link| {
            // Check if any of this link's spans are in this block
            let matched_spans = links::find_spans_in_link_json(spans, link);
            matched_spans.iter().any(|idx| block.spans.contains(idx))
        })
        .collect();

    if block_links.is_empty() {
        // No links for this block - return text as-is
        return block.text.clone();
    }

    // Emit the spans for this block with link support
    let block_spans: Vec<SpanJson> = block_span_indices
        .iter()
        .filter_map(|&idx| spans.get(idx).cloned())
        .collect();

    let block_links_refs: Vec<crate::schema::LinkJson> = block_links
        .iter()
        .map(|&link| link.clone())
        .collect();

    spans_to_markdown_with_links(&block_spans, &block_links_refs)
}

/// Emit all blocks from a page with inline link support.
///
/// This is a variant of `page_to_markdown_with_options` that also processes
/// link annotations and emits inline markdown links where applicable.
///
/// # Arguments
///
/// * `blocks` - The blocks to convert
/// * `spans` - All spans on the page (for link detection)
/// * `tables` - The tables array for looking up table structures
/// * `page_links` - Link annotations for this page (from Phase 7.6)
/// * `page_index` - Zero-based page index
/// * `include_anchor` - Whether to include HTML comment anchors
/// * `options` - Markdown emission options
///
/// # Returns
///
/// A markdown string with all blocks from the page, including inline links.
///
/// # Example
///
/// ```
/// use pdftract_core::markdown::page_to_markdown_with_links;
///
/// let md = page_to_markdown_with_links(
///     &blocks,
///     &spans,
///     &tables,
///     &links,
///     0,
///     true,
///     &MarkdownOptions::default(),
/// );
/// ```
pub fn page_to_markdown_with_links(
    blocks: &[BlockJson],
    spans: &[SpanJson],
    tables: &[TableJson],
    page_links: &[crate::schema::LinkJson],
    page_index: usize,
    include_anchor: bool,
    options: &MarkdownOptions,
) -> String {
    let mut result = String::new();

    // Emit page anchor for internal link targets
    // This allows links like [text](#page-N) to jump to this page
    result.push_str(&emit_page_anchor(page_index));
    result.push('\n');

    let mut i = 0;

    while i < blocks.len() {
        let block = &blocks[i];

        // Add anchor comment if requested
        if include_anchor {
            let anchor = Anchor::new(
                page_index,
                i,
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

        // Check if this is a list item and if there are consecutive list items
        if block.kind == "list" || block.kind == "list_item" {
            // Find the end of the consecutive list sequence
            let mut list_end = i + 1;
            while list_end < blocks.len()
                && (blocks[list_end].kind == "list" || blocks[list_end].kind == "list_item")
            {
                list_end += 1;
            }

            // Emit the entire list sequence as a group
            let list_blocks = &blocks[i..list_end];

            // For list items with links, emit each item with link support
            for list_block in list_blocks {
                let block_with_links = block_to_markdown_with_links(list_block, spans, page_links);
                if !block_with_links.is_empty() {
                    // Detect if numbered or bulleted
                    let is_numbered = block_with_links
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false);

                    if is_numbered {
                        result.push_str(&block_with_links);
                        result.push('\n');
                    } else {
                        result.push_str("* ");
                        result.push_str(&block_with_links);
                        result.push('\n');
                    }
                }
            }

            result.push('\n');
            i = list_end;
        } else {
            // Non-list block - emit individually
            let block_with_links = block_to_markdown_with_links(block, spans, page_links);

            // For non-list blocks, use the existing block emission logic
            // but replace the text content with link-aware content
            let kind_result = if block_with_links != block.text {
                // Links were detected - emit the link-aware version
                emit_block_kind_with_text(block, tables, options, &block_with_links)
            } else {
                // No links - use standard emission
                emit_block_kind(block, tables, options)
            };

            result.push_str(&kind_result);
            i += 1;
        }
    }

    // Add page break if requested and this isn't the last page
    if options.include_page_breaks {
        result.push_str("\n---\n\n");
    }

    result
}

/// Emit a block kind with custom text content.
///
/// This is a helper for `page_to_markdown_with_links` that allows overriding
/// the block's text with link-aware content while preserving the block's
/// formatting and structure.
fn emit_block_kind_with_text(
    block: &BlockJson,
    tables: &[TableJson],
    options: &MarkdownOptions,
    custom_text: &str,
) -> String {
    match block.kind.as_str() {
        "heading" => {
            let level = block.level.unwrap_or(1).clamp(1, 6);
            let prefix = "#".repeat(level as usize);
            format!("{} {}\n\n", prefix, custom_text)
        }

        "paragraph" => {
            let text = custom_text.replace('\n', "  \n");
            format!("{}\n\n", text)
        }

        "list" | "list_item" => {
            // Try to detect if this is a numbered list
            let is_numbered = custom_text
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false);

            if is_numbered {
                format!("{}\n", custom_text)
            } else {
                format!("* {}\n", custom_text)
            }
        }

        "caption" => format!("*{}\n\n", custom_text),

        _ => {
            // For other block kinds, fall back to standard emission
            emit_block_kind(block, tables, options)
        }
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

        let md = block_to_markdown(&block, &[], 0, 0, true);
        assert!(md.contains(
            "<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->"
        ));
        assert!(md.contains("## Chapter 1"));
    }

    #[test]
    fn test_block_to_markdown_paragraph_without_anchor() {
        let block = make_test_block("paragraph", "Some text.", [72.0, 600.0, 540.0, 630.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        assert!(!md.contains("<!-- pdftract:"));
        assert!(md.contains("Some text."));
    }

    #[test]
    fn test_block_to_markdown_list() {
        let block = make_test_block("list", "Item 1", [72.0, 500.0, 540.0, 520.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        assert!(md.contains("* Item 1"));
    }

    #[test]
    fn test_block_to_markdown_table() {
        let block = make_test_block("table", "Cell data", [72.0, 400.0, 540.0, 450.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        assert!(md.contains("| Cell data"));
    }

    #[test]
    fn test_block_to_markdown_figure() {
        let block = make_test_block("figure", "Alt text", [72.0, 300.0, 540.0, 350.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        assert!(md.contains("!["));  // Markdown image syntax start
        assert!(md.contains("]()"));  // Markdown image syntax end
        assert!(md.contains("Alt text"));
    }

    #[test]
    fn test_page_to_markdown_with_page_break() {
        let blocks = vec![
            make_test_block("heading", "Title", [72.0, 640.5, 540.0, 672.0]),
            make_test_block("paragraph", "Text", [72.0, 600.0, 540.0, 630.0]),
        ];

        let md = page_to_markdown(&blocks, &[], 0, false, true);
        assert!(md.contains("---"));
    }

    #[test]
    fn test_page_to_markdown_without_page_break() {
        let blocks = vec![
            make_test_block("heading", "Title", [72.0, 640.5, 540.0, 672.0]),
            make_test_block("paragraph", "Text", [72.0, 600.0, 540.0, 630.0]),
        ];

        let md = page_to_markdown(&blocks, &[], 0, false, false);
        assert!(!md.contains("---"));
    }

    #[test]
    fn test_page_to_markdown_with_anchors() {
        let blocks = vec![
            make_test_block("heading", "Title", [72.0, 640.5, 540.0, 672.0]),
            make_test_block("paragraph", "Text", [72.0, 600.0, 540.0, 630.0]),
        ];

        let md = page_to_markdown(&blocks, &[], 0, true, false);
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

        let md = page_to_markdown(&blocks, &[], 3, true, false);
        let anchors = parse_anchors(&md);

        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].page, 3);
        assert_eq!(anchors[0].block, 0);
        assert_eq!(anchors[0].kind, "heading");
    }

    #[test]
    fn test_block_to_markdown_paragraph_soft_line_break() {
        // Paragraph with internal newlines should emit soft breaks as "  \n"
        let block = make_test_block("paragraph", "Line 1\nLine 2\nLine 3", [72.0, 600.0, 540.0, 630.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        // Internal newlines become "  \n" (soft breaks)
        assert!(md.contains("Line 1  \n"));
        assert!(md.contains("Line 2  \n"));
        assert!(md.contains("Line 3\n\n")); // Final paragraph ends with \n\n
    }

    #[test]
    fn test_block_to_markdown_paragraph_no_soft_break() {
        // Paragraph without internal newlines
        let block = make_test_block("paragraph", "Single line text", [72.0, 600.0, 540.0, 630.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        assert_eq!(md, "Single line text\n\n");
    }

    #[test]
    fn test_block_to_markdown_formula_inline() {
        // Single-line formula should be inline: $E=mc^2$
        let block = make_test_block("formula", "E=mc^2", [72.0, 600.0, 540.0, 630.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        assert_eq!(md, "$E=mc^2$");
    }

    #[test]
    fn test_block_to_markdown_formula_display() {
        // Multi-line formula should be display: $$\n...\n$$
        let block = make_test_block(
            "formula",
            "\\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}",
            [72.0, 600.0, 540.0, 630.0],
        );
        let md = block_to_markdown(&block, &[], 0, 0, false);
        assert!(md.contains("$$\n"));
        assert!(md.contains("\n$$\n"));
    }

    #[test]
    fn test_block_to_markdown_list_numbered_preserves_numbering() {
        // Numbered list should preserve source numbering
        let block = make_test_block("list", "7. Seventh item", [72.0, 500.0, 540.0, 520.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        // Should preserve "7." numbering
        assert!(md.contains("7. Seventh item"));
    }

    #[test]
    fn test_block_to_markdown_list_bulleted() {
        // Bulleted list should use "* " prefix
        let block = make_test_block("list", "Item text", [72.0, 500.0, 540.0, 520.0]);
        let md = block_to_markdown(&block, &[], 0, 0, false);
        // Should add "* " prefix
        assert!(md.contains("* Item text"));
    }

    #[test]
    fn test_emit_list_blocks_nested_sublist() {
        // Critical test: nested sublist with proper indentation
        // Level 0: x0 = 72.0
        // Level 1: x0 = 90.0 (indented by 18 points)
        // Level 2: x0 = 108.0 (indented by 36 points)
        let list_blocks = vec![
            make_test_block("list", "Item 1", [72.0, 500.0, 540.0, 520.0]),
            make_test_block("list", "Item 2", [72.0, 480.0, 540.0, 500.0]),
            make_test_block("list", "Nested 1", [90.0, 460.0, 540.0, 480.0]),
            make_test_block("list", "Nested 2", [90.0, 440.0, 540.0, 460.0]),
            make_test_block("list", "Deep nested", [108.0, 420.0, 540.0, 440.0]),
            make_test_block("list", "Item 3", [72.0, 400.0, 540.0, 420.0]),
        ];

        let md = emit_list_blocks(&list_blocks);

        // Check that level 0 items have no indentation
        assert!(md.contains("* Item 1"));
        assert!(md.contains("* Item 2"));
        assert!(md.contains("* Item 3"));

        // Check that level 1 items are indented by 2 spaces
        assert!(md.contains("  * Nested 1"));
        assert!(md.contains("  * Nested 2"));

        // Check that level 2 items are indented by 4 spaces
        assert!(md.contains("    * Deep nested"));
    }

    #[test]
    fn test_emit_list_blocks_single_item() {
        // Single list item should still work
        let list_blocks = vec![make_test_block("list", "Single item", [72.0, 500.0, 540.0, 520.0])];
        let md = emit_list_blocks(&list_blocks);
        assert!(md.contains("* Single item"));
    }

    #[test]
    fn test_emit_list_blocks_empty() {
        // Empty list should return empty string
        let list_blocks: Vec<BlockJson> = vec![];
        let md = emit_list_blocks(&list_blocks);
        assert_eq!(md, "");
    }

    #[test]
    fn test_page_to_markdown_with_nested_list() {
        // Critical test: page with nested list in context
        let blocks = vec![
            make_test_block("heading", "Title", [72.0, 700.0, 540.0, 720.0]),
            make_test_block("list", "Item 1", [72.0, 650.0, 540.0, 670.0]),
            make_test_block("list", "Nested 1", [90.0, 630.0, 540.0, 650.0]),
            make_test_block("list", "Item 2", [72.0, 610.0, 540.0, 630.0]),
            make_test_block("paragraph", "Text after", [72.0, 580.0, 540.0, 600.0]),
        ];

        let md = page_to_markdown(&blocks, &[], 0, false, false);

        // Verify heading
        assert!(md.contains("# Title"));

        // Verify nested list structure
        assert!(md.contains("* Item 1"));
        assert!(md.contains("  * Nested 1"));
        assert!(md.contains("* Item 2"));

        // Verify paragraph after list
        assert!(md.contains("Text after"));
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

/// Emit a table as Markdown (GFM pipe table) or HTML fallback.
///
/// This function implements Phase 6.5 table emission:
/// - Simple tables (all 1x1 cells, no nested content) → GFM pipe table
/// - Complex tables (merged cells/colspan/rowspan/nested blocks) → HTML `<table>`
/// - Caption → italic line below the table
///
/// # Arguments
///
/// * `table` - The table to emit
///
/// # Returns
///
/// A Markdown string with the table in the appropriate format.
///
/// # Examples
///
/// ```
/// use pdftract_core::markdown::emit_table;
/// use pdftract_core::schema::{TableJson, RowJson, CellJson};
///
/// let table = TableJson {
///     id: "table_0".to_string(),
///     bbox: [50.0, 100.0, 400.0, 300.0],
///     rows: vec![
///         RowJson {
///             bbox: [50.0, 250.0, 400.0, 300.0],
///             cells: vec![
///                 CellJson {
///                     bbox: [50.0, 250.0, 200.0, 300.0],
///                     text: "Header 1".to_string(),
///                     spans: vec![],
///                     row: 0,
///                     col: 0,
///                     rowspan: 1,
///                     colspan: 1,
///                     is_header_row: true,
///                 },
///                 CellJson {
///                     bbox: [200.0, 250.0, 400.0, 300.0],
///                     text: "Header 2".to_string(),
///                     spans: vec![],
///                     row: 0,
///                     col: 1,
///                     rowspan: 1,
///                     colspan: 1,
///                     is_header_row: true,
///                 },
///             ],
///             is_header: true,
///         },
///         RowJson {
///             bbox: [50.0, 100.0, 400.0, 250.0],
///             cells: vec![
///                 CellJson {
///                     bbox: [50.0, 100.0, 200.0, 250.0],
///                     text: "Data 1".to_string(),
///                     spans: vec![],
///                     row: 1,
///                     col: 0,
///                     rowspan: 1,
///                     colspan: 1,
///                     is_header_row: false,
///                 },
///                 CellJson {
///                     bbox: [200.0, 100.0, 400.0, 250.0],
///                     text: "Data 2".to_string(),
///                     spans: vec![],
///                     row: 1,
///                     col: 1,
///                     rowspan: 1,
///                     colspan: 1,
///                     is_header_row: false,
///                 },
///             ],
///             is_header: false,
///         },
///     ],
///     header_rows: 1,
///     detection_method: "line_based".to_string(),
///     continued: false,
///     continued_from_prev: false,
///     page_index: 0,
/// };
///
/// let md = emit_table(&table);
/// assert!(md.contains("| Header 1 | Header 2 |"));
/// assert!(md.contains("| Data 1 | Data 2 |"));
/// ```
pub fn emit_table(table: &TableJson) -> String {
    // Check if table is simple (all cells 1x1) or complex (merged cells)
    let is_simple = table.rows.iter().all(|row| {
        row.cells
            .iter()
            .all(|cell| cell.rowspan == 1 && cell.colspan == 1)
    });

    if is_simple {
        emit_gfm_table(table)
    } else {
        emit_html_table(table)
    }
}

/// Emit a table as GitHub-Flavored Markdown pipe table.
///
/// GFM pipe tables require:
/// - All cells have rowspan=1 and colspan=1 (no merged cells)
/// - Header row (first row if is_header=true, otherwise synthesized)
/// - Separator row with `| --- | --- |` syntax
/// - Body rows with `| val | val |` syntax
fn emit_gfm_table(table: &TableJson) -> String {
    let mut result = String::new();

    // Find the maximum number of columns across all rows
    let max_cols = table
        .rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or(0);

    if max_cols == 0 {
        return String::new();
    }

    // Emit header row (use first row if it exists)
    if let Some(first_row) = table.rows.first() {
        result.push_str("| ");
        for (i, cell) in first_row.cells.iter().enumerate() {
            if i > 0 {
                result.push_str(" | ");
            }
            result.push_str(&escape_pipe(&cell.text));
        }
        // Pad missing columns
        for i in first_row.cells.len()..max_cols {
            if i > 0 || !first_row.cells.is_empty() {
                result.push_str(" | ");
            }
            result.push_str(" ");
        }
        result.push_str(" |\n");
    } else {
        // Empty header row for table with no rows
        for i in 0..max_cols {
            if i > 0 {
                result.push_str(" | ");
            }
            result.push_str(" ");
        }
        result.push_str(" |\n");
    }

    // Emit separator row
    result.push_str("|");
    for _ in 0..max_cols {
        result.push_str(" --- |");
    }
    result.push('\n');

    // Emit body rows (skip first row if it was header)
    let body_start = if table.rows.first().map_or(false, |r| r.is_header) {
        1
    } else {
        0
    };

    for row in table.rows.iter().skip(body_start) {
        result.push_str("| ");
        for (i, cell) in row.cells.iter().enumerate() {
            if i > 0 {
                result.push_str(" | ");
            }
            result.push_str(&escape_pipe(&cell.text));
        }
        // Pad missing columns
        for i in row.cells.len()..max_cols {
            if i > 0 || !row.cells.is_empty() {
                result.push_str(" | ");
            }
            result.push_str(" ");
        }
        result.push_str(" |\n");
    }

    result
}

/// Emit a table as inline HTML `<table>`.
///
/// HTML fallback is used when:
/// - Any cell has colspan > 1 or rowspan > 1 (merged cells)
/// - Nested blocks are present (future enhancement)
pub fn emit_html_table(table: &TableJson) -> String {
    let mut result = String::from("<table>\n");

    for row in &table.rows {
        result.push_str("  <tr>\n");

        for cell in &row.cells {
            let tag = if cell.is_header_row || row.is_header {
                "th"
            } else {
                "td"
            };

            result.push_str("    <");
            result.push_str(tag);

            // Add colspan if > 1
            if cell.colspan > 1 {
                result.push_str(&format!(" colspan=\"{}\"", cell.colspan));
            }

            // Add rowspan if > 1
            if cell.rowspan > 1 {
                result.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
            }

            result.push_str(">");
            result.push_str(&escape_pipe(&cell.text));
            result.push_str("</");
            result.push_str(tag);
            result.push_str(">\n");
        }

        result.push_str("  </tr>\n");
    }

    result.push_str("</table>\n");
    result
}

/// Escape pipe characters for markdown table cells.
///
/// This function escapes `|` as `\|` to prevent it from being interpreted
/// as a column separator in GFM pipe tables.
///
/// Also replaces newlines with `<br>` for GFM tables (HTML inside Markdown
/// table cells is allowed and widely supported).
fn escape_pipe(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);

    for c in s.chars() {
        match c {
            '|' => {
                result.push_str("\\|");
            }
            '\n' => {
                // Newlines in GFM tables become <br> tags
                result.push_str("<br>");
            }
            '<' => {
                // Escape < to prevent HTML injection
                result.push_str("&lt;");
            }
            '>' => {
                // Escape > to prevent HTML injection
                result.push_str("&gt;");
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

    // Table emission tests (Phase 6.5)

    fn make_test_cell(
        text: &str,
        row: usize,
        col: usize,
        rowspan: u32,
        colspan: u32,
        is_header_row: bool,
    ) -> crate::schema::CellJson {
        crate::schema::CellJson {
            bbox: [0.0, 0.0, 100.0, 20.0],
            text: text.to_string(),
            spans: vec![],
            row,
            col,
            rowspan,
            colspan,
            is_header_row,
        }
    }

    fn make_test_row(cells: Vec<crate::schema::CellJson>, is_header: bool) -> crate::schema::RowJson {
        crate::schema::RowJson {
            bbox: [0.0, 0.0, 100.0, 20.0],
            cells,
            is_header,
        }
    }

    #[test]
    fn test_emit_table_simple_3x3() {
        // Simple 3x3 table: GFM pipe format
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 300.0, 200.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("H1", 0, 0, 1, 1, true),
                        make_test_cell("H2", 0, 1, 1, 1, true),
                        make_test_cell("H3", 0, 2, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("D1", 1, 0, 1, 1, false),
                        make_test_cell("D2", 1, 1, 1, 1, false),
                        make_test_cell("D3", 1, 2, 1, 1, false),
                    ],
                    false,
                ),
                make_test_row(
                    vec![
                        make_test_cell("D4", 2, 0, 1, 1, false),
                        make_test_cell("D5", 2, 1, 1, 1, false),
                        make_test_cell("D6", 2, 2, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        assert!(md.contains("| H1 | H2 | H3 |"));
        assert!(md.contains("| --- | --- | --- |"));
        assert!(md.contains("| D1 | D2 | D3 |"));
        assert!(md.contains("| D4 | D5 | D6 |"));
        // Should NOT contain HTML table tags
        assert!(!md.contains("<table>"));
        assert!(!md.contains("<tr>"));
        assert!(!md.contains("<td>"));
    }

    #[test]
    fn test_emit_table_merged_cells_html_fallback() {
        // Critical test: merged-cell table input -> falls back to inline <table>
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 300.0, 200.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("Merged Header", 0, 0, 1, 2, true), // colspan=2
                        make_test_cell("H2", 0, 1, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("D1", 1, 0, 1, 1, false),
                        make_test_cell("D2", 1, 1, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Should contain HTML table tags
        assert!(md.contains("<table>"));
        assert!(md.contains("</table>"));
        assert!(md.contains("<tr>"));
        assert!(md.contains("</tr>"));
        // Should have colspan attribute
        assert!(md.contains("colspan=\"2\""));
        // Should NOT contain GFM pipe syntax
        assert!(!md.contains("| --- |"));
    }

    #[test]
    fn test_emit_table_rowspan_html_fallback() {
        // Table with rowspan -> HTML fallback
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 300.0, 200.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("Rowspan", 0, 0, 2, 1, true), // rowspan=2
                        make_test_cell("H2", 0, 1, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("D1", 1, 0, 1, 1, false), // This cell is below the rowspan cell
                        make_test_cell("D2", 1, 1, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Should have rowspan attribute
        assert!(md.contains("rowspan=\"2\""));
        // Should NOT contain GFM pipe syntax
        assert!(!md.contains("| --- |"));
    }

    #[test]
    fn test_escape_pipe() {
        // Cell with pipe character: escaped as \|
        assert_eq!(escape_pipe("A|B"), "A\\|B");
        assert_eq!(escape_pipe("|||"), "\\|\\|\\|");
        assert_eq!(escape_pipe("test"), "test");
    }

    #[test]
    fn test_escape_pipe_newline_to_br() {
        // Cell with newline: rendered with <br>
        assert_eq!(escape_pipe("line1\nline2"), "line1<br>line2");
        assert_eq!(escape_pipe("a\nb\nc"), "a<br>b<br>c");
    }

    #[test]
    fn test_escape_pipe_html_entities() {
        // < and > escaped as HTML entities
        assert_eq!(escape_pipe("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_pipe("a<b"), "a&lt;b");
    }

    #[test]
    fn test_emit_table_with_pipe_in_cell() {
        // Cell with pipe character: escaped as \|
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 200.0, 100.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("A|B", 0, 0, 1, 1, true),
                        make_test_cell("Normal", 0, 1, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("Data", 1, 0, 1, 1, false),
                        make_test_cell("Value", 1, 1, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Pipe should be escaped in the output
        assert!(md.contains("A\\|B"));
        // The table should still render correctly
        assert!(md.contains("| --- | --- |"));
    }

    #[test]
    fn test_emit_table_with_newline_in_cell() {
        // Cell with newline: rendered with <br>
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 200.0, 100.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("Line1\nLine2", 0, 0, 1, 1, true),
                        make_test_cell("Normal", 0, 1, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("Data", 1, 0, 1, 1, false),
                        make_test_cell("Value", 1, 1, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Newline should become <br> tag
        assert!(md.contains("Line1<br>Line2"));
    }

    #[test]
    fn test_emit_table_empty() {
        // Empty table (no rows)
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 100.0, 50.0],
            rows: vec![],
            header_rows: 0,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Empty table should return empty string
        assert_eq!(md, "");
    }

    #[test]
    fn test_emit_table_single_row() {
        // Table with single row (no body rows)
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 200.0, 50.0],
            rows: vec![make_test_row(
                vec![
                    make_test_cell("H1", 0, 0, 1, 1, true),
                    make_test_cell("H2", 0, 1, 1, 1, true),
                ],
                true,
            )],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Should have header row and separator
        assert!(md.contains("| H1 | H2 |"));
        assert!(md.contains("| --- | --- |"));
        // Should not have any body rows (no "| |" after separator)
        let parts: Vec<&str> = md.lines().collect();
        assert_eq!(parts.len(), 2); // Header row + separator
    }

    #[test]
    fn test_emit_table_no_header() {
        // Table with no header row (all rows are data)
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 200.0, 100.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("D1", 0, 0, 1, 1, false),
                        make_test_cell("D2", 0, 1, 1, 1, false),
                    ],
                    false,
                ),
                make_test_row(
                    vec![
                        make_test_cell("D3", 1, 0, 1, 1, false),
                        make_test_cell("D4", 1, 1, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 0,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Should use first row as header for GFM
        assert!(md.contains("| D1 | D2 |"));
        assert!(md.contains("| --- | --- |"));
        // Second row should be in body
        assert!(md.contains("| D3 | D4 |"));
    }

    #[test]
    fn test_emit_html_table_header_cells() {
        // HTML table with is_header_row cells should use <th> tags
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 200.0, 100.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("Header1", 0, 0, 1, 1, true), // is_header_row=true
                        make_test_cell("Header2", 0, 1, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("Data1", 1, 0, 1, 1, false), // is_header_row=false
                        make_test_cell("Data2", 1, 1, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_html_table(&table);
        // First row should use <th> tags
        assert!(md.contains("<th>Header1</th>"));
        assert!(md.contains("<th>Header2</th>"));
        // Second row should use <td> tags
        assert!(md.contains("<td>Data1</td>"));
        assert!(md.contains("<td>Data2</td>"));
    }

    #[test]
    fn test_emit_html_table_row_and_colspan() {
        // HTML table with both rowspan and colspan
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 300.0, 200.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("Both", 0, 0, 2, 2, true), // rowspan=2, colspan=2
                        make_test_cell("H2", 0, 1, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("D1", 1, 0, 1, 1, false),
                        make_test_cell("D2", 1, 1, 1, 1, false),
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_html_table(&table);
        // Should have both colspan and rowspan attributes
        assert!(md.contains("colspan=\"2\""));
        assert!(md.contains("rowspan=\"2\""));
    }

    #[test]
    fn test_emit_gfm_table_variable_width() {
        // GFM table with different column counts per row
        let table = TableJson {
            id: "table_0".to_string(),
            bbox: [0.0, 0.0, 300.0, 200.0],
            rows: vec![
                make_test_row(
                    vec![
                        make_test_cell("H1", 0, 0, 1, 1, true),
                        make_test_cell("H2", 0, 1, 1, 1, true),
                        make_test_cell("H3", 0, 2, 1, 1, true),
                    ],
                    true,
                ),
                make_test_row(
                    vec![
                        make_test_cell("D1", 1, 0, 1, 1, false),
                        make_test_cell("D2", 1, 1, 1, 1, false),
                        // Missing third cell - should pad
                    ],
                    false,
                ),
            ],
            header_rows: 1,
            detection_method: "line_based".to_string(),
            continued: false,
            continued_from_prev: false,
            page_index: 0,
        };

        let md = emit_table(&table);
        // Should have 3 columns in all rows (padded with empty cells)
        assert!(md.contains("| H1 | H2 | H3 |"));
        assert!(md.contains("| --- | --- | --- |"));
        // Second row should be padded
        let lines: Vec<&str> = md.lines().collect();
        let body_line = lines.get(2).unwrap();
        assert_eq!(body_line.matches('|').count(), 4); // 4 pipes = 3 cells
    }
}
