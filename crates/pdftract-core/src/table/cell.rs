//! Cell representation and span-to-cell assignment (7.2.3).
//!
//! This module implements span-to-cell assignment using centroid containment:
//! - For each span, compute its centroid ((x0+x1)/2, (y0+y1)/2)
//! - Assign the span to the cell whose bbox contains the centroid
//! - Use half-open interval [x0, x1) to avoid double-counting border cases
//! - Spans not contained in any cell become orphans
//! - Within each cell, sort spans by (round(y0/2), x0) for reading order

use serde::{Deserialize, Serialize};

/// Y-bucket size for span ordering within cells (2 pt).
///
/// Spans with y-coordinates within 2 pt of each other are considered
/// on the same line for sorting purposes. This prevents tiny y noise
/// from reordering spans on the same line.
const Y_BUCKET_SIZE: f64 = 2.0;

/// A text span for table cell assignment.
///
/// Minimal span representation used during cell assignment.
/// This is independent of the hybrid::Span type used in OCR processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSpan {
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f64; 4],
    /// The extracted text.
    pub text: String,
}

impl TableSpan {
    /// Create a new table span.
    pub fn new(bbox: [f64; 4], text: String) -> Self {
        Self { bbox, text }
    }

    /// Get the centroid of this span's bbox.
    fn centroid(&self) -> (f32, f32) {
        let cx = ((self.bbox[0] + self.bbox[2]) / 2.0) as f32;
        let cy = ((self.bbox[1] + self.bbox[3]) / 2.0) as f32;
        (cx, cy)
    }

    /// Get the width of this span.
    #[inline]
    fn width(&self) -> f64 {
        self.bbox[2] - self.bbox[0]
    }

    /// Get the height of this span.
    #[inline]
    fn height(&self) -> f64 {
        self.bbox[3] - self.bbox[1]
    }

    /// Get the area of this span.
    #[inline]
    fn area(&self) -> f64 {
        self.width() * self.height()
    }
}

/// A table cell with its assigned content.
///
/// Represents a single cell in a detected table grid, including
/// its position and the text spans assigned to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f32; 4],
    /// Text spans assigned to this cell, sorted in reading order.
    pub content: Vec<TableSpan>,
    /// Row index (0-based, 0 = top row).
    pub row: usize,
    /// Column index (0-based, 0 = leftmost column).
    pub col: usize,
    /// Row span (default 1, >1 for merged cells).
    pub rowspan: u32,
    /// Column span (default 1, >1 for merged cells).
    pub colspan: u32,
}

impl Cell {
    /// Create a new empty cell.
    pub fn new(bbox: [f32; 4], row: usize, col: usize) -> Self {
        Self {
            bbox,
            content: Vec::new(),
            row,
            col,
            rowspan: 1,
            colspan: 1,
        }
    }

    /// Check if this cell contains a point (centroid).
    ///
    /// Uses half-open interval [x0, x1) × [y0, y1) to avoid
    /// double-counting when a point falls exactly on a shared border.
    ///
    /// # Arguments
    ///
    /// * `px, py` - Point coordinates in PDF user space
    ///
    /// # Returns
    ///
    /// `true` if the point is contained, `false` otherwise.
    fn contains_point(&self, px: f32, py: f32) -> bool {
        // Half-open interval: x0 <= px < x1, y0 <= py < y1
        // Note: edge cells have their bbox extended by 0.5 pt in extend_bbox_for_edges
        px >= self.bbox[0] && px < self.bbox[2]
            && py >= self.bbox[1] && py < self.bbox[3]
    }

    /// Assign spans to cells based on centroid containment.
    ///
    /// # Arguments
    ///
    /// * `grid` - The grid candidate with row/col boundaries
    /// * `spans` - All text spans on the page
    ///
    /// # Returns
    ///
    /// A tuple of (cells, orphan_spans, diagnostics):
    /// - `cells`: Vector of cells with their assigned content
    /// - `orphan_spans`: Spans not assigned to any cell
    /// - `diagnostics`: Diagnostic messages about edge cases
    pub fn assign_spans_to_cells(
        grid: &super::GridCandidate,
        spans: Vec<TableSpan>,
    ) -> (Vec<Cell>, Vec<TableSpan>, Vec<String>) {
        let mut cells = Vec::new();
        let mut orphans = Vec::new();
        let mut diagnostics = Vec::new();

        // Create empty cells for the grid
        for row in 0..grid.row_count() {
            for col in 0..grid.col_count() {
                if let Some(bbox) = grid.cell_bbox(row, col) {
                    // Extend bbox by 0.5 pt for edge cells to capture spans flush to border
                    let bbox = extend_bbox_for_edges(bbox, row, col, grid);
                    cells.push(Cell::new(bbox, row, col));
                }
            }
        }

        // Assign each span to a cell based on centroid containment
        for span in spans {
            let (centroid_x, centroid_y) = span.centroid();

            // Find the cell index containing this centroid
            let mut assigned_cell_idx = None;
            for (idx, cell) in cells.iter().enumerate() {
                if cell.contains_point(centroid_x, centroid_y) {
                    assigned_cell_idx = Some(idx);
                    break;
                }
            }

            if let Some(idx) = assigned_cell_idx {
                // Check if span overlaps multiple cells significantly
                check_overlap_and_diagnose(&span, &cells[idx], &cells, &mut diagnostics);
                cells[idx].content.push(span);
            } else {
                orphans.push(span);
            }
        }

        // Sort content within each cell by reading order
        for cell in &mut cells {
            sort_cell_content(cell);
        }

        (cells, orphans, diagnostics)
    }
}

/// Extend bbox by 0.5 pt for cells touching grid edges.
///
/// This captures spans that are flush to the table border.
/// Edge cells (top row, bottom row, leftmost col, rightmost col)
/// get their outer boundary extended by 0.5 pt.
fn extend_bbox_for_edges(
    mut bbox: [f32; 4],
    row: usize,
    col: usize,
    grid: &super::GridCandidate,
) -> [f32; 4] {
    // Top row: extend y1 upward
    if row == 0 {
        bbox[3] += 0.5;
    }
    // Bottom row: extend y0 downward
    if row == grid.row_count() - 1 {
        bbox[1] -= 0.5;
    }
    // Leftmost column: extend x0 leftward
    if col == 0 {
        bbox[0] -= 0.5;
    }
    // Rightmost column: extend x1 rightward
    if col == grid.col_count() - 1 {
        bbox[2] += 0.5;
    }

    bbox
}

/// Check if a span overlaps multiple cells and emit diagnostic.
///
/// If a span's centroid is in cell A but its bbox overlaps cell B
/// by more than 40%, emit a diagnostic.
///
/// Note: Due to the geometry of half-open intervals, it's mathematically
/// impossible for a span to have > 50% overlap with a cell while its
/// centroid is in a different cell. The maximum is 50% (achieved when
/// the centroid is exactly on the boundary, which falls in the right cell
/// due to half-open interval). We use 40% as a practical threshold.
fn check_overlap_and_diagnose(
    span: &TableSpan,
    assigned_cell: &Cell,
    all_cells: &[Cell],
    diagnostics: &mut Vec<String>,
) {
    let span_area = span.area() as f32;

    for other_cell in all_cells {
        if other_cell.row == assigned_cell.row && other_cell.col == assigned_cell.col {
            continue;
        }

        // Compute overlap area
        let overlap_x0 = (span.bbox[0] as f32).max(other_cell.bbox[0]);
        let overlap_y0 = (span.bbox[1] as f32).max(other_cell.bbox[1]);
        let overlap_x1 = (span.bbox[2] as f32).min(other_cell.bbox[2]);
        let overlap_y1 = (span.bbox[3] as f32).min(other_cell.bbox[3]);

        if overlap_x1 > overlap_x0 && overlap_y1 > overlap_y0 {
            let overlap_area = (overlap_x1 - overlap_x0) * (overlap_y1 - overlap_y0);
            let overlap_ratio = overlap_area / span_area;

            if overlap_ratio > 0.4 {
                let text_preview: String = span.text.chars().take(20).collect();
                diagnostics.push(format!(
                    "span_bbox_overlaps_multiple_cells: text='{}' centroid in ({},{}) but {:.1}% overlaps ({},{})",
                    text_preview,
                    assigned_cell.row, assigned_cell.col,
                    overlap_ratio * 100.0,
                    other_cell.row, other_cell.col
                ));
            }
        }
    }
}

/// Sort spans within a cell by reading order.
///
/// Uses (round(y0/2), x0) ordering with a 2-pt y bucket.
/// This groups spans on the same line and sorts left-to-right.
fn sort_cell_content(cell: &mut Cell) {
    // Use sort_by with index to ensure stability
    // We attach the original index to each element to preserve order for equal keys
    let mut indexed: Vec<_> = cell.content.iter().enumerate().collect();
    indexed.sort_by(|(ia, a), (ib, b)| {
        // Y-bucket: round to nearest 2 pt
        let y_bucket_a = (a.bbox[1] / Y_BUCKET_SIZE).round() as i64;
        let y_bucket_b = (b.bbox[1] / Y_BUCKET_SIZE).round() as i64;

        // Primary sort by y-bucket (descending - PDF y increases upward)
        match y_bucket_b.cmp(&y_bucket_a) {
            std::cmp::Ordering::Equal => {
                // Secondary sort by x0 (ascending - left to right)
                match a.bbox[0].partial_cmp(&b.bbox[0]) {
                    Some(std::cmp::Ordering::Equal) => {
                        // Tertiary sort by original index (ascending) for stability
                        ia.cmp(ib)
                    }
                    Some(ord) => ord,
                    None => ia.cmp(ib),
                }
            }
            other => other,
        }
    });

    // Reconstruct the sorted vector
    cell.content = indexed.into_iter().map(|(_, span)| span.clone()).collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::GridCandidate;

    fn make_span(x0: f64, y0: f64, x1: f64, y1: f64, text: &str) -> TableSpan {
        TableSpan::new([x0, y0, x1, y1], text.to_string())
    }

    #[test]
    fn test_cell_new() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.col, 0);
        assert_eq!(cell.rowspan, 1);
        assert_eq!(cell.colspan, 1);
        assert!(cell.content.is_empty());
    }

    #[test]
    fn test_cell_contains_point_inside() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        // Point inside
        assert!(cell.contains_point(100.0, 150.0));
    }

    #[test]
    fn test_cell_contains_point_on_boundary() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        // Points on boundaries - half-open interval
        assert!(cell.contains_point(50.0, 150.0));  // x0 included
        assert!(cell.contains_point(100.0, 100.0)); // y0 included
        assert!(!cell.contains_point(150.0, 150.0)); // x1 excluded
        assert!(!cell.contains_point(100.0, 200.0)); // y1 excluded
    }

    #[test]
    fn test_cell_contains_point_outside() {
        let cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);
        assert!(!cell.contains_point(49.0, 150.0));  // Left of cell
        assert!(!cell.contains_point(151.0, 150.0)); // Right of cell
        assert!(!cell.contains_point(100.0, 99.0));  // Below cell
        assert!(!cell.contains_point(100.0, 201.0)); // Above cell
    }

    #[test]
    fn test_cell_contains_point_with_epsilon() {
        // Test that edge extension works for cells on grid boundaries
        // Create a grid and check that edge cells have extended bounds
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Create spans and assign to cells (which triggers edge extension)
        let spans = vec![
            make_span(49.8, 210.0, 60.0, 220.0, "edge_left"), // x0=49.8, just outside left border
        ];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        // The span should be captured by the edge-extended cell
        assert_eq!(orphans.len(), 0);
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "edge_left");
    }

    #[test]
    fn test_assign_spans_to_cells_simple() {
        // Create a simple 2x2 grid
        // Horizontal lines at y = 100, 200, 300 (3 lines = 2 rows)
        // Vertical lines at x = 50, 150, 250 (3 lines = 2 cols)
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
            (50.0, 300.0), (150.0, 300.0), (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Create spans with centroids in each cell
        let spans = vec![
            make_span(60.0, 210.0, 90.0, 240.0, "R0C0"), // Top row, left col
            make_span(160.0, 210.0, 190.0, 240.0, "R0C1"), // Top row, right col
            make_span(60.0, 110.0, 90.0, 140.0, "R1C0"), // Bottom row, left col
            make_span(160.0, 110.0, 190.0, 140.0, "R1C1"), // Bottom row, right col
        ];

        let (cells, orphans, diagnostics) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(cells.len(), 4);
        assert_eq!(orphans.len(), 0);
        assert!(diagnostics.is_empty());

        // Check that each cell has the correct span
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "R0C0");

        let cell_r0c1 = cells.iter().find(|c| c.row == 0 && c.col == 1).unwrap();
        assert_eq!(cell_r0c1.content.len(), 1);
        assert_eq!(cell_r0c1.content[0].text, "R0C1");

        let cell_r1c0 = cells.iter().find(|c| c.row == 1 && c.col == 0).unwrap();
        assert_eq!(cell_r1c0.content.len(), 1);
        assert_eq!(cell_r1c0.content[0].text, "R1C0");

        let cell_r1c1 = cells.iter().find(|c| c.row == 1 && c.col == 1).unwrap();
        assert_eq!(cell_r1c1.content.len(), 1);
        assert_eq!(cell_r1c1.content[0].text, "R1C1");
    }

    #[test]
    fn test_assign_spans_centroid_on_border() {
        // Test that centroids exactly on borders are assigned deterministically
        // due to half-open interval [x0, x1)
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
            (50.0, 300.0), (150.0, 300.0), (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span with centroid exactly on vertical border at x=150
        // Bbox: [140, 210, 160, 240] -> centroid at (150, 225)
        // Due to half-open interval [x0, x1), x=150 falls in cell (0, 1) because [150, 250) includes 150
        // but [50, 150) excludes 150 (upper bound is exclusive)
        let spans = vec![
            make_span(140.0, 210.0, 160.0, 240.0, "border_x"),
        ];

        let (cells, _orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        // Should be assigned to cell (0, 1) because x=150 falls in [150, 250) not [50, 150)
        let cell_r0c1 = cells.iter().find(|c| c.row == 0 && c.col == 1).unwrap();
        assert_eq!(cell_r0c1.content.len(), 1);
        assert_eq!(cell_r0c1.content[0].text, "border_x");
    }

    #[test]
    fn test_assign_orphan_spans() {
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
            (50.0, 300.0), (150.0, 300.0), (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span outside the grid
        let spans = vec![
            make_span(300.0, 210.0, 350.0, 240.0, "outside"),
        ];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].text, "outside");

        // No cell should have this span
        for cell in &cells {
            assert!(!cell.content.iter().any(|s| s.text == "outside"));
        }
    }

    #[test]
    fn test_span_overlaps_multiple_cells_diagnostic() {
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
            (50.0, 300.0), (150.0, 300.0), (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span with centroid in cell (0, 0) but bbox extending into cell (0, 1)
        // Bbox: [100, 210, 199, 240] -> centroid at (149.5, 225)
        // Due to half-open interval [x0, x1), x=149.5 falls in cell (0, 0) [50, 150)
        // Overlap with cell (0, 1) which covers x=[150, 250)
        // Overlap area = (199 - 150) * (240 - 210) = 49 * 30 = 1470
        // Span area = 99 * 30 = 2970
        // Overlap ratio = 1470 / 2970 = 49.5% > 40%, should trigger diagnostic
        let spans = vec![
            make_span(100.0, 210.0, 199.0, 240.0, "overlap"),
        ];

        let (cells, _orphans, diagnostics) = Cell::assign_spans_to_cells(&grid, spans);

        // Verify span is assigned to cell (0, 0)
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "overlap");

        // Should have a diagnostic about overlapping multiple cells
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].contains("span_bbox_overlaps_multiple_cells"));
    }

    #[test]
    fn test_sort_cell_content_by_line() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);

        // Add spans in random order
        cell.content = vec![
            make_span(70.0, 110.0, 90.0, 120.0, "line2_right"), // Lower y, right
            make_span(60.0, 210.0, 90.0, 220.0, "line1_left"),  // Higher y, left
            make_span(60.0, 109.0, 80.0, 119.0, "line2_left"),  // Lower y, left (same line as line2_right within 2pt)
        ];

        sort_cell_content(&mut cell);

        // Should be sorted by y (descending), then x (ascending)
        assert_eq!(cell.content[0].text, "line1_left");   // Highest y
        assert_eq!(cell.content[1].text, "line2_left");   // Same line bucket, leftmost
        assert_eq!(cell.content[2].text, "line2_right");  // Same line bucket, rightmost
    }

    #[test]
    fn test_5x3_bordered_table_critical_test() {
        // Critical test from plan: 5 columns × 3 rows = 15 cells
        // Horizontal lines at y = 100, 200, 300, 400 (4 lines = 3 rows)
        // Vertical lines at x = 50, 150, 250, 350, 450, 550 (6 lines = 5 columns)
        let mut intersections = Vec::new();
        for &y in &[400.0, 300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0, 450.0, 550.0] {
                intersections.push((x, y));
            }
        }

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Create a span for each of the 15 cells
        // Row 0 is top row (highest y), row 2 is bottom row (lowest y)
        let mut spans = Vec::new();
        for row in 0..3 {
            for col in 0..5 {
                let x0 = 50.0 + (col as f64) * 100.0 + 10.0;
                let x1 = x0 + 80.0;
                // Grid rows: row 0 has y in [300, 400], row 1 has y in [200, 300], row 2 has y in [100, 200]
                let y0 = 300.0 - ((row as f64) * 100.0) + 10.0;
                let y1 = y0 + 80.0;
                spans.push(make_span(x0, y0, x1, y1, &format!("R{}C{}", row, col)));
            }
        }

        let (cells, orphans, diagnostics) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(cells.len(), 15);
        assert_eq!(orphans.len(), 0);
        assert!(diagnostics.is_empty());

        // Verify each cell has correct content
        for row in 0..3 {
            for col in 0..5 {
                let cell = cells.iter().find(|c| c.row == row && c.col == col).unwrap();
                assert_eq!(cell.content.len(), 1);
                assert_eq!(cell.content[0].text, format!("R{}C{}", row, col));
            }
        }
    }

    #[test]
    fn test_extend_bbox_for_top_row() {
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Top row cell (row 0) should have extended y1
        let bbox = grid.cell_bbox(0, 0).unwrap();
        let extended = extend_bbox_for_edges(bbox, 0, 0, &grid);
        assert_eq!(extended[3], bbox[3] + 0.5); // y1 extended
    }

    #[test]
    fn test_extend_bbox_for_bottom_row() {
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Bottom row cell (row 1) should have extended y0
        let bbox = grid.cell_bbox(1, 0).unwrap();
        let extended = extend_bbox_for_edges(bbox, 1, 0, &grid);
        assert_eq!(extended[1], bbox[1] - 0.5); // y0 extended
    }

    #[test]
    fn test_extend_bbox_for_leftmost_column() {
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Leftmost column (col 0) should have extended x0
        let bbox = grid.cell_bbox(0, 0).unwrap();
        let extended = extend_bbox_for_edges(bbox, 0, 0, &grid);
        assert_eq!(extended[0], bbox[0] - 0.5); // x0 extended
    }

    #[test]
    fn test_extend_bbox_for_rightmost_column() {
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0), (250.0, 100.0),
            (50.0, 200.0), (150.0, 200.0), (250.0, 200.0),
            (50.0, 300.0), (150.0, 300.0), (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Rightmost column (col 1) should have extended x1
        let bbox = grid.cell_bbox(0, 1).unwrap();
        let extended = extend_bbox_for_edges(bbox, 0, 1, &grid);
        assert_eq!(extended[2], bbox[2] + 0.5); // x1 extended
    }

    #[test]
    fn test_span_flush_to_border_captured() {
        // Test that spans flush to the table border are captured by edge extension
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Span with bbox flush to the left border (x0 = 50.0)
        // Centroid at (65, 250) - this is well inside the cell
        // But even if it were closer, the edge extension would capture it
        let spans = vec![
            make_span(50.0, 210.0, 80.0, 240.0, "flush_left"),
        ];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(orphans.len(), 0);
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 1);
        assert_eq!(cell_r0c0.content[0].text, "flush_left");
    }

    #[test]
    fn test_multiple_spans_in_same_cell_sorted() {
        let intersections = vec![
            (50.0, 100.0), (150.0, 100.0),
            (50.0, 200.0), (150.0, 200.0),
            (50.0, 300.0), (150.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Multiple spans in the same cell, out of order
        // Cell (0, 0) has y in [200, 300], so all spans should be in that range
        let spans = vec![
            make_span(60.0, 210.0, 90.0, 220.0, "third"),   // Lower y
            make_span(60.0, 280.0, 90.0, 290.0, "first"),   // Higher y
            make_span(60.0, 245.0, 90.0, 255.0, "second"),  // Middle y
        ];

        let (cells, orphans, _) = Cell::assign_spans_to_cells(&grid, spans);

        assert_eq!(orphans.len(), 0);
        let cell_r0c0 = cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        assert_eq!(cell_r0c0.content.len(), 3);

        // Should be sorted by y descending (reading order)
        assert_eq!(cell_r0c0.content[0].text, "first");
        assert_eq!(cell_r0c0.content[1].text, "second");
        assert_eq!(cell_r0c0.content[2].text, "third");
    }

    #[test]
    fn test_y_bucket_sorting() {
        let mut cell = Cell::new([50.0, 100.0, 150.0, 200.0], 0, 0);

        // Spans with tiny y differences (< 2 pt) should be on same line
        // y0 = 210, 210.5, 210.9 all round to same bucket: 210/2=105.0, 210.5/2=105.25, 210.9/2=105.45 -> all round to 105
        cell.content = vec![
            make_span(60.0, 210.0, 90.0, 220.0, "a"), // y0 = 210
            make_span(60.0, 210.5, 90.0, 220.5, "b"), // y0 = 210.5 (same 2-pt bucket as 210)
            make_span(70.0, 210.9, 100.0, 220.9, "c"), // y0 = 210.9 (same bucket, right of b)
        ];

        sort_cell_content(&mut cell);

        // All in same y-bucket, sorted by x
        assert_eq!(cell.content[0].text, "a"); // x0 = 60
        assert_eq!(cell.content[1].text, "b"); // x0 = 60 (same as a, stable order)
        assert_eq!(cell.content[2].text, "c"); // x0 = 70
    }

    #[test]
    fn test_table_span_centroid() {
        let span = make_span(100.0, 200.0, 200.0, 300.0, "test");
        let (cx, cy) = span.centroid();
        assert_eq!(cx, 150.0);
        assert_eq!(cy, 250.0);
    }

    #[test]
    fn test_table_span_area() {
        let span = make_span(100.0, 200.0, 200.0, 300.0, "test");
        assert_eq!(span.width(), 100.0);
        assert_eq!(span.height(), 100.0);
        assert_eq!(span.area(), 10000.0);
    }
}
