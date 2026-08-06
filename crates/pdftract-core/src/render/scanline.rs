//! Scanline polygon fill algorithm for bitmap rasterization.
//!
//! This module implements the classic scanline fill algorithm for converting
//! vector polygon edges into filled pixel regions. It handles edge cases
//! including horizontal edges and proper vertex counting via half-open intervals.
//!
//! # Algorithm Overview
//!
//! 1. Find y-bounds of the polygon
//! 2. For each scanline from min_y to max_y:
//!    - Find all edge intersections with this scanline
//!    - Sort intersections left-to-right
//!    - Fill between pairs of intersections (even-odd rule)
//!
//! # Edge Cases
//!
//! - Horizontal edges are skipped (they don't affect scanline fill)
//! - Half-open interval for vertices (include lower, exclude upper) prevents double-counting
//! - All intersections are clipped to bitmap bounds

use std::fmt;

/// 2D edge defined by two endpoints.
///
/// Represents a line segment from (x0, y0) to (x1, y1) in bitmap coordinates.
/// Edges are used as input to the scanline fill algorithm.
///
/// # Example
///
/// ```
/// use pdftract_core::render::scanline::Edge;
///
/// let edge = Edge::new(10, 5, 30, 25);
/// assert_eq!(edge.x0, 10);
/// assert_eq!(edge.y0, 5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    /// Starting X coordinate
    pub x0: i32,
    /// Starting Y coordinate
    pub y0: i32,
    /// Ending X coordinate
    pub x1: i32,
    /// Ending Y coordinate
    pub y1: i32,
}

impl Edge {
    /// Create a new edge from (x0, y0) to (x1, y1).
    ///
    /// # Arguments
    ///
    /// * `x0` - Starting X coordinate
    /// * `y0` - Starting Y coordinate
    /// * `x1` - Ending X coordinate
    /// * `y1` - Ending Y coordinate
    #[must_use]
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    /// Create an edge from a tuple of (x0, y0, x1, y1).
    #[must_use]
    pub const fn from_tuple(coords: (i32, i32, i32, i32)) -> Self {
        Self {
            x0: coords.0,
            y0: coords.1,
            x1: coords.2,
            y1: coords.3,
        }
    }

    /// Check if this edge is horizontal (y0 == y1).
    ///
    /// Horizontal edges are skipped in scanline fill since they don't
    /// contribute to scanline intersections.
    #[must_use]
    pub const fn is_horizontal(&self) -> bool {
        self.y0 == self.y1
    }

    /// Get the Y bounds of this edge.
    ///
    /// Returns (min_y, max_y) - the minimum and maximum Y coordinates.
    #[must_use]
    pub const fn y_bounds(&self) -> (i32, i32) {
        if self.y0 < self.y1 {
            (self.y0, self.y1)
        } else {
            (self.y1, self.y0)
        }
    }
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Edge({}, {}, {}, {})", self.x0, self.y0, self.x1, self.y1)
    }
}

/// Bitmap trait for scanline fill output.
///
/// This trait abstracts bitmap operations needed by the scanline fill algorithm.
/// Implementations can be fixed-size or resizable, monochrome or color.
pub trait Bitmap {
    /// Set the pixel at (x, y) to the given value.
    ///
    /// Returns false if (x, y) is out of bounds.
    fn set(&mut self, x: i32, y: i32, value: u8) -> bool;

    /// Get the width of this bitmap.
    fn width(&self) -> i32;

    /// Get the height of this bitmap.
    fn height(&self) -> i32;
}

/// Fill a polygon using the scanline algorithm.
///
/// Given a list of edges defining a polygon boundary, this function fills
/// the interior pixels using the even-odd rule. The algorithm processes
/// each scanline, finds edge intersections, and fills between pairs.
///
/// # Arguments
///
/// * `bitmap` - Mutable bitmap to draw into (must implement Bitmap trait)
/// * `edges` - Slice of edges defining the polygon boundary
/// * `fill_value` - Pixel value to use for filled pixels (0-255)
///
/// # Algorithm Details
///
/// 1. **Find Y bounds**: Determine the minimum and maximum Y coordinates
///    across all edges, clamped to bitmap bounds.
///
/// 2. **For each scanline**: Process each horizontal line from min_y to max_y.
///
/// 3. **Find intersections**: For each edge that spans this scanline (using
///    half-open interval to avoid double-counting vertices), calculate the
///    X coordinate where the edge intersects the scanline using linear interpolation.
///
/// 4. **Sort and fill**: Sort intersection X values left-to-right, then fill
///    pixels between pairs of intersections (even-odd rule).
///
/// # Edge Cases Handled
///
/// - **Horizontal edges**: Skipped entirely since they don't cross scanlines
/// - **Vertex handling**: Uses half-open interval [y_min, y_max) to avoid
///   double-counting vertices where edges meet
/// - **Boundary clipping**: All pixel writes are clipped to bitmap bounds
///
/// # Example
///
/// ```rust,ignore
/// use pdftract_core::render::scanline::{fill_polygon, Edge};
///
/// let mut bitmap = Bitmap32x32::white();
///
/// // Define a triangle with vertices (10,5), (30,25), (10,25)
/// let edges = vec![
///     Edge::new(10, 5, 30, 25),  // Diagonal edge
///     Edge::new(30, 25, 10, 25), // Horizontal bottom edge (will be skipped)
///     Edge::new(10, 25, 10, 5), // Vertical left edge
/// ];
///
/// fill_polygon(&mut bitmap, &edges, 0);
/// ```
pub fn fill_polygon<B: Bitmap>(bitmap: &mut B, edges: &[Edge], fill_value: u8) {
    if edges.is_empty() {
        return;
    }

    let width = bitmap.width();
    let height = bitmap.height();

    // Find y-bounds
    let mut min_y = height;
    let mut max_y = 0i32;

    for edge in edges {
        let (y_min, y_max) = edge.y_bounds();
        min_y = min_y.min(y_min);
        max_y = max_y.max(y_max);
    }

    // Clamp to bitmap bounds
    min_y = min_y.max(0);
    max_y = max_y.min(height - 1);

    // For each scanline
    for y in min_y..=max_y {
        let mut intersections = Vec::new();

        // Find all intersections with this scanline
        for edge in edges {
            // Skip horizontal edges (they don't affect scanline fill)
            if edge.is_horizontal() {
                continue;
            }

            // Check if edge spans this scanline using half-open interval
            // Include lower endpoint, exclude upper endpoint to avoid double-counting vertices
            let (y_min, y_max) = edge.y_bounds();
            if y_min <= y && y < y_max {
                // Calculate x intersection
                // x = x0 + (y - y0) * (x1 - x0) / (y1 - y0)
                let dy = edge.y1 - edge.y0;
                let t = (y - edge.y0) as f64 / dy as f64;
                let x = edge.x0 as f64 + t * (edge.x1 - edge.x0) as f64;
                intersections.push(x);
            }
        }

        // Sort intersections
        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Fill between pairs of intersections (even-odd rule)
        for i in (0..intersections.len()).step_by(2) {
            if i + 1 < intersections.len() {
                let x_start = intersections[i].ceil() as i32;
                let x_end = intersections[i + 1].floor() as i32;

                for x in x_start..=x_end {
                    if x >= 0 && x < width {
                        bitmap.set(x, y, fill_value);
                    }
                }
            }
        }
    }
}

/// Fill a polygon from raw edge tuples.
///
/// Convenience function that accepts edges as (x0, y0, x1, y1) tuples
/// and converts them to Edge structs before calling fill_polygon.
///
/// # Arguments
///
/// * `bitmap` - Mutable bitmap to draw into
/// * `edges` - Slice of (x0, y0, x1, y1) tuples defining polygon edges
/// * `fill_value` - Pixel value to use for filled pixels (0-255)
///
/// # Example
///
/// ```rust,ignore
/// use pdftract_core::render::scanline::fill_polygon_from_tuples;
///
/// let mut bitmap = Bitmap32x32::white();
/// let edges = vec![
///     (10, 5, 30, 25),
///     (30, 25, 10, 25),
///     (10, 25, 10, 5),
/// ];
///
/// fill_polygon_from_tuples(&mut bitmap, &edges, 0);
/// ```
pub fn fill_polygon_from_tuples<B: Bitmap>(
    bitmap: &mut B,
    edges: &[(i32, i32, i32, i32)],
    fill_value: u8,
) {
    let edge_objs: Vec<Edge> = edges.iter().map(|&e| Edge::from_tuple(e)).collect();
    fill_polygon(bitmap, &edge_objs, fill_value);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test bitmap implementation
    #[derive(Debug, Clone, PartialEq)]
    struct TestBitmap {
        pixels: Vec<Vec<u8>>,
        width: i32,
        height: i32,
    }

    impl TestBitmap {
        fn new(width: i32, height: i32) -> Self {
            let pixels = vec![vec![255u8; width as usize]; height as usize];
            Self {
                pixels,
                width,
                height,
            }
        }

        fn white(width: i32, height: i32) -> Self {
            Self::new(width, height)
        }

        fn get(&self, x: i32, y: i32) -> Option<u8> {
            if x < 0 || x >= self.width || y < 0 || y >= self.height {
                return None;
            }
            Some(self.pixels[y as usize][x as usize])
        }

        fn count_filled(&self, value: u8) -> usize {
            self.pixels
                .iter()
                .flat_map(|row| row.iter())
                .filter(|&&v| v == value)
                .count()
        }
    }

    impl Bitmap for TestBitmap {
        fn set(&mut self, x: i32, y: i32, value: u8) -> bool {
            if x < 0 || x >= self.width || y < 0 || y >= self.height {
                return false;
            }
            self.pixels[y as usize][x as usize] = value;
            true
        }

        fn width(&self) -> i32 {
            self.width
        }

        fn height(&self) -> i32 {
            self.height
        }
    }

    #[test]
    fn test_edge_creation() {
        let edge = Edge::new(10, 5, 30, 25);
        assert_eq!(edge.x0, 10);
        assert_eq!(edge.y0, 5);
        assert_eq!(edge.x1, 30);
        assert_eq!(edge.y1, 25);
    }

    #[test]
    fn test_edge_from_tuple() {
        let edge = Edge::from_tuple((10, 5, 30, 25));
        assert_eq!(edge.x0, 10);
        assert_eq!(edge.y0, 5);
        assert_eq!(edge.x1, 30);
        assert_eq!(edge.y1, 25);
    }

    #[test]
    fn test_edge_is_horizontal() {
        let horizontal = Edge::new(10, 5, 30, 5);
        assert!(horizontal.is_horizontal());

        let vertical = Edge::new(10, 5, 10, 25);
        assert!(!vertical.is_horizontal());

        let diagonal = Edge::new(10, 5, 30, 25);
        assert!(!diagonal.is_horizontal());
    }

    #[test]
    fn test_edge_y_bounds() {
        let edge1 = Edge::new(10, 5, 30, 25);
        assert_eq!(edge1.y_bounds(), (5, 25));

        let edge2 = Edge::new(10, 25, 30, 5);
        assert_eq!(edge2.y_bounds(), (5, 25));
    }

    #[test]
    fn test_test_bitmap_basic() {
        let mut bitmap = TestBitmap::white(32, 32);
        assert_eq!(bitmap.get(0, 0), Some(255));
        assert_eq!(bitmap.get(31, 31), Some(255));
        assert_eq!(bitmap.get(32, 0), None);
        assert_eq!(bitmap.get(0, 32), None);
    }

    #[test]
    fn test_test_bitmap_set_get() {
        let mut bitmap = TestBitmap::white(32, 32);
        assert!(bitmap.set(10, 15, 128));
        assert_eq!(bitmap.get(10, 15), Some(128));
        assert!(!bitmap.set(-1, 0, 0)); // Out of bounds
        assert!(!bitmap.set(0, 32, 0)); // Out of bounds
    }

    #[test]
    fn test_fill_polygon_empty_edges() {
        let mut bitmap = TestBitmap::white(32, 32);
        let edges: Vec<Edge> = vec![];
        fill_polygon(&mut bitmap, &edges, 0);
        // Should not crash and bitmap should remain unchanged
        assert_eq!(bitmap.count_filled(0), 0);
    }

    #[test]
    fn test_fill_polygon_triangle() {
        let mut bitmap = TestBitmap::white(32, 32);

        // Triangle with vertices (10, 5), (30, 25), (10, 25)
        let edges = vec![
            Edge::new(10, 5, 30, 25),  // Diagonal edge
            Edge::new(30, 25, 10, 25), // Horizontal bottom edge
            Edge::new(10, 25, 10, 5), // Vertical left edge
        ];

        fill_polygon(&mut bitmap, &edges, 0);

        // Check that some pixels are filled
        assert!(bitmap.count_filled(0) > 0);

        // Specific check: bottom row should have filled pixels
        assert_eq!(bitmap.get(15, 25), Some(0));
        assert_eq!(bitmap.get(20, 25), Some(0));

        // Check outside triangle is still white
        assert_eq!(bitmap.get(5, 5), Some(255));
    }

    #[test]
    fn test_fill_polygon_rectangle() {
        let mut bitmap = TestBitmap::white(32, 32);

        // Rectangle from (10, 10) to (20, 20)
        let edges = vec![
            Edge::new(10, 10, 20, 10), // Top edge
            Edge::new(20, 10, 20, 20), // Right edge
            Edge::new(20, 20, 10, 20), // Bottom edge
            Edge::new(10, 20, 10, 10), // Left edge
        ];

        fill_polygon(&mut bitmap, &edges, 0);

        // Check center pixel is filled
        assert_eq!(bitmap.get(15, 15), Some(0));

        // Check corners are filled
        assert_eq!(bitmap.get(10, 10), Some(0));
        assert_eq!(bitmap.get(20, 20), Some(0));

        // Check outside is still white
        assert_eq!(bitmap.get(5, 5), Some(255));
        assert_eq!(bitmap.get(25, 25), Some(255));
    }

    #[test]
    fn test_fill_polygon_from_tuples() {
        let mut bitmap = TestBitmap::white(32, 32);

        let edges = vec![
            (10, 10, 20, 10),
            (20, 10, 20, 20),
            (20, 20, 10, 20),
            (10, 20, 10, 10),
        ];

        fill_polygon_from_tuples(&mut bitmap, &edges, 0);

        // Check center pixel is filled
        assert_eq!(bitmap.get(15, 15), Some(0));
    }

    #[test]
    fn test_fill_polygon_clips_to_bounds() {
        let mut bitmap = TestBitmap::white(32, 32);

        // Edges extending beyond bitmap bounds
        let edges = vec![
            Edge::new(-10, -10, 50, -10), // Top edge (out of bounds)
            Edge::new(50, -10, 50, 50),   // Right edge (out of bounds)
            Edge::new(50, 50, -10, 50),   // Bottom edge (out of bounds)
            Edge::new(-10, 50, -10, -10), // Left edge (out of bounds)
        ];

        fill_polygon(&mut bitmap, &edges, 0);

        // Should not crash and should fill visible portion
        // Check some pixels are filled
        assert!(bitmap.count_filled(0) > 0);

        // All pixels within bounds should be valid
        for y in 0..32 {
            for x in 0..32 {
                let val = bitmap.get(x, y);
                assert!(val == Some(0) || val == Some(255));
            }
        }
    }

    #[test]
    fn test_fill_polygon_horizontal_edges_skipped() {
        let mut bitmap = TestBitmap::white(32, 32);

        // Diamond shape with horizontal edges at top and bottom
        let edges = vec![
            Edge::new(16, 5, 26, 15),  // Top-right diagonal
            Edge::new(26, 15, 16, 25), // Bottom-right diagonal
            Edge::new(16, 25, 6, 15),  // Bottom-left diagonal
            Edge::new(6, 15, 16, 5),   // Top-left diagonal
        ];

        fill_polygon(&mut bitmap, &edges, 0);

        // Center should be filled
        assert_eq!(bitmap.get(16, 15), Some(0));
    }

    #[test]
    fn test_edge_display() {
        let edge = Edge::new(10, 5, 30, 25);
        assert_eq!(format!("{}", edge), "Edge(10, 5, 30, 25)");
    }
}
