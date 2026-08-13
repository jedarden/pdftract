//! PDF path command builder functions.
//!
//! This module provides helper functions for constructing valid PDF path command
//! strings for use in content streams. These functions handle proper formatting
//! of coordinates and PDF command syntax.
//!
//! # Path Command Syntax
//!
//! PDF path commands follow a specific syntax:
//! - moveto: "x y m" - Move current point without drawing
//! - lineto: "x y l" - Draw line from current point to (x, y)
//! - curveto: "x1 y1 x2 y2 x3 y3 c" - Draw cubic Bézier curve
//! - closepath: "h" - Close the current subpath
//!
//! # Example
//!
//! ```rust,no_run
//! use pdftract_core::font::path_commands::{moveto, lineto, closepath};
//!
//! // Build a simple triangle path
//! let path = format!(
//!     "{} {} {} {}",
//!     moveto(0.0, 0.0),
//!     lineto(100.0, 0.0),
//!     lineto(50.0, 100.0)
//! );
//! let path = format!("{} {}", path, closepath());
//!
//! // Result: "0 0 m 100 0 l 50 100 l h"
//! ```

/// Create a PDF moveto command string.
///
/// The moveto command moves the current point to the specified coordinates
/// without drawing anything. It begins a new subpath.
///
/// # Arguments
///
/// * `x` - X coordinate in user space units
/// * `y` - Y coordinate in user space units
///
/// # Returns
///
/// A string containing the formatted moveto command: "x y m"
///
/// # Example
///
/// ```
/// use pdftract_core::font::path_commands::moveto;
///
/// assert_eq!(moveto(100.0, 200.0), "100 200 m");
/// assert_eq!(moveto(0.0, 0.0), "0 0 m");
/// assert_eq!(moveto(50.5, 25.25), "50.5 25.25 m");
/// ```
pub fn moveto(x: f32, y: f32) -> String {
    format!("{} {} m", x, y)
}

/// Create a PDF lineto command string.
///
/// The lineto command draws a straight line from the current point
/// to the specified coordinates, which becomes the new current point.
///
/// # Arguments
///
/// * `x` - X coordinate of the endpoint in user space units
/// * `y` - Y coordinate of the endpoint in user space units
///
/// # Returns
///
/// A string containing the formatted lineto command: "x y l"
///
/// # Example
///
/// ```
/// use pdftract_core::font::path_commands::lineto;
///
/// assert_eq!(lineto(100.0, 200.0), "100 200 l");
/// assert_eq!(lineto(500.0, 500.0), "500 500 l");
/// assert_eq!(lineto(25.5, 12.75), "25.5 12.75 l");
/// ```
pub fn lineto(x: f32, y: f32) -> String {
    format!("{} {} l", x, y)
}

/// Create a PDF curveto command string for a cubic Bézier curve.
///
/// The curveto command draws a cubic Bézier curve from the current point
/// to (x3, y3) using (x1, y1) and (x2, y2) as the control points.
///
/// # Arguments
///
/// * `x1` - X coordinate of first control point
/// * `y1` - Y coordinate of first control point
/// * `x2` - X coordinate of second control point
/// * `y2` - Y coordinate of second control point
/// * `x3` - X coordinate of endpoint
/// * `y3` - Y coordinate of endpoint
///
/// # Returns
///
/// A string containing the formatted curveto command: "x1 y1 x2 y2 x3 y3 c"
///
/// # Example
///
/// ```
/// use pdftract_core::font::path_commands::curveto;
///
/// assert_eq!(curveto(0.0, 100.0, 100.0, 100.0, 100.0, 0.0),
///            "0 100 100 100 100 0 c");
/// assert_eq!(curveto(50.0, 50.0, 150.0, 50.0, 200.0, 0.0),
///            "50 50 150 50 200 0 c");
/// ```
pub fn curveto(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) -> String {
    format!("{} {} {} {} {} {} c", x1, y1, x2, y2, x3, y3)
}

/// Create a PDF closepath command string.
///
/// The closepath command closes the current subpath by drawing a straight
/// line from the current point to the starting point of the subpath.
///
/// # Returns
///
/// A string containing the closepath command: "h"
///
/// # Example
///
/// ```
/// use pdftract_core::font::path_commands::closepath;
///
/// assert_eq!(closepath(), "h");
/// ```
pub fn closepath() -> String {
    "h".to_string()
}

/// Create a PDF rectangle command string.
///
/// The rectangle command constructs a rectangle and appends it to the current
/// path as a complete subpath. This is a convenience shorthand for the
/// equivalent moveto, lineto, closepath sequence.
///
/// # Arguments
///
/// * `x` - X coordinate of lower-left corner
/// * `y` - Y coordinate of lower-left corner
/// * `width` - Width of the rectangle
/// * `height` - Height of the rectangle
///
/// # Returns
///
/// A string containing the formatted rectangle command: "x y width height re"
///
/// # Example
///
/// ```
/// use pdftract_core::font::path_commands::rectangle;
///
/// assert_eq!(rectangle(0.0, 0.0, 100.0, 50.0), "0 0 100 50 re");
/// assert_eq!(rectangle(50.0, 25.0, 200.0, 150.0), "50 25 200 150 re");
/// ```
pub fn rectangle(x: f32, y: f32, width: f32, height: f32) -> String {
    format!("{} {} {} {} re", x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moveto_basic() {
        assert_eq!(moveto(100.0, 200.0), "100 200 m");
        assert_eq!(moveto(0.0, 0.0), "0 0 m");
        assert_eq!(moveto(500.0, 500.0), "500 500 m");
    }

    #[test]
    fn test_moveto_negative() {
        assert_eq!(moveto(-10.0, -20.0), "-10 -20 m");
        assert_eq!(moveto(-100.5, -200.25), "-100.5 -200.25 m");
    }

    #[test]
    fn test_moveto_fractional() {
        assert_eq!(moveto(50.5, 25.25), "50.5 25.25 m");
        assert_eq!(moveto(10.125, 20.875), "10.125 20.875 m");
    }

    #[test]
    fn test_lineto_basic() {
        assert_eq!(lineto(100.0, 200.0), "100 200 l");
        assert_eq!(lineto(0.0, 0.0), "0 0 l");
        assert_eq!(lineto(500.0, 500.0), "500 500 l");
    }

    #[test]
    fn test_lineto_negative() {
        assert_eq!(lineto(-10.0, -20.0), "-10 -20 l");
        assert_eq!(lineto(-100.5, -200.25), "-100.5 -200.25 l");
    }

    #[test]
    fn test_lineto_fractional() {
        assert_eq!(lineto(25.5, 12.75), "25.5 12.75 l");
        assert_eq!(lineto(33.333, 66.666), "33.333 66.666 l");
    }

    #[test]
    fn test_curveto_basic() {
        assert_eq!(
            curveto(0.0, 100.0, 100.0, 100.0, 100.0, 0.0),
            "0 100 100 100 100 0 c"
        );
        assert_eq!(
            curveto(50.0, 50.0, 150.0, 50.0, 200.0, 0.0),
            "50 50 150 50 200 0 c"
        );
    }

    #[test]
    fn test_curveto_negative() {
        assert_eq!(
            curveto(-10.0, -20.0, -30.0, -40.0, -50.0, -60.0),
            "-10 -20 -30 -40 -50 -60 c"
        );
    }

    #[test]
    fn test_curveto_fractional() {
        assert_eq!(
            curveto(25.5, 12.75, 50.25, 75.125, 100.0, 0.0),
            "25.5 12.75 50.25 75.125 100 0 c"
        );
    }

    #[test]
    fn test_closepath() {
        assert_eq!(closepath(), "h");
        assert_eq!(closepath(), "h"); // Consistent output
    }

    #[test]
    fn test_rectangle_basic() {
        assert_eq!(rectangle(0.0, 0.0, 100.0, 50.0), "0 0 100 50 re");
        assert_eq!(rectangle(50.0, 25.0, 200.0, 150.0), "50 25 200 150 re");
    }

    #[test]
    fn test_rectangle_negative() {
        assert_eq!(rectangle(-10.0, -20.0, 50.0, 30.0), "-10 -20 50 30 re");
    }

    #[test]
    fn test_rectangle_fractional() {
        assert_eq!(rectangle(10.5, 20.25, 50.75, 100.125), "10.5 20.25 50.75 100.125 re");
    }

    #[test]
    fn test_combined_path_commands() {
        // Test that multiple commands can be combined
        let path = format!(
            "{} {} {} {}",
            moveto(0.0, 0.0),
            lineto(100.0, 0.0),
            lineto(50.0, 100.0),
            closepath()
        );

        assert!(path.contains("0 0 m"));
        assert!(path.contains("100 0 l"));
        assert!(path.contains("50 100 l"));
        assert!(path.contains("h"));
    }

    #[test]
    fn test_command_syntax_validation() {
        // Verify that all commands end with their correct operators
        assert!(moveto(0.0, 0.0).ends_with(" m"));
        assert!(lineto(0.0, 0.0).ends_with(" l"));
        assert!(curveto(0.0, 0.0, 0.0, 0.0, 0.0, 0.0).ends_with(" c"));
        assert_eq!(closepath(), "h");
        assert!(rectangle(0.0, 0.0, 10.0, 10.0).ends_with(" re"));
    }

    #[test]
    fn test_floating_point_formatting() {
        // Test that floating point numbers are formatted correctly
        // without unnecessary trailing zeros
        let cmd = moveto(100.0, 200.0);
        assert_eq!(cmd, "100 200 m"); // No decimal point for whole numbers

        let cmd = moveto(100.5, 200.25);
        assert_eq!(cmd, "100.5 200.25 m"); // Preserves fractional parts
    }

    #[test]
    fn test_path_command_compilation() {
        // This test verifies that all path command functions compile
        // and are callable
        let _ = moveto(0.0, 0.0);
        let _ = lineto(100.0, 100.0);
        let _ = curveto(0.0, 0.0, 10.0, 10.0, 20.0, 20.0);
        let _ = closepath();
        let _ = rectangle(0.0, 0.0, 50.0, 50.0);
    }
}
