//! Reading order determination for Phase 4.5.
//!
//! This module implements the XY-cut recursive algorithm for determining
//! the reading order of blocks within a page. XY-cut is the preferred
//! path for rectilinear layouts (academic papers, books).
//!
//! ## Algorithm
//!
//! 1. Find the widest vertical whitespace gap dividing the page's text bbox
//!    into left and right halves → split into two regions
//! 2. For each region, find the widest horizontal gap → split into top and bottom
//! 3. Recurse until regions contain a single column of text
//! 4. Reading order: left region before right; top before bottom within each region
//!
//! ## Docstrum Fallback
//!
//! When XY-cut produces > 10 regions with < 3 blocks each, the caller should
//! switch to the Docstrum algorithm (nearest-neighbor graph traversal).

use std::collections::{HashMap, HashSet};

/// Maximum recursion depth for XY-cut to prevent stack overflow on pathological layouts.
const MAX_DEPTH: u32 = 20;

/// Minimum block count to trigger Docstrum fallback.
/// If XY-cut produces > 10 regions with < 3 blocks each, use Docstrum instead.
const REGION_COUNT_THRESHOLD: usize = 10;

/// Minimum blocks per region to consider XY-cut successful.
const MIN_BLOCKS_PER_REGION: usize = 3;

/// Result of XY-cut reading order analysis.
///
/// Contains the ordered block indices and metadata about the analysis.
#[derive(Debug, Clone)]
pub struct XYCutResult {
    /// Block indices in reading order.
    pub order: Vec<usize>,
    /// Number of regions created during XY-cut.
    pub region_count: usize,
    /// Count of regions with fewer than 3 blocks (signals Docstrum trigger).
    pub small_region_count: usize,
    /// The algorithm used: "xy_cut" or "docstrum".
    pub algorithm: String,
}

/// XY-cut recursive widest-whitespace split.
///
/// Returns input block indices in reading order. Algorithm:
/// - Find widest VERTICAL whitespace gap, split into left+right; recurse on each half
/// - Find widest HORIZONTAL gap, split top-then-bottom; recurse
/// - Continue until single column
///
/// # Arguments
///
/// * `blocks` - Blocks to order (must have bbox accessible)
/// * `page_width` - Page width in points
/// * `page_height` - Page height in points
///
/// # Returns
///
/// `XYCutResult` with ordered block indices and metadata.
///
/// # Behavior
///
/// - Single block / empty: returns as-is with order = [0] or []
/// - Prefers vertical split first (columns dominate)
/// - > 10 regions with < 3 blocks: signals Docstrum trigger (caller switches)
/// - Leaf nodes (single column): sorted by y descending (top-to-bottom reading)
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::reading_order::{xy_cut, BlockWithBBox};
///
/// let blocks = vec![
///     BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]), // col 0, top
///     BlockWithBBox::new(1, [50.0, 600.0, 250.0, 650.0]), // col 0, mid
///     BlockWithBBox::new(2, [50.0, 500.0, 250.0, 550.0]), // col 0, bot
///     BlockWithBBox::new(3, [350.0, 700.0, 550.0, 750.0]), // col 1, top
///     BlockWithBBox::new(4, [350.0, 600.0, 550.0, 650.0]), // col 1, mid
///     BlockWithBBox::new(5, [350.0, 500.0, 550.0, 550.0]), // col 1, bot
/// ];
///
/// let result = xy_cut(&blocks, 612.0, 792.0);
/// // Order: col0 all (0,1,2), then col1 all (3,4,5)
/// assert_eq!(result.order, vec![0, 1, 2, 3, 4, 5]);
/// ```
pub fn xy_cut<B>(blocks: &[B], page_width: f32, page_height: f32) -> XYCutResult
where
    B: HasBBox + Clone,
{
    if blocks.is_empty() {
        return XYCutResult {
            order: vec![],
            region_count: 0,
            small_region_count: 0,
            algorithm: "xy_cut".to_string(),
        };
    }

    if blocks.len() == 1 {
        return XYCutResult {
            order: vec![0],
            region_count: 1,
            small_region_count: 0,
            algorithm: "xy_cut".to_string(),
        };
    }

    // Track region statistics
    let mut region_count = 0;
    let mut small_region_count = 0;

    // Initial call with all block indices
    let initial_indices: Vec<usize> = (0..blocks.len()).collect();
    let (order, stats) = xy_cut_recursive(blocks, &initial_indices, page_width, page_height, 0);

    region_count = stats.region_count;
    small_region_count = stats.small_region_count;

    XYCutResult {
        order,
        region_count,
        small_region_count,
        algorithm: "xy_cut".to_string(),
    }
}

/// Statistics tracked during recursion.
#[derive(Debug, Clone, Default)]
struct RecursionStats {
    region_count: usize,
    small_region_count: usize,
}

/// Recursive XY-cut implementation.
///
/// Returns (ordered_indices, stats) for the given subset of blocks.
fn xy_cut_recursive<B>(
    blocks: &[B],
    indices: &[usize],
    page_width: f32,
    page_height: f32,
    depth: u32,
) -> (Vec<usize>, RecursionStats)
where
    B: HasBBox + Clone,
{
    // Base case: single block or max depth reached
    if indices.len() <= 1 || depth >= MAX_DEPTH {
        let mut stats = RecursionStats::default();
        stats.region_count = 1;
        if indices.len() < MIN_BLOCKS_PER_REGION {
            stats.small_region_count = 1;
        }
        return (indices.to_vec(), stats);
    }

    // Get the subset of blocks
    let subset_indices = indices;
    let subset_bboxes: Vec<[f32; 4]> = subset_indices.iter().map(|&i| blocks[i].bbox()).collect();

    // Compute the overall bbox of this region
    let region_bbox = union_bboxes_from_coords(&subset_bboxes);

    // Check if all blocks are in a single column (vertically stacked)
    // Single column: all blocks have overlapping x-ranges (> 50% overlap with median x-range)
    if is_single_column(&subset_bboxes) {
        // Single column: no further splits needed, sort by y descending
        let mut sorted_indices = indices.to_vec();
        sorted_indices.sort_by(|&a, &b| {
            let bbox_a = blocks[a].bbox();
            let bbox_b = blocks[b].bbox();
            bbox_b[3]
                .partial_cmp(&bbox_a[3])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut stats = RecursionStats::default();
        stats.region_count = 1;
        if indices.len() < MIN_BLOCKS_PER_REGION {
            stats.small_region_count = 1;
        }

        return (sorted_indices, stats);
    }

    // Try vertical split first (columns dominate)
    if let Some((x_split, left_indices, right_indices)) =
        find_vertical_split(blocks, indices, region_bbox)
    {
        // Recurse on left and right halves
        let (left_order, left_stats) =
            xy_cut_recursive(blocks, &left_indices, page_width, page_height, depth + 1);
        let (right_order, right_stats) =
            xy_cut_recursive(blocks, &right_indices, page_width, page_height, depth + 1);

        // Combine: left before right
        let mut order = left_order;
        order.extend(right_order);

        let mut stats = RecursionStats::default();
        stats.region_count = left_stats.region_count + right_stats.region_count;
        stats.small_region_count = left_stats.small_region_count + right_stats.small_region_count;

        return (order, stats);
    }

    // Try horizontal split (top/bottom)
    if let Some((y_split, top_indices, bottom_indices)) =
        find_horizontal_split(blocks, indices, region_bbox)
    {
        // Recurse on top and bottom halves
        let (top_order, top_stats) =
            xy_cut_recursive(blocks, &top_indices, page_width, page_height, depth + 1);
        let (bottom_order, bottom_stats) =
            xy_cut_recursive(blocks, &bottom_indices, page_width, page_height, depth + 1);

        // Combine: top before bottom
        let mut order = top_order;
        order.extend(bottom_order);

        let mut stats = RecursionStats::default();
        stats.region_count = top_stats.region_count + bottom_stats.region_count;
        stats.small_region_count = top_stats.small_region_count + bottom_stats.small_region_count;

        return (order, stats);
    }

    // No valid split found: sort by y descending (top-to-bottom reading order)
    let mut sorted_indices = indices.to_vec();
    sorted_indices.sort_by(|&a, &b| {
        let bbox_a = blocks[a].bbox();
        let bbox_b = blocks[b].bbox();
        // Sort by y1 (top) descending, then y0 (bottom) descending
        bbox_b[3]
            .partial_cmp(&bbox_a[3])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                bbox_b[1]
                    .partial_cmp(&bbox_a[1])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut stats = RecursionStats::default();
    stats.region_count = 1;
    if indices.len() < MIN_BLOCKS_PER_REGION {
        stats.small_region_count = 1;
    }

    (sorted_indices, stats)
}

/// Find the widest vertical whitespace gap.
///
/// Projects x-extents of all blocks and finds the largest gap with no coverage.
/// Uses a projection approach: for each x position, count blocks covering it.
/// The widest contiguous region with zero coverage is the column gap.
///
/// Returns None if no valid gap exists (gap too small or wouldn't split blocks).
fn find_vertical_split<B>(
    blocks: &[B],
    indices: &[usize],
    region_bbox: [f32; 4],
) -> Option<(f32, Vec<usize>, Vec<usize>)>
where
    B: HasBBox,
{
    let region_width = region_bbox[2] - region_bbox[0];
    let region_x0 = region_bbox[0];

    // Minimum gap threshold: 3% of region width or 15 points, whichever is smaller
    // Using smaller threshold to detect narrower column gaps
    let min_gap = (region_width * 0.03).min(15.0);

    // Create a projection histogram: discretize x-axis and count coverage
    // Use 1-point bins for precision
    let x_start = region_bbox[0].floor() as i32;
    let x_end = region_bbox[2].ceil() as i32;
    let num_bins = (x_end - x_start) as usize;

    if num_bins == 0 {
        return None;
    }

    let mut coverage = vec![0u16; num_bins];
    let mut max_coverage = 0u16;

    for &idx in indices {
        let bbox = blocks[idx].bbox();
        let bin_start = (bbox[0].floor() as i32 - x_start).clamp(0, num_bins as i32 - 1) as usize;
        let bin_end = (bbox[2].ceil() as i32 - x_start).clamp(0, num_bins as i32) as usize;

        for bin in bin_start..bin_end.min(num_bins) {
            coverage[bin] = coverage[bin].saturating_add(1);
            max_coverage = max_coverage.max(coverage[bin]);
        }
    }

    // Find the widest contiguous gap (zero coverage)
    let mut best_gap: Option<(f32, Vec<usize>, Vec<usize>)> = None;
    let mut max_gap_width = 0.0;
    let mut gap_start: Option<usize> = None;

    for (i, &count) in coverage.iter().enumerate() {
        if count == 0 {
            if gap_start.is_none() {
                gap_start = Some(i);
            }
        } else {
            if let Some(start) = gap_start {
                let gap_width = (i - start) as f32;
                let gap_x0 = region_x0 + start as f32;
                let gap_x1 = region_x0 + i as f32;

                if gap_width >= min_gap && gap_width > max_gap_width {
                    max_gap_width = gap_width;

                    // Split indices by the gap midpoint
                    let split_x = (gap_x0 + gap_x1) / 2.0;
                    let left: Vec<usize> = indices
                        .iter()
                        .copied()
                        .filter(|&idx| {
                            let bbox = blocks[idx].bbox();
                            bbox[2] <= split_x // x1 <= split
                        })
                        .collect();
                    let right: Vec<usize> = indices
                        .iter()
                        .copied()
                        .filter(|&idx| {
                            let bbox = blocks[idx].bbox();
                            bbox[0] >= split_x // x0 >= split
                        })
                        .collect();

                    // Only accept if both sides have blocks
                    if !left.is_empty() && !right.is_empty() {
                        best_gap = Some((split_x, left, right));
                    }
                }

                gap_start = None;
            }
        }
    }

    // Handle gap at the end
    if let Some(start) = gap_start {
        let gap_width = (num_bins - start) as f32;
        let gap_x0 = region_x0 + start as f32;
        let gap_x1 = region_x0 + num_bins as f32;

        if gap_width >= min_gap && gap_width > max_gap_width {
            let split_x = (gap_x0 + gap_x1) / 2.0;
            let left: Vec<usize> = indices
                .iter()
                .copied()
                .filter(|&idx| {
                    let bbox = blocks[idx].bbox();
                    bbox[2] <= split_x
                })
                .collect();
            let right: Vec<usize> = indices
                .iter()
                .copied()
                .filter(|&idx| {
                    let bbox = blocks[idx].bbox();
                    bbox[0] >= split_x
                })
                .collect();

            if !left.is_empty() && !right.is_empty() {
                best_gap = Some((split_x, left, right));
            }
        }
    }

    best_gap
}

/// Find the widest horizontal whitespace gap.
///
/// Projects y-extents of all blocks and finds the largest gap with no coverage.
/// Uses a projection approach similar to find_vertical_split.
///
/// Returns None if no valid gap exists.
fn find_horizontal_split<B>(
    blocks: &[B],
    indices: &[usize],
    region_bbox: [f32; 4],
) -> Option<(f32, Vec<usize>, Vec<usize>)>
where
    B: HasBBox,
{
    let region_height = region_bbox[3] - region_bbox[1];
    let region_y0 = region_bbox[1];

    // Minimum gap threshold: 3% of region height or 10 points, whichever is smaller
    let min_gap = (region_height * 0.03).min(10.0);

    // Create a projection histogram
    let y_start = region_bbox[1].floor() as i32;
    let y_end = region_bbox[3].ceil() as i32;
    let num_bins = (y_end - y_start) as usize;

    if num_bins == 0 {
        return None;
    }

    let mut coverage = vec![0u16; num_bins];

    for &idx in indices {
        let bbox = blocks[idx].bbox();
        let bin_start = (bbox[1].floor() as i32 - y_start).clamp(0, num_bins as i32 - 1) as usize;
        let bin_end = (bbox[3].ceil() as i32 - y_start).clamp(0, num_bins as i32) as usize;

        for bin in bin_start..bin_end.min(num_bins) {
            coverage[bin] = coverage[bin].saturating_add(1);
        }
    }

    // Find the widest contiguous gap
    let mut best_gap: Option<(f32, Vec<usize>, Vec<usize>)> = None;
    let mut max_gap_width = 0.0;
    let mut gap_start: Option<usize> = None;

    for (i, &count) in coverage.iter().enumerate() {
        if count == 0 {
            if gap_start.is_none() {
                gap_start = Some(i);
            }
        } else {
            if let Some(start) = gap_start {
                let gap_width = (i - start) as f32;
                let gap_y0 = region_y0 + start as f32;
                let gap_y1 = region_y0 + i as f32;

                if gap_width >= min_gap && gap_width > max_gap_width {
                    max_gap_width = gap_width;

                    let split_y = (gap_y0 + gap_y1) / 2.0;
                    let top: Vec<usize> = indices
                        .iter()
                        .copied()
                        .filter(|&idx| {
                            let bbox = blocks[idx].bbox();
                            bbox[1] >= split_y // y0 >= split (above)
                        })
                        .collect();
                    let bottom: Vec<usize> = indices
                        .iter()
                        .copied()
                        .filter(|&idx| {
                            let bbox = blocks[idx].bbox();
                            bbox[3] <= split_y // y1 <= split (below)
                        })
                        .collect();

                    if !top.is_empty() && !bottom.is_empty() {
                        best_gap = Some((split_y, top, bottom));
                    }
                }

                gap_start = None;
            }
        }
    }

    // Handle gap at the end
    if let Some(start) = gap_start {
        let gap_width = (num_bins - start) as f32;
        let gap_y0 = region_y0 + start as f32;
        let gap_y1 = region_y0 + num_bins as f32;

        if gap_width >= min_gap && gap_width > max_gap_width {
            let split_y = (gap_y0 + gap_y1) / 2.0;
            let top: Vec<usize> = indices
                .iter()
                .copied()
                .filter(|&idx| {
                    let bbox = blocks[idx].bbox();
                    bbox[1] >= split_y
                })
                .collect();
            let bottom: Vec<usize> = indices
                .iter()
                .copied()
                .filter(|&idx| {
                    let bbox = blocks[idx].bbox();
                    bbox[3] <= split_y
                })
                .collect();

            if !top.is_empty() && !bottom.is_empty() {
                best_gap = Some((split_y, top, bottom));
            }
        }
    }

    best_gap
}

/// Check if all blocks are in a single column (vertically stacked).
///
/// A single column means there's no vertical gap that has blocks on BOTH sides.
fn is_single_column(bboxes: &[[f32; 4]]) -> bool {
    if bboxes.len() <= 1 {
        return true;
    }

    // Check for vertical gaps that indicate multiple columns
    let mut x_coords: Vec<f32> = bboxes.iter().flat_map(|b| [b[0], b[2]]).collect();
    x_coords.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    x_coords.dedup();

    if x_coords.len() < 2 {
        return true;
    }

    // Check each gap for blocks on both sides
    for i in 0..x_coords.len().saturating_sub(1) {
        let gap_start = x_coords[i];
        let gap_end = x_coords[i + 1];
        let gap_mid = (gap_start + gap_end) / 2.0;

        // Count blocks on each side of the gap
        let left_count = bboxes.iter().filter(|b| b[2] < gap_mid).count();
        let right_count = bboxes.iter().filter(|b| b[0] > gap_mid).count();

        // If both sides have blocks, this is a multi-column layout
        if left_count > 0 && right_count > 0 {
            return false;
        }
    }

    // No gap with blocks on both sides -> single column
    true
}

/// Compute the union bbox of a collection of bboxes.
fn union_bboxes_from_coords(bboxes: &[[f32; 4]]) -> [f32; 4] {
    if bboxes.is_empty() {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let first = bboxes[0];
    let mut x0 = first[0];
    let mut y0 = first[1];
    let mut x1 = first[2];
    let mut y1 = first[3];

    for bbox in &bboxes[1..] {
        x0 = x0.min(bbox[0]);
        y0 = y0.min(bbox[1]);
        x1 = x1.max(bbox[2]);
        y1 = y1.max(bbox[3]);
    }

    [x0, y0, x1, y1]
}

/// Compute the union bbox of a collection of blocks.
fn union_bboxes<B>(blocks: &[B]) -> [f32; 4]
where
    B: HasBBox,
{
    let bboxes: Vec<[f32; 4]> = blocks.iter().map(|b| b.bbox()).collect();
    union_bboxes_from_coords(&bboxes)
}

/// Trait for types with a bounding box.
pub trait HasBBox {
    /// Get the bounding box [x0, y0, x1, y1] in PDF user space.
    fn bbox(&self) -> [f32; 4];
}

/// A simple block with bbox for testing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockWithBBox {
    /// Original index in the input array.
    pub index: usize,
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f32; 4],
}

impl BlockWithBBox {
    /// Create a new test block.
    pub fn new(index: usize, bbox: [f32; 4]) -> Self {
        Self { index, bbox }
    }
}

impl HasBBox for BlockWithBBox {
    fn bbox(&self) -> [f32; 4] {
        self.bbox
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xy_cut_empty() {
        let blocks: Vec<BlockWithBBox> = vec![];
        let result = xy_cut(&blocks, 612.0, 792.0);

        assert_eq!(result.order, Vec::<usize>::new());
        assert_eq!(result.region_count, 0);
        assert_eq!(result.small_region_count, 0);
    }

    #[test]
    fn test_xy_cut_single_block() {
        let blocks = vec![BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0])];
        let result = xy_cut(&blocks, 612.0, 792.0);

        assert_eq!(result.order, vec![0usize]);
        assert_eq!(result.region_count, 1);
        assert_eq!(result.small_region_count, 0);
    }

    #[test]
    fn test_xy_cut_single_column_top_to_bottom() {
        // Single column: 3 blocks stacked vertically
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]), // top
            BlockWithBBox::new(1, [50.0, 600.0, 250.0, 650.0]), // middle
            BlockWithBBox::new(2, [50.0, 500.0, 250.0, 550.0]), // bottom
        ];
        let result = xy_cut(&blocks, 612.0, 792.0);

        // Order: top to bottom (0, 1, 2)
        assert_eq!(result.order, vec![0usize, 1, 2]);
        assert_eq!(result.region_count, 1);
    }

    #[test]
    fn test_xy_cut_two_columns_left_then_right() {
        // Two-column page: 5 blocks each
        let blocks = vec![
            // Column 0 (left)
            BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]),
            BlockWithBBox::new(1, [50.0, 600.0, 250.0, 650.0]),
            BlockWithBBox::new(2, [50.0, 500.0, 250.0, 550.0]),
            BlockWithBBox::new(3, [50.0, 400.0, 250.0, 450.0]),
            BlockWithBBox::new(4, [50.0, 300.0, 250.0, 350.0]),
            // Column 1 (right)
            BlockWithBBox::new(5, [350.0, 700.0, 550.0, 750.0]),
            BlockWithBBox::new(6, [350.0, 600.0, 550.0, 650.0]),
            BlockWithBBox::new(7, [350.0, 500.0, 550.0, 550.0]),
            BlockWithBBox::new(8, [350.0, 400.0, 550.0, 450.0]),
            BlockWithBBox::new(9, [350.0, 300.0, 550.0, 350.0]),
        ];
        let result = xy_cut(&blocks, 612.0, 792.0);

        // Order: all col0 blocks (0-4), then all col1 blocks (5-9)
        // Within each column: top to bottom
        eprintln!("Result order: {:?}", result.order);
        eprintln!("Region count: {}", result.region_count);
        eprintln!("Small region count: {}", result.small_region_count);

        // Check that column 0 blocks come before column 1 blocks
        let col0_blocks: Vec<_> = result.order.iter().filter(|&&i| i < 5).collect();
        let col1_blocks: Vec<_> = result.order.iter().filter(|&&i| i >= 5).collect();
        assert_eq!(col0_blocks, vec![&0, &1, &2, &3, &4]);
        assert_eq!(col1_blocks, vec![&5, &6, &7, &8, &9]);

        // Combined order should be all col0 then all col1
        assert_eq!(result.order, vec![0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_xy_cut_three_columns() {
        // Three-column page: 3 blocks each
        let blocks = vec![
            // Column 0
            BlockWithBBox::new(0, [20.0, 700.0, 180.0, 750.0]),
            BlockWithBBox::new(1, [20.0, 600.0, 180.0, 650.0]),
            BlockWithBBox::new(2, [20.0, 500.0, 180.0, 550.0]),
            // Column 1
            BlockWithBBox::new(3, [200.0, 700.0, 380.0, 750.0]),
            BlockWithBBox::new(4, [200.0, 600.0, 380.0, 650.0]),
            BlockWithBBox::new(5, [200.0, 500.0, 380.0, 550.0]),
            // Column 2
            BlockWithBBox::new(6, [400.0, 700.0, 580.0, 750.0]),
            BlockWithBBox::new(7, [400.0, 600.0, 580.0, 650.0]),
            BlockWithBBox::new(8, [400.0, 500.0, 580.0, 550.0]),
        ];
        let result = xy_cut(&blocks, 612.0, 792.0);

        // Order: col0 (0-2), col1 (3-5), col2 (6-8)
        assert_eq!(result.order, vec![0usize, 1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_xy_cut_full_width_heading_then_two_columns() {
        // Full-width heading at top, then 2 columns below
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 720.0, 550.0, 770.0]), // full-width heading
            // Column 0
            BlockWithBBox::new(1, [50.0, 600.0, 250.0, 650.0]),
            BlockWithBBox::new(2, [50.0, 500.0, 250.0, 550.0]),
            // Column 1
            BlockWithBBox::new(3, [350.0, 600.0, 550.0, 650.0]),
            BlockWithBBox::new(4, [350.0, 500.0, 550.0, 550.0]),
        ];
        let result = xy_cut(&blocks, 612.0, 792.0);

        // Order: heading (0), then horizontal split, then left column (1,2), right column (3,4)
        // The heading spans full width, so no vertical split at top level
        // Horizontal split separates heading from columns
        // Then vertical split separates columns
        assert_eq!(result.order, vec![0usize, 1, 2, 3, 4]);
    }

    #[test]
    fn test_xy_cut_small_region_count() {
        // Create many small regions to trigger Docstrum signal
        // 14 blocks in 7 columns x 2 rows (each region has 2 blocks < MIN_BLOCKS_PER_REGION)
        let blocks: Vec<BlockWithBBox> = (0..14)
            .map(|i| {
                let x = (i % 7) as f32 * 70.0 + 20.0; // 7 columns
                let y = (i / 7) as f32 * 150.0 + 500.0; // 2 rows
                BlockWithBBox::new(i, [x, y, x + 50.0, y + 50.0])
            })
            .collect();

        let result = xy_cut(&blocks, 612.0, 792.0);

        // With scattered blocks, XY-cut should produce many small regions
        assert!(result.region_count >= 4);
        // Each region has 2 blocks (< 3), so small_region_count should be high
        assert!(result.small_region_count > 0);
    }

    #[test]
    fn test_find_vertical_split_two_columns() {
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]),
            BlockWithBBox::new(1, [350.0, 700.0, 550.0, 750.0]),
        ];

        let indices = vec![0, 1];
        let region_bbox = [50.0, 700.0, 550.0, 750.0];

        let result = find_vertical_split(&blocks, &indices, region_bbox);

        assert!(result.is_some());
        let (split_x, left, right) = result.unwrap();
        // Split should be between the columns
        assert!(split_x > 250.0 && split_x < 350.0);
        assert_eq!(left, vec![0]);
        assert_eq!(right, vec![1]);
    }

    #[test]
    fn test_find_vertical_split_no_gap() {
        // Blocks with no gap between them
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]),
            BlockWithBBox::new(1, [250.0, 700.0, 450.0, 750.0]), // touches first block
        ];

        let indices = vec![0, 1];
        let region_bbox = [50.0, 700.0, 450.0, 750.0];

        let result = find_vertical_split(&blocks, &indices, region_bbox);

        // No valid gap (blocks touch)
        assert!(result.is_none());
    }

    #[test]
    fn test_find_horizontal_split_top_bottom() {
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]), // top
            BlockWithBBox::new(1, [50.0, 500.0, 250.0, 550.0]), // bottom
        ];

        let indices = vec![0, 1];
        let region_bbox = [50.0, 500.0, 250.0, 750.0];

        let result = find_horizontal_split(&blocks, &indices, region_bbox);

        assert!(result.is_some());
        let (split_y, top, bottom) = result.unwrap();
        // Split should be between the blocks
        assert!(split_y > 550.0 && split_y < 700.0);
        assert_eq!(top, vec![0]);
        assert_eq!(bottom, vec![1]);
    }

    #[test]
    fn test_union_bboxes() {
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]),
            BlockWithBBox::new(1, [100.0, 600.0, 300.0, 650.0]),
        ];

        let union = union_bboxes(&blocks);
        assert_eq!(union[0], 50.0); // min x0
        assert_eq!(union[1], 600.0); // min y0
        assert_eq!(union[2], 300.0); // max x1
        assert_eq!(union[3], 750.0); // max y1
    }

    #[test]
    fn test_block_with_bbox_bbox() {
        let block = BlockWithBBox::new(0, [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(block.bbox(), [10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn test_xy_cut_result_docstrum_trigger() {
        // When region_count is high and small_region_count is high,
        // caller should switch to Docstrum
        // 20 blocks in 10 columns x 2 rows (each region has 2 blocks)
        let blocks: Vec<BlockWithBBox> = (0..20)
            .map(|i| {
                let x = (i % 10) as f32 * 50.0 + 20.0; // 10 columns
                let y = (i / 10) as f32 * 150.0 + 500.0; // 2 rows
                BlockWithBBox::new(i, [x, y, x + 35.0, y + 50.0])
            })
            .collect();

        let result = xy_cut(&blocks, 612.0, 792.0);

        // Check that result contains trigger info
        assert!(result.region_count >= 5);
        // Each region has 2 blocks (< 3), so small_region_count should be significant
        assert_eq!(result.small_region_count, result.region_count);
    }
}
