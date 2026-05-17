//! 3x3 transformation matrix for PDF graphics.
//!
//! PDF uses 3x3 matrices for affine transformations in 2D space.
//! The matrix is represented as:
//! ```
//! | a  b  0 |
//! | c  d  0 |
//! | e  f  1 |
//! ```
//!
/// A 3x3 affine transformation matrix.
///
/// Only the first two columns are stored since the third column is always
/// [0, 0, 1] for affine transformations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3x3 {
    /// Scale X and horizontal shear (a, b)
    pub a: f64,
    pub b: f64,
    /// Vertical shear and scale Y (c, d)
    pub c: f64,
    pub d: f64,
    /// Translation X and Y (e, f)
    pub e: f64,
    pub f: f64,
}

impl Matrix3x3 {
    /// Create a new identity matrix.
    #[inline]
    pub fn identity() -> Self {
        Matrix3x3 {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create a translation matrix.
    #[inline]
    pub fn translate(tx: f64, ty: f64) -> Self {
        Matrix3x3 {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Create a scale matrix.
    #[inline]
    pub fn scale(sx: f64, sy: f64) -> Self {
        Matrix3x3 {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create a rotation matrix (counterclockwise, in radians).
    #[inline]
    pub fn rotate(theta: f64) -> Self {
        let cos = theta.cos();
        let sin = theta.sin();
        Matrix3x3 {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Multiply this matrix by another (this * other).
    #[inline]
    pub fn multiply(&self, other: &Matrix3x3) -> Self {
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
        let tx = self.a * x + self.c * y + self.e;
        let ty = self.b * x + self.d * y + self.f;
        (tx, ty)
    }

    /// Get the inverse of this matrix.
    ///
    /// Returns `None` if the matrix is not invertible (determinant is zero).
    pub fn inverse(&self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
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

    /// Get the scaling factor on the X axis.
    #[inline]
    pub fn scale_x(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    /// Get the scaling factor on the Y axis.
    #[inline]
    pub fn scale_y(&self) -> f64 {
        (self.c * self.c + self.d * self.d).sqrt()
    }

    /// Create a matrix from raw PDF array values [a, b, c, d, e, f].
    #[inline]
    pub fn from_pdf_array(values: [f64; 6]) -> Self {
        Matrix3x3 {
            a: values[0],
            b: values[1],
            c: values[2],
            d: values[3],
            e: values[4],
            f: values[5],
        }
    }

    /// Convert to PDF array format [a, b, c, d, e, f].
    #[inline]
    pub fn to_pdf_array(&self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }

    /// Check if this is an identity matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.a == 1.0
            && self.b == 0.0
            && self.c == 0.0
            && self.d == 1.0
            && self.e == 0.0
            && self.f == 0.0
    }
}

impl Default for Matrix3x3 {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let m = Matrix3x3::identity();
        assert_eq!(m.transform_point(10.0, 20.0), (10.0, 20.0));
        assert!(m.is_identity());
    }

    #[test]
    fn test_translate() {
        let m = Matrix3x3::translate(5.0, 10.0);
        assert_eq!(m.transform_point(10.0, 20.0), (15.0, 30.0));
    }

    #[test]
    fn test_scale() {
        let m = Matrix3x3::scale(2.0, 3.0);
        assert_eq!(m.transform_point(10.0, 20.0), (20.0, 60.0));
    }

    #[test]
    fn test_rotate() {
        let m = Matrix3x3::rotate(std::f64::consts::FRAC_PI_2);
        let (x, y) = m.transform_point(1.0, 0.0);
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_multiply() {
        let m1 = Matrix3x3::translate(5.0, 0.0);
        let m2 = Matrix3x3::scale(2.0, 1.0);
        let result = m1.multiply(&m2);
        // Scale then translate
        assert_eq!(result.transform_point(10.0, 20.0), (25.0, 20.0));
    }

    #[test]
    fn test_inverse() {
        let m = Matrix3x3::translate(10.0, 20.0);
        let inv = m.inverse().unwrap();
        let (x, y) = inv.transform_point(15.0, 25.0);
        assert!((x - 5.0).abs() < 1e-10);
        assert!((y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_factors() {
        let m = Matrix3x3::scale(2.0, 3.0);
        assert!((m.scale_x() - 2.0).abs() < 1e-10);
        assert!((m.scale_y() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_pdf_array_roundtrip() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let m = Matrix3x3::from_pdf_array(values);
        assert_eq!(m.to_pdf_array(), values);
    }
}
