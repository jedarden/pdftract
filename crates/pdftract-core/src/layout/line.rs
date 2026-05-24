//! Line formation for Phase 4.2.
//!
//! This module implements grouping spans into lines by baseline proximity
//! and computing line-level metadata including bbox, baseline, and direction.

use serde::{Deserialize, Serialize};

/// Text direction for a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineDirection {
    /// Left-to-right text (e.g., Latin, Cyrillic)
    Ltr,
    /// Right-to-left text (e.g., Arabic, Hebrew)
    Rtl,
    /// Mixed direction (bidi text)
    Mixed,
}

/// A line of text composed of one or more spans.
///
/// Lines are the third-level structural unit in the extraction pipeline,
/// after Glyphs and Spans. Line bbox drives column detection and reading
/// order; baseline drives clustering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line<S> {
    /// Spans that make up this line, in reading order.
    pub spans: Vec<S>,
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    ///
    /// This is the union of all span bboxes.
    pub bbox: [f32; 4],
    /// Baseline y-coordinate for this line.
    ///
    /// Computed as the average of member span baselines.
    pub baseline: f32,
    /// Text direction for this line.
    pub direction: LineDirection,
    /// Page-relative Y position (0=top, 1=bottom).
    ///
    /// Used for reading order sorting. Computed as:
    /// `(page_height - bbox[3]) / page_height`
    pub page_relative_y: f32,
}

impl<S> Line<S> {
    /// Get the left X coordinate of the line.
    #[inline]
    pub fn left(&self) -> f32 {
        self.bbox[0]
    }

    /// Get the bottom Y coordinate of the line.
    #[inline]
    pub fn bottom(&self) -> f32 {
        self.bbox[1]
    }

    /// Get the right X coordinate of the line.
    #[inline]
    pub fn right(&self) -> f32 {
        self.bbox[2]
    }

    /// Get the top Y coordinate of the line.
    #[inline]
    pub fn top(&self) -> f32 {
        self.bbox[3]
    }

    /// Get the width of the line's bbox.
    #[inline]
    pub fn width(&self) -> f32 {
        self.bbox[2] - self.bbox[0]
    }

    /// Get the height of the line's bbox.
    #[inline]
    pub fn height(&self) -> f32 {
        self.bbox[3] - self.bbox[1]
    }
}

/// Compute the baseline y-coordinate for a span.
///
/// The baseline is approximated as `y0 + (bbox_height * 0.2)`, where the
/// 0.2 multiplier is an empirical fit for most Latin fonts. The exact
/// value would require font descender metrics from the font dictionary.
///
/// # Arguments
///
/// * `bbox` - Span bounding box [x0, y0, x1, y1] in PDF user space
///
/// # Returns
///
/// The baseline y-coordinate.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::line::compute_baseline;
///
/// // Span bbox [0, 100, 50, 110] (height 10)
/// let baseline = compute_baseline(&[0.0, 100.0, 50.0, 110.0]);
/// assert_eq!(baseline, 102.0);
///
/// // Span bbox [0, 100, 50, 100] (zero height)
/// let baseline = compute_baseline(&[0.0, 100.0, 50.0, 100.0]);
/// assert_eq!(baseline, 100.0);
/// ```
#[inline]
pub fn compute_baseline(bbox: &[f32; 4]) -> f32 {
    let height = bbox[3] - bbox[1];
    bbox[1] + height * 0.2
}

/// Trait for types that have a bounding box.
///
/// This trait allows the line formation code to work with different
/// span representations (internal, JSON, etc.).
pub trait HasBBox {
    /// Get the bounding box [x0, y0, x1, y1] in PDF user space.
    fn bbox(&self) -> [f32; 4];
}

/// Compute the union of multiple bounding boxes.
///
/// # Arguments
///
/// * `bboxes` - Iterator of bounding boxes
///
/// # Returns
///
/// The union bounding box, or None if the iterator is empty.
pub fn union_bboxes<'a, I>(bboxes: I) -> Option<[f32; 4]>
where
    I: IntoIterator<Item = &'a [f32; 4]>,
{
    let mut iter = bboxes.into_iter();
    let first = *iter.next()?;
    let mut union = first;

    for bbox in iter {
        union[0] = union[0].min(bbox[0]); // x0
        union[1] = union[1].min(bbox[1]); // y0
        union[2] = union[2].max(bbox[2]); // x1
        union[3] = union[3].max(bbox[3]); // y1
    }

    Some(union)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_baseline_normal_span() {
        // Span bbox [0, 100, 50, 110] (height 10)
        // baseline = 100 + 10 * 0.2 = 102
        let bbox = [0.0, 100.0, 50.0, 110.0];
        assert_eq!(compute_baseline(&bbox), 102.0);
    }

    #[test]
    fn test_compute_baseline_zero_height() {
        // Span bbox [0, 100, 50, 100] (zero height)
        // baseline = 100 + 0 * 0.2 = 100
        let bbox = [0.0, 100.0, 50.0, 100.0];
        assert_eq!(compute_baseline(&bbox), 100.0);
    }

    #[test]
    fn test_compute_baseline_large_height() {
        // Span bbox [0, 0, 100, 50] (height 50)
        // baseline = 0 + 50 * 0.2 = 10
        let bbox = [0.0, 0.0, 100.0, 50.0];
        assert_eq!(compute_baseline(&bbox), 10.0);
    }

    #[test]
    fn test_line_direction_serdes_ltr() {
        let dir = LineDirection::Ltr;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"ltr\"");

        let deserialized: LineDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LineDirection::Ltr);
    }

    #[test]
    fn test_line_direction_serdes_rtl() {
        let dir = LineDirection::Rtl;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"rtl\"");

        let deserialized: LineDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LineDirection::Rtl);
    }

    #[test]
    fn test_line_direction_serdes_mixed() {
        let dir = LineDirection::Mixed;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, "\"mixed\"");

        let deserialized: LineDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LineDirection::Mixed);
    }

    #[test]
    fn test_line_accessors() {
        let line: Line<()> = Line {
            spans: vec![],
            bbox: [10.0, 20.0, 110.0, 70.0],
            baseline: 30.0,
            direction: LineDirection::Ltr,
            page_relative_y: 0.5,
        };

        assert_eq!(line.left(), 10.0);
        assert_eq!(line.bottom(), 20.0);
        assert_eq!(line.right(), 110.0);
        assert_eq!(line.top(), 70.0);
        assert_eq!(line.width(), 100.0);
        assert_eq!(line.height(), 50.0);
    }

    #[test]
    fn test_union_bboxes_single() {
        let bboxes = vec![[10.0, 20.0, 50.0, 40.0]];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, Some([10.0, 20.0, 50.0, 40.0]));
    }

    #[test]
    fn test_union_bboxes_multiple() {
        let bboxes = vec![
            [0.0, 0.0, 50.0, 20.0],
            [50.0, 0.0, 100.0, 20.0],
            [0.0, 20.0, 100.0, 40.0],
        ];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, Some([0.0, 0.0, 100.0, 40.0]));
    }

    #[test]
    fn test_union_bboxes_empty() {
        let bboxes: Vec<[f32; 4]> = vec![];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, None);
    }

    #[test]
    fn test_union_bboxes_nested() {
        // Small box inside larger box
        let bboxes = vec![
            [0.0, 0.0, 100.0, 100.0],
            [25.0, 25.0, 75.0, 75.0],
        ];
        let result = union_bboxes(&bboxes);
        // Union should be the larger box
        assert_eq!(result, Some([0.0, 0.0, 100.0, 100.0]));
    }

    #[test]
    fn test_union_bboxes_disjoint() {
        // Two disjoint boxes
        let bboxes = vec![
            [0.0, 0.0, 50.0, 50.0],
            [100.0, 100.0, 150.0, 150.0],
        ];
        let result = union_bboxes(&bboxes);
        assert_eq!(result, Some([0.0, 0.0, 150.0, 150.0]));
    }
}
