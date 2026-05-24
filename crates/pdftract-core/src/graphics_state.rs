//! Graphics state management for PDF content stream processing.
//!
//! This module implements the graphics state stack and CTM (Current Transformation Matrix)
//! tracking needed for Phase 3 content stream processing and Phase 5.2.1 image compositing.
//!
//! Per PDF spec section 8.4 "Graphics State":
//! - q operator pushes a copy of the current graphics state onto the stack
//! - Q operator pops the graphics state stack and restores the state
//! - cm operator concatenates a matrix with the CTM
//!
//! The CTM is a 3x3 transformation matrix that transforms coordinates from user space
//! to device space. For 2D operations, only 6 values are relevant: [a b c d e f]
//! representing the affine transformation:
//!   x' = a*x + c*y + e
//!   y' = b*x + d*y + f

use std::sync::Arc;

use crate::font::Font;

/// Maximum depth of graphics state stack (prevents stack overflow).
const MAX_GSTATE_DEPTH: usize = 32;

/// Color space and value for text extraction output.
///
/// Per PDF spec, color spaces include DeviceGray, DeviceRGB, DeviceCMYK,
/// and special color spaces (Spot, ICCBased, Pattern, etc.). For text extraction,
/// we only need to serialize DeviceGray/RGB/CMYK to CSS hex; Spot and Other
/// become null in JSON output.
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    /// DeviceGray: single component 0.0–1.0 (black to white)
    DeviceGray(f32),
    /// DeviceRGB: three components [R, G, B] each 0.0–1.0
    DeviceRGB([f32; 3]),
    /// DeviceCMYK: four components [C, M, Y, K] each 0.0–1.0
    DeviceCMYK([f32; 4]),
    /// Spot color: (colorant name, tint 0.0–1.0)
    Spot(Arc<str>, f32),
    /// Other color spaces (CalRGB, ICCBased, Pattern, etc.) — not serializable to CSS
    Other,
}

impl Color {
    /// Convert to CSS hex color string for JSON output.
    ///
    /// Returns `Some("#rrggbb")` for DeviceGray, DeviceRGB, and DeviceCMYK.
    /// Returns `None` for Spot and Other (serialized as null in JSON).
    ///
    /// # Conversion rules
    ///
    /// - `DeviceGray(v)`: treated as RGB `[v, v, v]` → `#rrggbb`
    /// - `DeviceRGB([r, g, b])`: direct mapping → `#rrggbb`
    /// - `DeviceCMYK([c, m, y, k])`: naive formula `R = (1-C)*(1-K)`, etc. → `#rrggbb`
    /// - `Spot`, `Other`: `None`
    pub fn to_css_hex(&self) -> Option<String> {
        match self {
            Color::DeviceGray(v) => {
                let r = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                let g = r;
                let b = r;
                Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
            }
            Color::DeviceRGB(rgb) => {
                let r = (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                let g = (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                let b = (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
            }
            Color::DeviceCMYK(cmyk) => {
                // Naive CMYK → RGB conversion: R = (1-C)*(1-K)
                let c = cmyk[0].clamp(0.0, 1.0);
                let m = cmyk[1].clamp(0.0, 1.0);
                let y = cmyk[2].clamp(0.0, 1.0);
                let k = cmyk[3].clamp(0.0, 1.0);
                let r = ((1.0 - c) * (1.0 - k) * 255.0).round() as u8;
                let g = ((1.0 - m) * (1.0 - k) * 255.0).round() as u8;
                let b = ((1.0 - y) * (1.0 - k) * 255.0).round() as u8;
                Some(format!("#{:02x}{:02x}{:02x}", r, g, b))
            }
            Color::Spot(_, _) | Color::Other => None,
        }
    }
}

/// 3x3 transformation matrix for PDF coordinate transformations.
///
/// Only the first 6 values are used for 2D affine transformations:
/// [a b 0]
/// [c d 0]
/// [e f 1]
///
/// Per PDF spec, the CTM transforms from user space to device space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3x3 {
    /// The a coefficient (x scale)
    pub a: f64,
    /// The b coefficient (y skew)
    pub b: f64,
    /// The c coefficient (x skew)
    pub c: f64,
    /// The d coefficient (y scale)
    pub d: f64,
    /// The e coefficient (x translation)
    pub e: f64,
    /// The f coefficient (y translation)
    pub f: f64,
}

impl Matrix3x3 {
    /// Create a new identity matrix.
    #[inline]
    pub fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create a matrix from a PDF-style 6-element array [a b c d e f].
    #[inline]
    pub fn from_pdf_array(arr: [f64; 6]) -> Self {
        Self {
            a: arr[0],
            b: arr[1],
            c: arr[2],
            d: arr[3],
            e: arr[4],
            f: arr[5],
        }
    }

    /// Check if this is the identity matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.a == 1.0
            && self.b == 0.0
            && self.c == 0.0
            && self.d == 1.0
            && self.e == 0.0
            && self.f == 0.0
    }

    /// Multiply this matrix by another (this * other).
    #[inline]
    pub fn multiply(&self, other: &Matrix3x3) -> Matrix3x3 {
        Matrix3x3 {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// Transform a point (x, y) by this matrix.
    #[inline]
    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        let new_x = self.a * x + self.c * y + self.e;
        let new_y = self.b * x + self.d * y + self.f;
        (new_x, new_y)
    }

    /// Get the determinant of this matrix.
    #[inline]
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// Check if the matrix has a negative determinant (flip).
    #[inline]
    pub fn has_flip(&self) -> bool {
        self.determinant() < 0.0
    }

    /// Create a scale matrix.
    #[inline]
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create a translation matrix.
    #[inline]
    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Create a rotation matrix (angle in radians).
    #[inline]
    pub fn rotate(angle: f64) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            a: cos_a,
            b: sin_a,
            c: -sin_a,
            d: cos_a,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Invert this matrix.
    ///
    /// Returns None if the matrix is not invertible (determinant is zero).
    #[inline]
    pub fn invert(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < f64::EPSILON {
            return None;
        }
        let inv_det = 1.0 / det;
        Some(Matrix3x3 {
            a: self.d * inv_det,
            b: -self.b * inv_det,
            c: -self.c * inv_det,
            d: self.a * inv_det,
            e: (self.c * self.f - self.d * self.e) * inv_det,
            f: (self.b * self.e - self.a * self.f) * inv_det,
        })
    }
}

impl Default for Matrix3x3 {
    fn default() -> Self {
        Self::identity()
    }
}

/// Graphics state as defined in PDF spec section 8.4.
///
/// This contains all 13 graphics state parameters needed for content stream processing.
/// Per INV-30, GraphicsState is Clone (cheap thanks to Arc<Font>) so q/Q can snapshot it.
#[derive(Clone)]
pub struct GraphicsState {
    /// Current Transformation Matrix (ctm)
    pub ctm: Matrix3x3,
    /// Text matrix (Tm)
    pub text_matrix: Matrix3x3,
    /// Text line matrix (Tlm)
    pub text_line_matrix: Matrix3x3,
    /// Current font (None until Tf operator)
    pub font: Option<Arc<Font>>,
    /// Font size (set by Tf operator)
    pub font_size: f64,
    /// Character spacing (Tc)
    pub char_spacing: f64,
    /// Word spacing (Tw)
    pub word_spacing: f64,
    /// Horizontal scaling (Tz, percentage, default 100)
    pub horiz_scaling: f64,
    /// Leading (TL)
    pub leading: f64,
    /// Text rise (Ts)
    pub text_rise: f64,
    /// Text rendering mode (Tr, 0–7)
    pub text_rendering_mode: u8,
    /// Fill color
    pub fill_color: Color,
    /// Stroke color
    pub stroke_color: Color,
}

impl std::fmt::Debug for GraphicsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Font doesn't implement Debug, so we show a placeholder
        f.debug_struct("GraphicsState")
            .field("ctm", &self.ctm)
            .field("text_matrix", &self.text_matrix)
            .field("text_line_matrix", &self.text_line_matrix)
            .field("font", &self.font.as_ref().map(|_| "<Arc<Font>>"))
            .field("font_size", &self.font_size)
            .field("char_spacing", &self.char_spacing)
            .field("word_spacing", &self.word_spacing)
            .field("horiz_scaling", &self.horiz_scaling)
            .field("leading", &self.leading)
            .field("text_rise", &self.text_rise)
            .field("text_rendering_mode", &self.text_rendering_mode)
            .field("fill_color", &self.fill_color)
            .field("stroke_color", &self.stroke_color)
            .finish()
    }
}

impl GraphicsState {
    /// Create a new graphics state with identity CTM.
    #[inline]
    pub fn new() -> Self {
        Self::initial()
    }

    /// Create the initial graphics state per PDF spec.
    ///
    /// Returns a state with:
    /// - CTM: identity matrix
    /// - text_matrix: identity matrix (will be reset on BT)
    /// - text_line_matrix: identity matrix (will be reset on BT)
    /// - font: None (must be set by Tf operator before use)
    /// - font_size: 0.0 (must be set by Tf operator)
    /// - char_spacing: 0.0
    /// - word_spacing: 0.0
    /// - horiz_scaling: 100.0
    /// - leading: 0.0
    /// - text_rise: 0.0
    /// - text_rendering_mode: 0
    /// - fill_color: DeviceGray(0.0) (black per PDF spec)
    /// - stroke_color: DeviceGray(0.0) (black per PDF spec)
    #[inline]
    pub fn initial() -> Self {
        Self {
            ctm: Matrix3x3::identity(),
            text_matrix: Matrix3x3::identity(),
            text_line_matrix: Matrix3x3::identity(),
            font: None,
            font_size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horiz_scaling: 100.0,
            leading: 0.0,
            text_rise: 0.0,
            text_rendering_mode: 0,
            fill_color: Color::DeviceGray(0.0),
            stroke_color: Color::DeviceGray(0.0),
        }
    }

    /// Concatenate a matrix with the current CTM.
    ///
    /// This implements the `cm` operator behavior: CTM' = CTM × M
    #[inline]
    pub fn concat_ctm(&mut self, matrix: &Matrix3x3) {
        self.ctm = self.ctm.multiply(matrix);
    }
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Graphics state stack for q/Q operators.
///
/// Per PDF spec, the graphics state stack has a maximum depth to prevent
/// stack overflow in malformed PDFs.
#[derive(Debug, Clone)]
pub struct GraphicsStateStack {
    /// The stack of saved graphics states
    stack: Vec<GraphicsState>,
}

impl GraphicsStateStack {
    /// Create a new empty graphics state stack.
    #[inline]
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(16),
        }
    }

    /// Push a graphics state onto the stack (implements `q` operator).
    ///
    /// Returns false if the stack would exceed the maximum depth.
    #[inline]
    pub fn push(&mut self, state: &GraphicsState) -> bool {
        if self.stack.len() >= MAX_GSTATE_DEPTH {
            return false;
        }
        self.stack.push(state.clone());
        true
    }

    /// Pop a graphics state from the stack (implements `Q` operator).
    ///
    /// Returns None if the stack is empty.
    #[inline]
    pub fn pop(&mut self) -> Option<GraphicsState> {
        self.stack.pop()
    }

    /// Get the current depth of the stack.
    #[inline]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Check if the stack is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

impl Default for GraphicsStateStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let m = Matrix3x3::identity();
        assert!(m.is_identity());
        assert_eq!(m.transform_point(1.0, 0.0), (1.0, 0.0));
        assert_eq!(m.transform_point(0.0, 1.0), (0.0, 1.0));
    }

    #[test]
    fn test_translation_matrix() {
        let m = Matrix3x3::from_pdf_array([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        let (x, y) = m.transform_point(0.0, 0.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    #[test]
    fn test_scale_matrix() {
        let m = Matrix3x3::from_pdf_array([2.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
        let (x, y) = m.transform_point(1.0, 1.0);
        assert_eq!(x, 2.0);
        assert_eq!(y, 3.0);
    }

    #[test]
    fn test_matrix_multiply() {
        let m1 = Matrix3x3::from_pdf_array([2.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let m2 = Matrix3x3::from_pdf_array([1.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
        let result = m1.multiply(&m2);

        // Should scale x by 2, y by 3
        let (x, y) = result.transform_point(1.0, 1.0);
        assert_eq!(x, 2.0);
        assert_eq!(y, 3.0);
    }

    #[test]
    fn test_determinant_positive() {
        let m = Matrix3x3::identity();
        assert_eq!(m.determinant(), 1.0);
        assert!(!m.has_flip());
    }

    #[test]
    fn test_determinant_negative() {
        // Y flip matrix
        let m = Matrix3x3::from_pdf_array([1.0, 0.0, 0.0, -1.0, 0.0, 0.0]);
        assert_eq!(m.determinant(), -1.0);
        assert!(m.has_flip());
    }

    #[test]
    fn test_gstate_stack_push_pop() {
        let mut stack = GraphicsStateStack::new();
        let state1 = GraphicsState::new();

        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);

        assert!(stack.push(&state1));
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_empty());

        let popped = stack.pop();
        assert!(popped.is_some());
        assert!(stack.is_empty());
    }

    #[test]
    fn test_gstate_stack_depth_limit() {
        let mut stack = GraphicsStateStack::new();
        let state = GraphicsState::new();

        // Fill to max depth
        for _ in 0..MAX_GSTATE_DEPTH {
            assert!(stack.push(&state));
        }

        // Should fail to push beyond max
        assert!(!stack.push(&state));
        assert_eq!(stack.depth(), MAX_GSTATE_DEPTH);
    }

    #[test]
    fn test_gstate_ctm_concat() {
        let mut state = GraphicsState::new();
        let translate = Matrix3x3::from_pdf_array([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        state.concat_ctm(&translate);

        let (x, y) = state.ctm.transform_point(0.0, 0.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    #[test]
    fn test_gstate_stack_restore() {
        let mut stack = GraphicsStateStack::new();
        let mut state1 = GraphicsState::new();
        let mut state2 = GraphicsState::new();

        // Modify state1
        let translate = Matrix3x3::from_pdf_array([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        state1.concat_ctm(&translate);

        // Push state1
        stack.push(&state1);

        // Modify state2
        let scale = Matrix3x3::from_pdf_array([2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        state2.concat_ctm(&scale);

        // Pop should restore state1
        let restored = stack.pop().unwrap();
        let (x, y) = restored.ctm.transform_point(0.0, 0.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    // Tests for GraphicsState::initial()

    #[test]
    fn test_gstate_initial_ctm_is_identity() {
        let state = GraphicsState::initial();
        assert!(state.ctm.is_identity());
    }

    #[test]
    fn test_gstate_initial_font_size_is_zero() {
        let state = GraphicsState::initial();
        assert_eq!(state.font_size, 0.0);
    }

    #[test]
    fn test_gstate_initial_fill_color_is_black() {
        let state = GraphicsState::initial();
        assert_eq!(state.fill_color, Color::DeviceGray(0.0));
    }

    #[test]
    fn test_gstate_initial_horiz_scaling_is_100() {
        let state = GraphicsState::initial();
        assert_eq!(state.horiz_scaling, 100.0);
    }

    #[test]
    fn test_gstate_initial_text_matrices_are_identity() {
        let state = GraphicsState::initial();
        assert!(state.text_matrix.is_identity());
        assert!(state.text_line_matrix.is_identity());
    }

    #[test]
    fn test_gstate_initial_font_is_none() {
        let state = GraphicsState::initial();
        assert!(state.font.is_none());
    }

    #[test]
    fn test_gstate_initial_text_rendering_mode_is_0() {
        let state = GraphicsState::initial();
        assert_eq!(state.text_rendering_mode, 0);
    }

    #[test]
    fn test_gstate_clone_deep_equal() {
        let state = GraphicsState::initial();
        let cloned = state.clone();
        assert_eq!(state.ctm, cloned.ctm);
        assert_eq!(state.text_matrix, cloned.text_matrix);
        assert_eq!(state.font_size, cloned.font_size);
        assert_eq!(state.fill_color, cloned.fill_color);
    }

    // Tests for Color::to_css_hex()

    #[test]
    fn test_color_device_rgb_to_css_hex() {
        let color = Color::DeviceRGB([1.0, 0.0, 0.0]);
        assert_eq!(color.to_css_hex(), Some("#ff0000".into()));
    }

    #[test]
    fn test_color_device_gray_to_css_hex() {
        let color = Color::DeviceGray(0.5);
        assert_eq!(color.to_css_hex(), Some("#808080".into()));
    }

    #[test]
    fn test_color_device_cmyk_to_css_hex() {
        let color = Color::DeviceCMYK([0.0, 0.0, 0.0, 0.0]); // No ink, should be white
        assert_eq!(color.to_css_hex(), Some("#ffffff".into()));
    }

    #[test]
    fn test_color_spot_to_css_hex_none() {
        let color = Color::Spot("PANTONE".into(), 0.5);
        assert_eq!(color.to_css_hex(), None);
    }

    #[test]
    fn test_color_other_to_css_hex_none() {
        let color = Color::Other;
        assert_eq!(color.to_css_hex(), None);
    }

    #[test]
    fn test_color_device_rgb_clamped() {
        let color = Color::DeviceRGB([1.5, -0.5, 0.5]);
        assert_eq!(color.to_css_hex(), Some("#ff8080".into()));
    }

    // Tests for matrix operations

    #[test]
    fn test_matrix_scale() {
        let m = Matrix3x3::scale(2.0, 3.0);
        let (x, y) = m.transform_point(1.0, 1.0);
        assert_eq!(x, 2.0);
        assert_eq!(y, 3.0);
    }

    #[test]
    fn test_matrix_translate() {
        let m = Matrix3x3::translate(10.0, 20.0);
        let (x, y) = m.transform_point(0.0, 0.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    #[test]
    fn test_matrix_rotate() {
        let m = Matrix3x3::rotate(std::f64::consts::FRAC_PI_2); // 90 degrees
        let (x, y) = m.transform_point(1.0, 0.0);
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_matrix_invert_identity() {
        let m = Matrix3x3::identity();
        let inv = m.invert().unwrap();
        assert!(inv.is_identity());
    }

    #[test]
    fn test_matrix_invert_translation() {
        let m = Matrix3x3::translate(10.0, 20.0);
        let inv = m.invert().unwrap();
        let (x, y) = inv.transform_point(10.0, 20.0);
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_matrix_invert_singular() {
        let m = Matrix3x3::from_pdf_array([0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(m.invert().is_none());
    }

    #[test]
    fn test_identity_multiply_identity() {
        let m1 = Matrix3x3::identity();
        let m2 = Matrix3x3::identity();
        let result = m1.multiply(&m2);
        assert!(result.is_identity());
    }

    #[test]
    fn test_multiply_within_tolerance() {
        let m1 = Matrix3x3::identity();
        let m2 = Matrix3x3::identity();
        let result = m1.multiply(&m2);
        assert!((result.a - 1.0).abs() < 1e-10);
        assert!((result.b - 0.0).abs() < 1e-10);
        assert!((result.c - 0.0).abs() < 1e-10);
        assert!((result.d - 1.0).abs() < 1e-10);
        assert!((result.e - 0.0).abs() < 1e-10);
        assert!((result.f - 0.0).abs() < 1e-10);
    }
}
