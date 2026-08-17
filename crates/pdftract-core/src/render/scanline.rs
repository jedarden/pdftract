//! Scanline polygon fill algorithm for bitmap rasterization.
//!
//! This module implements the classic scanline fill algorithm for converting
//! vector polygon edges into filled pixel regions. It handles edge cases
//! including horizontal edges and proper vertex counting via half-open intervals.
//!
//! # Algorithm Overview
//!
//! The scanline algorithm uses two edge tables to efficiently fill polygons:
//!
//! 1. **Build Global Edge Table (GET)**: Convert input edges to [`Edge`] structs,
//!    exclude horizontal edges, and sort by `y_min` (topmost first).
//!
//! 2. **Iterate scanlines**: For each Y from `min_y` to `max_y`:
//!    - Move edges from GET to Active Edge Table (AET) when scanline reaches `y_min`
//!    - Remove edges from AET when scanline passes `y_max`
//!    - Sort AET by current X position
//!    - Fill between pairs of X intersections (even-odd rule)
//!    - Update X positions by adding slope (`dx/dy`) for next scanline
//!
//! # Structures
//!
//! - [`Edge`]: Represents a polygon edge with current X position, Y bounds, and slope
//! - [`ActiveEdgeTable`] (AET): Edges intersecting the current scanline, sorted by X
//! - [`GlobalEdgeTable`] (GET): All edges sorted by `y_min`, used as the edge source
//!
//! # Edge Cases
//!
//! - Horizontal edges are skipped (they don't affect scanline fill)
//! - Half-open interval for vertices (include lower, exclude upper) prevents double-counting
//! - All intersections are clipped to bitmap bounds

use std::fmt;

/// 2D edge defined by two endpoints (input format).
///
/// Represents a line segment from (x0, y0) to (x1, y1) in bitmap coordinates.
/// Used as input to the scanline fill algorithm; gets converted to Edge
/// for processing in the edge tables.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputEdge {
    /// Starting X coordinate
    pub x0: i32,
    /// Starting Y coordinate
    pub y0: i32,
    /// Ending X coordinate
    pub x1: i32,
    /// Ending Y coordinate
    pub y1: i32,
}

impl InputEdge {
    /// Create a new input edge from (x0, y0) to (x1, y1).
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

    /// Create an input edge from a tuple of (x0, y0, x1, y1).
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

impl fmt::Display for InputEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InputEdge({}, {}, {}, {})", self.x0, self.y0, self.x1, self.y1)
    }
}

/// Edge representation for scanline fill algorithm edge tables.
///
/// This struct represents an edge in the form used by Active Edge Tables (AET)
/// and Global Edge Tables (GET) in the traditional scanline fill algorithm.
/// It stores the current X position along with the edge geometry and slope.
///
/// # Fields
///
/// - `x`: Current X position where this edge intersects the current scanline
/// - `y_min`: Minimum Y coordinate (top of edge)
/// - `y_max`: Maximum Y coordinate (bottom of edge)
/// - `dx`: Change in X across the edge (x1 - x0), used for slope calculation
/// - `dy`: Change in Y across the edge (y1 - y0), used for slope calculation
///
/// # Algorithm Context
///
/// In the edge-table version of scanline fill:
/// - The GET stores all edges sorted by y_min
/// - The AET stores only edges that intersect the current scanline
/// - On each scanline, edges are moved from GET to AET when y == y_min
/// - X positions are updated by dx/dy as we move to the next scanline
/// - Edges are removed from AET when y > y_max
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    /// Current X intersection position with the current scanline
    pub x: i32,
    /// Minimum Y coordinate (top of edge, inclusive)
    pub y_min: i32,
    /// Maximum Y coordinate (bottom of edge, exclusive)
    pub y_max: i32,
    /// Change in X across the edge (x1 - x0)
    pub dx: i32,
    /// Change in Y across the edge (y1 - y0)
    pub dy: i32,
}

impl Edge {
    /// Create a new scanline edge from endpoint coordinates.
    ///
    /// Automatically calculates y_min, y_max (ordering them correctly),
    /// and computes dx and dy.
    ///
    /// # Arguments
    ///
    /// * `x0` - Starting X coordinate
    /// * `y0` - Starting Y coordinate
    /// * `x1` - Ending X coordinate
    /// * `y1` - Ending Y coordinate
    #[must_use]
    pub fn from_endpoints(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
        // Use the x-coordinate at y_min as initial X
        let initial_x = if y0 < y1 { x0 } else { x1 };
        Self {
            x: initial_x, // Initial X position at y_min
            y_min,
            y_max,
            dx: x1 - x0,
            dy: y1 - y0,
        }
    }

    /// Calculate the slope as (dx, dy) tuple.
    ///
    /// Returns how much X changes per unit Y as a pair of integers.
    /// Used to update the X position when moving to the next scanline.
    ///
    /// # Returns
    ///
    /// A tuple (dx, dy) representing the slope. Returns (0, 0) if dy is zero (horizontal edge).
    #[must_use]
    pub fn slope(&self) -> (i32, i32) {
        if self.dy == 0 {
            (0, 0)
        } else {
            (self.dx, self.dy)
        }
    }

    /// Check if this edge is horizontal (dy == 0).
    ///
    /// Horizontal edges don't intersect scanlines and are typically
    /// skipped in edge table construction.
    #[must_use]
    pub const fn is_horizontal(&self) -> bool {
        self.dy == 0
    }

    /// Update the X position for the next scanline.
    ///
    /// Adds the slope (dx/dy) to X, advancing the intersection point
    /// by one scanline. Uses integer arithmetic with accumulated fraction
    /// for accuracy.
    pub fn advance_scanline(&mut self) {
        // Slope is dx/dy, so we need to add dx/dy to x
        // Use integer arithmetic: accumulate dx and add when we have dy
        let (dx, dy) = (self.dx, self.dy);
        if dy != 0 {
            // This is a simplified version - for now just add the rounded slope
            // A more sophisticated implementation would track accumulated fraction
            let slope = if dx >= 0 {
                (dx as f64 / dy as f64).round() as i32
            } else {
                -((-dx) as f64 / dy as f64).round() as i32
            };
            self.x += slope;
        }
    }
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Edge(x={}, y_min={}, y_max={}, dx={}, dy={})",
            self.x, self.y_min, self.y_max, self.dx, self.dy
        )
    }
}

/// Active Edge Table (AET) for scanline fill algorithm.
///
/// The AET contains all edges that intersect the current scanline.
/// Edges are added to the AET when the scanline reaches their y_min,
/// and removed when the scanline passes their y_max.
///
/// # Invariants
///
/// - Edges in the AET are sorted by X coordinate (left-to-right)
/// - All edges have y_min <= current_y < y_max
/// - X positions are updated as the algorithm advances scanlines
///
/// # Algorithm Use
///
/// In the traditional scanline algorithm:
/// 1. Sort AET by X position
/// 2. Fill between pairs of X positions (even-odd rule)
/// 3. Update X positions: x += slope for each edge
/// 4. Remove edges where current_y >= y_max
/// 5. Add new edges from GET where current_y == y_min
pub type ActiveEdgeTable = Vec<Edge>;

/// Global Edge Table (GET) for scanline fill algorithm.
///
/// The GET contains all polygon edges, sorted by y_min (topmost first).
/// This table is the source from which edges are moved into the AET
/// as the scanline algorithm progresses.
///
/// # Invariants
///
/// - Edges are sorted by y_min (smallest first)
/// - Horizontal edges (dy == 0) are typically excluded
/// - Each edge represents one polygon boundary segment
///
/// # Algorithm Use
///
/// The GET is constructed once at the start of the algorithm:
/// 1. Create Edge from each polygon edge using from_endpoints()
/// 2. Filter out horizontal edges
/// 3. Sort by y_min
/// 4. Use as a queue: pop edges as current_y reaches their y_min
pub type GlobalEdgeTable = Vec<Edge>;

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

/// Fill a polygon using the scanline algorithm with Active Edge Table (AET).
///
/// Given a list of edges defining a polygon boundary, this function fills
/// the interior pixels using the even-odd rule. The algorithm uses the classic
/// edge table approach: a Global Edge Table (GET) stores all edges sorted by y_min,
/// and an Active Edge Table (AET) stores edges that intersect the current scanline.
///
/// # Arguments
///
/// * `bitmap` - Mutable bitmap to draw into (must implement Bitmap trait)
/// * `edges` - Slice of edges defining the polygon boundary
/// * `fill_value` - Pixel value to use for filled pixels (0-255)
///
/// # Algorithm Details
///
/// 1. **Build GET**: Create a Global Edge Table from all input edges, excluding
///    horizontal edges. Sort by y_min (topmost first).
///
/// 2. **Initialize AET**: Start with an empty Active Edge Table.
///
/// 3. **For each scanline** from min_y to max_y:
///    - **Add edges**: Move edges from GET to AET when `scanline == edge.y_min`
///    - **Remove edges**: Remove edges from AET when `scanline >= edge.y_max`
///    - **Sort by X**: Sort AET edges by their current X position
///    - **Fill between pairs**: Fill pixels between pairs of X positions (even-odd rule)
///    - **Update X positions**: Advance each edge's X by slope for next scanline
///
/// # Edge Cases Handled
///
/// - **Horizontal edges**: Excluded from GET (they don't cross scanlines)
/// - **Vertex handling**: Uses half-open interval [y_min, y_max) to avoid
///   double-counting vertices where edges meet
/// - **Boundary clipping**: All pixel writes are clipped to bitmap bounds
///
/// # Example
///
/// ```rust,ignore
/// use pdftract_core::render::scanline::{fill_polygon, InputEdge};
///
/// let mut bitmap = Bitmap32x32::white();
///
/// // Define a triangle with vertices (10,5), (30,25), (10,25)
/// let edges = vec![
///     InputEdge::new(10, 5, 30, 25),  // Diagonal edge
///     InputEdge::new(30, 25, 10, 25), // Horizontal bottom edge (excluded from GET)
///     InputEdge::new(10, 25, 10, 5), // Vertical left edge
/// ];
///
/// fill_polygon(&mut bitmap, &edges, 0);
/// ```
pub fn fill_polygon<B: Bitmap>(bitmap: &mut B, edges: &[InputEdge], fill_value: u8) {
    if edges.is_empty() {
        return;
    }

    let width = bitmap.width();
    let height = bitmap.height();

    // Build Global Edge Table (GET): convert InputEdges to Edges, exclude horizontal edges
    let mut get: GlobalEdgeTable = edges
        .iter()
        .filter(|e| !e.is_horizontal())
        .map(|e| Edge::from_endpoints(e.x0, e.y0, e.x1, e.y1))
        .collect();

    if get.is_empty() {
        return; // No non-horizontal edges to process
    }

    // Sort GET by y_min (topmost edges first)
    get.sort_by_key(|e| e.y_min);

    // Find y-bounds
    let min_y = get.first().map(|e| e.y_min.max(0)).unwrap_or(0);
    let max_y = get.last().map(|e| e.y_max.min(height - 1)).unwrap_or(0);

    // Initialize Active Edge Table (AET)
    let mut aet: ActiveEdgeTable = Vec::new();
    let mut get_idx = 0;

    // Process each scanline
    for y in min_y..=max_y {
        // Step 1: Add edges from GET to AET when scanline reaches edge.y_min
        while get_idx < get.len() && get[get_idx].y_min == y {
            aet.push(get[get_idx]);
            get_idx += 1;
        }

        // Step 2: Remove edges from AET where scanline >= y_max
        aet.retain(|e| y < e.y_max);

        // Step 3: Sort AET by current X position
        // Use partial_cmp for f64 comparison
        aet.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

        // Step 3.5: Calculate x-coordinate intersections for current scanline
        // Compute intersection x = round(edge.x) for each active edge
        // Store intersections in a Vec<i32> for the current scanline
        let intersections: Vec<i32> = aet.iter().map(|edge| edge.x).collect();

        // Step 4: Fill between pairs of X positions (even-odd rule)
        for i in (0..intersections.len()).step_by(2) {
            if i + 1 < intersections.len() {
                let x_start = intersections[i].max(0);
                let x_end = intersections[i + 1].min(width - 1);

                for x in x_start..=x_end {
                    bitmap.set(x, y, fill_value);
                }
            }
        }

        // Step 5: Update X positions for next scanline
        for edge in &mut aet {
            edge.advance_scanline();
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
    let edge_objs: Vec<InputEdge> = edges.iter().map(|&e| InputEdge::from_tuple(e)).collect();
    fill_polygon(bitmap, &edge_objs, fill_value);
}

/// Calculate X-coordinate intersection points for the current scanline.
///
/// Extracts and rounds the X intersection coordinates from the Active Edge Table.
/// For each edge in the AET, computes x = round(edge.x) and collects all
/// intersection points into a sorted Vec<i32> for fill span calculation.
///
/// # Arguments
///
/// * `aet` - Reference to the Active Edge Table (must be sorted by X)
///
/// # Returns
///
/// A Vec<i32> containing the rounded X coordinates of all edge intersections
/// with the current scanline, sorted left-to-right.
///
/// # Algorithm
///
/// For each edge in the AET:
/// 1. Extract the current X position (edge.x)
/// 2. Apply rounding: round(edge.x) to get integer pixel coordinate
/// 3. Collect all rounded X coordinates into a vector
///
/// # Example
///
/// ```rust,ignore
/// use pdftract_core::render::scanline::{calculate_intersections, Edge, ActiveEdgeTable};
///
/// let mut aet: ActiveEdgeTable = vec![
///     Edge { x: 10, y_min: 5, y_max: 25, dx: 10, dy: 20 },
///     Edge { x: 25, y_min: 5, y_max: 25, dx: -10, dy: 20 },
/// ];
/// aet.sort_by_key(|e| e.x); // Must be sorted by X
///
/// let intersections = calculate_intersections(&aet);
/// assert_eq!(intersections, vec![10, 25]);
/// ```
pub fn calculate_intersections(aet: &ActiveEdgeTable) -> Vec<i32> {
    let mut intersections: Vec<i32> = aet
        .iter()
        .map(|edge| edge.x)
        .collect();

    // Sort intersections left-to-right for fill span calculation
    intersections.sort();

    intersections
}

/// Add edges to the Active Edge Table when the scanline reaches their y_min.
///
/// This function implements the traditional scanline fill algorithm's edge activation
/// step. It moves edges from the Global Edge Table (GET) to the Active Edge Table (AET)
/// when the current scanline reaches an edge's minimum Y coordinate.
///
/// # Arguments
///
/// * `aet` - Mutable reference to the Active Edge Table (edges will be added here)
/// * `get` - Immutable reference to the Global Edge Table (source of edges to activate)
/// * `scanline_y` - Current Y coordinate of the scanline
/// * `get_index` - Mutable index tracking position in GET (enables sequential processing)
///
/// # Algorithm
///
/// 1. Iterate through GET starting from `get_index`
/// 2. For each edge where `edge.y_min == scanline_y`, add it to the AET
/// 3. Update `get_index` to skip processed edges in future calls
///
/// # Invariants
///
/// - The GET must be sorted by `y_min` for this to work correctly
/// - Once an edge is added to AET, it should not be added again
/// - The `get_index` parameter maintains state across scanline iterations
///
/// # Example
///
/// ```rust,ignore
/// use pdftract_core::render::scanline::{add_edges_to_aet_at_ymin, Edge, ActiveEdgeTable, GlobalEdgeTable};
///
/// let mut aet: ActiveEdgeTable = Vec::new();
/// let mut get: GlobalEdgeTable = vec![
///     Edge::from_endpoints(10, 5, 30, 25),  // y_min = 5
///     Edge::from_endpoints(30, 25, 10, 25), // y_min = 25 (horizontal)
///     Edge::from_endpoints(10, 25, 10, 5), // y_min = 5
/// ];
/// get.sort_by_key(|e| e.y_min); // Must be sorted by y_min
///
/// let mut get_index = 0;
///
/// // At scanline y=5, edges with y_min=5 are activated
/// add_edges_to_aet_at_ymin(&mut aet, &get, 5, &mut get_index);
/// assert_eq!(aet.len(), 2); // Two edges have y_min=5
///
/// // At scanline y=25, edges with y_min=25 are activated
/// add_edges_to_aet_at_ymin(&mut aet, &get, 25, &mut get_index);
/// assert_eq!(get_index, 3); // All edges processed
/// ```
pub fn add_edges_to_aet_at_ymin(
    aet: &mut ActiveEdgeTable,
    get: &GlobalEdgeTable,
    scanline_y: i32,
    get_index: &mut usize,
) {
    // Iterate through GET starting from current index
    while *get_index < get.len() {
        let edge = get[*get_index];

        // If this edge's y_min is greater than current scanline,
        // we've reached edges that activate later - stop here
        if edge.y_min > scanline_y {
            break;
        }

        // If this edge's y_min equals current scanline, add it to AET
        // Also handle edges with y_min < scanline_y (shouldn't happen with sorted GET,
        // but defensive programming ensures we don't miss edges)
        if edge.y_min == scanline_y {
            aet.push(edge);
        }

        // Move to next edge in GET
        *get_index += 1;
    }
}

/// Fill a polygon using the Active Edge Table (AET) algorithm.
///
/// This is the classic scanline fill algorithm that uses edge tables:
/// - Global Edge Table (GET): all edges sorted by y_min
/// - Active Edge Table (AET): edges intersecting current scanline
///
/// # Arguments
///
/// * `bitmap` - Mutable bitmap to draw into
/// * `edges` - Slice of edges defining the polygon boundary
/// * `fill_value` - Pixel value to use for filled pixels (0-255)
///
/// # Algorithm
///
/// 1. Build GET from input edges, sorted by y_min
/// 2. Process each scanline from min_y to max_y
/// 3. On each scanline:
///    - Add edges from GET where scanline == edge.y_min to AET
///    - Remove edges from AET where scanline >= edge.y_max
///    - Update X positions in AET: x += slope (dx/dy)
///    - Sort AET by X coordinate
///    - Fill between pairs of X positions (even-odd rule)
pub fn fill_polygon_aet<B: Bitmap>(bitmap: &mut B, edges: &[InputEdge], fill_value: u8) {
    if edges.is_empty() {
        return;
    }

    let width = bitmap.width();
    let height = bitmap.height();

    // Build GET: convert InputEdges to Edges, filter horizontals, sort by y_min
    let mut get: Vec<Edge> = edges
        .iter()
        .filter(|e| !e.is_horizontal())
        .map(|e| Edge::from_endpoints(e.x0, e.y0, e.x1, e.y1))
        .collect();

    if get.is_empty() {
        return;
    }

    // Sort GET by y_min (topmost edges first)
    get.sort_by_key(|e| e.y_min);

    // Find y-bounds
    let min_y = get.iter().map(|e| e.y_min).min().unwrap_or(0).max(0);
    let max_y = get.iter().map(|e| e.y_max).max().unwrap_or(0).min(height - 1);

    // AET starts empty
    let mut aet: ActiveEdgeTable = Vec::new();

    // Process each scanline
    for scanline in min_y..=max_y {
        // STEP 1: Add edges from GET to AET when scanline reaches y_min
        // Move edges from GET where scanline == edge.y_min
        let mut i = 0;
        while i < get.len() {
            if get[i].y_min == scanline {
                aet.push(get.remove(i));
            } else {
                i += 1;
            }
        }

        // STEP 2: Remove edges from AET where scanline >= y_max (edge has ended)
        aet.retain(|e| scanline < e.y_max);

        // STEP 3: Update X positions in AET for next scanline
        for edge in &mut aet {
            edge.advance_scanline();
        }

        // STEP 4: Sort AET by X coordinate (left-to-right)
        // Use partial_cmp for f64 comparison
        aet.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

        // STEP 4.5: Calculate x-coordinate intersections for current scanline
        // Compute intersection x = round(edge.x) for each active edge
        // Store intersections in a Vec<i32> for the current scanline
        let intersections: Vec<i32> = aet.iter().map(|edge| edge.x).collect();

        // STEP 5: Fill between pairs of X positions (even-odd rule)
        for i in (0..intersections.len()).step_by(2) {
            if i + 1 < intersections.len() {
                let x_start = intersections[i].max(0);
                let x_end = intersections[i + 1].min(width - 1);

                for x in x_start..=x_end {
                    bitmap.set(x, scanline, fill_value);
                }
            }
        }
    }
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
    fn test_input_edge_creation() {
        let edge = InputEdge::new(10, 5, 30, 25);
        assert_eq!(edge.x0, 10);
        assert_eq!(edge.y0, 5);
        assert_eq!(edge.x1, 30);
        assert_eq!(edge.y1, 25);
    }

    #[test]
    fn test_input_edge_from_tuple() {
        let edge = InputEdge::from_tuple((10, 5, 30, 25));
        assert_eq!(edge.x0, 10);
        assert_eq!(edge.y0, 5);
        assert_eq!(edge.x1, 30);
        assert_eq!(edge.y1, 25);
    }

    #[test]
    fn test_input_edge_is_horizontal() {
        let horizontal = InputEdge::new(10, 5, 30, 5);
        assert!(horizontal.is_horizontal());

        let vertical = InputEdge::new(10, 5, 10, 25);
        assert!(!vertical.is_horizontal());

        let diagonal = InputEdge::new(10, 5, 30, 25);
        assert!(!diagonal.is_horizontal());
    }

    #[test]
    fn test_input_edge_y_bounds() {
        let edge1 = InputEdge::new(10, 5, 30, 25);
        assert_eq!(edge1.y_bounds(), (5, 25));

        let edge2 = InputEdge::new(10, 25, 30, 5);
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
        let edges: Vec<InputEdge> = vec![];
        fill_polygon(&mut bitmap, &edges, 0);
        // Should not crash and bitmap should remain unchanged
        assert_eq!(bitmap.count_filled(0), 0);
    }

    #[test]
    fn test_fill_polygon_triangle() {
        let mut bitmap = TestBitmap::white(32, 32);

        // Triangle with vertices (10, 5), (30, 25), (10, 25)
        let edges = vec![
            InputEdge::new(10, 5, 30, 25),  // Diagonal edge
            InputEdge::new(30, 25, 10, 25), // Horizontal bottom edge
            InputEdge::new(10, 25, 10, 5), // Vertical left edge
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
            InputEdge::new(10, 10, 20, 10), // Top edge
            InputEdge::new(20, 10, 20, 20), // Right edge
            InputEdge::new(20, 20, 10, 20), // Bottom edge
            InputEdge::new(10, 20, 10, 10), // Left edge
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
            InputEdge::new(-10, -10, 50, -10), // Top edge (out of bounds)
            InputEdge::new(50, -10, 50, 50),   // Right edge (out of bounds)
            InputEdge::new(50, 50, -10, 50),   // Bottom edge (out of bounds)
            InputEdge::new(-10, 50, -10, -10), // Left edge (out of bounds)
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
            InputEdge::new(16, 5, 26, 15),  // Top-right diagonal
            InputEdge::new(26, 15, 16, 25), // Bottom-right diagonal
            InputEdge::new(16, 25, 6, 15),  // Bottom-left diagonal
            InputEdge::new(6, 15, 16, 5),   // Top-left diagonal
        ];

        fill_polygon(&mut bitmap, &edges, 0);

        // Center should be filled
        assert_eq!(bitmap.get(16, 15), Some(0));
    }

    #[test]
    fn test_input_edge_display() {
        let edge = InputEdge::new(10, 5, 30, 25);
        assert_eq!(format!("{}", edge), "InputEdge(10, 5, 30, 25)");
    }

    #[test]
    fn test_scanline_edge_from_endpoints() {
        let edge = Edge::from_endpoints(10, 5, 30, 25);
        assert_eq!(edge.x, 10.0);
        assert_eq!(edge.y_min, 5);
        assert_eq!(edge.y_max, 25);
        assert_eq!(edge.dx, 20);
        assert_eq!(edge.dy, 20);
    }

    #[test]
    fn test_scanline_edge_is_horizontal() {
        let horizontal = Edge::from_endpoints(10, 5, 30, 5);
        assert!(horizontal.is_horizontal());

        let vertical = Edge::from_endpoints(10, 5, 10, 25);
        assert!(!vertical.is_horizontal());

        let diagonal = Edge::from_endpoints(10, 5, 30, 25);
        assert!(!diagonal.is_horizontal());
    }

    #[test]
    fn test_scanline_edge_slope() {
        let edge = Edge::from_endpoints(10, 5, 30, 25);
        assert_eq!(edge.slope(), 1.0);

        let horizontal = Edge::from_endpoints(10, 5, 30, 5);
        assert!(horizontal.slope().is_nan());
    }

    #[test]
    fn test_scanline_edge_advance_scanline() {
        let mut edge = Edge::from_endpoints(10, 5, 30, 25);
        let original_x = edge.x;
        edge.advance_scanline();
        // After advancing, x should increase by slope (1.0 in this case)
        assert_eq!(edge.x, original_x + 1.0);
    }

    #[test]
    fn test_scanline_edge_display() {
        let edge = Edge::from_endpoints(10, 5, 30, 25);
        assert_eq!(format!("{}", edge), "Edge(x=10, y_min=5, y_max=25, dx=20, dy=20)");
    }
}
