//! Line formation for Phase 4.2.
//!
//! This module implements grouping spans into lines by baseline proximity
//! and computing line-level metadata including bbox, baseline, and direction.
//!
//! Phase 4.4 block formation is also implemented here, providing the
//! `group_lines_into_blocks` function that applies 5 ordered heuristics
//! to group lines into semantic blocks.

use serde::{Deserialize, Serialize};

/// Text direction for a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineDirection {
    /// Left-to-right text (e.g., Latin, Cyrillic)
    Ltr,
    /// Right-to-left text (e.g., Arabic, Hebrew)
    Rtl,
    /// Mixed direction (bidi text)
    Mixed,
}

/// A line of text composed of one or more spans.
///
/// Lines are the third-level structural unit in the extraction pipeline,
/// after Glyphs and Spans. Line bbox drives column detection and reading
/// order; baseline drives clustering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line<S> {
    /// Spans that make up this line, in reading order.
    pub spans: Vec<S>,
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    ///
    /// This is the union of all span bboxes.
    pub bbox: [f32; 4],
    /// Baseline y-coordinate for this line.
    ///
    /// Computed as the average of member span baselines.
    pub baseline: f32,
    /// Text direction for this line.
    pub direction: LineDirection,
    /// Page-relative Y position (0=top, 1=bottom).
    ///
    /// Used for reading order sorting. Computed as:
    /// `(page_height - bbox[3]) / page_height`
    pub page_relative_y: f32,
    /// Median font size of spans in this line (points).
    ///
    /// Used for block formation heuristics (font size change detection).
    pub median_font_size: f32,
    /// Text rendering mode (PDF Tr operator).
    ///
    /// Tr=3 indicates invisible text. Used for block formation heuristics.
    pub rendering_mode: Option<u32>,
    /// Column index (0-based) assigned to this line.
    ///
    /// Set by Phase 4.3 column detection. None if not yet assigned.
    pub column: Option<usize>,
}

impl<S> Line<S> {
    /// Get the left X coordinate of the line.
    #[inline]
    pub fn left(&self) -> f32 {
        self.bbox[0]
    }

    /// Get the bottom Y coordinate of the line.
    #[inline]
    pub fn bottom(&self) -> f32 {
        self.bbox[1]
    }

    /// Get the right X coordinate of the line.
    #[inline]
    pub fn right(&self) -> f32 {
        self.bbox[2]
    }

    /// Get the top Y coordinate of the line.
    #[inline]
    pub fn top(&self) -> f32 {
        self.bbox[3]
    }

    /// Get the width of the line's bbox.
    #[inline]
    pub fn width(&self) -> f32 {
        self.bbox[2] - self.bbox[0]
    }

    /// Get the height of the line's bbox.
    #[inline]
    pub fn height(&self) -> f32 {
        self.bbox[3] - self.bbox[1]
    }
}

/// Trait for types that can provide line metadata needed for block formation.
///
/// This trait allows the block formation code to work with different
/// line representations while abstracting over the underlying span type.
pub trait LineMetadata {
    /// Get the baseline y-coordinate.
    fn baseline(&self) -> f32;
    /// Get the bounding box [x0, y0, x1, y1].
    fn bbox(&self) -> [f32; 4];
    /// Get the median font size.
    fn median_font_size(&self) -> f32;
    /// Get the rendering mode (None if not applicable).
    fn rendering_mode(&self) -> Option<u32>;
    /// Get the column index (None if not assigned).
    fn column(&self) -> Option<usize>;
}

impl<S> LineMetadata for Line<S> {
    fn baseline(&self) -> f32 {
        self.baseline
    }
    fn bbox(&self) -> [f32; 4] {
        self.bbox
    }
    fn median_font_size(&self) -> f32 {
        self.median_font_size
    }
    fn rendering_mode(&self) -> Option<u32> {
        self.rendering_mode
    }
    fn column(&self) -> Option<usize> {
        self.column
    }
}

/// A block of text composed of one or more lines.
///
/// Blocks are the fourth-level structural unit in the extraction pipeline,
/// after Glyphs, Spans, and Lines. Blocks represent semantic units like
/// paragraphs, headings, and list items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block<S> {
    /// Lines that make up this block, in reading order.
    pub lines: Vec<Line<S>>,
    /// Block kind (paragraph, heading, list, etc.).
    pub kind: String,
    /// Concatenated text content of all lines.
    pub text: String,
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f32; 4],
    /// Median font size in points.
    pub median_font_size: f32,
    /// Column index (0-based).
    pub column: usize,
}

/// Group lines into blocks using the 5 ordered heuristics from Phase 4.4.
///
/// This function sweeps lines top-down (sorted by column ASC, baseline DESC)
/// and applies the following triggers in order to determine block boundaries:
///
/// 1. **Vertical gap:** gap > 1.5 * line_height → new block
/// 2. **Indent change:** first-line x0 differs by > 0.03 * column_width → new block
/// 3. **Font size change:** median font size delta > 1pt → new block
/// 4. **Rendering mode change:** invisible (Tr=3) vs visible text → new block
/// 5. **Column boundary:** MANDATORY block break
///
/// # Arguments
///
/// * `lines` - Lines to group, with metadata (baseline, bbox, font_size, etc.)
/// * `column_widths` - Width of each column in points (must match line columns)
///
/// # Returns
///
/// A vector of blocks, each containing one or more lines.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::line::{group_lines_into_blocks, Line, LineDirection};
///
/// // Five lines with equal spacing: should form one block
/// // (example assumes lines are properly constructed with metadata)
/// ```
pub fn group_lines_into_blocks<L>(lines: Vec<L>, column_widths: &[f32]) -> Vec<BlockInput<L>>
where
    L: LineMetadata + Clone,
{
    if lines.is_empty() {
        return Vec::new();
    }

    // Sort lines by (column ASC, baseline DESC)
    // NaN columns go last (handled by Option::cmp)
    let mut sorted_lines = lines;
    sorted_lines.sort_by(|a, b| {
        match (a.column(), b.column()) {
            (Some(ca), Some(cb)) => {
                // Same column: compare baseline (descending)
                if ca == cb {
                    b.baseline()
                        .partial_cmp(&a.baseline())
                        .unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    ca.cmp(&cb)
                }
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b
                .baseline()
                .partial_cmp(&a.baseline())
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    });

    let mut blocks: Vec<BlockInput<L>> = Vec::new();
    let mut current_block_lines: Vec<L> = Vec::new();
    let mut block_avg_x0: Option<f32> = None;
    let mut block_median_font_size: Option<f32> = None;
    let mut block_rendering_mode: Option<u32> = None;
    let mut block_column: Option<usize> = None;
    let mut block_line_heights: Vec<f32> = Vec::new();
    let mut prev_baseline: Option<f32> = None;

    for line in &sorted_lines {
        let line_column = line.column();

        // Trigger 5: Column boundary is MANDATORY
        if let (Some(bc), Some(lc)) = (block_column, line_column) {
            if bc != lc {
                // Column changed: finalize current block and start new one
                if !current_block_lines.is_empty() {
                    blocks.push(finalize_block(
                        std::mem::take(&mut current_block_lines),
                        block_avg_x0.unwrap(),
                        block_median_font_size.unwrap(),
                        block_column.unwrap(),
                    ));
                    block_avg_x0 = None;
                    block_median_font_size = None;
                    block_rendering_mode = None;
                    block_column = None;
                    block_line_heights.clear();
                    prev_baseline = None;
                }
            }
        }

        let line_bbox = line.bbox();
        let line_x0 = line_bbox[0];
        let current_baseline = line.baseline();
        let column_width = line_column
            .and_then(|c| column_widths.get(c).copied())
            .unwrap_or(600.0); // Default fallback

        // Initialize block state on first line of block
        if current_block_lines.is_empty() {
            block_avg_x0 = Some(line_x0);
            block_median_font_size = Some(line.median_font_size());
            block_rendering_mode = line.rendering_mode();
            block_column = line_column;
            block_line_heights.clear(); // Start fresh
            prev_baseline = Some(current_baseline);
            current_block_lines.push(line.clone());
            continue;
        }

        // Compute vertical gap and line height
        let gap = prev_baseline.unwrap() - current_baseline;
        let line_height = prev_baseline.unwrap() - line_bbox[1]; // baseline to bottom

        // Add line height to block (for median calculation)
        block_line_heights.push(line_height);

        // Compute median line height in current block
        let mut sorted_heights = block_line_heights.clone();
        sorted_heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_line_height = sorted_heights[sorted_heights.len() / 2];

        // Trigger 1: Vertical gap > 1.5 * line_height
        if gap > 1.5 * median_line_height {
            blocks.push(finalize_block(
                std::mem::take(&mut current_block_lines),
                block_avg_x0.unwrap(),
                block_median_font_size.unwrap(),
                block_column.unwrap(),
            ));
            block_avg_x0 = Some(line_x0);
            block_median_font_size = Some(line.median_font_size());
            block_rendering_mode = line.rendering_mode();
            block_column = line_column;
            block_line_heights.clear();
            prev_baseline = Some(current_baseline);
            current_block_lines.push(line.clone());
            continue;
        }

        // Trigger 2: Indent change > 0.03 * column_width
        let indent_delta = (line_x0 - block_avg_x0.unwrap()).abs();
        if indent_delta > 0.03 * column_width {
            blocks.push(finalize_block(
                std::mem::take(&mut current_block_lines),
                block_avg_x0.unwrap(),
                block_median_font_size.unwrap(),
                block_column.unwrap(),
            ));
            block_avg_x0 = Some(line_x0);
            block_median_font_size = Some(line.median_font_size());
            block_rendering_mode = line.rendering_mode();
            block_column = line_column;
            block_line_heights.clear();
            prev_baseline = Some(current_baseline);
            current_block_lines.push(line.clone());
            continue;
        }

        // Trigger 3: Font size change > 1pt
        let font_delta = (line.median_font_size() - block_median_font_size.unwrap()).abs();
        if font_delta > 1.0 {
            blocks.push(finalize_block(
                std::mem::take(&mut current_block_lines),
                block_avg_x0.unwrap(),
                block_median_font_size.unwrap(),
                block_column.unwrap(),
            ));
            block_avg_x0 = Some(line_x0);
            block_median_font_size = Some(line.median_font_size());
            block_rendering_mode = line.rendering_mode();
            block_column = line_column;
            block_line_heights.clear();
            prev_baseline = Some(current_baseline);
            current_block_lines.push(line.clone());
            continue;
        }

        // Trigger 4: Rendering mode change
        if line.rendering_mode() != block_rendering_mode {
            blocks.push(finalize_block(
                std::mem::take(&mut current_block_lines),
                block_avg_x0.unwrap(),
                block_median_font_size.unwrap(),
                block_column.unwrap(),
            ));
            block_avg_x0 = Some(line_x0);
            block_median_font_size = Some(line.median_font_size());
            block_rendering_mode = line.rendering_mode();
            block_column = line_column;
            block_line_heights.clear();
            prev_baseline = Some(current_baseline);
            current_block_lines.push(line.clone());
            continue;
        }

        // No trigger fired: add line to current block
        current_block_lines.push(line.clone());
        prev_baseline = Some(current_baseline);
    }

    // Finalize the last block
    if !current_block_lines.is_empty() {
        blocks.push(finalize_block(
            current_block_lines,
            block_avg_x0.unwrap(),
            block_median_font_size.unwrap(),
            block_column.unwrap(),
        ));
    }

    blocks
}

/// Internal block representation used during formation.
///
/// This is a minimal block type used for grouping lines.
/// The public-facing Block type is in caption.rs.
#[derive(Debug, Clone)]
pub struct BlockInput<L> {
    /// Lines that make up this block.
    pub lines: Vec<L>,
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f32; 4],
    /// Median font size in points.
    pub median_font_size: f32,
    /// Column index (0-based).
    pub column: usize,
}

/// Finalize a block from accumulated lines.
fn finalize_block<L>(
    lines: Vec<L>,
    avg_x0: f32,
    median_font_size: f32,
    column: usize,
) -> BlockInput<L>
where
    L: LineMetadata,
{
    // Compute union bbox
    let mut union = lines[0].bbox();
    for line in &lines[1..] {
        let bbox = line.bbox();
        union[0] = union[0].min(bbox[0]);
        union[1] = union[1].min(bbox[1]);
        union[2] = union[2].max(bbox[2]);
        union[3] = union[3].max(bbox[3]);
    }

    BlockInput {
        lines,
        bbox: union,
        median_font_size,
        column,
    }
}

/// Compute the baseline y-coordinate for a span.
///
/// The baseline is approximated as `y0 + (bbox_height * 0.2)`, where the
/// 0.2 multiplier is an empirical fit for most Latin fonts. The exact
/// value would require font descender metrics from the font dictionary.
///
/// # Arguments
///
/// * `bbox` - Span bounding box [x0, y0, x1, y1] in PDF user space
///
/// # Returns
///
/// The baseline y-coordinate.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::line::compute_baseline;
///
/// // Span bbox [0, 100, 50, 110] (height 10)
/// let baseline = compute_baseline(&[0.0, 100.0, 50.0, 110.0]);
/// assert_eq!(baseline, 102.0);
///
/// // Span bbox [0, 100, 50, 100] (zero height)
/// let baseline = compute_baseline(&[0.0, 100.0, 50.0, 100.0]);
/// assert_eq!(baseline, 100.0);
/// ```
#[inline]
pub fn compute_baseline(bbox: &[f32; 4]) -> f32 {
    let height = bbox[3] - bbox[1];
    bbox[1] + height * 0.2
}

/// Trait for types that have a bounding box.
///
/// This trait allows the line formation code to work with different
/// span representations (internal, JSON, etc.).
pub trait HasBBox {
    /// Get the bounding box [x0, y0, x1, y1] in PDF user space.
    fn bbox(&self) -> [f32; 4];
}

/// Compute the union of multiple bounding boxes.
///
/// # Arguments
///
/// * `bboxes` - Iterator of bounding boxes
///
/// # Returns
///
/// The union bounding box, or None if the iterator is empty.
pub fn union_bboxes<'a, I>(bboxes: I) -> Option<[f32; 4]>
where
    I: IntoIterator<Item = &'a [f32; 4]>,
{
    let mut iter = bboxes.into_iter();
    let first = *iter.next()?;
    let mut union = first;

    for bbox in iter {
        union[0] = union[0].min(bbox[0]); // x0
        union[1] = union[1].min(bbox[1]); // y0
        union[2] = union[2].max(bbox[2]); // x1
        union[3] = union[3].max(bbox[3]); // y1
    }

    Some(union)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: create a mock line with minimal required fields.
    fn make_test_line(
        baseline: f32,
        bbox: [f32; 4],
        median_font_size: f32,
        column: Option<usize>,
    ) -> TestLine {
        TestLine {
            baseline,
            bbox,
            median_font_size,
            column,
            rendering_mode: None,
        }
    }

    /// Mock line type for testing.
    #[derive(Debug, Clone)]
    struct TestLine {
        baseline: f32,
        bbox: [f32; 4],
        median_font_size: f32,
        column: Option<usize>,
        rendering_mode: Option<u32>,
    }

    impl LineMetadata for TestLine {
        fn baseline(&self) -> f32 {
            self.baseline
        }
        fn bbox(&self) -> [f32; 4] {
            self.bbox
        }
        fn median_font_size(&self) -> f32 {
            self.median_font_size
        }
        fn rendering_mode(&self) -> Option<u32> {
            self.rendering_mode
        }
        fn column(&self) -> Option<usize> {
            self.column
        }
    }

    #[test]
    fn test_compute_baseline_normal_span() {
        // Span bbox [0, 100, 50, 110] (height 10)
        // baseline = 100 + 10 * 0.2 = 102
        let bbox = [0.0, 100.0, 50.0, 110.0];
        assert_eq!(compute_baseline(&bbox), 102.0);
    }

    #[test]
    fn test_compute_baseline_zero_height() {
        // Span bbox [0, 100, 50, 100] (zero height)
        // baseline = 100 + 0 * 0.2 = 100
        let bbox = [0.0, 100.0, 50.0, 100.0];
        assert_eq!(compute_baseline(&bbox), 100.0);
    }

    #[test]
    fn test_compute_baseline_large_height() {
        // Span bbox [0, 0, 100, 50] (height 50)
        // baseline = 0 + 50 * 0.2 = 10
        let bbox = [0.0, 0.0, 100.0, 50.0];
        assert_eq!(compute_baseline(&bbox), 10.0);
    }

    #[test]
    fn test_line_direction_serdes_ltr() {
        let dir = LineDirection::Ltr;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"ltr\"");

        let deserialized: LineDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LineDirection::Ltr);
    }

    #[test]
    fn test_line_direction_serdes_rtl() {
        let dir = LineDirection::Rtl;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"rtl\"");

        let deserialized: LineDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LineDirection::Rtl);
    }

    #[test]
    fn test_line_direction_serdes_mixed() {
        let dir = LineDirection::Mixed;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"mixed\"");

        let deserialized: LineDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LineDirection::Mixed);
    }

    #[test]
    fn test_line_accessors() {
        let line: Line<()> = Line {
            spans: vec![],
            bbox: [10.0, 20.0, 110.0, 70.0],
            baseline: 30.0,
            direction: LineDirection::Ltr,
            page_relative_y: 0.5,
            median_font_size: 12.0,
            rendering_mode: None,
            column: Some(0),
        };

        assert_eq!(line.left(), 10.0);
        assert_eq!(line.bottom(), 20.0);
        assert_eq!(line.right(), 110.0);
        assert_eq!(line.top(), 70.0);
        assert_eq!(line.width(), 100.0);
        assert_eq!(line.height(), 50.0);
    }

    #[test]
    fn test_union_bboxes_single() {
        let bboxes = vec![[10.0, 20.0, 50.0, 40.0]];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, Some([10.0, 20.0, 50.0, 40.0]));
    }

    #[test]
    fn test_union_bboxes_multiple() {
        let bboxes = vec![
            [0.0, 0.0, 50.0, 20.0],
            [50.0, 0.0, 100.0, 20.0],
            [0.0, 20.0, 100.0, 40.0],
        ];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, Some([0.0, 0.0, 100.0, 40.0]));
    }

    #[test]
    fn test_union_bboxes_empty() {
        let bboxes: Vec<[f32; 4]> = vec![];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, None);
    }

    #[test]
    fn test_union_bboxes_nested() {
        // Small box inside larger box
        let bboxes = vec![[0.0, 0.0, 100.0, 100.0], [25.0, 25.0, 75.0, 75.0]];
        let result = union_bboxes(&bboxes);
        // Union should be the larger box
        assert_eq!(result, Some([0.0, 0.0, 100.0, 100.0]));
    }

    #[test]
    fn test_union_bboxes_disjoint() {
        // Two disjoint boxes
        let bboxes = vec![[0.0, 0.0, 50.0, 50.0], [100.0, 100.0, 150.0, 150.0]];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, Some([0.0, 0.0, 150.0, 150.0]));
    }

    // Phase 4.4 Block Formation Tests

    #[test]
    fn test_five_lines_equal_spacing_one_block() {
        // 5 lines equal spacing/font: 1 block
        let lines = vec![
            make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0)),
            make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0)),
            make_test_line(80.0, [0.0, 75.0, 100.0, 85.0], 12.0, Some(0)),
            make_test_line(70.0, [0.0, 65.0, 100.0, 75.0], 12.0, Some(0)),
            make_test_line(60.0, [0.0, 55.0, 100.0, 65.0], 12.0, Some(0)),
        ];
        let column_widths = vec![100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(blocks.len(), 1, "All 5 lines should form 1 block");
        assert_eq!(blocks[0].lines.len(), 5);
    }

    #[test]
    fn test_thirty_pt_gap_creates_two_blocks() {
        // 5 lines, 30pt gap, 5 more: 2 blocks
        let lines = vec![
            make_test_line(200.0, [0.0, 195.0, 100.0, 205.0], 12.0, Some(0)),
            make_test_line(190.0, [0.0, 185.0, 100.0, 195.0], 12.0, Some(0)),
            make_test_line(180.0, [0.0, 175.0, 100.0, 185.0], 12.0, Some(0)),
            make_test_line(170.0, [0.0, 165.0, 100.0, 175.0], 12.0, Some(0)),
            make_test_line(160.0, [0.0, 155.0, 100.0, 165.0], 12.0, Some(0)),
            // 30pt gap here (160 - 120 = 40pt gap, but 160 - 120 > 1.5 * 10 = 15pt)
            make_test_line(120.0, [0.0, 115.0, 100.0, 125.0], 12.0, Some(0)),
            make_test_line(110.0, [0.0, 105.0, 100.0, 115.0], 12.0, Some(0)),
            make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0)),
            make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0)),
            make_test_line(80.0, [0.0, 75.0, 100.0, 85.0], 12.0, Some(0)),
        ];
        let column_widths = vec![100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(blocks.len(), 2, "30pt gap should create 2 blocks");
        assert_eq!(blocks[0].lines.len(), 5);
        assert_eq!(blocks[1].lines.len(), 5);
    }

    #[test]
    fn test_heading_18pt_above_12pt_body_two_blocks() {
        // Heading 18pt above 12pt body: 2 blocks
        let lines = vec![
            make_test_line(100.0, [0.0, 92.0, 100.0, 108.0], 18.0, Some(0)), // Heading
            make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0)),   // Body
            make_test_line(80.0, [0.0, 75.0, 100.0, 85.0], 12.0, Some(0)),   // Body
            make_test_line(70.0, [0.0, 65.0, 100.0, 75.0], 12.0, Some(0)),   // Body
        ];
        let column_widths = vec![100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(
            blocks.len(),
            2,
            "Font size change (18pt vs 12pt) should create 2 blocks"
        );
        assert_eq!(blocks[0].lines.len(), 1);
        assert_eq!(blocks[1].lines.len(), 3);
    }

    #[test]
    fn test_two_column_separate_blocks() {
        // Two-column: lines in col 0 separate from col 1
        let lines = vec![
            make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0)), // Col 0
            make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0)),   // Col 0
            make_test_line(100.0, [150.0, 95.0, 250.0, 105.0], 12.0, Some(1)), // Col 1
            make_test_line(90.0, [150.0, 85.0, 250.0, 95.0], 12.0, Some(1)), // Col 1
        ];
        let column_widths = vec![100.0, 100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(blocks.len(), 2, "Column boundary should create 2 blocks");
        assert_eq!(blocks[0].column, 0);
        assert_eq!(blocks[1].column, 1);
    }

    #[test]
    fn test_indented_first_line_new_block() {
        // Indented first line (>9pt offset, 300pt column_width): NEW BLOCK starts
        let lines = vec![
            make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0)), // Non-indented
            make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0)),   // Non-indented
            // Indented by 10pt (> 0.03 * 300 = 9pt)
            make_test_line(80.0, [10.0, 75.0, 100.0, 85.0], 12.0, Some(0)), // Indented
            make_test_line(70.0, [10.0, 65.0, 100.0, 75.0], 12.0, Some(0)), // Indented
        ];
        let column_widths = vec![300.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(blocks.len(), 2, "Indent change should create 2 blocks");
        assert_eq!(blocks[0].lines.len(), 2);
        assert_eq!(blocks[1].lines.len(), 2);
    }

    #[test]
    fn test_rendering_mode_change_creates_new_block() {
        // Rendering mode change (visible vs invisible) creates new block
        let lines = vec![
            {
                let mut l = make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0));
                l.rendering_mode = Some(0);
                l
            },
            {
                let mut l = make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0));
                l.rendering_mode = Some(3); // Invisible
                l
            },
        ];
        let column_widths = vec![100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(
            blocks.len(),
            2,
            "Rendering mode change should create 2 blocks"
        );
    }

    #[test]
    fn test_empty_lines_returns_empty_blocks() {
        let lines: Vec<TestLine> = vec![];
        let column_widths = vec![100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(blocks.len(), 0);
    }

    #[test]
    fn test_single_line_returns_single_block() {
        let lines = vec![make_test_line(
            100.0,
            [0.0, 95.0, 100.0, 105.0],
            12.0,
            Some(0),
        )];
        let column_widths = vec![100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lines.len(), 1);
    }

    #[test]
    fn test_lines_sorted_by_column_then_baseline() {
        // Verify sorting: lines should be processed column ASC, baseline DESC
        let lines = vec![
            make_test_line(80.0, [150.0, 75.0, 250.0, 85.0], 12.0, Some(1)), // Col 1, y=80
            make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0)), // Col 0, y=100
            make_test_line(90.0, [150.0, 85.0, 250.0, 95.0], 12.0, Some(1)), // Col 1, y=90
            make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0)),   // Col 0, y=90
        ];
        let column_widths = vec![100.0, 100.0];
        let blocks = group_lines_into_blocks(lines, &column_widths);
        assert_eq!(blocks.len(), 2);
        // First block should be column 0 (lines at y=100, y=90)
        assert_eq!(blocks[0].column, 0);
        assert_eq!(blocks[0].lines.len(), 2);
        // Second block should be column 1 (lines at y=90, y=80)
        assert_eq!(blocks[1].column, 1);
        assert_eq!(blocks[1].lines.len(), 2);
    }
}
