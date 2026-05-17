//! Graphics state for PDF content stream processing.
//!
//! Implements the full graphics state including CTM, text matrices,
//! font binding, and text state parameters.

use std::sync::Arc;

use super::{color::Color, matrix::Matrix3x3};

/// Text rendering mode (Tr operator).
///
/// Values 0-7 as defined in PDF specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextRenderingMode {
    #[default]
    Fill = 0,
    Stroke = 1,
    FillThenStroke = 2,
    Invisible = 3,
    FillClip = 4,
    StrokeClip = 5,
    FillThenStrokeClip = 6,
    Clip = 7,
}

impl TextRenderingMode {
    /// Create from u8 value, clamping to valid range 0-7.
    pub fn from_u8(value: u8) -> Self {
        match value.min(7) {
            0 => TextRenderingMode::Fill,
            1 => TextRenderingMode::Stroke,
            2 => TextRenderingMode::FillThenStroke,
            3 => TextRenderingMode::Invisible,
            4 => TextRenderingMode::FillClip,
            5 => TextRenderingMode::StrokeClip,
            6 => TextRenderingMode::FillThenStrokeClip,
            _ => TextRenderingMode::Clip,
        }
    }

    /// Check if text should be invisible.
    pub fn is_invisible(self) -> bool {
        self == TextRenderingMode::Invisible
    }
}

/// Font placeholder.
///
/// This is a minimal placeholder. The full Font type will be implemented
/// in Phase 3.2 (Text Operator Processing).
#[derive(Debug, Clone)]
pub struct Font {
    // Font fields will be added in Phase 3.2
    _phantom: std::marker::PhantomData<()>,
}

/// Complete graphics state for PDF content stream processing.
///
/// Contains all 13 fields specified in PDF specification Table 51
/// plus the text matrix state.
#[derive(Debug, Clone)]
pub struct GraphicsState {
    /// Current transformation matrix (CTM)
    pub ctm: Matrix3x3,
    /// Text matrix (Tm) - set by Tm/Td/TD/T*
    pub text_matrix: Matrix3x3,
    /// Text line matrix (Tlm) - reset by Td/TD/T*
    pub text_line_matrix: Matrix3x3,
    /// Currently bound font
    pub font: Option<Arc<Font>>,
    /// Font size (Tf operator)
    pub font_size: f64,
    /// Character spacing (Tc operator)
    pub char_spacing: f64,
    /// Word spacing (Tw operator)
    pub word_spacing: f64,
    /// Horizontal scaling (Tz operator) - percentage, default 100
    pub horiz_scaling: f64,
    /// Leading (TL operator)
    pub leading: f64,
    /// Text rise (Ts operator)
    pub text_rise: f64,
    /// Text rendering mode (Tr operator)
    pub text_rendering_mode: TextRenderingMode,
    /// Fill color
    pub fill_color: Color,
    /// Stroke color
    pub stroke_color: Color,
}

impl GraphicsState {
    /// Create a new graphics state with default values.
    pub fn new() -> Self {
        GraphicsState {
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
            text_rendering_mode: TextRenderingMode::default(),
            fill_color: Color::default(),
            stroke_color: Color::default(),
        }
    }

    /// Reset text matrices to identity (BT operator).
    pub fn begin_text(&mut self) {
        self.text_matrix = Matrix3x3::identity();
        self.text_line_matrix = Matrix3x3::identity();
    }

    /// Discard text matrices (ET operator).
    pub fn end_text(&mut self) {
        self.text_matrix = Matrix3x3::identity();
        self.text_line_matrix = Matrix3x3::identity();
    }

    /// Set character spacing (Tc operator).
    pub fn set_char_spacing(&mut self, value: f64) {
        self.char_spacing = value;
    }

    /// Set word spacing (Tw operator).
    pub fn set_word_spacing(&mut self, value: f64) {
        self.word_spacing = value;
    }

    /// Set horizontal scaling (Tz operator).
    pub fn set_horiz_scaling(&mut self, value: f64) {
        self.horiz_scaling = value;
    }

    /// Set leading (TL operator).
    pub fn set_leading(&mut self, value: f64) {
        self.leading = value;
    }

    /// Set text rise (Ts operator).
    pub fn set_text_rise(&mut self, value: f64) {
        self.text_rise = value;
    }

    /// Set text rendering mode (Tr operator).
    pub fn set_text_rendering_mode(&mut self, mode: TextRenderingMode) {
        self.text_rendering_mode = mode;
    }

    /// Bind font (Tf operator).
    pub fn set_font(&mut self, font: Arc<Font>, size: f64) {
        self.font = Some(font);
        self.font_size = size;
    }

    /// Move text position (Td operator).
    ///
    /// Sets text_line_matrix = translate(tx, ty) * text_line_matrix,
    /// then copies text_line_matrix to text_matrix.
    pub fn move_text(&mut self, tx: f64, ty: f64) {
        let translation = Matrix3x3::translate(tx, ty);
        self.text_line_matrix = translation.multiply(&self.text_line_matrix);
        self.text_matrix = self.text_line_matrix;
    }

    /// Move text position and set leading (TD operator).
    ///
    /// Same as Td, but also sets leading = -ty.
    pub fn move_text_set_leading(&mut self, tx: f64, ty: f64) {
        self.leading = -ty;
        self.move_text(tx, ty);
    }

    /// Set text matrix (Tm operator).
    ///
    /// Sets both text_matrix and text_line_matrix to the given matrix.
    pub fn set_text_matrix(&mut self, matrix: &Matrix3x3) {
        self.text_matrix = *matrix;
        self.text_line_matrix = *matrix;
    }

    /// Move to next line (T* operator).
    ///
    /// Equivalent to Td 0 -leading.
    pub fn next_line(&mut self) {
        self.move_text(0.0, -self.leading);
    }

    /// Set fill color (rg, k, g operators).
    pub fn set_fill_color(&mut self, color: Color) {
        self.fill_color = color;
    }

    /// Set stroke color (RG, K, G operators).
    pub fn set_stroke_color(&mut self, color: Color) {
        self.stroke_color = color;
    }

    /// Concatenate matrix to CTM (cm operator).
    pub fn concat_ctm(&mut self, matrix: &Matrix3x3) {
        self.ctm = matrix.multiply(&self.ctm);
    }

    /// Get the effective text matrix.
    ///
    /// Returns CTM * text_matrix, which is used to transform glyph coordinates.
    pub fn text_transform(&self) -> Matrix3x3 {
        self.ctm.multiply(&self.text_matrix)
    }

    /// Get horizontal scaling factor.
    ///
    /// Returns horiz_scaling / 100 as a multiplier.
    pub fn horiz_scale_factor(&self) -> f64 {
        self.horiz_scaling / 100.0
    }
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = GraphicsState::new();
        assert!(state.ctm.is_identity());
        assert!(state.text_matrix.is_identity());
        assert!(state.text_line_matrix.is_identity());
        assert!(state.font.is_none());
        assert_eq!(state.font_size, 0.0);
        assert_eq!(state.char_spacing, 0.0);
        assert_eq!(state.word_spacing, 0.0);
        assert_eq!(state.horiz_scaling, 100.0);
        assert_eq!(state.leading, 0.0);
        assert_eq!(state.text_rise, 0.0);
        assert_eq!(state.text_rendering_mode, TextRenderingMode::Fill);
    }

    #[test]
    fn test_begin_text() {
        let mut state = GraphicsState::new();
        state.text_matrix = Matrix3x3::translate(10.0, 20.0);
        state.text_line_matrix = Matrix3x3::translate(30.0, 40.0);
        state.begin_text();
        assert!(state.text_matrix.is_identity());
        assert!(state.text_line_matrix.is_identity());
    }

    #[test]
    fn test_end_text() {
        let mut state = GraphicsState::new();
        state.text_matrix = Matrix3x3::translate(10.0, 20.0);
        state.text_line_matrix = Matrix3x3::translate(30.0, 40.0);
        state.end_text();
        assert!(state.text_matrix.is_identity());
        assert!(state.text_line_matrix.is_identity());
    }

    #[test]
    fn test_text_rendering_mode_from_u8() {
        assert_eq!(
            TextRenderingMode::from_u8(0),
            TextRenderingMode::Fill
        );
        assert_eq!(
            TextRenderingMode::from_u8(3),
            TextRenderingMode::Invisible
        );
        assert_eq!(TextRenderingMode::from_u8(10), TextRenderingMode::Clip);
    }

    #[test]
    fn test_move_text() {
        let mut state = GraphicsState::new();
        state.move_text(100.0, 200.0);
        let (x, y) = state.text_matrix.transform_point(0.0, 0.0);
        assert!((x - 100.0).abs() < f64::EPSILON);
        assert!((y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_td_chain() {
        // BT 100 200 Td 50 0 Td ET should yield translation (150, 200)
        let mut state = GraphicsState::new();
        state.begin_text();
        state.move_text(100.0, 200.0);
        state.move_text(50.0, 0.0);
        let (x, y) = state.text_matrix.transform_point(0.0, 0.0);
        assert!((x - 150.0).abs() < f64::EPSILON);
        assert!((y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_move_text_set_leading() {
        let mut state = GraphicsState::new();
        state.move_text_set_leading(10.0, -15.0);
        assert!((state.leading - 15.0).abs() < f64::EPSILON);
        let (x, y) = state.text_matrix.transform_point(0.0, 0.0);
        assert!((x - 10.0).abs() < f64::EPSILON);
        assert!((y - (-15.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_next_line() {
        let mut state = GraphicsState::new();
        state.set_leading(15.0);
        state.next_line();
        let (x, y) = state.text_matrix.transform_point(0.0, 0.0);
        assert!((x - 0.0).abs() < f64::EPSILON);
        assert!((y - (-15.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_text_matrix() {
        let mut state = GraphicsState::new();
        let m = Matrix3x3::translate(100.0, 200.0);
        state.set_text_matrix(&m);
        assert_eq!(state.text_matrix, m);
        assert_eq!(state.text_line_matrix, m);
    }

    #[test]
    fn test_horiz_scale_factor() {
        let state = GraphicsState::new();
        assert!((state.horiz_scale_factor() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_concat_ctm() {
        let mut state = GraphicsState::new();
        let m = Matrix3x3::scale(2.0, 3.0);
        state.concat_ctm(&m);
        assert_eq!(state.ctm, m);
    }

    #[test]
    fn test_text_transform() {
        let mut state = GraphicsState::new();
        state.ctm = Matrix3x3::scale(2.0, 3.0);
        state.text_matrix = Matrix3x3::translate(10.0, 20.0);
        let tx = state.text_transform();
        let (x, y) = tx.transform_point(0.0, 0.0);
        assert!((x - 20.0).abs() < f64::EPSILON);
        assert!((y - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invisible_rendering_mode() {
        assert!(TextRenderingMode::Invisible.is_invisible());
        assert!(!TextRenderingMode::Fill.is_invisible());
    }
}
