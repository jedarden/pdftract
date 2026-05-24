//! Path segment representation for table detection.
//!
//! Segments are extracted from PDF path operators (m, l, re) terminated
//! by stroke (S/s) or fill (f/F/B/B*) operators.

use serde::{Deserialize, Serialize};

/// A path segment in PDF user space.
///
/// Segments are axis-aligned (horizontal or vertical) and represent
/// potential table ruling lines.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    /// Start point (x0, y0).
    pub x0: f32,
    pub y0: f32,
    /// End point (x1, y1).
    pub x1: f32,
    pub y1: f32,
    /// Orientation of the segment.
    pub orientation: SegmentOrientation,
}

impl Segment {
    /// Create a new segment from two points.
    ///
    /// # Arguments
    ///
    /// * `x0, y0` - Start point
    /// * `x1, y1` - End point
    /// * `epsilon` - Tolerance for determining orientation (default 1.0 pt)
    ///
    /// # Returns
    ///
    /// `Some(segment)` if the segment is axis-aligned, `None` otherwise.
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32, epsilon: f32) -> Option<Self> {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();

        let orientation = if dx < epsilon {
            // Vertical segment (|dx| < epsilon)
            SegmentOrientation::Vertical
        } else if dy < epsilon {
            // Horizontal segment (|dy| < epsilon)
            SegmentOrientation::Horizontal
        } else {
            // Diagonal - not useful for table detection
            return None;
        };

        // Normalize so x0 <= x1 and y0 <= y1 for easier comparison
        let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };

        Some(Self {
            x0,
            y0,
            x1,
            y1,
            orientation,
        })
    }

    /// Create a horizontal segment.
    pub fn horizontal(y: f32, x0: f32, x1: f32) -> Self {
        let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        Self {
            x0,
            y0: y,
            x1,
            y1: y,
            orientation: SegmentOrientation::Horizontal,
        }
    }

    /// Create a vertical segment.
    pub fn vertical(x: f32, y0: f32, y1: f32) -> Self {
        let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        Self {
            x0: x,
            y0,
            x1: x,
            y1,
            orientation: SegmentOrientation::Vertical,
        }
    }

    /// Get the length of this segment.
    #[inline]
    pub fn length(&self) -> f32 {
        match self.orientation {
            SegmentOrientation::Horizontal => self.x1 - self.x0,
            SegmentOrientation::Vertical => self.y1 - self.y0,
        }
    }

    /// Check if this segment intersects with another segment at a point.
    ///
    /// For horizontal vs vertical segments, returns the intersection point
    /// if the vertical segment's x falls within the horizontal's x range
    /// AND the horizontal's y falls within the vertical's y range.
    pub fn intersection(&self, other: &Segment, epsilon: f32) -> Option<(f32, f32)> {
        match (self.orientation, other.orientation) {
            (SegmentOrientation::Horizontal, SegmentOrientation::Vertical) => {
                // Self is horizontal, other is vertical
                if other.x0 >= self.x0 - epsilon
                    && other.x0 <= self.x1 + epsilon
                    && self.y0 >= other.y0 - epsilon
                    && self.y0 <= other.y1 + epsilon
                {
                    Some((other.x0, self.y0))
                } else {
                    None
                }
            }
            (SegmentOrientation::Vertical, SegmentOrientation::Horizontal) => {
                // Self is vertical, other is horizontal
                if self.x0 >= other.x0 - epsilon
                    && self.x0 <= other.x1 + epsilon
                    && other.y0 >= self.y0 - epsilon
                    && other.y0 <= self.y1 + epsilon
                {
                    Some((self.x0, other.y0))
                } else {
                    None
                }
            }
            _ => None, // Parallel segments don't intersect at a point
        }
    }

    /// Check if two horizontal segments are collinear (same y within epsilon).
    pub fn is_collinear_horizontal(&self, other: &Segment, epsilon: f32) -> bool {
        self.orientation == SegmentOrientation::Horizontal
            && other.orientation == SegmentOrientation::Horizontal
            && (self.y0 - other.y0).abs() < epsilon
    }

    /// Check if two vertical segments are collinear (same x within epsilon).
    pub fn is_collinear_vertical(&self, other: &Segment, epsilon: f32) -> bool {
        self.orientation == SegmentOrientation::Vertical
            && other.orientation == SegmentOrientation::Vertical
            && (self.x0 - other.x0).abs() < epsilon
    }

    /// Merge this segment with another collinear segment.
    ///
    /// Returns a new segment covering the union of both x or y ranges.
    /// Assumes segments are collinear and oriented the same way.
    pub fn merge(&self, other: &Segment) -> Segment {
        assert_eq!(
            self.orientation, other.orientation,
            "Cannot merge segments with different orientations"
        );

        match self.orientation {
            SegmentOrientation::Horizontal => {
                let y = self.y0;
                let x0 = self.x0.min(other.x0);
                let x1 = self.x1.max(other.x1);
                Self::horizontal(y, x0, x1)
            }
            SegmentOrientation::Vertical => {
                let x = self.x0;
                let y0 = self.y0.min(other.y0);
                let y1 = self.y1.max(other.y1);
                Self::vertical(x, y0, y1)
            }
        }
    }
}

/// Orientation of a path segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentOrientation {
    Horizontal,
    Vertical,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1.0;

    #[test]
    fn test_segment_new_horizontal() {
        let seg = Segment::new(10.0, 50.0, 100.0, 50.5, EPSILON).unwrap();
        assert_eq!(seg.orientation, SegmentOrientation::Horizontal);
        assert_eq!(seg.y0, 50.0);
        assert_eq!(seg.y1, 50.5); // Normalized so y0 <= y1
    }

    #[test]
    fn test_segment_new_vertical() {
        let seg = Segment::new(50.0, 10.0, 50.5, 100.0, EPSILON).unwrap();
        assert_eq!(seg.orientation, SegmentOrientation::Vertical);
        assert_eq!(seg.x0, 50.0);
        assert_eq!(seg.x1, 50.5); // Normalized so x0 <= x1
    }

    #[test]
    fn test_segment_new_diagonal_rejected() {
        let seg = Segment::new(10.0, 10.0, 100.0, 100.0, EPSILON);
        assert!(seg.is_none());
    }

    #[test]
    fn test_segment_horizontal_constructor() {
        let seg = Segment::horizontal(50.0, 100.0, 10.0);
        // Normalized
        assert_eq!(seg.y0, 50.0);
        assert_eq!(seg.y1, 50.0);
        assert_eq!(seg.x0, 10.0);
        assert_eq!(seg.x1, 100.0);
        assert_eq!(seg.orientation, SegmentOrientation::Horizontal);
    }

    #[test]
    fn test_segment_vertical_constructor() {
        let seg = Segment::vertical(50.0, 100.0, 10.0);
        // Normalized
        assert_eq!(seg.x0, 50.0);
        assert_eq!(seg.x1, 50.0);
        assert_eq!(seg.y0, 10.0);
        assert_eq!(seg.y1, 100.0);
        assert_eq!(seg.orientation, SegmentOrientation::Vertical);
    }

    #[test]
    fn test_segment_length() {
        let h = Segment::horizontal(50.0, 10.0, 100.0);
        assert_eq!(h.length(), 90.0);

        let v = Segment::vertical(50.0, 10.0, 100.0);
        assert_eq!(v.length(), 90.0);
    }

    #[test]
    fn test_segment_intersection() {
        let h = Segment::horizontal(50.0, 10.0, 100.0);
        let v = Segment::vertical(50.0, 25.0, 75.0);

        let intersection = h.intersection(&v, EPSILON);
        assert_eq!(intersection, Some((50.0, 50.0)));
    }

    #[test]
    fn test_segment_no_intersection() {
        let h = Segment::horizontal(50.0, 10.0, 100.0);
        let v = Segment::vertical(150.0, 25.0, 75.0); // x=150, outside horizontal range

        let intersection = h.intersection(&v, EPSILON);
        assert!(intersection.is_none());
    }

    #[test]
    fn test_is_collinear_horizontal() {
        let s1 = Segment::horizontal(50.0, 10.0, 100.0);
        let s2 = Segment::horizontal(50.5, 20.0, 80.0); // Within epsilon

        assert!(s1.is_collinear_horizontal(&s2, EPSILON));
    }

    #[test]
    fn test_is_collinear_vertical() {
        let s1 = Segment::vertical(50.0, 10.0, 100.0);
        let s2 = Segment::vertical(50.5, 20.0, 80.0); // Within epsilon

        assert!(s1.is_collinear_vertical(&s2, EPSILON));
    }

    #[test]
    fn test_merge_horizontal() {
        let s1 = Segment::horizontal(50.0, 10.0, 50.0);
        let s2 = Segment::horizontal(50.0, 40.0, 100.0);

        let merged = s1.merge(&s2);
        assert_eq!(merged.y0, 50.0);
        assert_eq!(merged.x0, 10.0);
        assert_eq!(merged.x1, 100.0);
    }

    #[test]
    fn test_merge_vertical() {
        let s1 = Segment::vertical(50.0, 10.0, 50.0);
        let s2 = Segment::vertical(50.0, 30.0, 100.0);

        let merged = s1.merge(&s2);
        assert_eq!(merged.x0, 50.0);
        assert_eq!(merged.y0, 10.0);
        assert_eq!(merged.y1, 100.0);
    }
}
