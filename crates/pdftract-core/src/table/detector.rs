//! Line-based table detector.
//!
//! Extracts tables by analyzing path segments (horizontal and vertical lines)
//! from PDF content streams and reconstructing grid structures.

use super::{PageContext, GridCandidate, Segment, SegmentOrientation};
use crate::parser::lexer::Lexer;
use std::collections::{HashMap, HashSet};

/// Tolerance for x0 alignment in borderless detection (2.0 pt).
const X0_TOLERANCE: f32 = 2.0;

/// Minimum number of spans per column candidate.
const MIN_SPANS_PER_COLUMN: usize = 3;

/// Minimum number of columns for a valid table.
const MIN_COLUMNS: usize = 3;

/// Minimum number of rows for a valid table.
const MIN_ROWS: usize = 3;

/// Maximum vertical gap between rows (100 pt).
const MAX_VERTICAL_GAP: f32 = 100.0;

/// A text position extracted from the content stream.
#[derive(Debug, Clone, Copy)]
struct TextPosition {
    /// X coordinate of the text origin.
    x0: f32,
    /// Y coordinate of the text origin.
    y0: f32,
}

/// Epsilon tolerance for collinearity detection (1.0 pt).
const EPSILON: f32 = 1.0;

/// Gap tolerance for merging collinear segments (2.0 pt).
const GAP_TOLERANCE: f32 = 2.0;

/// Line-based table detector.
///
/// Detects bordered tables by:
/// 1. Collecting horizontal/vertical path segments from stroke operators
/// 2. Clustering collinear segments
/// 3. Finding intersection points
/// 4. Building candidate grids
pub struct TableDetector {
    /// Minimum number of cells for a valid grid.
    min_cells: usize,
    /// Whether to filter out segments inside text objects (BT..ET).
    filter_text_objects: bool,
}

impl Default for TableDetector {
    fn default() -> Self {
        Self {
            min_cells: 4,
            filter_text_objects: true,
        }
    }
}

impl TableDetector {
    /// Create a new table detector with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the minimum cell count for valid grids.
    pub fn with_min_cells(mut self, min_cells: usize) -> Self {
        self.min_cells = min_cells;
        self
    }

    /// Set whether to filter segments inside text objects.
    pub fn with_text_object_filtering(mut self, filter: bool) -> Self {
        self.filter_text_objects = filter;
        self
    }

    /// Detect tables on a page using line-based detection.
    ///
    /// This is the main entry point for bordered table detection.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The page context containing page dict and content bytes
    ///
    /// # Returns
    ///
    /// A vector of grid candidates representing detected tables.
    pub fn detect_line_based(&self, ctx: &PageContext) -> Vec<GridCandidate> {
        // Step 1: Collect path segments from content stream
        let segments = self.collect_segments(ctx);

        // Step 2: Cluster collinear segments
        let horizontal_clusters = self.cluster_segments(&segments, SegmentOrientation::Horizontal);
        let vertical_clusters = self.cluster_segments(&segments, SegmentOrientation::Vertical);

        // Step 3: Find intersections
        let intersections = self.find_intersections(&horizontal_clusters, &vertical_clusters);

        // Step 4: Build grids from intersections
        self.build_grids(intersections, segments)
    }

    /// Detect borderless tables using x0 alignment heuristic.
    ///
    /// This method analyzes text positioning to find tables without ruling lines:
    /// 1. Collect text positions from content stream
    /// 2. Group by x0 positions (within tolerance)
    /// 3. Find column candidates (3+ spans at same x0)
    /// 4. Find row candidates (y positions with multiple columns)
    /// 5. Validate and build grid candidates
    ///
    /// # Arguments
    ///
    /// * `ctx` - The page context containing page dict and content bytes
    ///
    /// # Returns
    ///
    /// A vector of grid candidates representing detected borderless tables.
    pub fn detect_borderless(&self, ctx: &PageContext) -> Vec<GridCandidate> {
        // Step 1: Collect text positions from content stream
        let text_positions = self.collect_text_positions(ctx);

        if text_positions.is_empty() {
            return Vec::new();
        }

        // Step 2: Group by x0 positions (within tolerance)
        let column_buckets = self.group_by_x0(&text_positions);

        // Step 3: Find column candidates (3+ spans at same x0)
        let column_candidates: Vec<_> = column_buckets
            .into_iter()
            .filter(|(_, positions)| positions.len() >= MIN_SPANS_PER_COLUMN)
            .collect();

        if column_candidates.len() < MIN_COLUMNS {
            return Vec::new();
        }

        // Step 4: Find row candidates
        let row_candidates = self.find_row_candidates(&column_candidates);

        if row_candidates.len() < MIN_ROWS {
            return Vec::new();
        }

        // Step 5: Build grid from candidates
        self.build_borderless_grid(&column_candidates, &row_candidates, &text_positions)
    }

    /// Collect text positions from the content stream.
    ///
    /// Parses Tm, Td, TD, T*, Tj, TJ, ', " operators to track text positions.
    fn collect_text_positions(&self, ctx: &PageContext) -> Vec<TextPosition> {
        let mut positions = Vec::new();
        let mut lexer = Lexer::new(ctx.content_bytes);

        let mut operand_stack: Vec<f32> = Vec::new();

        // Current text matrix (Tm) and line matrix (Tlm)
        let mut tm: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // Identity matrix
        let mut tlm: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut in_text_block = false;

        while let Some(token) = lexer.next_token() {
            match token {
                crate::parser::lexer::Token::Integer(n) => {
                    operand_stack.push(n as f32);
                }
                crate::parser::lexer::Token::Real(r) => {
                    operand_stack.push(r as f32);
                }
                crate::parser::lexer::Token::Keyword(ref op) => {
                    match op.as_slice() {
                        b"BT" => {
                            // Begin text block
                            in_text_block = true;
                            // Reset Tm and Tlm to identity
                            tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                            tlm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                        }
                        b"ET" => {
                            // End text block
                            in_text_block = false;
                        }
                        b"Tm" => {
                            // Set text matrix: Tm (operands: a b c d e f)
                            if operand_stack.len() >= 6 {
                                for i in 0..6 {
                                    tm[i] = operand_stack[operand_stack.len() - 6 + i];
                                }
                                operand_stack.truncate(operand_stack.len() - 6);
                                tlm = tm; // Tm also sets Tlm
                            }
                        }
                        b"Td" => {
                            // Move text position: Td (tx ty)
                            if operand_stack.len() >= 2 {
                                let ty = operand_stack.pop().unwrap();
                                let tx = operand_stack.pop().unwrap();
                                // Td: Tm = Tlm * [1 0 0 1 tx ty]
                                tm[0] = tlm[0];
                                tm[1] = tlm[1];
                                tm[2] = tlm[2];
                                tm[3] = tlm[3];
                                tm[4] = tlm[0] * tx + tlm[2] * ty + tlm[4];
                                tm[5] = tlm[1] * tx + tlm[3] * ty + tlm[5];
                                tlm = tm; // Td also updates Tlm to the new Tm
                            }
                        }
                        b"TD" => {
                            // Move text position and set leading: TD (tx ty)
                            if operand_stack.len() >= 2 {
                                let ty = operand_stack.pop().unwrap();
                                let tx = operand_stack.pop().unwrap();
                                // TD: Tl = -ty, then Td
                                // For position tracking, same as Td
                                tm[0] = tlm[0];
                                tm[1] = tlm[1];
                                tm[2] = tlm[2];
                                tm[3] = tlm[3];
                                tm[4] = tlm[0] * tx + tlm[2] * ty + tlm[4];
                                tm[5] = tlm[1] * tx + tlm[3] * ty + tlm[5];
                                tlm = tm;
                            }
                        }
                        b"T*" => {
                            // Move to start of next line
                            // T*: Td (0 Tl)
                            // Tm[4] = Tlm[4], Tm[5] = Tlm[5] - Tl
                            // We don't track Tl, so approximate by using current y
                            tm[4] = tlm[4];
                            tm[5] = tlm[5]; // This is approximate; would need Tl for exact
                            tlm = tm;
                        }
                        b"Tj" => {
                            // Show text: Tj (string)
                            if in_text_block {
                                // Record position at current text origin
                                positions.push(TextPosition { x0: tm[4], y0: tm[5] });
                            }
                            operand_stack.clear(); // Tj consumes the string operand
                        }
                        b"TJ" => {
                            // Show text with individual glyph positioning: TJ (array)
                            if in_text_block {
                                // Record position
                                positions.push(TextPosition { x0: tm[4], y0: tm[5] });
                            }
                            operand_stack.clear(); // TJ consumes the array operand
                        }
                        b"'" => {
                            // Move to next line and show text: ' (string)
                            if in_text_block {
                                tm[4] = tlm[4];
                                tm[5] = tlm[5]; // Approximate
                                tlm = tm;
                                positions.push(TextPosition { x0: tm[4], y0: tm[5] });
                            }
                            operand_stack.clear();
                        }
                        b"\"" => {
                            // Set word and character spacing, move to next line, show text
                            // " (tw tc s) -> we just track position
                            if in_text_block && operand_stack.len() >= 3 {
                                operand_stack.truncate(operand_stack.len() - 3);
                                tm[4] = tlm[4];
                                tm[5] = tlm[5]; // Approximate
                                tlm = tm;
                                positions.push(TextPosition { x0: tm[4], y0: tm[5] });
                            }
                        }
                        _ => {
                            // Other operators - clear operand stack
                            operand_stack.clear();
                        }
                    }
                }
                _ => {
                    // Other tokens - ignore
                }
            }
        }

        positions
    }

    /// Group text positions by x0 coordinate within tolerance.
    ///
    /// Uses clustering: positions are grouped if their x0 values are within
    /// X0_TOLERANCE of each other. This is more accurate than fixed-width
    /// bucketing for detecting aligned columns.
    fn group_by_x0(&self, positions: &[TextPosition]) -> HashMap<i32, Vec<TextPosition>> {
        if positions.is_empty() {
            return HashMap::new();
        }

        let mut sorted_positions = positions.to_vec();
        sorted_positions.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));

        let mut clusters: Vec<Vec<TextPosition>> = Vec::new();
        let mut current_cluster = vec![sorted_positions[0]];

        for pos in &sorted_positions[1..] {
            if (pos.x0 - current_cluster[0].x0).abs() <= X0_TOLERANCE {
                // Within tolerance of cluster center, add to current cluster
                current_cluster.push(*pos);
            } else {
                // Start new cluster
                clusters.push(current_cluster);
                current_cluster = vec![*pos];
            }
        }
        clusters.push(current_cluster);

        // Convert to HashMap with sequential keys
        let mut buckets: HashMap<i32, Vec<TextPosition>> = HashMap::new();
        for (i, cluster) in clusters.into_iter().enumerate() {
            buckets.insert(i as i32, cluster);
        }

        buckets
    }

    /// Find row candidates from column buckets.
    ///
    /// A row candidate is a y position where >= 2 column candidates have spans.
    fn find_row_candidates(&self, column_buckets: &[(i32, Vec<TextPosition>)]) -> Vec<f32> {
        // Build a map of y positions to column count
        let mut y_to_column_count: HashMap<i32, HashSet<i32>> = HashMap::new();

        for &(key, ref positions) in column_buckets {
            for pos in positions {
                // Round y to nearest integer for grouping (same tolerance as x0)
                let y_key = (pos.y0 / X0_TOLERANCE).round() as i32;
                y_to_column_count
                    .entry(y_key)
                    .or_insert_with(HashSet::new)
                    .insert(key);
            }
        }

        // Extract y positions that have multiple columns
        let mut row_ys: Vec<f32> = y_to_column_count
            .into_iter()
            .filter(|(_, cols)| cols.len() >= 2)
            .map(|(y_key, _)| (y_key as f32) * X0_TOLERANCE)
            .collect();

        // Sort descending (PDF y increases upward)
        row_ys.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        row_ys
    }

    /// Build a borderless grid from column and row candidates.
    fn build_borderless_grid(
        &self,
        column_buckets: &[(i32, Vec<TextPosition>)],
        row_ys: &[f32],
        all_positions: &[TextPosition],
    ) -> Vec<GridCandidate> {
        if row_ys.is_empty() || column_buckets.is_empty() {
            return Vec::new();
        }

        // Find contiguous y ranges (no gap > MAX_VERTICAL_GAP)
        let mut y_ranges = Vec::new();
        let mut current_range_start = row_ys[0];
        let mut current_range_end = row_ys[0];

        for &y in row_ys.iter().skip(1) {
            if (current_range_end - y).abs() <= MAX_VERTICAL_GAP {
                // Extend current range
                current_range_end = y.min(current_range_end);
            } else {
                // Start new range
                y_ranges.push((current_range_start, current_range_end));
                current_range_start = y;
                current_range_end = y;
            }
        }
        y_ranges.push((current_range_start, current_range_end));

        // Build grid for each y range
        let mut grids = Vec::new();
        for (y_top, y_bottom) in y_ranges {
            if let Some(grid) = self.build_single_borderless_grid(column_buckets, y_top, y_bottom, all_positions) {
                grids.push(grid);
            }
        }

        grids
    }

    /// Build a single borderless grid for a specific y range.
    fn build_single_borderless_grid(
        &self,
        column_buckets: &[(i32, Vec<TextPosition>)],
        y_top: f32,
        y_bottom: f32,
        all_positions: &[TextPosition],
    ) -> Option<GridCandidate> {
        // Get sorted column x positions
        let mut col_xs: Vec<f32> = column_buckets
            .iter()
            .map(|(key, _)| (*key as f32) * X0_TOLERANCE)
            .collect();
        col_xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Filter rows to within y range (use integer keys for deduplication)
        let row_ys: Vec<f32> = all_positions
            .iter()
            .map(|p| p.y0)
            .filter(|&y| y <= y_top && y >= y_bottom)
            .map(|y| (y / X0_TOLERANCE).round() as i32)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .map(|y_key| (y_key as f32) * X0_TOLERANCE)
            .collect::<Vec<_>>();

        let mut row_ys_sorted = row_ys.clone();
        row_ys_sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        if row_ys_sorted.len() < MIN_ROWS || col_xs.len() < MIN_COLUMNS {
            return None;
        }

        // Compute bounding box
        let x0 = col_xs.first().copied()?;
        let x1 = col_xs.last().copied()?;
        let y0 = row_ys_sorted.last().copied()?; // Bottom
        let y1 = row_ys_sorted.first().copied()?; // Top

        // Reject if spans suggest single-column paragraph reflow
        if self.is_single_column_reflow(column_buckets) {
            return None;
        }

        let bbox = [x0, y0, x1, y1];

        Some(GridCandidate {
            bbox,
            row_ys: row_ys_sorted,
            col_xs,
            segments: Vec::new(), // No segments for borderless tables
            header_rows: 0, // Initialized to 0; set after header detection
        })
    }

    /// Check if the pattern suggests single-column paragraph reflow.
    ///
    /// Returns true if any column candidate's spans are all on consecutive
    /// lines without aligned neighbors in any other column candidate.
    fn is_single_column_reflow(&self, column_buckets: &[(i32, Vec<TextPosition>)]) -> bool {
        // Build a map of y positions to column keys
        let mut y_to_columns: HashMap<i32, Vec<i32>> = HashMap::new();
        for &(key, ref positions) in column_buckets {
            for pos in positions {
                let y_key = (pos.y0 / X0_TOLERANCE).round() as i32;
                y_to_columns
                    .entry(y_key)
                    .or_insert_with(Vec::new)
                    .push(key);
            }
        }

        // For each column, check if its y positions lack multi-column alignment
        for &(_key, ref positions) in column_buckets {
            let mut aligned_count = 0;
            for pos in positions {
                let y_key = (pos.y0 / X0_TOLERANCE).round() as i32;
                if let Some(cols) = y_to_columns.get(&y_key) {
                    if cols.len() >= 2 {
                        aligned_count += 1;
                    }
                }
            }
            // If most spans in this column are not aligned with other columns, reject
            // "Most" means more than half, so we check if aligned_count * 2 < positions.len()
            if aligned_count * 2 < positions.len() {
                return true;
            }
        }

        false
    }

    /// Collect horizontal and vertical path segments from content stream.
    fn collect_segments(&self, ctx: &PageContext) -> Vec<Segment> {
        let mut segments = Vec::new();
        let mut lexer = Lexer::new(ctx.content_bytes);

        // PDF uses postfix notation: operands come before operators
        // We maintain an operand stack
        let mut operand_stack: Vec<f32> = Vec::new();

        // Track path construction state
        let mut current_point: Option<(f32, f32)> = None;
        let mut in_text_object = false;

        while let Some(token) = lexer.next_token() {
            match token {
                crate::parser::lexer::Token::Integer(n) => {
                    operand_stack.push(n as f32);
                }
                crate::parser::lexer::Token::Real(r) => {
                    operand_stack.push(r as f32);
                }
                crate::parser::lexer::Token::Keyword(ref op) => {
                    match op.as_slice() {
                        b"BT" => {
                            // Begin text object - subsequent path ops are glyph outlines
                            in_text_object = true;
                        }
                        b"ET" => {
                            // End text object
                            in_text_object = false;
                        }
                        b"m" => {
                            // moveto - pops x y from stack
                            if operand_stack.len() >= 2 {
                                let y = operand_stack.pop().unwrap();
                                let x = operand_stack.pop().unwrap();
                                current_point = Some((x, y));
                            }
                        }
                        b"l" => {
                            // lineto - pops x y from stack, draws line from current point
                            if operand_stack.len() >= 2 {
                                let y = operand_stack.pop().unwrap();
                                let x = operand_stack.pop().unwrap();
                                if let Some((x0, y0)) = current_point {
                                    if !in_text_object || !self.filter_text_objects {
                                        if let Some(seg) = Segment::new(x0, y0, x, y, EPSILON) {
                                            segments.push(seg);
                                        }
                                    }
                                }
                                current_point = Some((x, y));
                            }
                        }
                        b"re" => {
                            // rectangle - pops x y w h from stack
                            // The 're' operator implicitly starts a new subpath
                            if operand_stack.len() >= 4 {
                                let h = operand_stack.pop().unwrap();
                                let w = operand_stack.pop().unwrap();
                                let y = operand_stack.pop().unwrap();
                                let x = operand_stack.pop().unwrap();

                                if !in_text_object || !self.filter_text_objects {
                                    // Rectangle emits 4 segments: top, right, bottom, left
                                    // Note: PDF rectangle is [x y w h] where y is bottom
                                    segments.push(Segment::horizontal(y + h, x, x + w)); // top
                                    segments.push(Segment::vertical(x + w, y, y + h));  // right
                                    segments.push(Segment::horizontal(y, x, x + w));    // bottom
                                    segments.push(Segment::vertical(x, y, y + h));      // left
                                }
                            }
                        }
                        b"S" | b"s" => {
                            // Stroke - path is complete
                            // For stroke operators, we've already emitted segments
                            // for the path construction operators above
                            // The path is implicitly terminated after stroke
                        }
                        b"f" | b"F" | b"B" | b"B*" => {
                            // Fill operators - rectangles are handled via 're'
                            // For other paths, we ignore fills (they're not table rules)
                            // Clear current point as path is terminated
                            current_point = None;
                        }
                        b"h" => {
                            // Close path - returns to the start of the subpath
                            // We don't need special handling here since segments
                            // are emitted as we go
                        }
                        b"c" | b"v" | b"y" => {
                            // Curve operators - pop operands and advance current point
                            // We don't extract segments from curves for table detection
                            // but we need to consume the operands
                            let n = match op.as_slice() {
                                b"c" => 6, // x1 y1 x2 y2 x3 y3
                                b"v" => 4, // x2 y2 x3 y3
                                b"y" => 4, // x1 y1 x3 y3
                                _ => 0,
                            };
                            while operand_stack.len() >= n && n > 0 {
                                operand_stack.pop();
                            }
                            current_point = None; // Curves complicate tracking
                        }
                        b"q" => {
                            // Save graphics state
                            // For nested text objects, we'd track depth here
                        }
                        b"Q" => {
                            // Restore graphics state
                        }
                        b"cm" => {
                            // Concatenate matrix - pops 6 values
                            while operand_stack.len() >= 6 {
                                operand_stack.pop();
                            }
                        }
                        _ => {
                            // Other operators - ignore for table detection
                            // Clear the operand stack to avoid stale values
                            operand_stack.clear();
                        }
                    }
                }
                _ => {
                    // Other tokens - ignore
                }
            }
        }

        segments
    }

    /// Cluster collinear segments of the given orientation.
    ///
    /// Returns a vector of merged segments, one per cluster.
    fn cluster_segments(&self, segments: &[Segment], orientation: SegmentOrientation) -> Vec<Segment> {
        let filtered: Vec<_> = segments.iter()
            .filter(|s| s.orientation == orientation)
            .cloned()
            .collect();

        if filtered.is_empty() {
            return Vec::new();
        }

        // Group by position (y for horizontal, x for vertical) within epsilon
        let mut clusters: HashMap<i32, Vec<Segment>> = HashMap::new();

        for seg in filtered {
            let key = match orientation {
                SegmentOrientation::Horizontal => (seg.y0 / EPSILON) as i32,
                SegmentOrientation::Vertical => (seg.x0 / EPSILON) as i32,
            };

            clusters.entry(key).or_insert_with(Vec::new).push(seg);
        }

        // Merge each cluster into a single segment
        let mut merged = Vec::new();
        for cluster in clusters.values() {
            if let Some(m) = self.merge_cluster(cluster) {
                merged.push(m);
            }
        }

        merged
    }

    /// Merge a cluster of collinear segments into one segment.
    fn merge_cluster(&self, cluster: &[Segment]) -> Option<Segment> {
        if cluster.is_empty() {
            return None;
        }

        let orientation = cluster[0].orientation;
        let mut merged = cluster[0];

        for seg in &cluster[1..] {
            // Check if overlapping or within gap tolerance
            let overlap = match orientation {
                SegmentOrientation::Horizontal => {
                    merged.x1 >= seg.x0 - GAP_TOLERANCE && seg.x1 >= merged.x0 - GAP_TOLERANCE
                }
                SegmentOrientation::Vertical => {
                    merged.y1 >= seg.y0 - GAP_TOLERANCE && seg.y1 >= merged.y0 - GAP_TOLERANCE
                }
            };

            if overlap {
                merged = merged.merge(seg);
            } else {
                // Non-overlapping segment - return as separate
                // For simplicity, we just return the first merged segment
                // A full implementation would return all merged segments
            }
        }

        Some(merged)
    }

    /// Find intersection points between horizontal and vertical segments.
    fn find_intersections(&self, horizontal: &[Segment], vertical: &[Segment]) -> Vec<(f32, f32)> {
        let mut intersections = Vec::new();
        let mut seen = HashSet::new();

        for h in horizontal {
            for v in vertical {
                if let Some((x, y)) = h.intersection(v, EPSILON) {
                    // Round to avoid duplicate intersections from floating-point noise
                    let key = ((x * 10.0) as i32, (y * 10.0) as i32);
                    if seen.insert(key) {
                        intersections.push((x, y));
                    }
                }
            }
        }

        intersections
    }

    /// Build grid candidates from intersection points.
    fn build_grids(&self, intersections: Vec<(f32, f32)>, segments: Vec<Segment>) -> Vec<GridCandidate> {
        let mut grids = Vec::new();

        // For now, create one grid from all intersections
        // A full implementation would detect disjoint table regions
        if let Some(grid) = GridCandidate::from_intersections(intersections.clone(), segments) {
            if grid.cell_count() >= self.min_cells {
                grids.push(grid);
            }
        }

        grids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::pages::PageDict;

    fn make_page(_content: &[u8]) -> PageDict {
        use std::sync::Arc;
        use crate::parser::object::ObjRef;
        use crate::parser::resources::ResourceDict;

        PageDict {
            obj_ref: ObjRef::new(1, 0),
            media_box: [0.0, 0.0, 612.0, 792.0],
            resources: Arc::new(ResourceDict::default()),
            contents: vec![],
            annots: vec![],
            actual_text: None,
            lang: None,
            aa: None,
            struct_parents: None,
            crop_box: None,
            bleed_box: None,
            trim_box: None,
            art_box: None,
            rotate: 0,
        }
    }

    #[test]
    fn test_detector_default() {
        let detector = TableDetector::new();
        assert_eq!(detector.min_cells, 4);
        assert!(detector.filter_text_objects);
    }

    #[test]
    fn test_detector_with_min_cells() {
        let detector = TableDetector::new().with_min_cells(10);
        assert_eq!(detector.min_cells, 10);
    }

    #[test]
    fn test_collect_empty_content() {
        let detector = TableDetector::new();
        let page = make_page(b"");
        let ctx = PageContext::new(&page, b"");

        let segments = detector.collect_segments(&ctx);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_collect_horizontal_line() {
        let detector = TableDetector::new();
        let page = make_page(b"");
        // Content: moveto 10 50, lineto 100 50, stroke
        let content = b"10 50 m 100 50 l S";
        let ctx = PageContext::new(&page, content);

        let segments = detector.collect_segments(&ctx);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].orientation, SegmentOrientation::Horizontal);
    }

    #[test]
    fn test_collect_vertical_line() {
        let detector = TableDetector::new();
        let page = make_page(b"");
        // Content: moveto 50 10, lineto 50 100, stroke
        let content = b"50 10 m 50 100 l S";
        let ctx = PageContext::new(&page, content);

        let segments = detector.collect_segments(&ctx);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].orientation, SegmentOrientation::Vertical);
    }

    #[test]
    fn test_collect_rectangle() {
        let detector = TableDetector::new();
        let page = make_page(b"");
        // Content: rect 50 100 200 50, stroke (x=50, y=100, w=200, h=50)
        let content = b"50 100 200 50 re S";
        let ctx = PageContext::new(&page, content);

        let segments = detector.collect_segments(&ctx);
        assert_eq!(segments.len(), 4); // 4 sides of rectangle
    }

    #[test]
    fn test_filter_text_object_segments() {
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Content with path inside text object (should be filtered)
        let content = b"BT 10 50 m 100 50 l ET";
        let ctx = PageContext::new(&page, content);

        let segments = detector.collect_segments(&ctx);
        assert!(segments.is_empty(), "Segments inside text objects should be filtered");
    }

    #[test]
    fn test_no_filter_text_object_segments() {
        let detector = TableDetector::new().with_text_object_filtering(false);
        let page = make_page(b"");

        // Content with path inside text object (should NOT be filtered)
        let content = b"BT 10 50 m 100 50 l ET";
        let ctx = PageContext::new(&page, content);

        let segments = detector.collect_segments(&ctx);
        assert_eq!(segments.len(), 1, "Segments should be collected when filtering is disabled");
    }

    #[test]
    fn test_detect_simple_grid() {
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Draw a simple 2x2 table
        // Horizontal lines at y=100, 200, 300
        // Vertical lines at x=50, 150, 250
        let content = b"\
            50 100 m 250 100 l S \
            50 200 m 250 200 l S \
            50 300 m 250 300 l S \
            50 100 m 50 300 l S \
            150 100 m 150 300 l S \
            250 100 m 250 300 l S";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_line_based(&ctx);

        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].row_count(), 2);
        assert_eq!(grids[0].col_count(), 2);
        assert_eq!(grids[0].cell_count(), 4);
    }

    #[test]
    fn test_cluster_horizontal_segments() {
        let detector = TableDetector::new();

        let segments = vec![
            Segment::horizontal(50.0, 10.0, 50.0),
            Segment::horizontal(50.5, 40.0, 80.0), // Collinear within epsilon
        ];

        let clustered = detector.cluster_segments(&segments, SegmentOrientation::Horizontal);
        assert_eq!(clustered.len(), 1);
        // Merged segment should span from x=10 to x=80
        assert_eq!(clustered[0].x0, 10.0);
        assert_eq!(clustered[0].x1, 80.0);
    }

    #[test]
    fn test_find_intersections() {
        let detector = TableDetector::new();

        let horizontal = vec![Segment::horizontal(50.0, 10.0, 100.0)];
        let vertical = vec![Segment::vertical(50.0, 25.0, 75.0)];

        let intersections = detector.find_intersections(&horizontal, &vertical);
        assert_eq!(intersections.len(), 1);
        assert_eq!(intersections[0], (50.0, 50.0));
    }

    #[test]
    fn test_detect_5x3_table() {
        // Critical test from plan: 5x3 bordered table (5 rows × 3 columns)
        // Expected: row_ys.len() == 6 (6 horizontal lines for 5 rows)
        //           col_xs.len() == 4 (4 vertical lines for 3 columns)
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Draw a 5 row × 3 column table (4 vertical lines, 6 horizontal lines)
        // Horizontal lines at y = 100, 180, 260, 340, 420, 500 (6 lines = 5 rows)
        // Vertical lines at x = 50, 200, 350, 500 (4 lines = 3 columns)
        let mut content = Vec::new();
        for &y in &[500.0, 420.0, 340.0, 260.0, 180.0, 100.0] {
            content.extend(format!("50 {} m 500 {} l S ", y, y).as_bytes());
        }
        for &x in &[50.0, 200.0, 350.0, 500.0] {
            content.extend(format!("{} 100 m {} 500 l S ", x, x).as_bytes());
        }

        let ctx = PageContext::new(&page, &content);
        let grids = detector.detect_line_based(&ctx);

        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].row_count(), 5);
        assert_eq!(grids[0].col_count(), 3);
        assert_eq!(grids[0].cell_count(), 15);
        assert_eq!(grids[0].row_ys.len(), 6);
        assert_eq!(grids[0].col_xs.len(), 4);
    }

    #[test]
    fn test_detect_nested_rectangles() {
        // Test handling of nested rectangles (e.g., table within a table)
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Outer rectangle: (50, 50) to (350, 250)
        // Inner rectangle: (100, 100) to (300, 200)
        let content = b"\
            50 50 300 200 re S \
            100 100 200 100 re S";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_line_based(&ctx);

        // Should detect at least one grid
        assert!(!grids.is_empty());
    }

    #[test]
    fn test_detect_disjoint_tables() {
        // Test detection of multiple disjoint tables on the same page
        let detector = TableDetector::new();
        let page = make_page(b"");

        // First table at top of page (2x2 grid)
        // Horizontal lines at y=400, 450, 500; Vertical lines at x=50, 100, 150
        // Second table at bottom of page (2x2 grid)
        // Horizontal lines at y=100, 150, 200; Vertical lines at x=50, 100, 150
        let content = b"\
            50 400 m 150 400 l S 50 450 m 150 450 l S 50 500 m 150 500 l S \
            50 400 m 50 500 l S 100 400 m 100 500 l S 150 400 m 150 500 l S \
            50 100 m 150 100 l S 50 150 m 150 150 l S 50 200 m 150 200 l S \
            50 100 m 50 200 l S 100 100 m 100 200 l S 150 100 m 150 200 l S";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_line_based(&ctx);

        // Current implementation creates one grid from all intersections
        // A full implementation would detect separate regions
        assert!(!grids.is_empty());
    }

    // Borderless table detection tests

    #[test]
    fn test_detect_borderless_empty_content() {
        let detector = TableDetector::new();
        let page = make_page(b"");
        let ctx = PageContext::new(&page, b"");

        let grids = detector.detect_borderless(&ctx);
        assert!(grids.is_empty());
    }

    #[test]
    fn test_detect_borderless_no_text_block() {
        let detector = TableDetector::new();
        let page = make_page(b"");
        // Content without text block (only path operators)
        let content = b"50 100 m 150 100 l S";
        let ctx = PageContext::new(&page, content);

        let grids = detector.detect_borderless(&ctx);
        assert!(grids.is_empty());
    }

    #[test]
    fn test_detect_borderless_paragraph_rejected() {
        // Single column text should be rejected (not a table)
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Simulate a paragraph with left-aligned text at x=50
        // Multiple lines but all at same x0
        let content = b"\
            BT \
            50 700 Td (Line 1) Tj \
            0 -15 Td (Line 2) Tj \
            0 -15 Td (Line 3) Tj \
            0 -15 Td (Line 4) Tj \
            ET";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_borderless(&ctx);

        // Should not detect a table (only 1 column)
        assert!(grids.is_empty());
    }

    #[test]
    fn test_detect_borderless_one_row_pseudo_table_rejected() {
        // Single row with multiple columns should be rejected (< 3 rows)
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Simulate one row with 3 columns
        let content = b"\
            BT \
            50 700 Td (Col1) Tj \
            100 700 Td (Col2) Tj \
            150 700 Td (Col3) Tj \
            ET";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_borderless(&ctx);

        // Should not detect a table (only 1 row)
        assert!(grids.is_empty());
    }

    #[test]
    fn test_detect_borderless_3x3_table_accepted() {
        // Critical test: 3 rows x 3 columns borderless table
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Simulate a 3x3 table with aligned columns
        // Column 1 at x=50, Column 2 at x=150, Column 3 at x=250
        // Rows at y=700, 650, 600
        let content = b"\
            BT \
            50 700 Td (R1C1) Tj 100 0 Td (R1C2) Tj 100 0 Td (R1C3) Tj \
            -200 -50 Td (R2C1) Tj 100 0 Td (R2C2) Tj 100 0 Td (R2C3) Tj \
            -200 -50 Td (R3C1) Tj 100 0 Td (R3C2) Tj 100 0 Td (R3C3) Tj \
            ET";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_borderless(&ctx);

        // Should detect a table
        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].row_count(), 2); // 3 rows = 2 intervals
        assert_eq!(grids[0].col_count(), 2); // 3 columns = 2 intervals
        assert_eq!(grids[0].cell_count(), 4);
        // Verify segments are empty for borderless tables
        assert!(grids[0].segments.is_empty());
    }

    #[test]
    fn test_detect_borderless_vertical_gap_test() {
        // Two separate tables with a large vertical gap (> 100 pt)
        let detector = TableDetector::new();
        let page = make_page(b"");

        // First table at y=700, 650, 600
        // Second table at y=400, 350, 300
        // Gap = 600 - 400 = 200 pt > 100 pt threshold
        let content = b"\
            BT \
            50 700 Td (R1C1) Tj 100 0 Td (R1C2) Tj 100 0 Td (R1C3) Tj \
            -200 -50 Td (R2C1) Tj 100 0 Td (R2C2) Tj 100 0 Td (R2C3) Tj \
            -200 -50 Td (R3C1) Tj 100 0 Td (R3C2) Tj 100 0 Td (R3C3) Tj \
            ET \
            BT \
            50 400 Td (R1C1) Tj 100 0 Td (R1C2) Tj 100 0 Td (R1C3) Tj \
            -200 -50 Td (R2C1) Tj 100 0 Td (R2C2) Tj 100 0 Td (R2C3) Tj \
            -200 -50 Td (R3C1) Tj 100 0 Td (R3C2) Tj 100 0 Td (R3C3) Tj \
            ET";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_borderless(&ctx);

        // Should detect two separate tables
        assert_eq!(grids.len(), 2);
    }

    #[test]
    fn test_collect_text_positions_basic() {
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Basic text positioning with Tm and Tj
        let content = b"BT 1 0 0 1 50 700 Tm (Hello) Tj ET";
        let ctx = PageContext::new(&page, content);

        let positions = detector.collect_text_positions(&ctx);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].x0, 50.0);
        assert_eq!(positions[0].y0, 700.0);
    }

    #[test]
    fn test_collect_text_positions_with_td() {
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Text positioning with Td
        let content = b"BT 50 700 Td (Hello) Tj 100 0 Td (World) Tj ET";
        let ctx = PageContext::new(&page, content);

        let positions = detector.collect_text_positions(&ctx);
        assert_eq!(positions.len(), 2);
        // First position at (50, 700)
        assert_eq!(positions[0].x0, 50.0);
        assert_eq!(positions[0].y0, 700.0);
        // Second position at (150, 700) - Td adds to current position
        // The actual x position depends on Tm calculation
    }

    #[test]
    fn test_collect_text_positions_with_tj() {
        let detector = TableDetector::new();
        let page = make_page(b"");

        // Text positioning with TJ (array)
        let content = b"BT 50 700 Td [(Hello) 100 (World)] TJ ET";
        let ctx = PageContext::new(&page, content);

        let positions = detector.collect_text_positions(&ctx);
        // Should record position for TJ operator
        assert!(!positions.is_empty());
    }

    #[test]
    fn test_group_by_x0_tolerance() {
        let detector = TableDetector::new();
        let positions = vec![
            TextPosition { x0: 50.0, y0: 700.0 },
            TextPosition { x0: 51.0, y0: 650.0 }, // Within 2 pt tolerance
            TextPosition { x0: 52.0, y0: 600.0 }, // Within 2 pt tolerance
            TextPosition { x0: 150.0, y0: 700.0 }, // Different column
        ];

        let buckets = detector.group_by_x0(&positions);
        // x0=50, 51, 52 should be in same bucket (within tolerance)
        // x0=150 should be in different bucket
        assert_eq!(buckets.len(), 2);
        // One bucket should have 3 positions, one should have 1
        let counts: Vec<_> = buckets.values().map(|v| v.len()).collect();
        assert!(counts.contains(&3));
        assert!(counts.contains(&1));
    }

    #[test]
    fn test_find_row_candidates_basic() {
        let detector = TableDetector::new();
        let column_buckets = vec![
            (0, vec![
                TextPosition { x0: 50.0, y0: 700.0 },
                TextPosition { x0: 50.0, y0: 650.0 },
                TextPosition { x0: 50.0, y0: 600.0 },
            ]),
            (25, vec![
                TextPosition { x0: 150.0, y0: 700.0 },
                TextPosition { x0: 150.0, y0: 650.0 },
                TextPosition { x0: 150.0, y0: 600.0 },
            ]),
            (50, vec![
                TextPosition { x0: 250.0, y0: 700.0 },
                TextPosition { x0: 250.0, y0: 650.0 },
                TextPosition { x0: 250.0, y0: 600.0 },
            ]),
        ];

        let rows = detector.find_row_candidates(&column_buckets);
        // Should find 3 row positions (700, 650, 600)
        assert_eq!(rows.len(), 3);
        // Rows should be sorted descending
        assert_eq!(rows[0], 700.0);
        assert_eq!(rows[1], 650.0);
        assert_eq!(rows[2], 600.0);
    }

    #[test]
    fn test_is_single_column_reflow_true() {
        let detector = TableDetector::new();
        // Column 1 has positions that don't align with other columns
        let column_buckets = vec![
            (0, vec![
                TextPosition { x0: 50.0, y0: 700.0 },
                TextPosition { x0: 50.0, y0: 685.0 }, // Different y
                TextPosition { x0: 50.0, y0: 670.0 }, // Different y
            ]),
            (25, vec![
                TextPosition { x0: 150.0, y0: 700.0 }, // Only aligns with first
            ]),
        ];

        let is_reflow = detector.is_single_column_reflow(&column_buckets);
        // First column has mostly non-aligned positions, should be detected as reflow
        assert!(is_reflow);
    }

    #[test]
    fn test_is_single_column_reflow_false() {
        let detector = TableDetector::new();
        // All columns have good alignment
        let column_buckets = vec![
            (0, vec![
                TextPosition { x0: 50.0, y0: 700.0 },
                TextPosition { x0: 50.0, y0: 650.0 },
                TextPosition { x0: 50.0, y0: 600.0 },
            ]),
            (25, vec![
                TextPosition { x0: 150.0, y0: 700.0 },
                TextPosition { x0: 150.0, y0: 650.0 },
                TextPosition { x0: 150.0, y0: 600.0 },
            ]),
        ];

        let is_reflow = detector.is_single_column_reflow(&column_buckets);
        // Good alignment across all rows, not a reflow
        assert!(!is_reflow);
    }

    #[test]
    fn test_borderless_table_has_empty_segments() {
        // Borderless tables should not have segments (no ruling lines)
        let detector = TableDetector::new();
        let page = make_page(b"");

        let content = b"\
            BT \
            50 700 Td (R1C1) Tj 100 0 Td (R1C2) Tj 100 0 Td (R1C3) Tj \
            -200 -50 Td (R2C1) Tj 100 0 Td (R2C2) Tj 100 0 Td (R2C3) Tj \
            -200 -50 Td (R3C1) Tj 100 0 Td (R3C2) Tj 100 0 Td (R3C3) Tj \
            ET";

        let ctx = PageContext::new(&page, content);
        let grids = detector.detect_borderless(&ctx);

        assert!(!grids.is_empty());
        assert!(grids[0].segments.is_empty());
    }
}
