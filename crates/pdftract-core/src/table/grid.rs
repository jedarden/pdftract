//! Grid candidate representation for detected tables.
//!
//! A GridCandidate represents a potential table reconstructed from
//! horizontal and vertical ruling lines.

use serde::{Deserialize, Serialize};

/// Epsilon tolerance for floating point comparison.
const EPSILON: f32 = 0.1;

/// A candidate table grid reconstructed from path segments.
///
/// Represents a bounded rectangular grid with row and column boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridCandidate {
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f32; 4],
    /// Y-coordinates of row boundaries (horizontal lines).
    /// Sorted in descending order (PDF y increases upward).
    pub row_ys: Vec<f32>,
    /// X-coordinates of column boundaries (vertical lines).
    /// Sorted in ascending order (left to right).
    pub col_xs: Vec<f32>,
    /// The path segments that contributed to this grid.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<super::Segment>,
    /// Number of contiguous header rows from the top of the table.
    /// Detected via bold font detection or StructTree TH tags.
    /// Set to 0 if no header rows are detected.
    #[serde(skip_serializing_if = "is_zero_header_rows")]
    pub header_rows: u32,
}

/// Helper for serde to skip serializing header_rows when it's 0.
fn is_zero_header_rows(v: &u32) -> bool {
    *v == 0
}

impl GridCandidate {
    /// Create a new grid candidate from intersection points.
    ///
    /// # Arguments
    ///
    /// * `intersections` - (x, y) intersection points
    /// * `segments` - The path segments that formed this grid
    ///
    /// # Returns
    ///
    /// `Some(grid)` if at least 4 intersection points form a closed grid,
    /// `None` otherwise.
    pub fn from_intersections(
        intersections: Vec<(f32, f32)>,
        segments: Vec<super::Segment>,
    ) -> Option<Self> {
        if intersections.len() < 4 {
            return None;
        }

        // Extract distinct y coordinates (row boundaries)
        let mut row_ys: Vec<f32> = intersections.iter().map(|&(_, y)| y).collect::<Vec<_>>();

        // Sort descending (PDF y increases upward) and deduplicate
        row_ys.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        row_ys.dedup_by(|a, b| (*a - *b).abs() < EPSILON);

        // Extract distinct x coordinates (column boundaries)
        let mut col_xs: Vec<f32> = intersections.iter().map(|&(x, _)| x).collect::<Vec<_>>();

        // Sort ascending (left to right) and deduplicate
        col_xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        col_xs.dedup_by(|a, b| (*a - *b).abs() < EPSILON);

        // Must have at least 2 rows and 2 columns to form cells
        if row_ys.len() < 2 || col_xs.len() < 2 {
            return None;
        }

        // Compute bounding box
        let x0 = col_xs.first().copied()?;
        let x1 = col_xs.last().copied()?;
        let y0 = row_ys.last().copied()?; // Minimum y (bottom)
        let y1 = row_ys.first().copied()?; // Maximum y (top)

        let bbox = [x0, y0, x1, y1];

        Some(Self {
            bbox,
            row_ys,
            col_xs,
            segments,
            header_rows: 0, // Initialized to 0; set after header detection
        })
    }

    /// Get the number of rows in this grid.
    ///
    /// This is row_ys.len() - 1 (rows are between horizontal lines).
    #[inline]
    pub fn row_count(&self) -> usize {
        self.row_ys.len().saturating_sub(1)
    }

    /// Get the number of columns in this grid.
    ///
    /// This is col_xs.len() - 1 (columns are between vertical lines).
    #[inline]
    pub fn col_count(&self) -> usize {
        self.col_xs.len().saturating_sub(1)
    }

    /// Get the total number of cells in this grid.
    #[inline]
    pub fn cell_count(&self) -> usize {
        self.row_count() * self.col_count()
    }

    /// Get the bounding box of a specific cell.
    ///
    /// # Arguments
    ///
    /// * `row` - 0-based row index (0 = top row)
    /// * `col` - 0-based column index (0 = leftmost column)
    ///
    /// # Returns
    ///
    /// `Some([x0, y0, x1, y1])` if the cell indices are valid, `None` otherwise.
    pub fn cell_bbox(&self, row: usize, col: usize) -> Option<[f32; 4]> {
        if row >= self.row_count() || col >= self.col_count() {
            return None;
        }

        // Row 0 is the top row (highest y)
        let y1 = self.row_ys.get(row)?;
        let y0 = self.row_ys.get(row + 1)?;

        let x0 = self.col_xs.get(col)?;
        let x1 = self.col_xs.get(col + 1)?;

        Some([*x0, *y0, *x1, *y1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Segment;

    #[test]
    fn test_grid_from_intersections_5x3() {
        // 5 columns × 3 rows = 15 cells
        // Horizontal lines at y = 100, 200, 300, 400 (4 lines = 3 rows)
        // Vertical lines at x = 50, 150, 250, 350, 450, 550 (6 lines = 5 columns)
        let mut intersections = Vec::new();
        for &y in &[400.0, 300.0, 200.0, 100.0] {
            for &x in &[50.0, 150.0, 250.0, 350.0, 450.0, 550.0] {
                intersections.push((x, y));
            }
        }

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Check row boundaries (descending)
        assert_eq!(grid.row_ys, vec![400.0, 300.0, 200.0, 100.0]);
        assert_eq!(grid.row_count(), 3);

        // Check column boundaries (ascending)
        assert_eq!(grid.col_xs, vec![50.0, 150.0, 250.0, 350.0, 450.0, 550.0]);
        assert_eq!(grid.col_count(), 5);

        // Check total cells
        assert_eq!(grid.cell_count(), 15);

        // Check bounding box
        assert_eq!(grid.bbox, [50.0, 100.0, 550.0, 400.0]);
    }

    #[test]
    fn test_grid_insufficient_intersections() {
        // Less than 4 intersections can't form a closed grid
        let intersections = vec![(50.0, 100.0), (150.0, 100.0), (50.0, 200.0)];
        let grid = GridCandidate::from_intersections(intersections, vec![]);
        assert!(grid.is_none());
    }

    #[test]
    fn test_grid_single_row() {
        // Single row (2 horizontal lines, 2 vertical lines)
        let intersections = vec![(50.0, 100.0), (150.0, 100.0), (50.0, 200.0), (150.0, 200.0)];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();
        assert_eq!(grid.row_count(), 1);
        assert_eq!(grid.col_count(), 1);
        assert_eq!(grid.cell_count(), 1);
    }

    #[test]
    fn test_cell_bbox() {
        let intersections = vec![
            (50.0, 100.0),
            (150.0, 100.0),
            (250.0, 100.0),
            (50.0, 200.0),
            (150.0, 200.0),
            (250.0, 200.0),
            (50.0, 300.0),
            (150.0, 300.0),
            (250.0, 300.0),
        ];

        let grid = GridCandidate::from_intersections(intersections, vec![]).unwrap();

        // Top-left cell (row 0, col 0)
        let bbox = grid.cell_bbox(0, 0).unwrap();
        assert_eq!(bbox, [50.0, 200.0, 150.0, 300.0]);

        // Bottom-right cell (row 1, col 1)
        let bbox = grid.cell_bbox(1, 1).unwrap();
        assert_eq!(bbox, [150.0, 100.0, 250.0, 200.0]);

        // Out of bounds
        assert!(grid.cell_bbox(5, 0).is_none());
        assert!(grid.cell_bbox(0, 5).is_none());
    }

    #[test]
    fn test_grid_with_segments() {
        let segments = vec![
            Segment::horizontal(100.0, 50.0, 150.0),
            Segment::vertical(50.0, 100.0, 200.0),
        ];

        let intersections = vec![(50.0, 100.0), (150.0, 100.0), (50.0, 200.0), (150.0, 200.0)];

        let grid = GridCandidate::from_intersections(intersections, segments).unwrap();
        assert_eq!(grid.segments.len(), 2);
    }
}
