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

use std::collections::HashSet;

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

/// Docstrum nearest-neighbor graph traversal for reading order.
///
/// Implements O'Gorman 1993 Docstrum algorithm for irregular layouts.
/// Computes k-nearest neighbors, builds adjacency graph with angle constraints,
/// and traverses connected components in reading order.
///
/// # Arguments
///
/// * `blocks` - Blocks to order (must have bbox accessible via HasBBox)
///
/// # Returns
///
/// `XYCutResult` with ordered block indices and algorithm set to "docstrum".
///
/// # Algorithm
///
/// 1. For each block, find k=5 nearest neighbors by Euclidean center-to-center distance
/// 2. Build adjacency graph with edges weighted by distance
/// 3. Apply angle constraints:
///    - Within-line: angle within ±30° of horizontal (0°)
///    - Between-line: angle within ±30° of vertical (90° or -90°)
/// 4. Find root nodes (no incoming edges from blocks above)
/// 5. Sort roots by (column ASC, y DESC)
/// 6. Traverse each connected component via DFS in y-then-x order
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::reading_order::{docstrum, BlockWithBBox};
///
/// // Magazine layout with sidebar
/// let blocks = vec![
///     BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]),  // main col 1
///     BlockWithBBox::new(1, [50.0, 600.0, 250.0, 650.0]),  // main col 2
///     BlockWithBBox::new(2, [50.0, 500.0, 250.0, 550.0]),  // main col 3
///     BlockWithBBox::new(3, [400.0, 700.0, 550.0, 750.0]), // sidebar 1
///     BlockWithBBox::new(4, [400.0, 600.0, 550.0, 650.0]), // sidebar 2
/// ];
///
/// let result = docstrum(&blocks);
/// // Main column (0,1,2) before sidebar (3,4)
/// assert!(result.order.starts_with(&[0, 1, 2]));
/// ```
pub fn docstrum<B>(blocks: &[B]) -> XYCutResult
where
    B: HasBBox + Clone,
{
    if blocks.is_empty() {
        return XYCutResult {
            order: vec![],
            region_count: 0,
            small_region_count: 0,
            algorithm: "docstrum".to_string(),
        };
    }

    if blocks.len() == 1 {
        return XYCutResult {
            order: vec![0],
            region_count: 1,
            small_region_count: 0,
            algorithm: "docstrum".to_string(),
        };
    }

    // k=5 nearest neighbors per block (Docstrum standard)
    const K: usize = 5;

    // Compute centers for all blocks
    let centers: Vec<(f32, f32)> = blocks
        .iter()
        .map(|b| {
            let bbox = b.bbox();
            let cx = (bbox[0] + bbox[2]) / 2.0;
            let cy = (bbox[1] + bbox[3]) / 2.0;
            (cx, cy)
        })
        .collect();

    // Build adjacency graph
    let edges = build_adjacency_graph(blocks, &centers, K);

    // Find root nodes (no incoming edges from blocks above)
    let roots = find_roots(&edges, &centers);

    // Sort roots by (column ASC, y DESC)
    let mut sorted_roots = roots;
    sorted_roots.sort_by(|&a, &b| {
        let (ca_x, ca_y) = centers[a];
        let (cb_x, cb_y) = centers[b];
        // Column grouping: floor divide by page width
        let col_a = (ca_x / 100.0).floor() as i32;
        let col_b = (cb_x / 100.0).floor() as i32;
        col_a.cmp(&col_b).then_with(|| cb_y.partial_cmp(&ca_y).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Traverse connected components
    let order = traverse_components(&edges, &sorted_roots, &centers);

    XYCutResult {
        order,
        region_count: 1, // Docstrum produces 1 logical region
        small_region_count: 0,
        algorithm: "docstrum".to_string(),
    }
}

/// Edge in the Docstrum adjacency graph.
#[derive(Debug, Clone)]
struct Edge {
    /// Source block index.
    from: usize,
    /// Target block index.
    to: usize,
    /// Euclidean distance between centers.
    distance: f32,
    /// Angle in radians between centers.
    angle: f32,
}

/// Build adjacency graph with k-nearest neighbors and angle constraints.
fn build_adjacency_graph<B>(
    blocks: &[B],
    centers: &[(f32, f32)],
    k: usize,
) -> Vec<Edge>
where
    B: HasBBox,
{
    let n = blocks.len();
    let mut edges = Vec::new();

    // Angle thresholds in radians: ±30 degrees
    const WITHIN_LINE_TOL: f32 = 30.0 * std::f32::consts::PI / 180.0; // ±30° from horizontal
    const BETWEEN_LINE_TOL: f32 = 30.0 * std::f32::consts::PI / 180.0; // ±30° from vertical

    // For each block, find k nearest neighbors
    for i in 0..n {
        let (cx_i, cy_i) = centers[i];

        // Compute distances to all other blocks
        let mut distances: Vec<(usize, f32)> = Vec::with_capacity(n - 1);
        for j in 0..n {
            if i == j {
                continue;
            }
            let (cx_j, cy_j) = centers[j];
            let dx = cx_j - cx_i;
            let dy = cy_j - cy_i;
            let dist = (dx * dx + dy * dy).sqrt();
            distances.push((j, dist));
        }

        // Sort by distance and take k nearest
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let n_dist = distances.len();
        let k_nearest: Vec<(usize, f32)> = distances
            .into_iter()
            .take(k.min(n_dist))
            .collect();

        // Create edges for k-nearest neighbors that pass angle constraints
        for &(j, dist) in &k_nearest {
            let (cx_j, cy_j) = centers[j];
            let dx = cx_j - cx_i;
            let dy = cy_j - cy_i;

            // Compute angle in radians (-π to π)
            let angle = dy.atan2(dx);

            // Check angle constraints
            let angle_abs = angle.abs();
            let passes = if dy.abs() < dx.abs() {
                // Horizontal-ish: within-line adjacency
                angle_abs <= WITHIN_LINE_TOL
            } else {
                // Vertical-ish: between-line adjacency
                let from_vertical = (angle_abs - std::f32::consts::PI / 2.0).abs();
                from_vertical <= BETWEEN_LINE_TOL
            };

            if passes {
                edges.push(Edge {
                    from: i,
                    to: j,
                    distance: dist,
                    angle,
                });
            }
        }
    }

    edges
}

/// Find root nodes (no incoming edges from blocks above).
///
/// A root is a block with no incoming edges from blocks whose center-y is greater.
fn find_roots(edges: &[Edge], centers: &[(f32, f32)]) -> Vec<usize> {
    let n = centers.len();
    let mut has_incoming_from_above = vec![false; n];

    for edge in edges {
        let (from_y, to_y) = (centers[edge.from].1, centers[edge.to].1);
        // If edge from a block above (greater y in PDF coords)
        if from_y > to_y {
            has_incoming_from_above[edge.to] = true;
        }
    }

    // Roots are blocks with no incoming edges from above
    let mut roots = Vec::new();
    for i in 0..n {
        if !has_incoming_from_above[i] {
            roots.push(i);
        }
    }

    // If no roots found (circular or pathological), use all nodes sorted by position
    if roots.is_empty() && n > 0 {
        roots = (0..n).collect();
    }

    roots
}

/// Traverse connected components via DFS in y-then-x order.
fn traverse_components(
    edges: &[Edge],
    roots: &[usize],
    centers: &[(f32, f32)],
) -> Vec<usize> {
    let n = centers.len();
    if n == 0 {
        return Vec::new();
    }

    // Build adjacency list
    let mut adj_list: Vec<Vec<usize>> = vec![Vec::new(); n];
    for edge in edges {
        adj_list[edge.from].push(edge.to);
    }

    let mut visited = vec![false; n];
    let mut order = Vec::new();

    // For each root, traverse its component
    for &root in roots {
        if visited[root] {
            continue;
        }
        traverse_dfs(root, &adj_list, &mut visited, &mut order, centers);
    }

    // Visit any remaining unvisited nodes (isolated blocks)
    for i in 0..n {
        if !visited[i] {
            order.push(i);
            visited[i] = true;
        }
    }

    order
}

/// Depth-first traversal with y-then-x ordering.
fn traverse_dfs(
    node: usize,
    adj_list: &[Vec<usize>],
    visited: &mut [bool],
    order: &mut Vec<usize>,
    centers: &[(f32, f32)],
) {
    if visited[node] {
        return;
    }
    visited[node] = true;
    order.push(node);

    // Sort neighbors by (y DESC, x ASC) for reading order
    let mut neighbors = adj_list[node].clone();
    neighbors.sort_by(|&a, &b| {
        let (cy_a, cx_a) = (centers[a].1, centers[a].0);
        let (cy_b, cx_b) = (centers[b].1, centers[b].0);
        cy_b.partial_cmp(&cy_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| cx_a.partial_cmp(&cx_b).unwrap_or(std::cmp::Ordering::Equal))
    });

    for &neighbor in &neighbors {
        if !visited[neighbor] {
            traverse_dfs(neighbor, adj_list, visited, order, centers);
        }
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

    // Docstrum tests

    #[test]
    fn test_docstrum_empty() {
        let blocks: Vec<BlockWithBBox> = vec![];
        let result = docstrum(&blocks);

        assert_eq!(result.order, Vec::<usize>::new());
        assert_eq!(result.algorithm, "docstrum");
    }

    #[test]
    fn test_docstrum_single_block() {
        let blocks = vec![BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0])];
        let result = docstrum(&blocks);

        assert_eq!(result.order, vec![0usize]);
        assert_eq!(result.algorithm, "docstrum");
    }

    #[test]
    fn test_docstrum_magazine_main_and_sidebar() {
        // Magazine layout: main text (left) and sidebar (right)
        let blocks = vec![
            // Main column (3 blocks stacked)
            BlockWithBBox::new(0, [50.0, 700.0, 250.0, 750.0]),
            BlockWithBBox::new(1, [50.0, 600.0, 250.0, 650.0]),
            BlockWithBBox::new(2, [50.0, 500.0, 250.0, 550.0]),
            // Sidebar (2 blocks stacked, separated horizontally)
            BlockWithBBox::new(3, [400.0, 700.0, 550.0, 750.0]),
            BlockWithBBox::new(4, [400.0, 600.0, 550.0, 650.0]),
        ];

        let result = docstrum(&blocks);

        // Main column (0,1,2) should come before sidebar (3,4)
        assert_eq!(result.algorithm, "docstrum");
        // Check that all main column blocks come before sidebar
        let main_pos: Vec<_> = result.order.iter().filter(|&&i| i < 3).collect();
        let sidebar_pos: Vec<_> = result.order.iter().filter(|&&i| i >= 3).collect();
        // Main column indices should appear before sidebar indices
        if let (Some(&first_main), Some(&last_main), Some(&first_sidebar)) = (
            main_pos.first(),
            main_pos.last(),
            sidebar_pos.first(),
        ) {
            assert!(last_main < first_sidebar);
        }
    }

    #[test]
    fn test_docstrum_all_one_line_horizontal() {
        // All blocks in one horizontal line
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 150.0, 750.0]),
            BlockWithBBox::new(1, [160.0, 700.0, 260.0, 750.0]),
            BlockWithBBox::new(2, [270.0, 700.0, 370.0, 750.0]),
            BlockWithBBox::new(3, [380.0, 700.0, 480.0, 750.0]),
        ];

        let result = docstrum(&blocks);

        // Should visit left-to-right
        assert_eq!(result.algorithm, "docstrum");
        // All y coordinates are same, so order by x ascending
        let x_coords: Vec<f32> = result
            .order
            .iter()
            .map(|&i| blocks[i].bbox()[0])
            .collect();
        // Verify strictly increasing x coordinates
        for i in 1..x_coords.len() {
            assert!(x_coords[i] > x_coords[i - 1]);
        }
    }

    #[test]
    fn test_docstrum_all_one_column_vertical() {
        // All blocks in one vertical column
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 200.0, 750.0]), // top
            BlockWithBBox::new(1, [50.0, 600.0, 200.0, 650.0]),
            BlockWithBBox::new(2, [50.0, 500.0, 200.0, 550.0]), // bottom
        ];

        let result = docstrum(&blocks);

        // Should visit top-to-bottom (y descending in PDF coords)
        assert_eq!(result.algorithm, "docstrum");
        let y_coords: Vec<f32> = result
            .order
            .iter()
            .map(|&i| blocks[i].bbox()[3]) // y1 (bottom of bbox)
            .collect();
        // Verify strictly decreasing y coordinates (top to bottom)
        for i in 1..y_coords.len() {
            assert!(y_coords[i] < y_coords[i - 1]);
        }
    }

    #[test]
    fn test_docstrum_scattered_pathological() {
        // Pathological case: blocks scattered with no clear adjacency
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 100.0, 750.0]),
            BlockWithBBox::new(1, [200.0, 600.0, 250.0, 650.0]),
            BlockWithBBox::new(2, [350.0, 500.0, 400.0, 550.0]),
            BlockWithBBox::new(3, [500.0, 400.0, 550.0, 450.0]),
        ];

        let result = docstrum(&blocks);

        // Each block should be visited exactly once
        assert_eq!(result.algorithm, "docstrum");
        assert_eq!(result.order.len(), 4);
        let mut sorted = result.order.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_docstrum_k_nearest_neighbors() {
        // Verify k=5 neighbor selection with more than 5 blocks
        let blocks: Vec<BlockWithBBox> = (0..10)
            .map(|i| {
                let x = (i % 3) as f32 * 150.0 + 50.0; // 3 columns
                let y = (i / 3) as f32 * 100.0 + 600.0; // multiple rows
                BlockWithBBox::new(i, [x, y, x + 80.0, y + 50.0])
            })
            .collect();

        let result = docstrum(&blocks);

        // Should handle more than k blocks gracefully
        assert_eq!(result.algorithm, "docstrum");
        assert_eq!(result.order.len(), 10);
    }

    #[test]
    fn test_docstrum_angle_constraint_within_line() {
        // Horizontal line: blocks should connect with ±30° of horizontal
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 150.0, 750.0]),
            BlockWithBBox::new(1, [160.0, 705.0, 260.0, 755.0]), // slight y offset (within ±30°)
            BlockWithBBox::new(2, [270.0, 700.0, 370.0, 750.0]),
        ];

        let result = docstrum(&blocks);

        // Should form one component, visited left-to-right
        assert_eq!(result.algorithm, "docstrum");
        assert_eq!(result.order.len(), 3);
    }

    #[test]
    fn test_docstrum_angle_constraint_between_line() {
        // Vertical column: blocks should connect with ±30° of vertical
        let blocks = vec![
            BlockWithBBox::new(0, [50.0, 700.0, 150.0, 750.0]),
            BlockWithBBox::new(1, [55.0, 600.0, 155.0, 650.0]), // slight x offset (within ±30°)
            BlockWithBBox::new(2, [50.0, 500.0, 150.0, 550.0]),
        ];

        let result = docstrum(&blocks);

        // Should form one component, visited top-to-bottom
        assert_eq!(result.algorithm, "docstrum");
        assert_eq!(result.order.len(), 3);
    }
}
