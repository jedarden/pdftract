//! Charproc content stream data structure for Type3 glyphs.
//!
//! This module provides a structured way to build and manage Type3 glyph
//! content streams. Rather than working with raw byte strings, this module
//! provides a typed representation of PDF path commands that can be
//! serialized to PDF content stream format.
//!
//! # Overview
//!
//! A charproc stream is a sequence of PDF drawing commands that define
//! how a Type3 glyph appears. This module provides:
//!
//! - [`CharProcStream`] struct for building content streams
//! - [`PathCommand`] enum for typed path commands
//! - Builder methods for adding path construction commands
//! - Serialization to PDF content stream format
//!
//! # Example
//!
//! ```rust,no_run
//! use pdftract_core::font::charproc_stream::{CharProcStream, PathCommand};
//!
//! // Create a new charproc stream for a triangle glyph
//! let mut stream = CharProcStream::new();
//!
//! // Add path commands to draw a triangle
//! stream.add_moveto(10.0, 10.0);
//! stream.add_lineto(20.0, 10.0);
//! stream.add_lineto(15.0, 20.0);
//! stream.add_closepath();
//! stream.add_fill();
//!
//! // Serialize to PDF content stream format
//! let pdf_bytes = stream.to_pdf_bytes();
//! assert_eq!(pdf_bytes, b"10 10 m 20 10 l 15 20 l h f");
//! ```

use std::fmt;

/// PDF path drawing commands.
///
/// Represents the various path construction and painting operators
/// defined in the PDF specification for content streams.
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    /// Move to (x, y) - begin new subpath
    MoveTo(f32, f32),
    /// Line to (x, y) - draw straight line from current point
    LineTo(f32, f32),
    /// Cubic Bézier curve with two control points
    CurveTo {
        /// First control point (x1, y1)
        control1: (f32, f32),
        /// Second control point (x2, y2)
        control2: (f32, f32),
        /// Endpoint (x3, y3)
        endpoint: (f32, f32),
    },
    /// Close current subpath
    ClosePath,
    /// Rectangle with lower-left corner (x, y) and dimensions (width, height)
    Rectangle {
        /// X coordinate of lower-left corner
        x: f32,
        /// Y coordinate of lower-left corner
        y: f32,
        /// Width of rectangle
        width: f32,
        /// Height of rectangle
        height: f32,
    },
    /// Fill path using non-zero winding rule
    Fill,
    /// Stroke path (draw outline)
    Stroke,
    /// Close path and stroke
    CloseAndStroke,
    /// End path without drawing (no-op)
    NoOp,
}

impl fmt::Display for PathCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathCommand::MoveTo(x, y) => write!(f, "{} {} m", x, y),
            PathCommand::LineTo(x, y) => write!(f, "{} {} l", x, y),
            PathCommand::CurveTo { control1, control2, endpoint } => {
                write!(f, "{} {} {} {} {} {} c",
                    control1.0, control1.1,
                    control2.0, control2.1,
                    endpoint.0, endpoint.1
                )
            }
            PathCommand::ClosePath => write!(f, "h"),
            PathCommand::Rectangle { x, y, width, height } => {
                write!(f, "{} {} {} {} re", x, y, width, height)
            }
            PathCommand::Fill => write!(f, "f"),
            PathCommand::Stroke => write!(f, "S"),
            PathCommand::CloseAndStroke => write!(f, "s"),
            PathCommand::NoOp => write!(f, "n"),
        }
    }
}

/// Charproc content stream for Type3 glyphs.
///
/// A charproc stream contains a sequence of PDF drawing commands that
/// define the appearance of a Type3 glyph. This struct provides a
/// builder interface for constructing content streams programmatically.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::charproc_stream::CharProcStream;
///
/// // Create a charproc stream that draws a filled rectangle
/// let mut stream = CharProcStream::new();
/// stream.add_rectangle(0.0, 0.0, 100.0, 50.0);
/// stream.add_fill();
///
/// let pdf_bytes = stream.to_pdf_bytes();
/// assert_eq!(pdf_bytes, b"0 0 100 50 re f");
/// ```
#[derive(Debug, Clone, Default)]
pub struct CharProcStream {
    /// Sequence of path commands in execution order
    commands: Vec<PathCommand>,
}

impl CharProcStream {
    /// Create a new empty charproc stream.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pdftract_core::font::charproc_stream::CharProcStream;
    ///
    /// let stream = CharProcStream::new();
    /// assert!(stream.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Add a moveto command.
    ///
    /// Moves the current point to (x, y) without drawing, beginning a new subpath.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in glyph space
    /// * `y` - Y coordinate in glyph space
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(10.0, 20.0);
    /// ```
    pub fn add_moveto(&mut self, x: f32, y: f32) -> &mut Self {
        self.commands.push(PathCommand::MoveTo(x, y));
        self
    }

    /// Add a lineto command.
    ///
    /// Draws a straight line from the current point to (x, y).
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of endpoint in glyph space
    /// * `y` - Y coordinate of endpoint in glyph space
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(0.0, 0.0);
    /// stream.add_lineto(100.0, 100.0);
    /// ```
    pub fn add_lineto(&mut self, x: f32, y: f32) -> &mut Self {
        self.commands.push(PathCommand::LineTo(x, y));
        self
    }

    /// Add a curveto command (cubic Bézier curve).
    ///
    /// Draws a cubic Bézier curve from the current point to (x3, y3) using
    /// (x1, y1) and (x2, y2) as control points.
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
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(0.0, 0.0);
    /// stream.add_curveto(50.0, 50.0, 100.0, 50.0, 150.0, 0.0);
    /// ```
    pub fn add_curveto(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) -> &mut Self {
        self.commands.push(PathCommand::CurveTo {
            control1: (x1, y1),
            control2: (x2, y2),
            endpoint: (x3, y3),
        });
        self
    }

    /// Add a closepath command.
    ///
    /// Closes the current subpath by drawing a line from the current point
    /// back to the starting point.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(0.0, 0.0);
    /// stream.add_lineto(10.0, 0.0);
    /// stream.add_lineto(10.0, 10.0);
    /// stream.add_closepath();
    /// ```
    pub fn add_closepath(&mut self) -> &mut Self {
        self.commands.push(PathCommand::ClosePath);
        self
    }

    /// Add a rectangle command.
    ///
    /// Constructs a rectangle and appends it as a complete subpath.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of lower-left corner
    /// * `y` - Y coordinate of lower-left corner
    /// * `width` - Width of rectangle
    /// * `height` - Height of rectangle
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_rectangle(5.0, 5.0, 100.0, 50.0);
    /// ```
    pub fn add_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) -> &mut Self {
        self.commands.push(PathCommand::Rectangle {
            x, y, width, height,
        });
        self
    }

    /// Add a fill command.
    ///
    /// Fills the current path using the non-zero winding rule.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_rectangle(0.0, 0.0, 50.0, 50.0);
    /// stream.add_fill();
    /// ```
    pub fn add_fill(&mut self) -> &mut Self {
        self.commands.push(PathCommand::Fill);
        self
    }

    /// Add a stroke command.
    ///
    /// Strokes the current path (draws the outline).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_rectangle(0.0, 0.0, 50.0, 50.0);
    /// stream.add_stroke();
    /// ```
    pub fn add_stroke(&mut self) -> &mut Self {
        self.commands.push(PathCommand::Stroke);
        self
    }

    /// Add a close-and-stroke command.
    ///
    /// Closes the current subpath and strokes it.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(0.0, 0.0);
    /// stream.add_lineto(10.0, 10.0);
    /// stream.add_close_and_stroke();
    /// ```
    pub fn add_close_and_stroke(&mut self) -> &mut Self {
        self.commands.push(PathCommand::CloseAndStroke);
        self
    }

    /// Add a no-op (end path) command.
    ///
    /// Ends the current path without drawing it.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(0.0, 0.0);
    /// stream.add_lineto(10.0, 10.0);
    /// stream.add_noop();  // Path is discarded
    /// ```
    pub fn add_noop(&mut self) -> &mut Self {
        self.commands.push(PathCommand::NoOp);
        self
    }

    /// Add a raw path command to the stream.
    ///
    /// This method allows adding any PathCommand directly to the stream,
    /// providing flexibility for complex scenarios.
    ///
    /// # Arguments
    ///
    /// * `command` - The PathCommand to add
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::{CharProcStream, PathCommand};
    /// let mut stream = CharProcStream::new();
    /// stream.add_command(PathCommand::MoveTo(5.0, 10.0));
    /// ```
    pub fn add_command(&mut self, command: PathCommand) -> &mut Self {
        self.commands.push(command);
        self
    }

    /// Check if the stream is empty (contains no commands).
    ///
    /// # Returns
    ///
    /// `true` if the stream has no commands, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let stream = CharProcStream::new();
    /// assert!(stream.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get the number of commands in the stream.
    ///
    /// # Returns
    ///
    /// The count of path commands in the stream
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(0.0, 0.0);
    /// stream.add_lineto(10.0, 10.0);
    /// assert_eq!(stream.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Clear all commands from the stream.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(0.0, 0.0);
    /// stream.clear();
    /// assert!(stream.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Serialize the charproc stream to PDF content stream format.
    ///
    /// Converts the sequence of path commands to a byte string in PDF
    /// content stream syntax, suitable for use as a Type3 glyph charproc.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the PDF content stream bytes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_rectangle(0.0, 0.0, 10.0, 10.0);
    /// stream.add_fill();
    ///
    /// let pdf_bytes = stream.to_pdf_bytes();
    /// assert_eq!(pdf_bytes, b"0 0 10 10 re f");
    /// ```
    pub fn to_pdf_bytes(&self) -> Vec<u8> {
        let mut result = String::new();

        for (i, command) in self.commands.iter().enumerate() {
            if i > 0 {
                result.push(' ');
            }
            result.push_str(&command.to_string());
        }

        result.into_bytes()
    }

    /// Get a reference to the commands in the stream.
    ///
    /// # Returns
    ///
    /// A slice containing the path commands in execution order
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pdftract_core::font::charproc_stream::CharProcStream;
    /// let mut stream = CharProcStream::new();
    /// stream.add_moveto(5.0, 10.0);
    /// let commands = stream.commands();
    /// assert_eq!(commands.len(), 1);
    /// ```
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_charproc_stream_new() {
        let stream = CharProcStream::new();
        assert!(stream.is_empty());
        assert_eq!(stream.len(), 0);
    }

    #[test]
    fn test_charproc_stream_default() {
        let stream = CharProcStream::default();
        assert!(stream.is_empty());
        assert_eq!(stream.len(), 0);
    }

    #[test]
    fn test_add_moveto() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(10.0, 20.0);

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::MoveTo(10.0, 20.0)]);
    }

    #[test]
    fn test_add_lineto() {
        let mut stream = CharProcStream::new();
        stream.add_lineto(100.0, 200.0);

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::LineTo(100.0, 200.0)]);
    }

    #[test]
    fn test_add_curveto() {
        let mut stream = CharProcStream::new();
        stream.add_curveto(0.0, 100.0, 100.0, 100.0, 100.0, 0.0);

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::CurveTo {
            control1: (0.0, 100.0),
            control2: (100.0, 100.0),
            endpoint: (100.0, 0.0),
        }]);
    }

    #[test]
    fn test_add_closepath() {
        let mut stream = CharProcStream::new();
        stream.add_closepath();

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::ClosePath]);
    }

    #[test]
    fn test_add_rectangle() {
        let mut stream = CharProcStream::new();
        stream.add_rectangle(5.0, 10.0, 50.0, 25.0);

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::Rectangle {
            x: 5.0, y: 10.0, width: 50.0, height: 25.0,
        }]);
    }

    #[test]
    fn test_add_fill() {
        let mut stream = CharProcStream::new();
        stream.add_fill();

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::Fill]);
    }

    #[test]
    fn test_add_stroke() {
        let mut stream = CharProcStream::new();
        stream.add_stroke();

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::Stroke]);
    }

    #[test]
    fn test_add_close_and_stroke() {
        let mut stream = CharProcStream::new();
        stream.add_close_and_stroke();

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::CloseAndStroke]);
    }

    #[test]
    fn test_add_noop() {
        let mut stream = CharProcStream::new();
        stream.add_noop();

        assert_eq!(stream.len(), 1);
        assert_eq!(stream.commands(), &[PathCommand::NoOp]);
    }

    #[test]
    fn test_builder_chaining() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(0.0, 0.0)
            .add_lineto(10.0, 0.0)
            .add_lineto(10.0, 10.0)
            .add_closepath()
            .add_fill();

        assert_eq!(stream.len(), 5);
    }

    #[test]
    fn test_clear() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(0.0, 0.0);
        stream.add_lineto(10.0, 10.0);

        assert_eq!(stream.len(), 2);

        stream.clear();

        assert!(stream.is_empty());
        assert_eq!(stream.len(), 0);
    }

    #[test]
    fn test_to_pdf_bytes_empty() {
        let stream = CharProcStream::new();
        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"");
    }

    #[test]
    fn test_to_pdf_bytes_moveto() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(100.0, 200.0);

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"100 200 m");
    }

    #[test]
    fn test_to_pdf_bytes_lineto() {
        let mut stream = CharProcStream::new();
        stream.add_lineto(50.0, 75.0);

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"50 75 l");
    }

    #[test]
    fn test_to_pdf_bytes_curveto() {
        let mut stream = CharProcStream::new();
        stream.add_curveto(0.0, 100.0, 100.0, 100.0, 100.0, 0.0);

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"0 100 100 100 100 0 c");
    }

    #[test]
    fn test_to_pdf_bytes_closepath() {
        let mut stream = CharProcStream::new();
        stream.add_closepath();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"h");
    }

    #[test]
    fn test_to_pdf_bytes_rectangle() {
        let mut stream = CharProcStream::new();
        stream.add_rectangle(0.0, 0.0, 100.0, 50.0);

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"0 0 100 50 re");
    }

    #[test]
    fn test_to_pdf_bytes_fill() {
        let mut stream = CharProcStream::new();
        stream.add_fill();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"f");
    }

    #[test]
    fn test_to_pdf_bytes_stroke() {
        let mut stream = CharProcStream::new();
        stream.add_stroke();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"S");
    }

    #[test]
    fn test_to_pdf_bytes_close_and_stroke() {
        let mut stream = CharProcStream::new();
        stream.add_close_and_stroke();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"s");
    }

    #[test]
    fn test_to_pdf_bytes_noop() {
        let mut stream = CharProcStream::new();
        stream.add_noop();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"n");
    }

    #[test]
    fn test_to_pdf_bytes_complex() {
        let mut stream = CharProcStream::new();

        // Draw a triangle
        stream.add_moveto(10.0, 10.0);
        stream.add_lineto(20.0, 10.0);
        stream.add_lineto(15.0, 20.0);
        stream.add_closepath();
        stream.add_fill();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"10 10 m 20 10 l 15 20 l h f");
    }

    #[test]
    fn test_to_pdf_bytes_rectangle_filled() {
        let mut stream = CharProcStream::new();
        stream.add_rectangle(0.0, 0.0, 10.0, 10.0);
        stream.add_fill();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"0 0 10 10 re f");
    }

    #[test]
    fn test_to_pdf_bytes_multiple_shapes() {
        let mut stream = CharProcStream::new();

        // Two separate rectangles
        stream.add_rectangle(0.0, 0.0, 5.0, 5.0);
        stream.add_fill();
        stream.add_rectangle(10.0, 10.0, 5.0, 5.0);
        stream.add_fill();

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"0 0 5 5 re f 10 10 5 5 re f");
    }

    #[test]
    fn test_path_command_display_moveto() {
        let cmd = PathCommand::MoveTo(100.5, 200.25);
        assert_eq!(cmd.to_string(), "100.5 200.25 m");
    }

    #[test]
    fn test_path_command_display_lineto() {
        let cmd = PathCommand::LineTo(50.75, 12.125);
        assert_eq!(cmd.to_string(), "50.75 12.125 l");
    }

    #[test]
    fn test_path_command_display_curveto() {
        let cmd = PathCommand::CurveTo {
            control1: (0.0, 100.0),
            control2: (100.0, 100.0),
            endpoint: (100.0, 0.0),
        };
        assert_eq!(cmd.to_string(), "0 100 100 100 100 0 c");
    }

    #[test]
    fn test_path_command_display_rectangle() {
        let cmd = PathCommand::Rectangle {
            x: 10.5,
            y: 20.25,
            width: 50.75,
            height: 100.125,
        };
        assert_eq!(cmd.to_string(), "10.5 20.25 50.75 100.125 re");
    }

    #[test]
    fn test_path_command_partial_eq() {
        let cmd1 = PathCommand::MoveTo(10.0, 20.0);
        let cmd2 = PathCommand::MoveTo(10.0, 20.0);
        let cmd3 = PathCommand::LineTo(10.0, 20.0);

        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }

    #[test]
    fn test_path_command_clone() {
        let cmd = PathCommand::MoveTo(10.0, 20.0);
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn test_charproc_stream_clone() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(0.0, 0.0);
        stream.add_lineto(10.0, 10.0);

        let cloned = stream.clone();
        assert_eq!(stream.len(), cloned.len());
        assert_eq!(stream.to_pdf_bytes(), cloned.to_pdf_bytes());
    }

    #[test]
    fn test_add_command_raw() {
        let mut stream = CharProcStream::new();
        stream.add_command(PathCommand::MoveTo(5.0, 10.0));
        stream.add_command(PathCommand::LineTo(15.0, 20.0));

        assert_eq!(stream.len(), 2);
    }

    #[test]
    fn test_negative_coordinates() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(-10.0, -20.0);

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"-10 -20 m");
    }

    #[test]
    fn test_fractional_coordinates() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(10.125, 20.875);

        let bytes = stream.to_pdf_bytes();
        assert_eq!(bytes, b"10.125 20.875 m");
    }

    #[test]
    fn test_commands_slice() {
        let mut stream = CharProcStream::new();
        stream.add_moveto(0.0, 0.0);
        stream.add_lineto(10.0, 10.0);

        let commands = stream.commands();
        assert_eq!(commands.len(), 2);
    }
}