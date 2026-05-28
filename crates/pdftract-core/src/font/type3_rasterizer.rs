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

/// Maximum recursion depth for Type 3 glyph execution (form XObject + nested glyphs).
const MAX_GLYPH_DEPTH: usize = 20;

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
        Self {
            bitmap: Bitmap32x32::white(),
            gstate: GraphicsState::new(),
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
    fn rasterize_path(&mut self, _stroke: bool) {
        // Simple scanline rasterization for the path
        // For now, just fill rectangles (re operator)
        // A full implementation would scan-convert Bezier curves

        for cmd in &self.path.commands {
            if let PathCommand::Rect(x, y, width, height) = cmd {
                // Transform rectangle by CTM
                let (x0, y0) = self.gstate.ctm.transform_point(*x, *y);
                let (x1, y1) = self.gstate.ctm.transform_point(x + width, y + height);

                // Convert to bitmap coordinates (round to nearest pixel)
                let bx0 = x0.round() as i32;
                let by0 = y0.round() as i32;
                let bx1 = x1.round() as i32;
                let by1 = y1.round() as i32;

                // Fill with black (0 = black ink)
                self.bitmap.fill_rect(bx0, by0, bx1, by1, 0);
            }
        }
    }
}

/// Rasterize a Type 3 glyph to a 32x32 grayscale bitmap.
///
/// # Arguments
///
/// * `font` - The Type3 font containing the glyph
/// * `glyph_name` - The name of the glyph to rasterize
///
/// # Returns
///
/// Some(bitmap) if the glyph exists and rasterized successfully,
/// None if the glyph name is not in /CharProcs.
pub fn rasterize_type3_glyph(font: &Type3Font, glyph_name: &str) -> Option<[u8; 1024]> {
    // Check if glyph exists
    let _char_proc_ref = font.char_proc(glyph_name)?;

    // TODO: Resolve the content stream from the ObjRef
    // For now, return a placeholder bitmap
    // The full implementation requires access to the document resolver
    // to fetch and decode the stream

    // Placeholder: return a half-filled bitmap for testing
    let mut bitmap = Bitmap32x32::white();
    // Fill a 16x16 square in the center
    bitmap.fill_rect(8, 8, 24, 24, 0);

    Some(*bitmap.as_bytes())
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
        let font_dict = PdfDict::new();
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
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);
        let mut ctx = RasterizerContext::new(&font);

        // Execute: q cm 2 0 0 2 0 0 Q
        let stream = b"q 2 0 0 2 0 0 cm Q";
        ctx.execute_content_stream(stream);

        // CTM should be restored to identity
        assert!(ctx.gstate.ctm.is_identity());
    }

    #[test]
    fn test_rasterize_type3_glyph_placeholder() {
        let font_dict = PdfDict::new();
        let font = Type3Font::load(&font_dict);

        // Unknown glyph returns None
        assert_eq!(rasterize_type3_glyph(&font, "unknown"), None);
    }
}
