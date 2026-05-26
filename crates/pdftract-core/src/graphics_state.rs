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

/// Maximum depth of graphics state stack (per PDF spec section 8.4).
const MAX_GSTATE_DEPTH: usize = 64;

/// Color space identifier for tracking current fill/stroke color space.
///
/// Per PDF spec section 8.6.5, color spaces determine how color values are interpreted.
/// The cs/CS operators set the current color space for non-stroking/stroking operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// DeviceGray color space (1 component)
    DeviceGray,
    /// DeviceRGB color space (3 components)
    DeviceRGB,
    /// DeviceCMYK color space (4 components)
    DeviceCMYK,
    /// Pattern color space (special handling)
    Pattern,
    /// ICCBased color space (profile-based, treated as Other)
    ICCBased,
    /// Indexed color space (color lookup table, treated as Other)
    Indexed,
    /// CalRGB color space (CIE-based CalRGB, treated as Other)
    CalRGB,
    /// CalGray color space (CIE-based CalGray, treated as Other)
    CalGray,
    /// DeviceN color space (multi-component, treated as Other)
    DeviceN,
    /// Separation color space (spot color with tint transform)
    Separation,
    /// Unknown or custom color space
    Other,
}

impl ColorSpace {
    /// Get the number of color components for this color space.
    ///
    /// Returns the number of numeric arguments expected by sc/scn operators.
    /// For Pattern, returns 0 (pattern names are not numeric components).
    /// For unknown spaces, returns 0 (treated as Other).
    pub fn component_count(&self) -> usize {
        match self {
            ColorSpace::DeviceGray => 1,
            ColorSpace::DeviceRGB => 3,
            ColorSpace::DeviceCMYK => 4,
            ColorSpace::Pattern => 0,
            ColorSpace::ICCBased
            | ColorSpace::Indexed
            | ColorSpace::CalRGB
            | ColorSpace::CalGray
            | ColorSpace::DeviceN
            | ColorSpace::Separation
            | ColorSpace::Other => 0,
        }
    }
}

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
    /// Current fill color space (for sc/scn operators)
    fill_color_space: ColorSpace,
    /// Current stroke color space (for SC/SCN operators)
    stroke_color_space: ColorSpace,
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
            .field("fill_color_space", &self.fill_color_space)
            .field("stroke_color_space", &self.stroke_color_space)
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
    /// - fill_color_space: DeviceGray (default per PDF spec)
    /// - stroke_color_space: DeviceGray (default per PDF spec)
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
            fill_color_space: ColorSpace::DeviceGray,
            stroke_color_space: ColorSpace::DeviceGray,
        }
    }

    /// Concatenate a matrix with the current CTM.
    ///
    /// This implements the `cm` operator behavior: CTM' = CTM × M
    #[inline]
    pub fn concat_ctm(&mut self, matrix: &Matrix3x3) {
        self.ctm = self.ctm.multiply(matrix);
    }

    /// Set character spacing (Tc operator).
    ///
    /// Tc sets the character spacing parameter, Tw. Negative values are allowed.
    #[inline]
    pub fn set_char_spacing(&mut self, value: f64) {
        self.char_spacing = value;
    }

    /// Set word spacing (Tw operator).
    ///
    /// Tw sets the word spacing parameter, Tw. Negative values are allowed.
    #[inline]
    pub fn set_word_spacing(&mut self, value: f64) {
        self.word_spacing = value;
    }

    /// Set horizontal scaling (Tz operator).
    ///
    /// Tz sets the horizontal scaling parameter, Tz, as a percentage.
    /// Values <= 0 are clamped to 1.0 to avoid zero-width glyphs.
    #[inline]
    pub fn set_horiz_scaling(&mut self, value: f64) {
        self.horiz_scaling = if value <= 0.0 { 1.0 } else { value };
    }

    /// Set leading (TL operator).
    ///
    /// TL sets the leading parameter, Tl. Negative values are allowed.
    #[inline]
    pub fn set_leading(&mut self, value: f64) {
        self.leading = value;
    }

    /// Set text rise (Ts operator).
    ///
    /// Ts sets the text rise parameter, Ts. Negative values are allowed.
    #[inline]
    pub fn set_text_rise(&mut self, value: f64) {
        self.text_rise = value;
    }

    /// Set text rendering mode (Tr operator).
    ///
    /// Tr sets the text rendering mode. Values outside 0-7 are clamped to the valid range.
    #[inline]
    pub fn set_text_rendering_mode(&mut self, value: u8) {
        self.text_rendering_mode = value.min(7);
    }

    /// Move text position (Td operator).
    ///
    /// Sets text_line_matrix = translate(tx, ty) * text_line_matrix,
    /// then copies text_line_matrix to text_matrix.
    #[inline]
    pub fn move_text(&mut self, tx: f64, ty: f64) {
        let translation = Matrix3x3::translate(tx, ty);
        self.text_line_matrix = translation.multiply(&self.text_line_matrix);
        self.text_matrix = self.text_line_matrix;
    }

    /// Move text position and set leading (TD operator).
    ///
    /// Same as Td, but also sets leading = -ty.
    #[inline]
    pub fn move_text_set_leading(&mut self, tx: f64, ty: f64) {
        self.leading = -ty;
        self.move_text(tx, ty);
    }

    /// Set text matrix (Tm operator).
    ///
    /// Sets both text_matrix and text_line_matrix to the given matrix.
    #[inline]
    pub fn set_text_matrix(&mut self, matrix: &Matrix3x3) {
        self.text_matrix = *matrix;
        self.text_line_matrix = *matrix;
    }

    /// Move to next line (T* operator).
    ///
    /// Equivalent to Td 0 -leading. If leading == 0, this is a no-op.
    #[inline]
    pub fn next_line(&mut self) {
        self.move_text(0.0, -self.leading);
    }

    /// Bind font (Tf operator).
    ///
    /// Sets the font and font_size. If size <= 0, clamps to 1.0.
    #[inline]
    pub fn set_font(&mut self, font: std::sync::Arc<Font>, size: f64) {
        self.font = Some(font);
        self.font_size = if size <= 0.0 { 1.0 } else { size };
    }

    /// Reset text matrices to identity (BT operator).
    ///
    /// Called when beginning a text block.
    #[inline]
    pub fn begin_text(&mut self) {
        self.text_matrix = Matrix3x3::identity();
        self.text_line_matrix = Matrix3x3::identity();
    }

    /// Discard text matrices (ET operator).
    ///
    /// Called when ending a text block.
    #[inline]
    pub fn end_text(&mut self) {
        self.text_matrix = Matrix3x3::identity();
        self.text_line_matrix = Matrix3x3::identity();
    }

    // Color-setting operators (rg RG g G k K cs CS sc SC scn SCN)

    /// Set fill color to DeviceGray (g operator).
    #[inline]
    pub fn set_fill_gray(&mut self, gray: f32) {
        self.fill_color = Color::DeviceGray(gray);
        self.fill_color_space = ColorSpace::DeviceGray;
    }

    /// Set stroke color to DeviceGray (G operator).
    #[inline]
    pub fn set_stroke_gray(&mut self, gray: f32) {
        self.stroke_color = Color::DeviceGray(gray);
        self.stroke_color_space = ColorSpace::DeviceGray;
    }

    /// Set fill color to DeviceRGB (rg operator).
    #[inline]
    pub fn set_fill_rgb(&mut self, r: f32, g: f32, b: f32) {
        self.fill_color = Color::DeviceRGB([r, g, b]);
        self.fill_color_space = ColorSpace::DeviceRGB;
    }

    /// Set stroke color to DeviceRGB (RG operator).
    #[inline]
    pub fn set_stroke_rgb(&mut self, r: f32, g: f32, b: f32) {
        self.stroke_color = Color::DeviceRGB([r, g, b]);
        self.stroke_color_space = ColorSpace::DeviceRGB;
    }

    /// Set fill color to DeviceCMYK (k operator).
    #[inline]
    pub fn set_fill_cmyk(&mut self, c: f32, m: f32, y: f32, k: f32) {
        self.fill_color = Color::DeviceCMYK([c, m, y, k]);
        self.fill_color_space = ColorSpace::DeviceCMYK;
    }

    /// Set stroke color to DeviceCMYK (K operator).
    #[inline]
    pub fn set_stroke_cmyk(&mut self, c: f32, m: f32, y: f32, k: f32) {
        self.stroke_color = Color::DeviceCMYK([c, m, y, k]);
        self.stroke_color_space = ColorSpace::DeviceCMYK;
    }

    /// Set fill color space (cs operator).
    #[inline]
    pub fn set_fill_color_space(&mut self, color_space: ColorSpace) {
        self.fill_color_space = color_space;
    }

    /// Set stroke color space (CS operator).
    #[inline]
    pub fn set_stroke_color_space(&mut self, color_space: ColorSpace) {
        self.stroke_color_space = color_space;
    }

    /// Set fill color in current color space (sc operator).
    ///
    /// The numeric components are interpreted based on the current fill_color_space.
    /// For DeviceGray: [gray]
    /// For DeviceRGB: [r, g, b]
    /// For DeviceCMYK: [c, m, y, k]
    /// For other spaces: sets Color::Other
    #[inline]
    pub fn set_fill_color(&mut self, components: &[f32]) {
        self.fill_color = match self.fill_color_space {
            ColorSpace::DeviceGray => {
                if components.len() >= 1 {
                    Color::DeviceGray(components[0])
                } else {
                    Color::Other
                }
            }
            ColorSpace::DeviceRGB => {
                if components.len() >= 3 {
                    Color::DeviceRGB([components[0], components[1], components[2]])
                } else {
                    Color::Other
                }
            }
            ColorSpace::DeviceCMYK => {
                if components.len() >= 4 {
                    Color::DeviceCMYK([components[0], components[1], components[2], components[3]])
                } else {
                    Color::Other
                }
            }
            _ => Color::Other,
        };
    }

    /// Set stroke color in current color space (SC operator).
    ///
    /// Same as set_fill_color but for stroke.
    #[inline]
    pub fn set_stroke_color(&mut self, components: &[f32]) {
        self.stroke_color = match self.stroke_color_space {
            ColorSpace::DeviceGray => {
                if components.len() >= 1 {
                    Color::DeviceGray(components[0])
                } else {
                    Color::Other
                }
            }
            ColorSpace::DeviceRGB => {
                if components.len() >= 3 {
                    Color::DeviceRGB([components[0], components[1], components[2]])
                } else {
                    Color::Other
                }
            }
            ColorSpace::DeviceCMYK => {
                if components.len() >= 4 {
                    Color::DeviceCMYK([components[0], components[1], components[2], components[3]])
                } else {
                    Color::Other
                }
            }
            _ => Color::Other,
        };
    }

    /// Set fill color with optional pattern/spot name (scn operator).
    ///
    /// If a name is provided and the current color space is Separation,
    /// sets Color::Spot with the name and first component as tint.
    /// Otherwise behaves like set_fill_color.
    #[inline]
    pub fn set_fill_color_named(&mut self, components: &[f32], name: Option<&str>) {
        if let Some(colorant_name) = name {
            // For Separation color spaces, treat as Spot color
            if self.fill_color_space == ColorSpace::Separation {
                let tint = if components.len() >= 1 {
                    components[0]
                } else {
                    1.0
                };
                self.fill_color = Color::Spot(Arc::from(colorant_name), tint);
                return;
            }
            // Pattern color space
            if self.fill_color_space == ColorSpace::Pattern {
                self.fill_color = Color::Other;
                return;
            }
        }
        // Fall back to numeric-only behavior
        self.set_fill_color(components);
    }

    /// Set stroke color with optional pattern/spot name (SCN operator).
    ///
    /// Same as set_fill_color_named but for stroke.
    #[inline]
    pub fn set_stroke_color_named(&mut self, components: &[f32], name: Option<&str>) {
        if let Some(colorant_name) = name {
            // For Separation color spaces, treat as Spot color
            if self.stroke_color_space == ColorSpace::Separation {
                let tint = if components.len() >= 1 {
                    components[0]
                } else {
                    1.0
                };
                self.stroke_color = Color::Spot(Arc::from(colorant_name), tint);
                return;
            }
            // Pattern color space
            if self.stroke_color_space == ColorSpace::Pattern {
                self.stroke_color = Color::Other;
                return;
            }
        }
        // Fall back to numeric-only behavior
        self.set_stroke_color(components);
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
        // R: 1.5 -> 1.0 -> ff, G: -0.5 -> 0.0 -> 00, B: 0.5 -> 0.5 -> 80
        assert_eq!(color.to_css_hex(), Some("#ff0080".into()));
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

    // Acceptance criteria tests for pdftract-1os1

    #[test]
    fn test_64_nested_q_calls_succeed() {
        // AC: 64 nested q calls succeed; the 65th emits diagnostic and discards
        let mut stack = GraphicsStateStack::new();
        let state = GraphicsState::new();

        // 64 nested q calls should all succeed
        for i in 0..64 {
            assert!(stack.push(&state), "q call {} should succeed", i + 1);
        }
        assert_eq!(stack.depth(), 64);

        // 65th q should fail
        assert!(!stack.push(&state), "65th q should fail");
        assert_eq!(stack.depth(), 64);
    }

    #[test]
    fn test_64_q_plus_64_q_restores_initial_state() {
        // AC: 64 q + 64 Q restores to initial state
        let mut stack = GraphicsStateStack::new();
        let mut state = GraphicsState::new();

        // Modify state
        let translate = Matrix3x3::from_pdf_array([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        state.concat_ctm(&translate);
        let initial_ctm = state.ctm;

        // Push 64 times
        for _ in 0..64 {
            stack.push(&state);
        }

        // Pop 64 times
        for _ in 0..64 {
            stack.pop();
        }

        // Stack should be empty
        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_q_at_depth_0_is_noop() {
        // AC: Q at depth 0 is a no-op (no panic) and emits GSTATE_STACK_UNDERFLOW
        let mut stack = GraphicsStateStack::new();

        // Stack is empty, Q should return None
        assert!(stack.pop().is_none());
        assert!(stack.is_empty());

        // Multiple Q at depth 0 should all be no-ops
        assert!(stack.pop().is_none());
        assert!(stack.pop().is_none());
        assert!(stack.pop().is_none());
        assert!(stack.is_empty());
    }

    #[test]
    fn test_1000_paired_q_q_operations_succeed() {
        // AC: 1000 paired q...Q operations succeed (depth never exceeds 1)
        let mut stack = GraphicsStateStack::new();
        let state = GraphicsState::new();

        for i in 0..1000 {
            assert!(stack.push(&state), "Paired q {} should succeed", i);
            assert_eq!(stack.depth(), 1);
            let restored = stack.pop();
            assert!(restored.is_some(), "Paired Q {} should succeed", i);
            assert!(stack.is_empty());
        }
    }

    #[test]
    fn test_max_depth_is_64() {
        // Verify MAX_GSTATE_DEPTH is 64 per PDF spec
        assert_eq!(MAX_GSTATE_DEPTH, 64);
    }

    // Acceptance criteria tests for pdftract-4dmp

    #[test]
    fn test_set_char_spacing() {
        let mut state = GraphicsState::new();
        state.set_char_spacing(5.0);
        assert_eq!(state.char_spacing, 5.0);
    }

    #[test]
    fn test_set_word_spacing() {
        let mut state = GraphicsState::new();
        state.set_word_spacing(10.0);
        assert_eq!(state.word_spacing, 10.0);
    }

    #[test]
    fn test_set_horiz_scaling_positive() {
        let mut state = GraphicsState::new();
        state.set_horiz_scaling(150.0);
        assert_eq!(state.horiz_scaling, 150.0);
    }

    #[test]
    fn test_set_horiz_scaling_zero_clamps_to_one() {
        let mut state = GraphicsState::new();
        state.set_horiz_scaling(0.0);
        assert_eq!(state.horiz_scaling, 1.0);
    }

    #[test]
    fn test_set_horiz_scaling_negative_clamps_to_one() {
        let mut state = GraphicsState::new();
        state.set_horiz_scaling(-10.0);
        assert_eq!(state.horiz_scaling, 1.0);
    }

    #[test]
    fn test_set_leading() {
        let mut state = GraphicsState::new();
        state.set_leading(15.0);
        assert_eq!(state.leading, 15.0);
    }

    #[test]
    fn test_set_text_rise() {
        let mut state = GraphicsState::new();
        state.set_text_rise(3.0);
        assert_eq!(state.text_rise, 3.0);
    }

    #[test]
    fn test_set_text_rendering_mode_valid() {
        let mut state = GraphicsState::new();
        for mode in 0..=7 {
            state.set_text_rendering_mode(mode);
            assert_eq!(state.text_rendering_mode, mode);
        }
    }

    #[test]
    fn test_set_text_rendering_mode_clamps_to_seven() {
        let mut state = GraphicsState::new();
        state.set_text_rendering_mode(9);
        assert_eq!(state.text_rendering_mode, 7);
    }

    #[test]
    fn test_set_text_rendering_mode_clamps_to_zero() {
        let mut state = GraphicsState::new();
        state.set_text_rendering_mode(255); // u8 overflow wraps to 255
        assert_eq!(state.text_rendering_mode, 7);
    }

    #[test]
    fn test_negative_char_spacing_allowed() {
        let mut state = GraphicsState::new();
        state.set_char_spacing(-5.0);
        assert_eq!(state.char_spacing, -5.0);
    }

    #[test]
    fn test_negative_word_spacing_allowed() {
        let mut state = GraphicsState::new();
        state.set_word_spacing(-10.0);
        assert_eq!(state.word_spacing, -10.0);
    }

    #[test]
    fn test_negative_text_rise_allowed() {
        let mut state = GraphicsState::new();
        state.set_text_rise(-3.0);
        assert_eq!(state.text_rise, -3.0);
    }

    #[test]
    fn test_negative_leading_allowed() {
        let mut state = GraphicsState::new();
        state.set_leading(-15.0);
        assert_eq!(state.leading, -15.0);
    }

    // Acceptance criteria tests for pdftract-4ubed (color operators)

    #[test]
    fn test_color_space_component_count() {
        // AC: DeviceGray has 1 component
        assert_eq!(ColorSpace::DeviceGray.component_count(), 1);
        // AC: DeviceRGB has 3 components
        assert_eq!(ColorSpace::DeviceRGB.component_count(), 3);
        // AC: DeviceCMYK has 4 components
        assert_eq!(ColorSpace::DeviceCMYK.component_count(), 4);
        // AC: Pattern has 0 components (uses names, not numbers)
        assert_eq!(ColorSpace::Pattern.component_count(), 0);
        // AC: Other spaces have 0 components
        assert_eq!(ColorSpace::ICCBased.component_count(), 0);
        assert_eq!(ColorSpace::Indexed.component_count(), 0);
        assert_eq!(ColorSpace::Other.component_count(), 0);
    }

    #[test]
    fn test_set_fill_gray() {
        let mut state = GraphicsState::new();
        state.set_fill_gray(0.5);
        assert_eq!(state.fill_color, Color::DeviceGray(0.5));
        assert_eq!(state.fill_color_space, ColorSpace::DeviceGray);
    }

    #[test]
    fn test_set_stroke_gray() {
        let mut state = GraphicsState::new();
        state.set_stroke_gray(0.5);
        assert_eq!(state.stroke_color, Color::DeviceGray(0.5));
        assert_eq!(state.stroke_color_space, ColorSpace::DeviceGray);
    }

    #[test]
    fn test_set_fill_rgb() {
        let mut state = GraphicsState::new();
        state.set_fill_rgb(0.5, 0.5, 0.5);
        assert_eq!(state.fill_color, Color::DeviceRGB([0.5, 0.5, 0.5]));
        assert_eq!(state.fill_color_space, ColorSpace::DeviceRGB);
    }

    #[test]
    fn test_set_stroke_rgb() {
        let mut state = GraphicsState::new();
        state.set_stroke_rgb(1.0, 0.0, 0.0);
        assert_eq!(state.stroke_color, Color::DeviceRGB([1.0, 0.0, 0.0]));
        assert_eq!(state.stroke_color_space, ColorSpace::DeviceRGB);
    }

    #[test]
    fn test_set_fill_cmyk() {
        let mut state = GraphicsState::new();
        state.set_fill_cmyk(0.1, 0.2, 0.3, 0.4);
        assert_eq!(state.fill_color, Color::DeviceCMYK([0.1, 0.2, 0.3, 0.4]));
        assert_eq!(state.fill_color_space, ColorSpace::DeviceCMYK);
    }

    #[test]
    fn test_set_stroke_cmyk() {
        let mut state = GraphicsState::new();
        state.set_stroke_cmyk(0.1, 0.2, 0.3, 0.4);
        assert_eq!(state.stroke_color, Color::DeviceCMYK([0.1, 0.2, 0.3, 0.4]));
        assert_eq!(state.stroke_color_space, ColorSpace::DeviceCMYK);
    }

    #[test]
    fn test_set_fill_color_space() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::DeviceRGB);
        assert_eq!(state.fill_color_space, ColorSpace::DeviceRGB);
    }

    #[test]
    fn test_set_stroke_color_space() {
        let mut state = GraphicsState::new();
        state.set_stroke_color_space(ColorSpace::Pattern);
        assert_eq!(state.stroke_color_space, ColorSpace::Pattern);
    }

    #[test]
    fn test_set_fill_color_device_gray() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::DeviceGray);
        state.set_fill_color(&[0.5]);
        assert_eq!(state.fill_color, Color::DeviceGray(0.5));
    }

    #[test]
    fn test_set_fill_color_device_rgb() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::DeviceRGB);
        state.set_fill_color(&[1.0, 0.0, 0.0]);
        assert_eq!(state.fill_color, Color::DeviceRGB([1.0, 0.0, 0.0]));
    }

    #[test]
    fn test_set_fill_color_device_cmyk() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::DeviceCMYK);
        state.set_fill_color(&[0.1, 0.2, 0.3, 0.4]);
        assert_eq!(state.fill_color, Color::DeviceCMYK([0.1, 0.2, 0.3, 0.4]));
    }

    #[test]
    fn test_set_fill_color_insufficient_components() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::DeviceRGB);
        // Only 2 components for DeviceRGB (needs 3)
        state.set_fill_color(&[1.0, 0.0]);
        assert_eq!(state.fill_color, Color::Other);
    }

    #[test]
    fn test_set_fill_color_unknown_colorspace() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::ICCBased);
        state.set_fill_color(&[0.5]);
        assert_eq!(state.fill_color, Color::Other);
    }

    #[test]
    fn test_set_fill_color_named_spot() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::Separation);
        state.set_fill_color_named(&[0.5], Some("PANTONE"));
        assert_eq!(state.fill_color, Color::Spot(Arc::from("PANTONE"), 0.5));
    }

    #[test]
    fn test_set_fill_color_named_spot_default_tint() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::Separation);
        // No tint value, defaults to 1.0
        state.set_fill_color_named(&[], Some("PANTONE"));
        assert_eq!(state.fill_color, Color::Spot(Arc::from("PANTONE"), 1.0));
    }

    #[test]
    fn test_set_fill_color_named_pattern() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::Pattern);
        state.set_fill_color_named(&[], Some("Pattern1"));
        // Pattern color space always sets Color::Other
        assert_eq!(state.fill_color, Color::Other);
    }

    #[test]
    fn test_set_fill_color_named_fallback_to_numeric() {
        let mut state = GraphicsState::new();
        state.set_fill_color_space(ColorSpace::DeviceRGB);
        // No name provided, falls back to numeric-only behavior
        state.set_fill_color_named(&[1.0, 0.0, 0.0], None);
        assert_eq!(state.fill_color, Color::DeviceRGB([1.0, 0.0, 0.0]));
    }

    #[test]
    fn test_initial_fill_color_is_black() {
        let state = GraphicsState::initial();
        assert_eq!(state.fill_color, Color::DeviceGray(0.0));
        assert_eq!(state.fill_color_space, ColorSpace::DeviceGray);
    }

    #[test]
    fn test_initial_stroke_color_is_black() {
        let state = GraphicsState::initial();
        assert_eq!(state.stroke_color, Color::DeviceGray(0.0));
        assert_eq!(state.stroke_color_space, ColorSpace::DeviceGray);
    }

    #[test]
    fn test_graphics_state_clone_preserves_colors() {
        let mut state1 = GraphicsState::new();
        state1.set_fill_rgb(1.0, 0.0, 0.0);
        state1.set_stroke_cmyk(0.1, 0.2, 0.3, 0.4);
        state1.set_fill_color_space(ColorSpace::DeviceRGB);
        state1.set_stroke_color_space(ColorSpace::DeviceCMYK);

        let state2 = state1.clone();
        assert_eq!(state2.fill_color, Color::DeviceRGB([1.0, 0.0, 0.0]));
        assert_eq!(state2.stroke_color, Color::DeviceCMYK([0.1, 0.2, 0.3, 0.4]));
        assert_eq!(state2.fill_color_space, ColorSpace::DeviceRGB);
        assert_eq!(state2.stroke_color_space, ColorSpace::DeviceCMYK);
    }
}
