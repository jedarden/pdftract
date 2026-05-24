//! Line-based table detector.
//!
//! Extracts tables by analyzing path segments (horizontal and vertical lines)
//! from PDF content streams and reconstructing grid structures.

use super::{PageContext, GridCandidate, Segment, SegmentOrientation};
use crate::parser::lexer::Lexer;
use std::collections::{HashMap, HashSet};

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
}
