//! Caption block classifier (Phase 4).
//!
//! This module implements classification of blocks as captions based on:
//! 1. Small font size (median < page body median)
//! 2. Proximity to a Figure block (within 2 line heights)
//! 3. Same column as the Figure
//!
//! Captions are typically short text blocks immediately below figures
//! in scholarly papers, technical documents, and reports.

/// Block with layout properties for caption classification.
///
/// This extends the base block structure with properties needed
/// for caption detection: font size metrics, bounding box, and
/// column membership.
#[derive(Debug, Clone)]
pub struct Block {
    /// Block kind (will be set to "caption" if classified as such)
    pub kind: String,
    /// Block text content
    pub text: String,
    /// Median font size in points
    pub median_font_size: f32,
    /// Bounding box [x0, y0, x1, y1] in PDF user space
    pub bbox: [f32; 4],
    /// Column index (0-based)
    pub column: usize,
}

impl Block {
    /// Get the top Y coordinate of the block.
    pub fn top(&self) -> f32 {
        self.bbox[3]
    }

    /// Get the bottom Y coordinate of the block.
    pub fn bottom(&self) -> f32 {
        self.bbox[1]
    }

    /// Get the left X coordinate of the block.
    pub fn left(&self) -> f32 {
        self.bbox[0]
    }

    /// Get the right X coordinate of the block.
    pub fn right(&self) -> f32 {
        self.bbox[2]
    }

    /// Check if this block is a figure.
    pub fn is_figure(&self) -> bool {
        self.kind == "figure"
    }

    /// Check if this block is a caption.
    pub fn is_caption(&self) -> bool {
        self.kind == "caption"
    }

    /// Set the block kind to caption.
    pub fn set_caption(&mut self) {
        self.kind = "caption".to_string();
    }
}

/// Page context containing metrics needed for caption classification.
///
/// This context is populated by earlier phases of the extraction pipeline:
/// - Phase 4.2 provides line height
/// - Phase 4.3 provides column boundaries
/// - Body font median is computed from all paragraph blocks on the page
#[derive(Debug, Clone)]
pub struct PageContext {
    /// Median font size across all paragraph blocks on the page
    pub page_body_median: f32,
    /// Typical line height on the page (from Phase 4.2)
    pub line_height: f32,
    /// Number of columns on the page (from Phase 4.3)
    pub num_columns: usize,
}

impl PageContext {
    /// Create a new page context with default values.
    pub fn new() -> Self {
        Self {
            page_body_median: 12.0, // Typical body text is ~12pt
            line_height: 14.0,      // Typical line spacing is ~1.2x font size
            num_columns: 1,         // Default single-column layout
        }
    }

    /// Create a new page context with specific values.
    pub fn with_values(page_body_median: f32, line_height: f32, num_columns: usize) -> Self {
        Self {
            page_body_median,
            line_height,
            num_columns,
        }
    }
}

impl Default for PageContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Classify a block as a caption based on layout criteria.
///
/// A block is classified as a caption if ALL of the following are true:
/// 1. The block has a smaller font size than the page body median
/// 2. The block follows a Figure block within 2 line heights
/// 3. The block is in the same column as the Figure
///
/// # Arguments
///
/// * `block` - The block to classify
/// * `prev_block` - The previous block in page order (may be a Figure)
/// * `ctx` - Page context with metrics needed for classification
///
/// # Returns
///
/// `true` if the block should be classified as a caption, `false` otherwise.
pub fn classify_caption(block: &Block, prev_block: Option<&Block>, ctx: &PageContext) -> bool {
    // Criterion 1: Small font size
    // Captions are typically smaller than body text (e.g., 9-10pt vs 12pt)
    if block.median_font_size >= ctx.page_body_median {
        return false;
    }

    // Criterion 2: Must follow a Figure block
    let figure = match prev_block {
        Some(pb) if pb.is_figure() => pb,
        _ => return false,
    };

    // Criterion 3: Vertical proximity
    // Distance from block top to figure bottom must be < 2 * line_height
    let vertical_distance = block.top() - figure.bottom();
    if vertical_distance < 0.0 {
        // Block is above the figure - captions are below
        return false;
    }
    if vertical_distance >= 2.0 * ctx.line_height {
        // Too far below - gap is more than 2 lines
        return false;
    }

    // Criterion 4: Same column
    // In single-column layouts (num_columns == 1), all blocks are in the same column
    if ctx.num_columns > 1 && block.column != figure.column {
        return false;
    }

    true
}

/// Classify all blocks on a page, updating their kinds to "caption" where appropriate.
///
/// This function processes blocks in page order and classifies each block
/// based on its relationship to the previous block.
///
/// # Arguments
///
/// * `blocks` - Mutable slice of blocks to classify (processed in page order)
/// * `ctx` - Page context with metrics needed for classification
pub fn classify_page_captions(blocks: &mut [Block], ctx: &PageContext) {
    // Sort blocks by top Y coordinate (page order: top to bottom)
    blocks.sort_by_key(|b| std::cmp::Reverse(b.top() as i32));

    let mut prev_block: Option<&Block> = None;

    for i in 0..blocks.len() {
        let is_caption = classify_caption(&blocks[i], prev_block, ctx);

        if is_caption {
            blocks[i].set_caption();
        }

        // Update previous block for next iteration
        // Note: we use a reference to the block before any modification
        prev_block = if i < blocks.len() {
            Some(&blocks[i])
        } else {
            None
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(kind: &str, text: &str, font_size: f32, bbox: [f32; 4], column: usize) -> Block {
        Block {
            kind: kind.to_string(),
            text: text.to_string(),
            median_font_size: font_size,
            bbox,
            column,
        }
    }

    fn make_figure(bbox: [f32; 4], column: usize) -> Block {
        make_block("figure", "", 0.0, bbox, column)
    }

    #[test]
    fn test_caption_immediately_below_figure() {
        // Figure at y=[100, 200], caption at y=[90, 100] (1 line below)
        let figure = make_figure([50.0, 100.0, 150.0, 200.0], 0);
        let caption = make_block(
            "paragraph",
            "Figure 1: A chart",
            9.0,
            [50.0, 90.0, 150.0, 100.0],
            0,
        );

        let ctx = PageContext::with_values(12.0, 10.0, 1);

        assert!(classify_caption(&caption, Some(&figure), &ctx));
    }

    #[test]
    fn test_caption_too_far_below_figure() {
        // Figure at y=[100, 200], caption at y=[70, 80] (3 lines below = 30pt)
        let figure = make_figure([50.0, 100.0, 150.0, 200.0], 0);
        let caption = make_block(
            "paragraph",
            "Figure 1: A chart",
            9.0,
            [50.0, 70.0, 150.0, 80.0],
            0,
        );

        let ctx = PageContext::with_values(12.0, 10.0, 1);

        assert!(!classify_caption(&caption, Some(&figure), &ctx));
    }

    #[test]
    fn test_caption_font_not_smaller() {
        // Caption with same font size as body text
        let figure = make_figure([50.0, 100.0, 150.0, 200.0], 0);
        let not_caption = make_block(
            "paragraph",
            "Figure 1: A chart",
            12.0,
            [50.0, 90.0, 150.0, 100.0],
            0,
        );

        let ctx = PageContext::with_values(12.0, 10.0, 1);

        assert!(!classify_caption(&not_caption, Some(&figure), &ctx));
    }

    #[test]
    fn test_caption_different_column() {
        // Figure in column 0, caption in column 1 (two-column layout)
        let figure = make_figure([50.0, 100.0, 150.0, 200.0], 0);
        let caption = make_block(
            "paragraph",
            "Figure 1: A chart",
            9.0,
            [200.0, 90.0, 300.0, 100.0],
            1,
        );

        let ctx = PageContext::with_values(12.0, 10.0, 2);

        assert!(!classify_caption(&caption, Some(&figure), &ctx));
    }

    #[test]
    fn test_no_previous_figure() {
        // Block with no previous block
        let block = make_block("paragraph", "Some text", 9.0, [50.0, 90.0, 150.0, 100.0], 0);
        let ctx = PageContext::with_values(12.0, 10.0, 1);

        assert!(!classify_caption(&block, None, &ctx));
    }

    #[test]
    fn test_caption_above_figure() {
        // Caption positioned above the figure (not detected in v0.1.0)
        let caption = make_block(
            "paragraph",
            "Figure 1: A chart",
            9.0,
            [50.0, 200.0, 150.0, 210.0],
            0,
        );
        let figure = make_figure([50.0, 100.0, 150.0, 200.0], 0);

        let ctx = PageContext::with_values(12.0, 10.0, 1);

        assert!(!classify_caption(&caption, Some(&figure), &ctx));
    }

    #[test]
    fn test_page_classification() {
        let mut blocks = vec![
            make_figure([50.0, 100.0, 150.0, 200.0], 0), // Figure
            make_block(
                "paragraph",
                "Figure 1: A chart",
                9.0,
                [50.0, 90.0, 150.0, 100.0],
                0,
            ), // Caption
            make_block(
                "paragraph",
                "Next paragraph",
                12.0,
                [50.0, 70.0, 150.0, 80.0],
                0,
            ), // Regular text
        ];

        let ctx = PageContext::with_values(12.0, 10.0, 1);

        classify_page_captions(&mut blocks, &ctx);

        assert_eq!(blocks[0].kind, "figure");
        assert_eq!(blocks[1].kind, "caption");
        assert_eq!(blocks[2].kind, "paragraph"); // Unchanged
    }

    #[test]
    fn test_block_accessors() {
        let block = make_block("paragraph", "Test", 10.0, [10.0, 20.0, 30.0, 40.0], 0);

        assert_eq!(block.top(), 40.0);
        assert_eq!(block.bottom(), 20.0);
        assert_eq!(block.left(), 10.0);
        assert_eq!(block.right(), 30.0);
    }
}
