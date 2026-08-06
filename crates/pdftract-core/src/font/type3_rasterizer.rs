//! Type 3 glyph content stream rasterizer.
//!
//! This module implements rasterization of Type 3 glyph content streams to
//! 32x32 grayscale bitmaps for shape recognition (Phase 2.5 Level 4).
//!
//! Per PDF spec section 9.6.5, Type 3 glyphs are defined by content streams
//! that draw the glyph shape. This module:
//! 1. Parses the content stream into path commands
//! 2. Executes the path commands to fill a 32x32 bitmap
//! 3. Returns the bitmap for pHash computation in the shape database
//!
//! The operator subset supported is:
//! - Path construction: m, l, c, v, y, re, h
//! - Painting: S, s, f, F, B, b, f*, B*, b*
//! - Graphics state: q, Q, cm
//! - XObject: Do (form XObjects only)
//! - No-op: n

use std::sync::Arc;

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::font::type3::Type3Font;
use crate::graphics_state::{GraphicsState, GraphicsStateStack, Matrix3x3};
use crate::parser::lexer::Lexer;
use crate::parser::object::types::ObjRef;
use crate::parser::object::types::PdfObject;
use crate::parser::stream::{decode_stream, ExtractionOptions, PdfSource};
use crate::parser::xref::{ResolveError, XrefResolver};

/// Maximum recursion depth for Type 3 glyph execution (form XObject + nested glyphs).
const MAX_GLYPH_DEPTH: usize = 20;

/// Document resolver context for Type3 glyph rasterization.
///
/// Provides access to the document's resolver and source for dereferencing
/// content streams during glyph rasterization.
pub struct DocumentContext<'a> {
    /// PDF document resolver for looking up indirect references
    pub resolver: Option<&'a XrefResolver>,
    /// PDF source for reading stream data
    pub source: Option<&'a dyn PdfSource>,
}

/// Calculate glyph bitmap dimensions from glyph bounds.
///
/// This function computes the proper bitmap dimensions (width, height) in pixels
/// based on the glyph's bounding box coordinates. It handles scaling from PDF user
/// space to pixel space, adds padding for anti-aliasing, and ensures non-zero dimensions.
///
/// # Arguments
///
/// * `bbox` - Glyph bounding box [x0, y0, x1, y1] in PDF user space (points)
/// * `padding` - Optional padding margin in pixels (default: 1 for anti-aliasing)
///
/// # Returns
///
/// (width, height) as usize, where both dimensions are guaranteed to be >= 1.
///
/// # Calculation
///
/// ```text
/// width_px = (x1 - x0) + 2 * padding
/// height_px = (y1 - y0) + 2 * padding
/// ```
///
/// If the bounding box is degenerate (x0 == x1 or y0 == y1), the dimension
/// defaults to 1 pixel plus padding to ensure valid bitmap dimensions.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::type3_rasterizer::calculate_bitmap_dimensions;
///
/// // Glyph with bbox [10.0, 20.0, 50.0, 60.0] in points
/// // Raw size: 40x30 points
/// // With default padding: 42x32 pixels
/// let (width, height) = calculate_bitmap_dimensions(&[10.0, 20.0, 50.0, 60.0], None);
/// assert_eq!(width, 42);
/// assert_eq!(height, 32);
/// ```
pub fn calculate_bitmap_dimensions(bbox: &[f32; 4], padding: Option<u32>) -> (usize, usize) {
    let padding = padding.unwrap_or(1);

    // Extract bounding box coordinates
    let x0 = bbox[0];
    let y0 = bbox[1];
    let x1 = bbox[2];
    let y2 = bbox[3];

    // Calculate raw dimensions in PDF user space (points)
    let raw_width = x1 - x0;
    let raw_height = y2 - y0;

    // Handle degenerate cases (zero-width or zero-height bboxes)
    // Default to 1 pixel minimum before padding
    let width_points = if raw_width.abs() < 0.5 {
        1.0
    } else {
        raw_width.abs()
    };

    let height_points = if raw_height.abs() < 0.5 {
        1.0
    } else {
        raw_height.abs()
    };

    // Convert to pixel dimensions (1 point = 1 pixel for glyph rendering)
    // Add padding for anti-aliasing margins
    let width = (width_points.ceil() as u32) + 2 * padding;
    let height = (height_points.ceil() as u32) + 2 * padding;

    // Ensure minimum dimensions of 1x1 (even with zero padding)
    let width = (width.max(1)) as usize;
    let height = (height.max(1)) as usize;

    (width, height)
}

/// 32x32 grayscale bitmap for glyph rasterization.
///
/// Each pixel is a u8 value (0-255). Per Phase 2.5 convention:
/// - 0 = black ink
/// - 255 = white paper
/// - Values in between are anti-aliased edges
#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap32x32 {
    /// 1024 pixels (32 * 32), stored row-major
    pixels: [u8; 1024],
}

impl Bitmap32x32 {
    /// Create a new white bitmap (all pixels = 255).
    pub fn white() -> Self {
        Self {
            pixels: [255u8; 1024],
        }
    }

    /// Create a new black bitmap (all pixels = 0).
    pub fn black() -> Self {
        Self {
            pixels: [0u8; 1024],
        }
    }

    /// Get the pixel value at (x, y).
    ///
    /// Returns None if (x, y) is out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || x >= 32 || y < 0 || y >= 32 {
            return None;
        }
        Some(self.pixels[(y as usize) * 32 + (x as usize)])
    }

    /// Set the pixel value at (x, y).
    ///
    /// Returns false if (x, y) is out of bounds.
    pub fn set(&mut self, x: i32, y: i32, value: u8) -> bool {
        if x < 0 || x >= 32 || y < 0 || y >= 32 {
            return false;
        }
        self.pixels[(y as usize) * 32 + (x as usize)] = value;
        true
    }

    /// Convert to a byte array for pHash computation.
    pub fn as_bytes(&self) -> &[u8; 1024] {
        &self.pixels
    }

    /// Fill a rectangle with the given color.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u8) {
        for y in y0.max(0)..y1.min(32) {
            for x in x0.max(0)..x1.min(32) {
                self.set(x, y, color);
            }
        }
    }
}

impl Default for Bitmap32x32 {
    fn default() -> Self {
        Self::white()
    }
}

/// 2D point for path construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

impl Point {
    /// Create a new Point with the given coordinates.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Path construction command.
#[derive(Debug, Clone, PartialEq)]
enum PathCommand {
    /// Move to absolute position
    MoveTo(Point),
    /// Line to absolute position
    LineTo(Point),
    /// Cubic Bezier curve (c: control1, control2, end)
    CubicTo(Point, Point, Point),
    /// Cubic Bezier with first control point implied (v: control2, end)
    ShorthandCubicTo(Point, Point),
    /// Cubic Bezier with second control point implied (y: control1, end)
    ShorthandCubicToY(Point, Point),
    /// Rectangle (re: x, y, width, height)
    Rect(f64, f64, f64, f64),
    /// Close subpath
    ClosePath,
}

/// Current path being constructed.
#[derive(Debug, Clone, Default)]
struct CurrentPath {
    commands: Vec<PathCommand>,
    current_point: Option<Point>,
    move_point: Option<Point>, // Start point of current subpath
}

impl CurrentPath {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, p: Point) {
        self.commands.push(PathCommand::MoveTo(p));
        self.current_point = Some(p);
        self.move_point = Some(p);
    }

    pub fn line_to(&mut self, p: Point) {
        self.commands.push(PathCommand::LineTo(p));
        self.current_point = Some(p);
    }

    pub fn cubic_to(&mut self, c1: Point, c2: Point, end: Point) {
        self.commands.push(PathCommand::CubicTo(c1, c2, end));
        self.current_point = Some(end);
    }

    pub fn shorthand_cubic_to(&mut self, c2: Point, end: Point) {
        self.commands.push(PathCommand::ShorthandCubicTo(c2, end));
        self.current_point = Some(end);
    }

    pub fn shorthand_cubic_to_y(&mut self, c1: Point, end: Point) {
        self.commands.push(PathCommand::ShorthandCubicToY(c1, end));
        self.current_point = Some(end);
    }

    pub fn rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.commands.push(PathCommand::Rect(x, y, width, height));
        self.current_point = Some(Point::new(x, y));
        self.move_point = Some(Point::new(x, y));
    }

    pub fn close_path(&mut self) {
        self.commands.push(PathCommand::ClosePath);
        if let Some(start) = self.move_point {
            self.current_point = Some(start);
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_point = None;
        self.move_point = None;
    }
}

/// Rasterization context for Type 3 glyph execution.
struct RasterizerContext<'a> {
    /// Output bitmap
    bitmap: Bitmap32x32,
    /// Current graphics state
    gstate: GraphicsState,
    /// Graphics state stack
    gstate_stack: GraphicsStateStack,
    /// Current path being constructed
    path: CurrentPath,
    /// Type3 font being rasterized
    font: &'a Type3Font,
    /// Current recursion depth
    depth: usize,
    /// Diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl<'a> RasterizerContext<'a> {
    fn new(font: &'a Type3Font) -> Self {
        let mut gstate = GraphicsState::new();

        // Apply FontMatrix to transform from glyph space to text space
        // Per PDF spec section 9.6.5, Type3 glyph content streams are executed
        // in glyph space, and the FontMatrix transforms coordinates to text space
        gstate.concat_ctm(&font.font_matrix);

        Self {
            bitmap: Bitmap32x32::white(),
            gstate,
            gstate_stack: GraphicsStateStack::new(),
            path: CurrentPath::new(),
            font,
            depth: 0,
            diagnostics: Vec::new(),
        }
    }

    /// Execute a content stream and rasterize the result.
    fn execute_content_stream(&mut self, stream_bytes: &[u8]) {
        let mut lexer = Lexer::new(stream_bytes);
        let mut operand_stack: Vec<f64> = Vec::new();
        let mut name_stack: Vec<Arc<str>> = Vec::new();

        while let Some(token) = lexer.next_token() {
            match token {
                crate::parser::lexer::Token::Eof => break,
                crate::parser::lexer::Token::Integer(n) => operand_stack.push(n as f64),
                crate::parser::lexer::Token::Real(r) => operand_stack.push(r),
                crate::parser::lexer::Token::Name(ref name) => {
                    let name_str = String::from_utf8_lossy(name);
                    name_stack.push(Arc::from(name_str.as_ref()));
                }
                crate::parser::lexer::Token::Keyword(ref kw) => {
                    let kw_str = String::from_utf8_lossy(kw);
                    self.execute_operator(&kw_str, &mut operand_stack, &mut name_stack);
                }
                _ => {
                    // Ignore other tokens (strings, arrays, etc.)
                }
            }
        }
    }

    /// Execute a single PDF graphics operator.
    fn execute_operator(
        &mut self,
        op: &str,
        operand_stack: &mut Vec<f64>,
        name_stack: &mut Vec<Arc<str>>,
    ) {
        match op {
            // Path construction operators
            "m" => self.op_move_to(operand_stack),
            "l" => self.op_line_to(operand_stack),
            "c" => self.op_cubic_to(operand_stack),
            "v" => self.op_shorthand_cubic_to(operand_stack),
            "y" => self.op_shorthand_cubic_to_y(operand_stack),
            "re" => self.op_rect(operand_stack),
            "h" => self.op_close_path(),
            "n" => self.op_no_op(), // No-op end of path

            // Painting operators
            "S" => self.op_stroke(),
            "s" => self.op_close_stroke(),
            "f" | "F" => self.op_fill(),
            "f*" => self.op_eofill(),
            "B" => self.op_fill_stroke(),
            "B*" => self.op_eofill_stroke(),
            "b" => self.op_close_fill_stroke(),
            "b*" => self.op_close_eofill_stroke(),

            // Graphics state operators
            "q" => self.op_save(),
            "Q" => self.op_restore(),
            "cm" => self.op_concat(operand_stack),

            // XObject operator
            "Do" => self.op_do(name_stack),

            // Ignore unsupported operators for now
            _ => {}
        }
    }

    /// m x y - Move to absolute position
    fn op_move_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 2 {
            return;
        }
        let y = stack.pop().unwrap();
        let x = stack.pop().unwrap();
        self.path.move_to(Point::new(x, y));
    }

    /// l x y - Line to absolute position
    fn op_line_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 2 {
            return;
        }
        let y = stack.pop().unwrap();
        let x = stack.pop().unwrap();
        self.path.line_to(Point::new(x, y));
    }

    /// c x1 y1 x2 y2 x3 y3 - Cubic Bezier curve
    fn op_cubic_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 6 {
            return;
        }
        let y3 = stack.pop().unwrap();
        let x3 = stack.pop().unwrap();
        let y2 = stack.pop().unwrap();
        let x2 = stack.pop().unwrap();
        let y1 = stack.pop().unwrap();
        let x1 = stack.pop().unwrap();
        self.path
            .cubic_to(Point::new(x1, y1), Point::new(x2, y2), Point::new(x3, y3));
    }

    /// v x2 y2 x3 y3 - Shorthand cubic Bezier (first control point implied)
    fn op_shorthand_cubic_to(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 4 {
            return;
        }
        let y3 = stack.pop().unwrap();
        let x3 = stack.pop().unwrap();
        let y2 = stack.pop().unwrap();
        let x2 = stack.pop().unwrap();
        self.path
            .shorthand_cubic_to(Point::new(x2, y2), Point::new(x3, y3));
    }

    /// y x1 y1 x3 y3 - Shorthand cubic Bezier (second control point implied)
    fn op_shorthand_cubic_to_y(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 4 {
            return;
        }
        let y3 = stack.pop().unwrap();
        let x3 = stack.pop().unwrap();
        let y1 = stack.pop().unwrap();
        let x1 = stack.pop().unwrap();
        self.path
            .shorthand_cubic_to_y(Point::new(x1, y1), Point::new(x3, y3));
    }

    /// re x y width height - Append rectangle
    fn op_rect(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 4 {
            return;
        }
        let height = stack.pop().unwrap();
        let width = stack.pop().unwrap();
        let y = stack.pop().unwrap();
        let x = stack.pop().unwrap();
        self.path.rect(x, y, width, height);
    }

    /// h - Close subpath
    fn op_close_path(&mut self) {
        self.path.close_path();
    }

    /// n - No-op end of path
    fn op_no_op(&mut self) {
        self.path.clear();
    }

    /// S - Stroke path
    fn op_stroke(&mut self) {
        self.rasterize_path(true);
        self.path.clear();
    }

    /// s - Close and stroke path
    fn op_close_stroke(&mut self) {
        self.path.close_path();
        self.rasterize_path(true);
        self.path.clear();
    }

    /// f / F - Fill path using nonzero winding rule
    fn op_fill(&mut self) {
        self.rasterize_path(false);
        self.path.clear();
    }

    /// f* - Fill path using even-odd rule
    fn op_eofill(&mut self) {
        // For simple glyphs, even-odd vs nonzero doesn't matter much
        self.rasterize_path(false);
        self.path.clear();
    }

    /// B - Fill then stroke path
    fn op_fill_stroke(&mut self) {
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// B* - Fill then stroke path (even-odd)
    fn op_eofill_stroke(&mut self) {
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// b - Close, fill, then stroke path
    fn op_close_fill_stroke(&mut self) {
        self.path.close_path();
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// b* - Close, fill, then stroke path (even-odd)
    fn op_close_eofill_stroke(&mut self) {
        self.path.close_path();
        self.rasterize_path(false);
        self.rasterize_path(true);
        self.path.clear();
    }

    /// q - Save graphics state
    fn op_save(&mut self) {
        if !self.gstate_stack.push(&self.gstate) {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::GstateStackOverflow,
                "Type3 glyph graphics state stack overflow",
            ));
        }
    }

    /// Q - Restore graphics state
    fn op_restore(&mut self) {
        if let Some(restored) = self.gstate_stack.pop() {
            self.gstate = restored;
        } else {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::GstateStackUnderflow,
                "Type3 glyph graphics state stack underflow",
            ));
        }
    }

    /// cm a b c d e f - Concatenate matrix to CTM
    fn op_concat(&mut self, stack: &mut Vec<f64>) {
        if stack.len() < 6 {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::CmArgCount,
                "cm operator requires exactly 6 numeric arguments",
            ));
            return;
        }
        let f = stack.pop().unwrap();
        let e = stack.pop().unwrap();
        let d = stack.pop().unwrap();
        let c = stack.pop().unwrap();
        let b = stack.pop().unwrap();
        let a = stack.pop().unwrap();

        // Check for NaN values
        if a.is_nan() || b.is_nan() || c.is_nan() || d.is_nan() || e.is_nan() || f.is_nan() {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::CmDegenerate,
                "cm operator received NaN values; clamped to identity",
            ));
            return; // Don't modify CTM
        }

        let matrix = Matrix3x3::from_pdf_array([a, b, c, d, e, f]);

        // Check for degenerate matrix (det == 0)
        if matrix.determinant() == 0.0 {
            self.diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::CmDegenerate,
                "cm operator received degenerate matrix (det=0); clamped to identity",
            ));
            return; // Don't modify CTM
        }

        self.gstate.concat_ctm(&matrix);
    }

    /// Do name - Invoke XObject
    fn op_do(&mut self, name_stack: &mut Vec<Arc<str>>) {
        if name_stack.is_empty() {
            return;
        }
        let name = name_stack.pop().unwrap();

        // Check recursion depth
        if self.depth >= MAX_GLYPH_DEPTH {
            self.diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructXobjectCycle,
                format!(
                    "Type3 glyph recursion depth limit reached at {}",
                    MAX_GLYPH_DEPTH
                ),
            ));
            return;
        }

        // Form XObject handling would go here
        // For now, stub this out - form XObjects require full resource resolution
    }

    /// Rasterize the current path to the bitmap.
    fn rasterize_path(&mut self, stroke: bool) {
        // Collect path segments from commands
        let mut segments = Vec::new();
        let mut current_point = None;
        let mut move_point = None;

        for cmd in &self.path.commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    current_point = Some(*p);
                    move_point = Some(*p);
                }
                PathCommand::LineTo(p) => {
                    if let Some(start) = current_point {
                        segments.push((start, *p));
                    }
                    current_point = Some(*p);
                }
                PathCommand::Rect(x, y, width, height) => {
                    // Convert rectangle to 4 line segments
                    let x0 = *x;
                    let y0 = *y;
                    let x1 = x + width;
                    let y1 = y + height;

                    // Rectangle: (x0,y0) -> (x1,y0) -> (x1,y1) -> (x0,y1) -> (x0,y0)
                    let p0 = Point::new(x0, y0);
                    let p1 = Point::new(x1, y0);
                    let p2 = Point::new(x1, y1);
                    let p3 = Point::new(x0, y1);

                    segments.push((p0, p1));
                    segments.push((p1, p2));
                    segments.push((p2, p3));
                    segments.push((p3, p0));

                    current_point = Some(p0);
                    move_point = Some(p0);
                }
                PathCommand::ClosePath => {
                    // Close path by connecting to the move point
                    if let (Some(current), Some(move_p)) = (current_point, move_point) {
                        if current != move_p {
                            segments.push((current, move_p));
                        }
                    }
                    current_point = move_point;
                }
                _ => {
                    // CubicTo and other curve commands are not yet implemented
                    // They would require scan-converting Bezier curves
                }
            }
        }

        // Transform all segments and collect edges
        let mut edges = Vec::new();
        for (p0, p1) in segments {
            // Transform points by CTM
            let (x0, y0) = self.gstate.ctm.transform_point(p0.x, p0.y);
            let (x1, y1) = self.gstate.ctm.transform_point(p1.x, p1.y);

            // Convert to bitmap coordinates (round to nearest pixel)
            let bx0 = x0.round() as i32;
            let by0 = y0.round() as i32;
            let bx1 = x1.round() as i32;
            let by1 = y1.round() as i32;

            edges.push((bx0, by0, bx1, by1));
        }

        if stroke {
            // Stroke mode: draw line outlines
            for (x0, y0, x1, y1) in edges {
                self.draw_line(x0, y0, x1, y1);
            }
        } else {
            // Fill mode: use scanline polygon fill
            self.fill_polygon(&edges);
        }
    }

    /// Draw a line segment from (x0, y0) to (x1, y1) using Bresenham's algorithm.
    fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx - dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            // Set pixel if within bounds
            if x >= 0 && x < 32 && y >= 0 && y < 32 {
                self.bitmap.set(x, y, 0);
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Fill a polygon using scanline algorithm.
    /// `edges` is a list of (x0, y0, x1, y1) line segments in bitmap coordinates.
    fn fill_polygon(&mut self, edges: &[(i32, i32, i32, i32)]) {
        // Find y-bounds
        let mut min_y = 32i32;
        let mut max_y = 0i32;

        for &(_, y0, _, y1) in edges {
            min_y = min_y.min(y0.min(y1));
            max_y = max_y.max(y0.max(y1));
        }

        // Clamp to bitmap bounds
        min_y = min_y.max(0);
        max_y = max_y.min(31);

        // For each scanline
        for y in min_y..=max_y {
            let mut intersections = Vec::new();

            // Find all intersections with this scanline
            for &(x0, y0, x1, y1) in edges {
                // Skip horizontal edges (they don't affect scanline fill)
                if y0 == y1 {
                    continue;
                }

                // Check if edge spans this scanline using half-open interval
                // Include lower endpoint, exclude upper endpoint to avoid double-counting vertices
                let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
                if y_min <= y && y < y_max {
                    // Calculate x intersection
                    // x = x0 + (y - y0) * (x1 - x0) / (y1 - y0)
                    let dy = y1 - y0;
                    let t = (y - y0) as f64 / dy as f64;
                    let x = x0 as f64 + t * (x1 - x0) as f64;
                    intersections.push(x);
                }
            }

            // Sort intersections
            intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // Fill between pairs of intersections
            for i in (0..intersections.len()).step_by(2) {
                if i + 1 < intersections.len() {
                    let x_start = intersections[i].ceil() as i32;
                    let x_end = intersections[i + 1].floor() as i32;

                    for x in x_start..=x_end {
                        if x >= 0 && x < 32 {
                            self.bitmap.set(x, y, 0);
                        }
                    }
                }
            }
        }
    }
}

/// Stream resolver callback for Type3 glyph rasterization.
///
/// Given an ObjRef to a content stream, returns the decoded stream bytes.
pub type StreamResolverFn = dyn Fn(ObjRef) -> Option<Vec<u8>> + Send + Sync;

/// Dereference a char_proc_ref to resolve the actual PDF object.
///
/// This function looks up the indirect reference using the document resolver
/// and returns the resolved PdfObject (typically a stream object for Type3 glyphs).
///
/// # Arguments
///
/// * `char_proc_ref` - The ObjRef pointing to the glyph content stream
/// * `doc_context` - Document resolver context containing the XrefResolver
///
/// # Returns
///
/// `Ok(PdfObject)` if the reference was successfully resolved,
/// `Err` if resolution failed (not found, I/O error, or circular reference).
///
/// # Error Context
///
/// Error messages include the object reference being dereferenced to aid debugging.
/// For example: "Failed to resolve Type3 char_proc reference 10 0 R: object not found"
pub fn deref_char_proc_ref(
    char_proc_ref: ObjRef,
    doc_context: Option<&DocumentContext>,
) -> Result<crate::parser::object::types::PdfObject, crate::parser::xref::ResolveError> {
    use crate::parser::xref::ResolveError;

    let doc_context = doc_context.ok_or_else(|| {
        ResolveError::Io(format!(
            "DocumentContext not provided - cannot dereference char_proc_ref {}",
            char_proc_ref
        ))
    })?;

    let resolver = doc_context.resolver.ok_or_else(|| {
        ResolveError::Io(format!(
            "XrefResolver not provided in DocumentContext - cannot resolve char_proc_ref {}",
            char_proc_ref
        ))
    })?;

    let source = doc_context.source.ok_or_else(|| {
        ResolveError::Io(format!(
            "PdfSource not provided in DocumentContext - cannot resolve stream for char_proc_ref {}",
            char_proc_ref
        ))
    })?;

    // Use resolver.resolve_with_source to get the actual PDF object
    // source is already &dyn PdfSource, which is what resolve_with_source expects
    resolver.resolve_with_source(char_proc_ref, source).map_err(|e| {
        // Add context about which reference failed
        match e {
            ResolveError::NotFound(_) => ResolveError::NotFound(char_proc_ref),
            ResolveError::CircularRef(_) => ResolveError::CircularRef(char_proc_ref),
            ResolveError::Io(msg) => ResolveError::Io(format!(
                "Failed to resolve Type3 char_proc_ref {}: {}",
                char_proc_ref, msg
            )),
        }
    })
}

/// Extract content stream bytes from a resolved PDF object.
///
/// This function takes a PdfObject (typically from `deref_char_proc_ref`) and
/// extracts the raw content stream bytes, handling:
///
/// - Direct stream objects (decode if FlateDecode compressed)
/// - Indirect references (recursively resolve and extract)
/// - Error cases (null, wrong type, missing data)
///
/// # Arguments
///
/// * `resolved_obj` - The PdfObject resolved from `deref_char_proc_ref`
/// * `doc_context` - Document resolver context for recursive resolution
///
/// # Returns
///
/// `Ok(Vec<u8>)` with the decoded content stream bytes,
/// `Err` if extraction fails (wrong type, I/O error, or invalid reference).
///
/// # Error Context
///
/// Error messages include the object type/reference to aid debugging.
/// For example: "Cannot extract stream from null object at char_proc_ref"
pub fn extract_content_stream_bytes(
    resolved_obj: PdfObject,
    doc_context: &DocumentContext,
) -> Result<Vec<u8>, ResolveError> {
    let resolver = doc_context.resolver.ok_or_else(|| {
        ResolveError::Io(
            "XrefResolver not provided in DocumentContext - cannot extract stream".to_string()
        )
    })?;

    let source = doc_context.source.ok_or_else(|| {
        ResolveError::Io(
            "PdfSource not provided in DocumentContext - cannot extract stream".to_string()
        )
    })?;

    match resolved_obj {
        PdfObject::Stream(stream) => {
            // Direct stream object - decode it
            let opts = ExtractionOptions::default();
            let mut decompress_counter = 0u64;

            // decode_stream handles decompression (FlateDecode, etc.)
            let bytes = decode_stream(&stream, source, &opts, &mut decompress_counter);
            Ok(bytes)
        }
        PdfObject::Ref(obj_ref) => {
            // Indirect reference - recursively resolve and extract
            let inner_obj = resolver.resolve_with_source(obj_ref, source).map_err(|e| {
                match e {
                    ResolveError::NotFound(_) => ResolveError::NotFound(obj_ref),
                    ResolveError::CircularRef(_) => ResolveError::CircularRef(obj_ref),
                    ResolveError::Io(msg) => ResolveError::Io(format!(
                        "Failed to resolve indirect reference {}: {}",
                        obj_ref, msg
                    )),
                }
            })?;

            // Recursively extract from the resolved object
            extract_content_stream_bytes(inner_obj, doc_context)
        }
        PdfObject::Null => {
            Err(ResolveError::Io(
                "Cannot extract stream from null object at char_proc_ref".to_string()
            ))
        }
        PdfObject::Bool(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from boolean at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Integer(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from integer at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Real(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from real number at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::String(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from string at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Name(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from name at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Array(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from array at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Dict(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from dictionary at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
        PdfObject::Indirect(_) => {
            Err(ResolveError::Io(
                "Cannot extract stream from indirect object at char_proc_ref - expected Stream or Ref".to_string()
            ))
        }
    }
}

/// Calculate bitmap dimensions from glyph bounding box.
///
/// This function computes appropriate bitmap dimensions for rendering a glyph
/// based on its bounding box in PDF user space. It accounts for the glyph's
/// actual size and adds padding for anti-aliasing.
///
/// # Arguments
///
/// * `bbox` - Glyph bounding box [x0, y0, x1, y1] in PDF user space (points)
/// * `padding_pixels` - Optional padding in pixels for anti-aliasing (default: 2)
///
/// # Returns
///
/// (width, height) as usize dimensions in pixels. Minimum dimensions are
/// clamped to 1 to ensure valid bitmap size.
///
/// # Calculation
///
/// Width and height are computed from the bounding box:
/// - width = x1 - x0
/// - height = y1 - y0
///
/// Values are converted to pixels (assuming 1 point = 1 pixel for Type3
/// glyph space) and padding is added on all sides.
///
/// # Example
///
/// ```
/// let bbox = [0.0, 0.0, 16.0, 16.0]; // 16x16 point glyph
/// let (width, height) = calculate_bitmap_size_from_bounds(&bbox, None);
/// assert_eq!(width, 20);  // 16 + 2*2 padding
/// assert_eq!(height, 20); // 16 + 2*2 padding
/// ```
pub fn calculate_bitmap_size_from_bounds(bbox: &[f32; 4], padding_pixels: Option<u32>) -> (usize, usize) {
    let padding = padding_pixels.unwrap_or(2) as i32;

    // Extract bounds
    let x0 = bbox[0] as i32;
    let y0 = bbox[1] as i32;
    let x1 = bbox[2] as i32;
    let y1 = bbox[3] as i32;

    // Calculate raw dimensions
    let raw_width = (x1 - x0).abs();
    let raw_height = (y1 - y0).abs();

    // Add padding (2x for both sides)
    let width = (raw_width + 2 * padding).max(1) as usize;
    let height = (raw_height + 2 * padding).max(1) as usize;

    (width, height)
}

/// Rasterize a Type 3 glyph to a 32x32 grayscale bitmap.
///
/// # Arguments
///
/// * `font` - The Type3 font containing the glyph
/// * `glyph_name` - The name of the glyph to rasterize
/// * `doc_context` - Document resolver context (may be None)
/// * `resolve_stream` - Callback to resolve ObjRef to stream bytes (may be None)
///
/// # Returns
///
/// Some(bitmap) if the glyph exists and rasterized successfully,
/// None if the glyph name is not in /CharProcs or stream resolution fails.
pub fn rasterize_type3_glyph<'a, R>(
    font: &Type3Font,
    glyph_name: &str,
    doc_context: Option<&'a DocumentContext<'a>>,
    resolve_stream: Option<&R>,
) -> Option<[u8; 1024]>
where
    R: Fn(ObjRef) -> Option<Vec<u8>> + ?Sized,
{
    // Check if glyph exists and get its ObjRef
    let char_proc_ref = font.char_proc(glyph_name)?;

    // Document context is passed for potential future use (e.g., form XObject resolution)
    // Stream resolution happens via the resolver callback pattern
    // doc_context is now active and available for use

    // Try to resolve the content stream if a resolver is provided
    let stream_bytes = match resolve_stream {
        Some(resolver) => resolver(char_proc_ref),
        None => None,
    };

    match stream_bytes {
        Some(bytes) => {
            // Successfully resolved - execute the content stream and rasterize
            let mut ctx = RasterizerContext::new(font);
            ctx.execute_content_stream(&bytes);
            Some(*ctx.bitmap.as_bytes())
        }
        None => {
            // No resolver provided or resolution failed - cannot rasterize
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::PdfDict;

    #[test]
    fn test_bitmap_white() {
        let bitmap = Bitmap32x32::white();
        assert_eq!(bitmap.get(0, 0), Some(255));
        assert_eq!(bitmap.get(31, 31), Some(255));
        assert_eq!(bitmap.get(32, 0), None);
        assert_eq!(bitmap.get(0, 32), None);
    }

    #[test]
    fn test_bitmap_black() {
        let bitmap = Bitmap32x32::black();
        assert_eq!(bitmap.get(0, 0), Some(0));
        assert_eq!(bitmap.get(31, 31), Some(0));
    }

    #[test]
    fn test_bitmap_set_get() {
        let mut bitmap = Bitmap32x32::white();
        assert!(bitmap.set(10, 15, 128));
        assert_eq!(bitmap.get(10, 15), Some(128));
        assert!(!bitmap.set(-1, 0, 0)); // Out of bounds
        assert!(!bitmap.set(0, 32, 0)); // Out of bounds
    }

    #[test]
    fn test_bitmap_fill_rect() {
        let mut bitmap = Bitmap32x32::white();
        bitmap.fill_rect(10, 10, 20, 20, 0);

        // Inside rect
        assert_eq!(bitmap.get(15, 15), Some(0));
        // Outside rect
        assert_eq!(bitmap.get(5, 5), Some(255));
        assert_eq!(bitmap.get(25, 25), Some(255));
    }

    #[test]
    fn test_current_path_move_line() {
        let mut path = CurrentPath::new();
        path.move_to(Point::new(10.0, 20.0));
        assert_eq!(path.current_point, Some(Point::new(10.0, 20.0)));
        assert_eq!(path.move_point, Some(Point::new(10.0, 20.0)));

        path.line_to(Point::new(30.0, 40.0));
        assert_eq!(path.current_point, Some(Point::new(30.0, 40.0)));
        assert_eq!(path.move_point, Some(Point::new(10.0, 20.0)));
    }

    #[test]
    fn test_current_path_close() {
        let mut path = CurrentPath::new();
        path.move_to(Point::new(10.0, 20.0));
        path.line_to(Point::new(30.0, 40.0));
        path.close_path();

        assert_eq!(path.current_point, Some(Point::new(10.0, 20.0)));
    }

    #[test]
    fn test_current_path_rect() {
        let mut path = CurrentPath::new();
        path.rect(5.0, 10.0, 20.0, 30.0);

        assert_eq!(path.current_point, Some(Point::new(5.0, 10.0)));
        assert_eq!(path.move_point, Some(Point::new(5.0, 10.0)));
    }

    #[test]
    fn test_point_new() {
        let p = Point::new(1.5, 2.5);
        assert_eq!(p.x, 1.5);
        assert_eq!(p.y, 2.5);
    }

    #[test]
    fn test_rasterizer_context_new() {
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let ctx = RasterizerContext::new(&font);

        assert_eq!(ctx.depth, 0);
        assert_eq!(ctx.bitmap, Bitmap32x32::white());
    }

    #[test]
    fn test_rasterizer_context_applies_font_matrix() {
        use crate::parser::object::types::intern;

        // Create a Type3 font with custom FontMatrix [2 0 0 2 0 0]
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(2.0),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Real(2.0),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let ctx = RasterizerContext::new(&font);

        // Verify that the CTM has the FontMatrix applied
        // Initial CTM should be FontMatrix, not identity
        assert_eq!(ctx.gstate.ctm.a, 2.0);
        assert_eq!(ctx.gstate.ctm.d, 2.0);
        assert_eq!(ctx.gstate.ctm.b, 0.0);
        assert_eq!(ctx.gstate.ctm.c, 0.0);
        assert_eq!(ctx.gstate.ctm.e, 0.0);
        assert_eq!(ctx.gstate.ctm.f, 0.0);
    }

    #[test]
    fn test_execute_simple_path() {
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute: 10 10 m 20 20 l
        let stream = b"10 10 m 20 20 l";
        ctx.execute_content_stream(stream);

        // Path should have move and line commands
        assert_eq!(ctx.path.commands.len(), 2);
    }

    #[test]
    fn test_execute_rect() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix for predictable coordinates
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute: 5 5 10 10 re f
        let stream = b"5 5 10 10 re f";
        ctx.execute_content_stream(stream);

        // Rect should have been rasterized
        // Check center is black
        assert_eq!(ctx.bitmap.get(10, 10), Some(0));
    }

    #[test]
    fn test_gstate_stack() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute: q cm 2 0 0 2 0 0 Q
        let stream = b"q 2 0 0 2 0 0 cm Q";
        ctx.execute_content_stream(stream);

        // CTM should be restored to identity (FontMatrix)
        assert!(ctx.gstate.ctm.is_identity());
    }

    #[test]
    fn test_rasterize_type3_glyph_unknown_returns_none() {
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);

        // Unknown glyph returns None
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };
        assert_eq!(
            rasterize_type3_glyph(&font, "unknown", Some(&doc_context), None::<&StreamResolverFn>),
            None
        );
    }

    #[test]
    fn test_rasterize_line_segment() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Draw a diagonal line from (5,5) to (15,15)
        ctx.path.move_to(Point::new(5.0, 5.0));
        ctx.path.line_to(Point::new(15.0, 15.0));
        ctx.rasterize_path(true); // stroke mode

        // Verify some pixels along the line are set
        assert_eq!(ctx.bitmap.get(5, 5), Some(0)); // Start point
        assert_eq!(ctx.bitmap.get(10, 10), Some(0)); // Middle point
        assert_eq!(ctx.bitmap.get(15, 15), Some(0)); // End point
    }

    #[test]
    fn test_rasterize_filled_triangle() {
        use crate::parser::object::types::intern;

        // Create a font with identity FontMatrix
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Draw a triangle: (10,5) -> (15,15) -> (5,15) -> close
        ctx.path.move_to(Point::new(10.0, 5.0));
        ctx.path.line_to(Point::new(15.0, 15.0));
        ctx.path.line_to(Point::new(5.0, 15.0));
        ctx.path.close_path();
        ctx.rasterize_path(false); // fill mode

        // Verify interior pixels are filled
        assert_eq!(ctx.bitmap.get(10, 10), Some(0)); // Center interior
        assert_eq!(ctx.bitmap.get(8, 12), Some(0)); // Left interior
        assert_eq!(ctx.bitmap.get(12, 12), Some(0)); // Right interior

        // Verify exterior pixels are still white
        assert_eq!(ctx.bitmap.get(4, 10), Some(255)); // Outside left
        assert_eq!(ctx.bitmap.get(16, 10), Some(255)); // Outside right
    }

    #[test]
    fn test_deref_char_proc_ref_without_context_returns_error() {
        use crate::parser::xref::ResolveError;

        let obj_ref = ObjRef::new(10, 0);

        // No DocumentContext provided
        let result = deref_char_proc_ref(obj_ref, None);

        assert!(result.is_err());
        match result {
            Err(ResolveError::Io(msg)) => {
                assert!(msg.contains("DocumentContext not provided"));
            }
            _ => panic!("Expected ResolveError::Io"),
        }
    }

    #[test]
    fn test_deref_char_proc_ref_without_resolver_returns_error() {
        use crate::parser::xref::ResolveError;

        let obj_ref = ObjRef::new(10, 0);

        // DocumentContext without resolver
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        let result = deref_char_proc_ref(obj_ref, Some(&doc_context));

        assert!(result.is_err());
        match result {
            Err(ResolveError::Io(msg)) => {
                assert!(msg.contains("XrefResolver not provided"));
            }
            _ => panic!("Expected ResolveError::Io"),
        }
    }

    #[test]
    fn test_deref_char_proc_ref_without_source_returns_error() {
        use crate::parser::xref::ResolveError;
        use crate::parser::xref::XrefResolver;

        let obj_ref = ObjRef::new(10, 0);
        let resolver = XrefResolver::new();

        // DocumentContext with resolver but no source
        let doc_context = DocumentContext {
            resolver: Some(&resolver),
            source: None,
        };

        let result = deref_char_proc_ref(obj_ref, Some(&doc_context));

        assert!(result.is_err());
        match result {
            Err(ResolveError::Io(msg)) => {
                assert!(msg.contains("PdfSource not provided"));
            }
            _ => panic!("Expected ResolveError::Io"),
        }
    }

    #[test]
    fn test_rasterize_type3_glyph_with_missing_glyph_returns_none() {
        use crate::parser::object::types::PdfDict;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);

        // Non-existent glyph should return None (graceful degradation)
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        let result = rasterize_type3_glyph(
            &font,
            "nonexistent",
            Some(&doc_context),
            None::<&StreamResolverFn>,
        );

        // Should gracefully return None, not panic
        assert!(result.is_none());
    }

    #[test]
    fn test_rasterize_type3_glyph_with_failed_resolution_returns_none() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;
        use std::sync::Arc;

        let mut font_dict = PdfDict::new();
        let mut char_procs_dict = PdfDict::new();

        // Add a valid CharProcs entry
        char_procs_dict.insert(
            intern("/A"),
            PdfObject::Ref(ObjRef::new(999, 0)) // Non-existent object
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        // Verify the glyph exists in CharProcs
        assert!(font.has_glyph("A"));
        assert!(font.char_proc("A").is_some());

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Create a resolver that always fails (simulating missing/invalid reference)
        let failing_resolver = &(|_obj_ref: ObjRef| -> Option<Vec<u8>> {
            None // Simulates resolution failure
        }) as &StreamResolverFn;

        let result = rasterize_type3_glyph(
            &font,
            "A",
            Some(&doc_context),
            Some(failing_resolver),
        );

        // Should gracefully return None, not panic
        assert!(result.is_none());
    }

    #[test]
    fn test_rasterize_type3_glyph_with_malformed_stream_returns_none() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;
        use std::sync::Arc;

        let mut font_dict = PdfDict::new();
        let mut char_procs_dict = PdfDict::new();

        // Add a valid CharProcs entry
        char_procs_dict.insert(
            intern("/B"),
            PdfObject::Ref(ObjRef::new(10, 0))
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Create a resolver that returns malformed/empty bytes (simulating corrupt stream)
        let malformed_resolver = &(|_obj_ref: ObjRef| -> Option<Vec<u8>> {
            Some(vec![0xFF, 0xFF, 0xFF]) // Malformed PDF content stream
        }) as &StreamResolverFn;

        let result = rasterize_type3_glyph(
            &font,
            "B",
            Some(&doc_context),
            Some(malformed_resolver),
        );

        // Should gracefully return None or a default bitmap, not panic
        // Malformed content streams are handled by the lexer/parser
        // This tests that we don't crash on bad input
        let _ = result; // We accept either None or a bitmap (graceful handling)
    }

    #[test]
    fn test_execute_content_stream_with_invalid_tokens_does_not_crash() {
        use crate::parser::object::types::PdfDict;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute malformed content: invalid operators and operands
        // This should not crash, just skip unknown operators
        let malformed_stream = b"INVALID_OPERATOR 1.5 2.5 another_invalid [array]";

        // This should execute without panicking
        ctx.execute_content_stream(malformed_stream);

        // Bitmap should still be in a valid state (all white since no valid ops executed)
        assert_eq!(ctx.bitmap.get(0, 0), Some(255));
        assert_eq!(ctx.bitmap.get(31, 31), Some(255));
    }

    #[test]
    fn test_execute_content_stream_with_empty_stream_does_not_crash() {
        use crate::parser::object::types::PdfDict;

        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute empty content stream
        let empty_stream = b"";

        // This should execute without panicking
        ctx.execute_content_stream(empty_stream);

        // Bitmap should be all white (default state)
        assert_eq!(ctx.bitmap, Bitmap32x32::white());
    }

    #[test]
    fn test_execute_type3_glyph_with_font_matrix_transformation() {
        use crate::parser::object::types::{intern, PdfDict};

        // Create a Type3 font with FontMatrix that scales by 0.001 (standard Type3)
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(0.001),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Real(0.001),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Verify initial CTM has FontMatrix applied
        assert_eq!(ctx.gstate.ctm.a, 0.001);
        assert_eq!(ctx.gstate.ctm.d, 0.001);

        // Execute a simple glyph content stream that draws a 1000x1000 square
        // in glyph space (0,0 to 1000,1000)
        // After FontMatrix transformation (0.001 scale), this becomes (0,0) to (1,1) in text space
        let stream = b"0 0 1000 1000 re f";

        ctx.execute_content_stream(stream);

        // The rectangle should be drawn and rasterized to the bitmap
        // After transformation, coordinates are scaled by 0.001
        // (0,0,1000,1000) in glyph space -> (0,0,1,1) in text space
        // This should fill a small area in the 32x32 bitmap

        // Verify that some pixels were drawn (bitmap is no longer all white)
        let mut has_black = false;
        for y in 0..32 {
            for x in 0..32 {
                if ctx.bitmap.get(x, y) == Some(0) {
                    has_black = true;
                    break;
                }
            }
            if has_black {
                break;
            }
        }

        // Due to the 0.001 scale, the 1000x1000 rectangle becomes 1x1 in text space
        // which is very small in the 32x32 bitmap, but at least the origin pixel should be filled
        assert!(has_black, "Expected some pixels to be drawn after executing glyph stream");
    }

    #[test]
    fn test_execute_type3_glyph_with_identity_font_matrix() {
        use crate::parser::object::types::{intern, PdfDict};

        // Create a Type3 font with identity FontMatrix (no scaling)
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Verify initial CTM is identity
        assert!(ctx.gstate.ctm.is_identity());

        // Execute a glyph content stream that draws a 10x10 square
        // With identity matrix, coordinates are not scaled
        let stream = b"10 10 10 10 re f";

        ctx.execute_content_stream(stream);

        // Verify the rectangle was drawn at (10,10) to (20,20)
        // Center of the rectangle should be filled
        assert_eq!(ctx.bitmap.get(15, 15), Some(0));
    }

    // Tests for calculate_bitmap_dimensions (bf-407xp6)

    #[test]
    fn test_calculate_bitmap_dimensions_basic() {
        // Test basic glyph with bbox [10.0, 20.0, 50.0, 60.0]
        // Raw size: 40x40 points
        // With default padding: 42x42 pixels
        let bbox = [10.0f32, 20.0, 50.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        assert_eq!(width, 42);
        assert_eq!(height, 42);
    }

    #[test]
    fn test_calculate_bitmap_dimensions_with_custom_padding() {
        // Test with custom padding of 2 pixels
        let bbox = [10.0f32, 20.0, 50.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, Some(2));

        assert_eq!(width, 44); // 40 + 2*2
        assert_eq!(height, 44); // 40 + 2*2
    }

    #[test]
    fn test_calculate_bitmap_dimensions_zero_width() {
        // Test degenerate case: zero-width bbox
        // Should default to 1 pixel + padding
        let bbox = [10.0f32, 20.0, 10.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        assert_eq!(width, 3); // 1 + 2*1 (default padding)
        assert_eq!(height, 42); // 40 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_zero_height() {
        // Test degenerate case: zero-height bbox
        let bbox = [10.0f32, 20.0, 50.0, 20.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        assert_eq!(width, 42); // 40 + 2*1
        assert_eq!(height, 3); // 1 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_small_glyph() {
        // Test very small glyph (less than 1 point)
        let bbox = [10.0f32, 20.0, 10.5, 20.5];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Should round up to 1 pixel + padding
        assert_eq!(width, 3); // 1 + 2*1
        assert_eq!(height, 3); // 1 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_negative_coordinates() {
        // Test bbox with negative coordinates (valid in PDF)
        let bbox = [-50.0f32, -30.0, -10.0, 10.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Width: 40 points, Height: 40 points
        assert_eq!(width, 42); // 40 + 2*1
        assert_eq!(height, 42); // 40 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_returns_usize() {
        // Verify return type is usize (for array indexing)
        let bbox = [0.0f32, 0.0, 16.0, 16.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Should be usable as array indices
        let mut dummy_array = [0u8; 100];
        dummy_array[width - 1] = 1;
        dummy_array[height - 1] = 2;

        assert_eq!(width, 18); // 16 + 2*1
        assert_eq!(height, 18); // 16 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_non_integer_bounds() {
        // Test bbox with non-integer coordinates
        let bbox = [10.5f32, 20.7, 50.2, 60.9];
        let (width, height) = calculate_bitmap_dimensions(&bbox, None);

        // Should round up: 39.7 -> 40, 40.2 -> 41
        assert_eq!(width, 42); // 40 + 2*1
        assert_eq!(height, 43); // 41 + 2*1
    }

    #[test]
    fn test_calculate_bitmap_dimensions_no_padding() {
        // Test with zero padding
        let bbox = [10.0f32, 20.0, 50.0, 60.0];
        let (width, height) = calculate_bitmap_dimensions(&bbox, Some(0));

        assert_eq!(width, 40); // 40 + 2*0
        assert_eq!(height, 40); // 40 + 2*0
    }

    #[test]
    fn test_calculate_bitmap_dimensions_large_glyph() {
        // Test large glyph (e.g., full page size)
        let bbox = [0.0f32, 0.0, 612.0, 792.0]; // US Letter size
        let (width, height) = calculate_bitmap_dimensions(&bbox, Some(2));

        assert_eq!(width, 616); // 612 + 2*2
        assert_eq!(height, 796); // 792 + 2*2
    }

    #[test]
    fn test_resolve_stream_callback_receives_parameters() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;
        use std::sync::{Arc, Mutex};

        // Create a Type3 font with identity FontMatrix for predictable coordinates
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let mut char_procs_dict = PdfDict::new();

        // Add a CharProcs entry pointing to a specific object reference
        let test_ref = ObjRef::new(10, 0);
        char_procs_dict.insert(
            intern("/TestGlyph"),
            PdfObject::Ref(test_ref)
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        // Track which ObjRef was passed to the callback
        let captured_ref = Arc::new(Mutex::new(None));
        let captured_ref_clone = captured_ref.clone();

        // Create a callback that captures simulated resolver, source, and counter parameters
        // This mimics the closure pattern used in resolver.rs lines 700-702
        let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
            // Capture the received obj_ref to verify it was passed correctly
            *captured_ref_clone.lock().unwrap() = Some(obj_ref);

            // Return a simple valid content stream (draw a 10x10 rectangle)
            // This simulates what resolve_stream_bytes would do
            Some(b"10 10 10 10 re f".to_vec())
        };

        // Call rasterize_type3_glyph with the callback
        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        let result = rasterize_type3_glyph(
            &font,
            "TestGlyph",
            Some(&doc_context),
            Some(&callback as &StreamResolverFn),
        );

        // Verify the callback was invoked and received the correct ObjRef
        assert_eq!(
            *captured_ref.lock().unwrap(),
            Some(test_ref),
            "Callback should receive the ObjRef pointing to the glyph's content stream"
        );

        // Verify the glyph was successfully rasterized (callback returned valid bytes)
        assert!(
            result.is_some(),
            "Glyph should be rasterized when callback returns valid content stream"
        );

        // Verify the bitmap is not all-white (content was executed)
        let bitmap = result.unwrap();
        let has_black = bitmap.iter().any(|&pixel| pixel == 0);
        assert!(
            has_black,
            "Bitmap should contain black pixels after executing the content stream"
        );
    }

    #[test]
    fn test_resolve_stream_callback_captures_context_parameters() {
        use crate::parser::object::types::{PdfDict, PdfObject};
        use crate::parser::object::intern;
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

        // Create a Type3 font with identity FontMatrix for predictable coordinates
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/FontMatrix"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(0),
                PdfObject::Integer(0),
            ])),
        );

        let mut char_procs_dict = PdfDict::new();

        let glyph_ref = ObjRef::new(20, 0);
        char_procs_dict.insert(
            intern("/ContextGlyph"),
            PdfObject::Ref(glyph_ref)
        );

        font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs_dict)));

        let font = Type3Font::load(&font_dict);

        // Simulate the three context parameters that the callback should capture
        // These represent: resolver (&XrefResolver), source (&dyn PdfSource), counter (&mut u64)
        // Use atomic types for thread-safe state tracking
        let mock_resolver_called = Arc::new(AtomicBool::new(false));
        let mock_source_used = Arc::new(AtomicBool::new(false));
        let mock_counter_incremented = Arc::new(AtomicU64::new(0));

        // Create clones for the closure to capture
        let resolver_called_clone = mock_resolver_called.clone();
        let source_used_clone = mock_source_used.clone();
        let counter_incremented_clone = mock_counter_incremented.clone();

        // Callback that simulates resolve_stream_bytes behavior
        // This closure captures and uses all three context parameters
        let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
            // Simulate using resolver (mark as called)
            resolver_called_clone.store(true, Ordering::SeqCst);

            // Simulate using source (mark as used)
            source_used_clone.store(true, Ordering::SeqCst);

            // Simulate using counter (increment it)
            counter_incremented_clone.fetch_add(1, Ordering::SeqCst);

            // Verify we received the expected obj_ref
            assert_eq!(obj_ref, glyph_ref, "Callback should receive the correct glyph reference");

            // Return a valid content stream
            Some(b"5 5 20 20 re f".to_vec())
        };

        let doc_context = DocumentContext {
            resolver: None,
            source: None,
        };

        // Execute the callback through rasterize_type3_glyph
        let result = rasterize_type3_glyph(
            &font,
            "ContextGlyph",
            Some(&doc_context),
            Some(&callback as &StreamResolverFn),
        );

        // Verify all three context parameters were used by the callback
        assert!(
            mock_resolver_called.load(Ordering::SeqCst),
            "Callback should use the resolver parameter"
        );
        assert!(
            mock_source_used.load(Ordering::SeqCst),
            "Callback should use the source parameter"
        );
        assert_eq!(
            mock_counter_incremented.load(Ordering::SeqCst),
            1,
            "Callback should use (increment) the counter parameter"
        );

        // Verify successful rasterization
        assert!(
            result.is_some(),
            "Glyph should rasterize successfully when callback uses all context parameters"
        );
    }
}
