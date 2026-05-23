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

use crate::diagnostics::{Diagnostic, DiagCode};

/// Maximum depth of graphics state stack (prevents stack overflow).
const MAX_GSTATE_DEPTH: usize = 32;

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
        self.a == 1.0 && self.b == 0.0 && self.c == 0.0 &&
        self.d == 1.0 && self.e == 0.0 && self.f == 0.0
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
}

impl Default for Matrix3x3 {
    fn default() -> Self {
        Self::identity()
    }
}

/// Graphics state as defined in PDF spec section 8.4.
///
/// This contains the CTM and other graphics state parameters.
/// For Phase 5.2.1 image compositing, we only need the CTM.
#[derive(Debug, Clone)]
pub struct GraphicsState {
    /// Current Transformation Matrix
    pub ctm: Matrix3x3,
}

impl GraphicsState {
    /// Create a new graphics state with identity CTM.
    #[inline]
    pub fn new() -> Self {
        Self {
            ctm: Matrix3x3::identity(),
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
}
