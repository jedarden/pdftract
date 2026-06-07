//! Line formation for Phase 4.2.
//!
//! This module implements grouping spans into lines by baseline proximity
//! and computing line-level metadata including bbox, baseline, and direction.
//!
//! Phase 4.4 block formation is also implemented here, providing the
//! `group_lines_into_blocks` function that applies 5 ordered heuristics
//! to group lines into semantic blocks.

use serde::{Deserialize, Serialize};
use unicode_bidi::{BidiClass, bidi_class};

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
        // Only trigger when current line is MORE indented (to the right, larger x0)
        // than the block average. This detects new paragraphs starting after non-indented text.
        // It does NOT trigger for drop-cap style indents (first line indented, rest flush-left).
        let indent_delta = line_x0 - block_avg_x0.unwrap();
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

/// Detect the text direction for a line of text.
///
/// This function implements Phase 4.2 RTL detection by counting Unicode
/// bidi classes in the text and returning the dominant direction.
///
/// # Algorithm
///
/// Walk each character in the text and count bidi classes:
/// - **L (Left-to-Right):** LTR characters (Latin, Cyrillic, etc.)
/// - **R (Right-to-Left):** RTL characters (Arabic, Hebrew)
/// - **AL (Arabic Letter):** RTL characters (Arabic)
/// - All other classes (EN, ES, ET, AN, CS, NSM, BN, B, S, WS, ON, LRE, LRO, RLE, RLO, PDF, LRI, RLI, FSI, PDI) are ignored
///
/// # Returns
///
/// - `LineDirection::Ltr` if LTR count > RTL count OR both counts are zero (empty/neutral-only)
/// - `LineDirection::Rtl` if RTL count > LTR count
/// - `LineDirection::Mixed` if counts are equal (and both > 0)
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::line::{detect_line_direction, LineDirection};
///
/// // Latin text -> Ltr
/// assert_eq!(detect_line_direction("Hello, World!"), LineDirection::Ltr);
///
/// // Arabic text -> Rtl
/// assert_eq!(detect_line_direction("مرحبا بالعالم"), LineDirection::Rtl);
///
/// // Empty string -> Ltr (default)
/// assert_eq!(detect_line_direction(""), LineDirection::Ltr);
///
/// // Digits only -> Ltr (default, numerals are bidi-neutral)
/// assert_eq!(detect_line_direction("123 456"), LineDirection::Ltr);
/// ```
///
/// # INV
///
/// Numerals are bidi-neutral and do not drive direction. Punctuation is also neutral.
/// Empty lines default to Ltr.
pub fn detect_line_direction(line_text: &str) -> LineDirection {
    let mut ltr_count = 0u32;
    let mut rtl_count = 0u32;

    for ch in line_text.chars() {
        match bidi_class(ch) {
            BidiClass::L => ltr_count += 1,
            BidiClass::R | BidiClass::AL => rtl_count += 1,
            _ => {
                // All other bidi classes (EN, ES, ET, AN, CS, NSM, BN, B, S, WS, ON,
                // LRE, LRO, RLE, RLO, PDF, LRI, RLI, FSI, PDI) are ignored per INV:
                // numerals are bidi-neutral; punctuation is neutral
            }
        }
    }

    // Default to Ltr when both counts are zero (empty line or neutral-only text like digits)
    if ltr_count == 0 && rtl_count == 0 {
        return LineDirection::Ltr;
    }

    if rtl_count > ltr_count {
        LineDirection::Rtl
    } else if ltr_count > rtl_count {
        LineDirection::Ltr
    } else {
        // Mixed when counts are tied (and both > 0)
        LineDirection::Mixed
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

/// Trait for types that have font size.
///
/// This trait allows the clustering algorithm to work with different
/// span representations.
pub trait HasFontSize {
    /// Get the font size in points.
    fn font_size(&self) -> f32;
}

/// Trait for types that have text content.
///
/// This trait allows direction detection to work with different
/// span representations.
pub trait HasText {
    /// Get the text content.
    fn text(&self) -> &str;
}

/// Cluster spans into lines by baseline proximity.
///
/// This function implements Phase 4.2 Algorithm step 2: grouping spans
/// with baselines within `0.5 * median_font_size` of each other into
/// the same line.
///
/// # Algorithm
///
/// 1. Compute baseline for each span using `compute_baseline`
/// 2. Sort spans by baseline ASC
/// 3. Sweep through sorted spans:
///    - Track `cluster_max_baseline` (maximum baseline in current cluster)
///    - If `new_baseline - cluster_max_baseline <= 0.5 * median_font_size`, append to cluster
///    - Otherwise, close current cluster and start a new one
/// 4. Emit one `Line` per cluster
///
/// # Arguments
///
/// * `spans` - Spans to cluster, with bbox and font_size
/// * `median_font_size` - Median font size of all spans on the page (points)
///
/// # Returns
///
/// A vector of lines, each containing one or more spans sorted by x0 (left-to-right).
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::line::{cluster_spans_into_lines, TestSpan};
///
/// // Spans at baselines 100, 100.5, 105 with median 12 (threshold 6): all one line
/// let spans = vec![
///     TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // baseline ≈ 100
///     TestSpan::new([0.0, 98.5, 30.0, 108.5], 12.0), // baseline ≈ 100.5
///     TestSpan::new([0.0, 103.0, 40.0, 113.0], 12.0), // baseline ≈ 105
/// ];
/// let lines = cluster_spans_into_lines(spans, 12.0);
/// assert_eq!(lines.len(), 1);
///
/// // Spans at baselines 100, 110 with median 12 (threshold 6): two lines
/// let spans = vec![
///     TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // baseline ≈ 100
///     TestSpan::new([0.0, 108.0, 50.0, 118.0], 12.0), // baseline ≈ 110
/// ];
/// let lines = cluster_spans_into_lines(spans, 12.0);
/// assert_eq!(lines.len(), 2);
/// ```
///
/// # INV
///
/// The threshold is `0.5 * median_font_size`, never hardcoded.
/// This ensures superscripts (small font, slightly higher baseline) stay
/// on the same line as the base text.
pub fn cluster_spans_into_lines<S>(spans: Vec<S>, median_font_size: f32) -> Vec<Line<S>>
where
    S: HasBBox + HasFontSize + HasText + Clone,
{
    if spans.is_empty() {
        return Vec::new();
    }

    // INV: threshold = 0.5 * median_font_size; do NOT hardcode
    let threshold = 0.5 * median_font_size;

    // Step 1: Compute baseline for each span and sort by baseline ASC
    let mut baselines: Vec<(f32, S)> = spans
        .into_iter()
        .map(|span| {
            let baseline = compute_baseline(&span.bbox());
            (baseline, span)
        })
        .collect();

    baselines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Step 2: Sweep through sorted spans, clustering within threshold
    let mut lines: Vec<Line<S>> = Vec::new();
    let mut current_cluster_spans: Vec<S> = Vec::new();
    let mut cluster_max_baseline: Option<f32> = None;
    let mut cluster_union_bbox: Option<[f32; 4]> = None;

    for (baseline, span) in baselines {
        if current_cluster_spans.is_empty() {
            // First span in cluster
            current_cluster_spans.push(span.clone());
            cluster_max_baseline = Some(baseline);
            cluster_union_bbox = Some(span.bbox());
            continue;
        }

        let cluster_max = cluster_max_baseline.unwrap();
        let delta = baseline - cluster_max;

        if delta <= threshold {
            // Within threshold: append to current cluster
            current_cluster_spans.push(span.clone());
            cluster_max_baseline = Some(baseline); // Update max baseline

            // Update union bbox
            if let Some(ref mut union) = cluster_union_bbox {
                let bbox = span.bbox();
                union[0] = union[0].min(bbox[0]); // x0
                union[1] = union[1].min(bbox[1]); // y0
                union[2] = union[2].max(bbox[2]); // x1
                union[3] = union[3].max(bbox[3]); // y1
            }
        } else {
            // Beyond threshold: close current cluster and start new one
            lines.push(finalize_line_cluster(
                std::mem::take(&mut current_cluster_spans),
                cluster_union_bbox.unwrap(),
            ));

            // Start new cluster with this span
            current_cluster_spans.push(span.clone());
            cluster_max_baseline = Some(baseline);
            cluster_union_bbox = Some(span.bbox());
        }
    }

    // Finalize the last cluster
    if !current_cluster_spans.is_empty() {
        lines.push(finalize_line_cluster(
            current_cluster_spans,
            cluster_union_bbox.unwrap(),
        ));
    }

    lines
}

/// Finalize a line cluster by sorting spans by x0 and computing metadata.
fn finalize_line_cluster<S>(mut spans: Vec<S>, union_bbox: [f32; 4]) -> Line<S>
where
    S: HasBBox + HasFontSize + HasText,
{
    // Sort spans by x0 (left-to-right for LTR scripts)
    spans.sort_by(|a, b| {
        a.bbox()[0]
            .partial_cmp(&b.bbox()[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Compute line metadata
    let baseline = if spans.is_empty() {
        union_bbox[1] + (union_bbox[3] - union_bbox[1]) * 0.2
    } else {
        // Average of member span baselines
        let sum: f32 = spans.iter().map(|s| compute_baseline(&s.bbox())).sum();
        sum / spans.len() as f32
    };

    // Compute median font size of spans in this line
    let mut font_sizes: Vec<f32> = spans.iter().map(|s| s.font_size()).collect();
    font_sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_font_size = font_sizes[font_sizes.len() / 2];

    // Detect text direction by concatenating span text
    let line_text: String = spans.iter().map(|s| s.text()).collect();
    let direction = detect_line_direction(&line_text);

    Line {
        spans,
        bbox: union_bbox,
        baseline,
        direction,
        page_relative_y: 0.0,          // TODO: Compute from page_height
        median_font_size,
        rendering_mode: None, // TODO: Extract from span metadata
        column: None,         // Set by Phase 4.3 column detection
    }
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

/// Classify a block as a heading based on font size and line count.
///
/// A block is classified as a heading if ALL of the following are true:
/// 1. The block's median font size > 1.2 * page_body_median_font_size
/// 2. The block has exactly 1 line (or 0 lines for empty blocks, though empty blocks won't pass the font size check)
///
/// # Arguments
///
/// * `block` - The block to classify (will have kind updated to "heading" if criteria met)
/// * `page_body_median_font_size` - The median font size of paragraph blocks on the page
///
/// # Returns
///
/// `true` if the block was classified as a heading, `false` otherwise.
///
/// # INV
///
/// - Threshold is strictly `> 1.2`, not `>= 1.2`
/// - Single-line criterion is `lines.len() <= 1`
pub fn classify_heading<L>(block: &mut BlockInput<L>, page_body_median_font_size: f32) -> bool
where
    L: LineMetadata + Clone,
{
    // INV: threshold is strictly > 1.2
    let ratio = block.median_font_size / page_body_median_font_size;
    let size_criterion = ratio > 1.2;

    // Single-line criterion (must be exactly 1 line, not 0)
    let line_count_criterion = block.lines.len() == 1;

    if size_criterion && line_count_criterion {
        // Note: BlockInput doesn't have a kind field, so we can't set it here
        // The calling code should set the kind based on the return value
        true
    } else {
        false
    }
}

/// Classify all blocks on a page as headings where appropriate.
///
/// This function processes blocks and classifies each block as a heading
/// if it meets the font size and line count criteria.
///
/// # Arguments
///
/// * `blocks` - Mutable slice of BlockInput to classify
/// * `page_body_median_font_size` - The median font size of paragraph blocks on the page
///
/// # Returns
///
/// A vector of indices indicating which blocks were classified as headings.
pub fn classify_page_headings<L>(
    blocks: &mut [BlockInput<L>],
    page_body_median_font_size: f32,
) -> Vec<usize>
where
    L: LineMetadata + Clone,
{
    let mut heading_indices = Vec::new();

    for (idx, block) in blocks.iter_mut().enumerate() {
        if classify_heading(block, page_body_median_font_size) {
            heading_indices.push(idx);
        }
    }

    heading_indices
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

    /// Mock span type for testing cluster_spans_into_lines.
    #[derive(Debug, Clone)]
    struct TestSpan {
        bbox: [f32; 4],
        font_size: f32,
        text: String,
    }

    impl TestSpan {
        /// Create a new test span.
        fn new(bbox: [f32; 4], font_size: f32) -> Self {
            Self { bbox, font_size, text: String::new() }
        }

        /// Create a new test span with text.
        fn with_text(bbox: [f32; 4], font_size: f32, text: &str) -> Self {
            Self { bbox, font_size, text: text.to_string() }
        }
    }

    impl HasBBox for TestSpan {
        fn bbox(&self) -> [f32; 4] {
            self.bbox
        }
    }

    impl HasFontSize for TestSpan {
        fn font_size(&self) -> f32 {
            self.font_size
        }
    }

    impl HasText for TestSpan {
        fn text(&self) -> &str {
            &self.text
        }
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

    // Phase 4.2 RTL Direction Detection Tests

    #[test]
    fn test_detect_line_direction_latin_text() {
        // "Hello, World!" -> Ltr
        assert_eq!(detect_line_direction("Hello, World!"), LineDirection::Ltr);
    }

    #[test]
    fn test_detect_line_direction_arabic_text() {
        // "مرحبا بالعالم" -> Rtl (Arabic greeting "Hello world")
        assert_eq!(detect_line_direction("مرحبا بالعالم"), LineDirection::Rtl);
    }

    #[test]
    fn test_detect_line_direction_empty_string() {
        // "" -> Ltr (default per bead acceptance criteria)
        assert_eq!(detect_line_direction(""), LineDirection::Ltr);
    }

    #[test]
    fn test_detect_line_direction_digits_only() {
        // "123 456" -> Ltr (default per bead acceptance criteria)
        assert_eq!(detect_line_direction("123 456"), LineDirection::Ltr);
    }

    #[test]
    fn test_detect_line_direction_punctuation_only() {
        // "!?,." -> Ltr (default per bead acceptance criteria)
        assert_eq!(detect_line_direction("!?,."), LineDirection::Ltr);
    }

    #[test]
    fn test_detect_line_direction_latin_dominant() {
        // Latin text with some punctuation -> Ltr
        assert_eq!(detect_line_direction("Hello, World! 123"), LineDirection::Ltr);
    }

    #[test]
    fn test_detect_line_direction_arabic_dominant() {
        // Arabic text with digits -> Rtl (Arabic characters dominate)
        assert_eq!(detect_line_direction("مرحبا 123"), LineDirection::Rtl);
    }

    #[test]
    fn test_detect_line_direction_mixed_latin_arabic() {
        // Equal Latin and Arabic characters -> Mixed
        let text = "Hello مرحبا"; // 5 Latin + 1 space + 5 Arabic
        assert_eq!(detect_line_direction(text), LineDirection::Mixed);
    }

    #[test]
    fn test_detect_line_direction_latin_more_than_arabic() {
        // More Latin than Arabic -> Ltr
        let text = "Hello world مرحبا"; // 10 Latin + 1 space + 5 Arabic
        assert_eq!(detect_line_direction(text), LineDirection::Ltr);
    }

    #[test]
    fn test_detect_line_direction_arabic_more_than_latin() {
        // More Arabic than Latin -> Rtl
        let text = "مرحبا بالعالم Hi"; // 10 Arabic + 1 space + 2 Latin
        assert_eq!(detect_line_direction(text), LineDirection::Rtl);
    }

    #[test]
    fn test_detect_line_direction_hebrew_text() {
        // Hebrew text -> Rtl
        assert_eq!(detect_line_direction("שלום עולם"), LineDirection::Rtl);
    }

    #[test]
    fn test_detect_line_direction_cyrillic_text() {
        // Cyrillic text -> Ltr
        assert_eq!(detect_line_direction("Привет мир"), LineDirection::Ltr);
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
    fn test_indented_first_line_of_paragraph_not_split() {
        // Indented first line of paragraph (like a drop cap): should NOT split into two blocks
        // Coordinator acceptance criterion: "Indented first line of paragraph: NOT split into two blocks unconditionally."
        // Scenario: First line indented (like a drop cap at x0=10), subsequent lines at x0=0
        // Expected: ONE block (entire paragraph stays together)
        let lines = vec![
            make_test_line(100.0, [10.0, 95.0, 100.0, 105.0], 12.0, Some(0)), // Indented first line (drop cap)
            make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0)),    // Not indented (continuation)
            make_test_line(80.0, [0.0, 75.0, 100.0, 85.0], 12.0, Some(0)),    // Not indented
        ];
        let column_widths = vec![300.0]; // 0.03 * 300 = 9pt threshold, indent delta = 10pt
        let blocks = group_lines_into_blocks(lines, &column_widths);
        // Currently this FAILS (creates 2 blocks), but the coordinator acceptance criterion says it should PASS (1 block)
        // TODO: Fix indent trigger to not split at first line of block
        assert_eq!(blocks.len(), 1, "Indented first line of paragraph should NOT split into two blocks");
        assert_eq!(blocks[0].lines.len(), 3, "All three lines should be in one block");
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

    // Phase 4.2 Line Formation Tests (cluster_spans_into_lines)

    #[test]
    fn test_cluster_spans_baselines_100_100_5_105_median_12_one_line() {
        // Spans baselines 100, 100.5, 105 with median 12 (threshold 6): all one line
        let spans = vec![
            TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // baseline ≈ 100
            TestSpan::new([0.0, 98.5, 30.0, 108.5], 12.0), // baseline ≈ 100.5
            TestSpan::new([0.0, 103.0, 40.0, 113.0], 12.0), // baseline ≈ 105
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(lines.len(), 1, "All 3 spans should form 1 line");
        assert_eq!(lines[0].spans.len(), 3);
    }

    #[test]
    fn test_cluster_spans_baselines_100_110_median_12_two_lines() {
        // Same with 100, 110: 2 lines (delta 10 > 6)
        let spans = vec![
            TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // baseline ≈ 100
            TestSpan::new([0.0, 108.0, 50.0, 118.0], 12.0), // baseline ≈ 110
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(
            lines.len(),
            2,
            "Delta 10 > threshold 6 should create 2 lines"
        );
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[1].spans.len(), 1);
    }

    #[test]
    fn test_cluster_spans_superscript_stays_on_same_line() {
        // Superscript at 105, line baseline 100, font 12: SAME line
        let spans = vec![
            TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // baseline ≈ 100
            TestSpan::new([50.0, 103.0, 70.0, 113.0], 8.0), // superscript, baseline ≈ 105
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(
            lines.len(),
            1,
            "Superscript should stay on same line as base text"
        );
        assert_eq!(lines[0].spans.len(), 2);
    }

    #[test]
    fn test_cluster_spans_empty_input_empty_output() {
        let spans: Vec<TestSpan> = vec![];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(lines.len(), 0, "Empty input should produce empty output");
    }

    #[test]
    fn test_cluster_spans_single_span_single_line() {
        let spans = vec![TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0)];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 1);
    }

    #[test]
    fn test_cluster_spans_threshold_is_0_5_times_median_font_size() {
        // INV: threshold = 0.5 * median_font_size; do NOT hardcode
        // Test with median 20 (threshold 10): baselines 100 and 109 should be one line
        let spans = vec![
            TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // baseline ≈ 100
            TestSpan::new([0.0, 107.0, 50.0, 117.0], 12.0), // baseline ≈ 109
        ];
        let lines = cluster_spans_into_lines(spans, 20.0);
        assert_eq!(
            lines.len(),
            1,
            "Delta 9 <= threshold 10 should create 1 line"
        );
    }

    #[test]
    fn test_cluster_spans_sorted_by_x0_within_line() {
        // Spans within a line should be sorted by x0 (left-to-right)
        let spans = vec![
            TestSpan::new([50.0, 98.0, 70.0, 108.0], 12.0), // Right side
            TestSpan::new([0.0, 98.0, 30.0, 108.0], 12.0),  // Left side
            TestSpan::new([30.0, 98.0, 50.0, 108.0], 12.0), // Middle
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
        // Verify sorted by x0
        assert_eq!(lines[0].spans[0].bbox()[0], 0.0);
        assert_eq!(lines[0].spans[1].bbox()[0], 30.0);
        assert_eq!(lines[0].spans[2].bbox()[0], 50.0);
    }

    #[test]
    fn test_cluster_spans_two_column_at_same_y_one_line() {
        // Two-column at same y: cluster into one Line; Phase 4.4 splits per column
        let spans = vec![
            TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // Column 0
            TestSpan::new([150.0, 98.0, 200.0, 108.0], 12.0), // Column 1
            TestSpan::new([50.0, 98.0, 80.0, 108.0], 12.0), // Column 0
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        // All spans at same baseline should be in one line
        assert_eq!(
            lines.len(),
            1,
            "Two-column at same y should cluster into one Line"
        );
        assert_eq!(lines[0].spans.len(), 3);
    }

    #[test]
    fn test_cluster_spans_union_bbox_computed_correctly() {
        // Verify union bbox is computed correctly
        let spans = vec![
            TestSpan::new([10.0, 90.0, 40.0, 100.0], 12.0),
            TestSpan::new([40.0, 90.0, 70.0, 100.0], 12.0),
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(lines.len(), 1);
        // Union bbox should be [10, 90, 70, 100]
        assert_eq!(lines[0].bbox[0], 10.0);
        assert_eq!(lines[0].bbox[1], 90.0);
        assert_eq!(lines[0].bbox[2], 70.0);
        assert_eq!(lines[0].bbox[3], 100.0);
    }

    #[test]
    fn test_cluster_spans_baseline_computed_as_average() {
        // Verify baseline is average of member span baselines
        let spans = vec![
            TestSpan::new([0.0, 98.0, 50.0, 108.0], 12.0), // baseline ≈ 100
            TestSpan::new([0.0, 92.0, 50.0, 102.0], 12.0), // baseline ≈ 94
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(lines.len(), 1);
        // Average baseline should be (100 + 94) / 2 = 97
        assert!((lines[0].baseline - 97.0).abs() < 0.1);
    }

    #[test]
    fn test_cluster_spans_median_font_size_computed() {
        // Verify median font size is computed from line spans
        let spans = vec![
            TestSpan::new([0.0, 98.0, 50.0, 108.0], 10.0),
            TestSpan::new([0.0, 92.0, 50.0, 102.0], 12.0),
            TestSpan::new([0.0, 86.0, 50.0, 96.0], 14.0),
        ];
        let lines = cluster_spans_into_lines(spans, 12.0);
        assert_eq!(lines.len(), 1);
        // Median of [10, 12, 14] is 12
        assert_eq!(lines[0].median_font_size, 12.0);
    }

    // Phase 4.4 Heading Detection Tests

    #[test]
    fn test_classify_heading_18pt_block_12pt_body_one_line_heading() {
        // AC: 18pt block, body 12pt, 1 line: Heading (1.5 > 1.2)
        let mut block = BlockInput {
            lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 18.0, Some(0))],
            bbox: [0.0, 95.0, 100.0, 105.0],
            median_font_size: 18.0,
            column: 0,
        };
        let page_body_median = 12.0;

        assert!(classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_14pt_block_12pt_body_one_line_not_heading() {
        // AC: 14pt block, body 12pt, 1 line: NOT (1.17 < 1.2)
        let mut block = BlockInput {
            lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 14.0, Some(0))],
            bbox: [0.0, 95.0, 100.0, 105.0],
            median_font_size: 14.0,
            column: 0,
        };
        let page_body_median = 12.0;

        // 14 / 12 = 1.167 < 1.2, so NOT heading
        assert!(!classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_18pt_block_three_lines_not_heading() {
        // AC: 18pt block, 3 lines: NOT (too many lines)
        let mut block = BlockInput {
            lines: vec![
                make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 18.0, Some(0)),
                make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 18.0, Some(0)),
                make_test_line(80.0, [0.0, 75.0, 100.0, 85.0], 18.0, Some(0)),
            ],
            bbox: [0.0, 75.0, 100.0, 105.0],
            median_font_size: 18.0,
            column: 0,
        };
        let page_body_median = 12.0;

        // Too many lines, even though font size is large
        assert!(!classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_12pt_block_12pt_body_not_heading() {
        // AC: 12pt block, body 12pt: NOT
        let mut block = BlockInput {
            lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0))],
            bbox: [0.0, 95.0, 100.0, 105.0],
            median_font_size: 12.0,
            column: 0,
        };
        let page_body_median = 12.0;

        // 12 / 12 = 1.0 < 1.2, so NOT heading
        assert!(!classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_threshold_exactly_1_2_not_heading() {
        // Exactly 1.2 threshold: NOT heading (strict inequality)
        let mut block = BlockInput {
            lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.0, Some(0))],
            bbox: [0.0, 95.0, 100.0, 105.0],
            median_font_size: 12.0,
            column: 0,
        };
        let page_body_median = 10.0;

        // 12 / 10 = 1.2 exactly, NOT > 1.2, so NOT heading
        assert!(!classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_threshold_just_above_1_2_is_heading() {
        // Just above 1.2 threshold: IS heading
        let mut block = BlockInput {
            lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 12.1, Some(0))],
            bbox: [0.0, 95.0, 100.0, 105.0],
            median_font_size: 12.1,
            column: 0,
        };
        let page_body_median = 10.0;

        // 12.1 / 10 = 1.21 > 1.2, so IS heading
        assert!(classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_empty_lines_not_heading() {
        // Empty block (0 lines): NOT heading
        let mut block: BlockInput<TestLine> = BlockInput {
            lines: vec![],
            bbox: [0.0, 0.0, 0.0, 0.0],
            median_font_size: 18.0,
            column: 0,
        };
        let page_body_median = 12.0;

        // Empty lines, even though font size is large
        assert!(!classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_two_lines_not_heading() {
        // Two lines: NOT heading
        let mut block = BlockInput {
            lines: vec![
                make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 18.0, Some(0)),
                make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 18.0, Some(0)),
            ],
            bbox: [0.0, 85.0, 100.0, 105.0],
            median_font_size: 18.0,
            column: 0,
        };
        let page_body_median = 12.0;

        // Two lines, even though font size is large
        assert!(!classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_small_page_body_median() {
        // Small page body median (e.g., 8pt) with 10pt block
        let mut block = BlockInput {
            lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 10.0, Some(0))],
            bbox: [0.0, 95.0, 100.0, 105.0],
            median_font_size: 10.0,
            column: 0,
        };
        let page_body_median = 8.0;

        // 10 / 8 = 1.25 > 1.2, so IS heading
        assert!(classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_heading_large_page_body_median() {
        // Large page body median (e.g., 16pt) with 20pt block
        let mut block = BlockInput {
            lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 20.0, Some(0))],
            bbox: [0.0, 95.0, 100.0, 105.0],
            median_font_size: 20.0,
            column: 0,
        };
        let page_body_median = 16.0;

        // 20 / 16 = 1.25 > 1.2, so IS heading
        assert!(classify_heading(&mut block, page_body_median));
    }

    #[test]
    fn test_classify_page_headings_multiple() {
        // Test classify_page_headings with multiple blocks
        let mut blocks = vec![
            BlockInput {
                lines: vec![make_test_line(100.0, [0.0, 95.0, 100.0, 105.0], 18.0, Some(0))],
                bbox: [0.0, 95.0, 100.0, 105.0],
                median_font_size: 18.0,
                column: 0,
            },
            BlockInput {
                lines: vec![make_test_line(90.0, [0.0, 85.0, 100.0, 95.0], 12.0, Some(0))],
                bbox: [0.0, 85.0, 100.0, 95.0],
                median_font_size: 12.0,
                column: 0,
            },
            BlockInput {
                lines: vec![make_test_line(80.0, [0.0, 75.0, 100.0, 85.0], 15.0, Some(0))],
                bbox: [0.0, 75.0, 100.0, 85.0],
                median_font_size: 15.0,
                column: 0,
            },
        ];
        let page_body_median = 12.0;

        let heading_indices = classify_page_headings(&mut blocks, page_body_median);

        // First block (18pt > 1.2*12pt, 1 line) IS heading
        // Second block (12pt = 12pt) NOT heading
        // Third block (15pt > 1.2*12pt, 1 line) IS heading
        assert_eq!(heading_indices, vec![0, 2]);
    }
}
