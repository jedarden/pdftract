//! Path data structures for bitmap rasterization.
//!
//! This module defines the core data structures used to represent
//! vector paths for scanline rasterization to bitmaps.
//!
//! # Overview
//!
//! Path data structures model PDF graphics paths as sequences of commands
//! that draw lines, curves, and shapes. These commands are collected during
//! content stream parsing and then rasterized to pixel bitmaps.
//!
//! # Example
//!
//! ```rust
//! use pdftract_core::render::path::{Point, CurrentPath, PathCommand};
//!
//! let mut path = CurrentPath::new();
//! path.move_to(Point::new(10.0, 20.0));
//! path.line_to(Point::new(50.0, 60.0));
//! path.close_path();
//!
//! for command in &path.commands {
//!     match command {
//!         PathCommand::MoveTo(p) => println!("Move to {:?}", p),
//!         PathCommand::LineTo(p) => println!("Line to {:?}", p),
//!         PathCommand::ClosePath => println!("Close path"),
//!         _ => {}
//!     }
//! }
//! ```

/// 2D point in user space coordinates.
///
/// Represents a position in PDF user space, where coordinates are
/// expressed in points (1/72 inch). The Y axis increases upward.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// X coordinate (horizontal position)
    pub x: f64,
    /// Y coordinate (vertical position)
    pub y: f64,
}

impl Point {
    /// Create a new Point with the given coordinates.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in user space
    /// * `y` - Y coordinate in user space
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::render::path::Point;
    ///
    /// let p = Point::new(10.0, 20.0);
    /// assert_eq!(p.x, 10.0);
    /// assert_eq!(p.y, 20.0);
    /// ```
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Create a Point at the origin (0, 0).
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::render::path::Point;
    ///
    /// let p = Point::origin();
    /// assert_eq!(p.x, 0.0);
    /// assert_eq!(p.y, 0.0);
    /// ```
    #[must_use]
    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Path construction command.
///
/// Represents a single command in a PDF graphics path. Commands are
/// executed sequentially to build up vector shapes that can then be
/// filled or stroked (rasterized to bitmaps).
///
/// # PDF Graphics Operators
///
/// Most variants correspond to PDF content stream operators:
/// - `MoveTo` → `m x y` operator
/// - `LineTo` → `l x y` operator
/// - `CubicTo` → `c x1 y1 x2 y2 x3 y3` operator
/// - `ShorthandCubicTo` → `v x2 y2 x3 y3` operator (first control point implied)
/// - `ShorthandCubicToY` → `y x1 y1 x3 y3` operator (second control point implied)
/// - `Rect` → `re x y width height` operator
/// - `ClosePath` → `h` operator
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    /// Move to absolute position, starting a new subpath.
    ///
    /// Corresponds to PDF `m x y` operator.
    /// Updates the current point without drawing.
    MoveTo(Point),

    /// Line to absolute position.
    ///
    /// Corresponds to PDF `l x y` operator.
    /// Draws a straight line from the current point to the given position.
    LineTo(Point),

    /// Cubic Bézier curve with two explicit control points.
    ///
    /// Corresponds to PDF `c x1 y1 x2 y2 x3 y3` operator.
    /// Draws a curve from the current point to (x3, y3) using
    /// control points (x1, y1) and (x2, y2).
    CubicTo(Point, Point, Point),

    /// Cubic Bézier curve with first control point implied.
    ///
    /// Corresponds to PDF `v x2 y2 x3 y3` operator.
    /// The first control point is the current point (reflection symmetry).
    /// Draws a curve from the current point to (x3, y3) using
    /// control points (current_point reflected) and (x2, y2).
    ShorthandCubicTo(Point, Point),

    /// Cubic Bézier curve with second control point implied.
    ///
    /// Corresponds to PDF `y x1 y1 x3 y3` operator.
    /// The second control point is the end point (reflection symmetry).
    /// Draws a curve from the current point to (x3, y3) using
    /// control points (x1, y1) and (x3, y3).
    ShorthandCubicToY(Point, Point),

    /// Rectangle.
    ///
    /// Corresponds to PDF `re x y width height` operator.
    /// Appends a rectangle as a complete subpath (equivalent to
    /// moving to (x,y), lining to (x+w,y), (x+w,y+h), (x,y+h), and closing).
    Rect(f64, f64, f64, f64),

    /// Close the current subpath.
    ///
    /// Corresponds to PDF `h` operator.
    /// Draws a straight line from the current point back to the start
    /// point of the current subpath (established by the most recent MoveTo).
    ClosePath,
}

/// Current path being constructed.
///
/// Collects a sequence of path commands to form a complete vector shape.
/// Tracks the current point (position of the last drawing operation) and
/// the move point (start of the current subpath) for use in close operations.
///
/// # State Tracking
///
/// - `commands`: Ordered sequence of path commands
/// - `current_point`: Position of the last drawing operation endpoint
/// - `move_point`: Start point of the current subpath (set by MoveTo)
#[derive(Debug, Clone, Default)]
pub struct CurrentPath {
    /// Ordered sequence of path commands
    pub commands: Vec<PathCommand>,
    /// Current drawing position (endpoint of last command)
    current_point: Option<Point>,
    /// Start point of current subpath (for close operations)
    move_point: Option<Point>,
}

impl CurrentPath {
    /// Create a new empty path.
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::render::path::CurrentPath;
    ///
    /// let path = CurrentPath::new();
    /// assert!(path.commands.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a new subpath at the given position.
    ///
    /// Updates both current_point and move_point to the given position.
    /// Does not draw anything; this is a positioning operation.
    ///
    /// # Arguments
    ///
    /// * `p` - Position to move to
    ///
    /// # Corresponds To
    ///
    /// PDF `m x y` operator
    pub fn move_to(&mut self, p: Point) {
        self.commands.push(PathCommand::MoveTo(p));
        self.current_point = Some(p);
        self.move_point = Some(p);
    }

    /// Draw a straight line to the given position.
    ///
    /// Draws from the current point to the given position,
    /// then updates current_point to the new position.
    ///
    /// # Arguments
    ///
    /// * `p` - Position to draw line to
    ///
    /// # Corresponds To
    ///
    /// PDF `l x y` operator
    pub fn line_to(&mut self, p: Point) {
        self.commands.push(PathCommand::LineTo(p));
        self.current_point = Some(p);
    }

    /// Draw a cubic Bézier curve with explicit control points.
    ///
    /// Draws from the current point to `end` using the two control points.
    ///
    /// # Arguments
    ///
    /// * `c1` - First control point (pulls curve away from start)
    /// * `c2` - Second control point (pulls curve toward end)
    /// * `end` - Endpoint of the curve
    ///
    /// # Corresponds To
    ///
    /// PDF `c x1 y1 x2 y2 x3 y3` operator
    pub fn cubic_to(&mut self, c1: Point, c2: Point, end: Point) {
        self.commands.push(PathCommand::CubicTo(c1, c2, end));
        self.current_point = Some(end);
    }

    /// Draw a cubic Bézier curve with first control point implied.
    ///
    /// The first control point is symmetric to the second control point
    /// of the previous curve (reflection symmetry), providing smooth joins.
    ///
    /// # Arguments
    ///
    /// * `c2` - Second control point
    /// * `end` - Endpoint of the curve
    ///
    /// # Corresponds To
    ///
    /// PDF `v x2 y2 x3 y3` operator
    pub fn shorthand_cubic_to(&mut self, c2: Point, end: Point) {
        self.commands.push(PathCommand::ShorthandCubicTo(c2, end));
        self.current_point = Some(end);
    }

    /// Draw a cubic Bézier curve with second control point implied.
    ///
    /// The second control point is the endpoint (reflection symmetry).
    ///
    /// # Arguments
    ///
    /// * `c1` - First control point
    /// * `end` - Endpoint of the curve (also serves as second control point)
    ///
    /// # Corresponds To
    ///
    /// PDF `y x1 y1 x3 y3` operator
    pub fn shorthand_cubic_to_y(&mut self, c1: Point, end: Point) {
        self.commands.push(PathCommand::ShorthandCubicToY(c1, end));
        self.current_point = Some(end);
    }

    /// Append a rectangle as a complete subpath.
    ///
    /// The rectangle is added as a closed subpath consisting of four line segments.
    /// Updates both current_point and move_point to the rectangle's origin.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of rectangle's lower-left corner
    /// * `y` - Y coordinate of rectangle's lower-left corner
    /// * `width` - Rectangle width
    /// * `height` - Rectangle height
    ///
    /// # Corresponds To
    ///
    /// PDF `re x y width height` operator
    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.commands.push(PathCommand::Rect(x, y, width, height));
        self.current_point = Some(Point::new(x, y));
        self.move_point = Some(Point::new(x, y));
    }

    /// Close the current subpath.
    ///
    /// Draws a straight line from the current point back to the subpath's
    /// starting point (established by the most recent MoveTo).
    /// Updates current_point to the move point.
    ///
    /// # Corresponds To
    ///
    /// PDF `h` operator
    pub fn close_path(&mut self) {
        self.commands.push(PathCommand::ClosePath);
        if let Some(start) = self.move_point {
            self.current_point = Some(start);
        }
    }

    /// Clear all path commands and reset state.
    ///
    /// Removes all commands and resets both current_point and move_point to None.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_point = None;
        self.move_point = None;
    }

    /// Get the current drawing position.
    ///
    /// Returns the position of the last drawing operation's endpoint,
    /// or None if no position has been established yet.
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::render::path::{CurrentPath, Point};
    ///
    /// let mut path = CurrentPath::new();
    /// assert!(path.current_point().is_none());
    ///
    /// path.move_to(Point::new(10.0, 20.0));
    /// assert_eq!(path.current_point(), Some(Point::new(10.0, 20.0)));
    /// ```
    #[must_use]
    pub fn current_point(&self) -> Option<Point> {
        self.current_point
    }

    /// Get the starting point of the current subpath.
    ///
    /// Returns the position established by the most recent MoveTo operation,
    /// or None if no subpath has been started.
    #[must_use]
    pub fn move_point(&self) -> Option<Point> {
        self.move_point
    }

    /// Check if the path is empty (has no commands).
    ///
    /// # Example
    ///
    /// ```
    /// use pdftract_core::render::path::{CurrentPath, Point};
    ///
    /// let mut path = CurrentPath::new();
    /// assert!(path.is_empty());
    ///
    /// path.move_to(Point::new(10.0, 20.0));
    /// assert!(!path.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get the number of commands in the path.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_creation() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);

        let origin = Point::origin();
        assert_eq!(origin.x, 0.0);
        assert_eq!(origin.y, 0.0);
    }

    #[test]
    fn test_current_path_empty() {
        let path = CurrentPath::new();
        assert!(path.is_empty());
        assert_eq!(path.len(), 0);
        assert!(path.current_point().is_none());
        assert!(path.move_point().is_none());
    }

    #[test]
    fn test_move_to() {
        let mut path = CurrentPath::new();
        let p = Point::new(10.0, 20.0);
        path.move_to(p);

        assert_eq!(path.len(), 1);
        assert_eq!(path.current_point(), Some(p));
        assert_eq!(path.move_point(), Some(p));

        if let Some(PathCommand::MoveTo(pt)) = path.commands.first() {
            assert_eq!(pt, &p);
        } else {
            panic!("Expected MoveTo command");
        }
    }

    #[test]
    fn test_line_to() {
        let mut path = CurrentPath::new();
        let p1 = Point::new(10.0, 20.0);
        let p2 = Point::new(30.0, 40.0);
        path.move_to(p1);
        path.line_to(p2);

        assert_eq!(path.len(), 2);
        assert_eq!(path.current_point(), Some(p2));
        assert_eq!(path.move_point(), Some(p1));

        assert!(matches!(path.commands[0], PathCommand::MoveTo(_)));
        assert!(matches!(path.commands[1], PathCommand::LineTo(_)));
    }

    #[test]
    fn test_cubic_to() {
        let mut path = CurrentPath::new();
        let p1 = Point::new(10.0, 20.0);
        let c1 = Point::new(15.0, 25.0);
        let c2 = Point::new(25.0, 35.0);
        let end = Point::new(30.0, 40.0);
        path.move_to(p1);
        path.cubic_to(c1, c2, end);

        assert_eq!(path.len(), 2);
        assert_eq!(path.current_point(), Some(end));

        if let Some(PathCommand::CubicTo(cp1, cp2, e)) = path.commands.get(1) {
            assert_eq!(cp1, &c1);
            assert_eq!(cp2, &c2);
            assert_eq!(e, &end);
        } else {
            panic!("Expected CubicTo command");
        }
    }

    #[test]
    fn test_rect() {
        let mut path = CurrentPath::new();
        path.rect(10.0, 20.0, 30.0, 40.0);

        assert_eq!(path.len(), 1);
        assert_eq!(path.current_point(), Some(Point::new(10.0, 20.0)));
        assert_eq!(path.move_point(), Some(Point::new(10.0, 20.0)));

        if let Some(PathCommand::Rect(x, y, w, h)) = path.commands.first() {
            assert_eq!(x, &10.0);
            assert_eq!(y, &20.0);
            assert_eq!(w, &30.0);
            assert_eq!(h, &40.0);
        } else {
            panic!("Expected Rect command");
        }
    }

    #[test]
    fn test_close_path() {
        let mut path = CurrentPath::new();
        let p1 = Point::new(10.0, 20.0);
        let p2 = Point::new(30.0, 40.0);
        path.move_to(p1);
        path.line_to(p2);
        path.close_path();

        assert_eq!(path.len(), 3);
        assert_eq!(path.current_point(), Some(p1)); // Returns to move point

        assert!(matches!(path.commands[0], PathCommand::MoveTo(_)));
        assert!(matches!(path.commands[1], PathCommand::LineTo(_)));
        assert!(matches!(path.commands[2], PathCommand::ClosePath));
    }

    #[test]
    fn test_clear() {
        let mut path = CurrentPath::new();
        path.move_to(Point::new(10.0, 20.0));
        path.line_to(Point::new(30.0, 40.0));

        assert_eq!(path.len(), 2);
        path.clear();
        assert_eq!(path.len(), 0);
        assert!(path.current_point().is_none());
        assert!(path.move_point().is_none());
    }

    #[test]
    fn test_shorthand_cubic_to() {
        let mut path = CurrentPath::new();
        let p1 = Point::new(10.0, 20.0);
        let c2 = Point::new(25.0, 35.0);
        let end = Point::new(30.0, 40.0);
        path.move_to(p1);
        path.shorthand_cubic_to(c2, end);

        assert_eq!(path.len(), 2);
        assert_eq!(path.current_point(), Some(end));

        if let Some(PathCommand::ShorthandCubicTo(cp2, e)) = path.commands.get(1) {
            assert_eq!(cp2, &c2);
            assert_eq!(e, &end);
        } else {
            panic!("Expected ShorthandCubicTo command");
        }
    }

    #[test]
    fn test_shorthand_cubic_to_y() {
        let mut path = CurrentPath::new();
        let p1 = Point::new(10.0, 20.0);
        let c1 = Point::new(15.0, 25.0);
        let end = Point::new(30.0, 40.0);
        path.move_to(p1);
        path.shorthand_cubic_to_y(c1, end);

        assert_eq!(path.len(), 2);
        assert_eq!(path.current_point(), Some(end));

        if let Some(PathCommand::ShorthandCubicToY(cp1, e)) = path.commands.get(1) {
            assert_eq!(cp1, &c1);
            assert_eq!(e, &end);
        } else {
            panic!("Expected ShorthandCubicToY command");
        }
    }
}
