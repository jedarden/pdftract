//! Figure block classifier (Phase 4 figure detection).
//!
//! This module implements classification of image regions as figure blocks
//! based on text overlap analysis. Image XObjects with < 50% text overlap
//! are classified as figures.
//!
//! # Algorithm
//!
//! For each image XObject on a page:
//! 1. Compute the union of all text glyph bboxes intersecting the image bbox
//! 2. Calculate overlap ratio = (text_overlap_area / image_bbox_area)
//! 3. If overlap < 0.5 (50%), create a Figure block with the image's bbox
//!
//! This distinction separates:
//! - **Figures:** Images that are primarily visual content (charts, photos, diagrams)
//! - **Text-on-image:** Screenshots of text, scanned text images, document thumbnails
//!
//! # References
//!
//! - Plan section: Phase 4.4 Block Formation → Figure block kind assignment
//! - Phase 3.3: Do operator (XObject image placement)
//! - Phase 3.5: Inline images (BI/ID/EI sequence)

use crate::content_stream::ImageXObject;
use std::sync::Arc;

/// Block with layout properties for figure classification.
///
/// This reuses the caption.rs Block structure for consistency
/// across layout classifiers.
pub use crate::layout::caption::Block;

/// Page context for figure classification.
///
/// Contains the image list and glyph bboxes needed for figure detection.
#[derive(Debug, Clone)]
pub struct FigurePageContext {
    /// Image XObjects on this page (from Phase 3.3 Do + Phase 3.5 inline images).
    pub images: Vec<ImageXObject>,
    /// Glyph bounding boxes on this page (for text overlap computation).
    pub glyph_bboxes: Vec<[f32; 4]>,
}

impl FigurePageContext {
    /// Create a new empty page context.
    pub fn new() -> Self {
        Self {
            images: Vec::new(),
            glyph_bboxes: Vec::new(),
        }
    }

    /// Create a new page context with images and glyph bboxes.
    pub fn with_data(images: Vec<ImageXObject>, glyph_bboxes: Vec<[f32; 4]>) -> Self {
        Self {
            images,
            glyph_bboxes,
        }
    }
}

impl Default for FigurePageContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Classify figures on a page based on image XObjects and text overlap.
///
/// This function analyzes each image XObject and classifies it as a Figure
/// block if less than 50% of its area is covered by text glyphs.
///
/// # Arguments
///
/// * `ctx` - Page context with images and glyph bboxes
///
/// # Returns
///
/// A vector of Figure blocks, one per qualifying image, sorted by bbox top y
/// (descending, i.e., highest on the page first).
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::figure::{classify_figure, FigurePageContext};
/// use pdftract_core::content_stream::ImageXObject;
/// use std::sync::Arc;
///
/// // Page with two images: one mostly visual, one covered by text
/// let ctx = FigurePageContext::with_data(
///     vec![
///         // Pure figure (no text overlap)
///         ImageXObject {
///             bbox: [100.0, 400.0, 300.0, 600.0],
///             xobject_ref: pdftract_core::parser::object::ObjRef { object_number: 1, generation_number: 0 },
///             name: Arc::from("figure1"),
///         },
///         // Text-on-image (screenshot with text annotation)
///         ImageXObject {
///             bbox: [100.0, 100.0, 300.0, 300.0],
///             xobject_ref: pdftract_core::parser::object::ObjRef { object_number: 2, generation_number: 0 },
///             name: Arc::from("figure2"),
///         },
///     ],
///     vec![
///         // Text overlaps the second image
///         [120.0, 120.0, 280.0, 280.0],
///     ],
/// );
///
/// let figures = classify_figure(&ctx);
/// assert_eq!(figures.len(), 1); // Only the first image is a figure
/// ```
pub fn classify_figure(ctx: &FigurePageContext) -> Vec<Block> {
    let mut figures = Vec::new();

    for image in &ctx.images {
        let image_bbox = image.bbox;
        let image_area = bbox_area(image_bbox);

        // Skip zero-area images (degenerate CTM)
        if image_area <= 0.0 {
            continue;
        }

        // Compute text overlap area
        let text_overlap_area = compute_text_overlap_area(&image_bbox, &ctx.glyph_bboxes);

        // Classify as figure if < 50% text overlap
        if text_overlap_area / image_area < 0.5 {
            figures.push(Block {
                kind: "figure".to_string(),
                text: String::new(),
                median_font_size: 0.0,
                bbox: image_bbox,
                column: 0, // TODO: assign column based on image center x position
            });
        }
    }

    // Sort by bbox top y (descending) so highest figures appear first
    figures.sort_by(|a, b| b.top().partial_cmp(&a.top()).unwrap_or(std::cmp::Ordering::Equal));

    figures
}

/// Compute the area of a bounding box.
fn bbox_area(bbox: [f32; 4]) -> f32 {
    let width = bbox[2] - bbox[0];
    let height = bbox[3] - bbox[1];
    width * height
}

/// Compute the union area of all glyph bboxes intersecting the image bbox.
///
/// This function:
/// 1. Filters to glyphs that intersect the image bbox
/// 2. Computes the union of all intersecting glyph bboxes
/// 3. Returns the area of the union (clipped to the image bbox)
///
/// Uses a sweep line algorithm: for each vertical strip between unique x coordinates,
/// compute the total y coverage and sum (strip_width * y_coverage).
///
/// # Arguments
///
/// * `image_bbox` - The image's bounding box [x0, y0, x1, y1]
/// * `glyph_bboxes` - All glyph bboxes on the page
///
/// # Returns
///
/// The area of the union of all intersecting glyph bboxes, clipped to the image bbox.
fn compute_text_overlap_area(image_bbox: &[f32; 4], glyph_bboxes: &[[f32; 4]]) -> f32 {
    // Collect all intersecting rectangles (clipped to image bbox)
    let mut rects: Vec<[f32; 4]> = Vec::new();

    for glyph_bbox in glyph_bboxes {
        if bboxes_intersect(image_bbox, glyph_bbox) {
            let intersection = [
                image_bbox[0].max(glyph_bbox[0]),
                image_bbox[1].max(glyph_bbox[1]),
                image_bbox[2].min(glyph_bbox[2]),
                image_bbox[3].min(glyph_bbox[3]),
            ];

            // Skip empty intersections
            if intersection[0] < intersection[2] && intersection[1] < intersection[3] {
                rects.push(intersection);
            }
        }
    }

    if rects.is_empty() {
        return 0.0;
    }

    // Sweep line algorithm: compute union area
    // 1. Collect all unique x coordinates
    let mut xs: Vec<f32> = rects.iter().flat_map(|r| [r[0], r[2]]).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let mut total_area = 0.0;

    // 2. For each vertical strip between consecutive x coordinates
    for i in 0..xs.len() - 1 {
        let x_left = xs[i];
        let x_right = xs[i + 1];

        // Skip zero-width strips
        if x_right <= x_left {
            continue;
        }

        // 3. Collect all y-intervals that cover this x-strip
        let mut intervals: Vec<[f32; 2]> = Vec::new();
        for rect in &rects {
            // Check if rectangle overlaps this x-strip (not fully contained)
            if rect[2] > x_left && rect[0] < x_right {
                intervals.push([rect[1], rect[3]]);
            }
        }

        if intervals.is_empty() {
            continue;
        }

        // 4. Merge overlapping y-intervals
        intervals.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
        let mut merged: Vec<[f32; 2]> = Vec::new();

        for interval in intervals {
            if let Some(last) = merged.last_mut() {
                if interval[0] <= last[1] {
                    // Overlapping or adjacent - merge
                    last[1] = last[1].max(interval[1]);
                } else {
                    merged.push(interval);
                }
            } else {
                merged.push(interval);
            }
        }

        // 5. Sum up y coverage for this strip
        let y_coverage: f32 = merged.iter().map(|i| i[1] - i[0]).sum();
        let strip_width = x_right - x_left;
        total_area += strip_width * y_coverage;
    }

    total_area
}

/// Check if two bounding boxes intersect.
fn bboxes_intersect(a: &[f32; 4], b: &[f32; 4]) -> bool {
    // No intersection if one is completely to the left/right/above/below the other
    !(a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::ObjRef;

    fn make_image(x0: f32, y0: f32, x1: f32, y1: f32) -> ImageXObject {
        ImageXObject {
            bbox: [x0, y0, x1, y1],
            xobject_ref: ObjRef { object: 1, generation: 0 },
            name: Arc::from("test"),
        }
    }

    #[test]
    fn test_bbox_area() {
        assert_eq!(bbox_area([0.0, 0.0, 100.0, 50.0]), 5000.0);
        assert_eq!(bbox_area([10.0, 20.0, 30.0, 40.0]), 400.0);
    }

    #[test]
    fn test_bboxes_intersect() {
        // Overlapping
        assert!(bboxes_intersect(&[0.0, 0.0, 10.0, 10.0], &[5.0, 5.0, 15.0, 15.0]));

        // Touching at edge (no actual overlap)
        assert!(!bboxes_intersect(&[0.0, 0.0, 10.0, 10.0], &[10.0, 0.0, 20.0, 10.0]));

        // Disjoint
        assert!(!bboxes_intersect(&[0.0, 0.0, 10.0, 10.0], &[20.0, 20.0, 30.0, 30.0]));

        // One inside the other
        assert!(bboxes_intersect(&[0.0, 0.0, 100.0, 100.0], &[10.0, 10.0, 20.0, 20.0]));
    }

    #[test]
    fn test_classify_figure_pure_visual_image() {
        // Image with no text overlap → classified as figure
        let ctx = FigurePageContext::with_data(
            vec![make_image(100.0, 400.0, 300.0, 600.0)],
            vec![
                [400.0, 400.0, 500.0, 500.0], // Text far to the right
            ],
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 1);
        assert_eq!(figures[0].kind, "figure");
        assert_eq!(figures[0].bbox, [100.0, 400.0, 300.0, 600.0]);
    }

    #[test]
    fn test_classify_figure_text_on_image() {
        // Image fully covered by text → NOT classified as figure
        let ctx = FigurePageContext::with_data(
            vec![make_image(100.0, 100.0, 300.0, 300.0)],
            vec![
                [90.0, 90.0, 310.0, 310.0], // Text fully covers image
            ],
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 0); // Too much text overlap
    }

    #[test]
    fn test_classify_figure_partial_text_below_threshold() {
        // Image with 40% text overlap → classified as figure
        let ctx = FigurePageContext::with_data(
            vec![make_image(0.0, 0.0, 100.0, 100.0)],
            vec![
                [0.0, 0.0, 60.0, 60.0], // 36% coverage (60*60 / 100*100 = 0.36)
            ],
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 1);
    }

    #[test]
    fn test_classify_figure_partial_text_above_threshold() {
        // Image with 60% text overlap → NOT classified as figure
        let ctx = FigurePageContext::with_data(
            vec![make_image(0.0, 0.0, 100.0, 100.0)],
            vec![
                [0.0, 0.0, 80.0, 80.0], // 64% coverage (80*80 / 100*100 = 0.64)
            ],
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 0);
    }

    #[test]
    fn test_classify_figure_exactly_at_threshold() {
        // Image with exactly 50% text overlap → classified as figure
        // (overlap < 0.5, so 0.5 exactly is NOT a figure per the spec)
        // Actually, re-reading the spec: "If overlap < 0.5 (50%)"
        // So overlap < 0.5 means figure, overlap >= 0.5 means NOT figure
        // Let's verify: 49% should be figure, 50% should NOT be figure, 51% should NOT be figure

        // 49% overlap (70.7 * 70.7 ≈ 5000, which is 50% of 10000)
        // Let's use simpler numbers: 70*70 = 4900, which is 49% of 10000
        let ctx = FigurePageContext::with_data(
            vec![make_image(0.0, 0.0, 100.0, 100.0)],
            vec![
                [0.0, 0.0, 70.0, 70.0], // 49% coverage
            ],
        );
        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 1, "49% overlap should be classified as figure");

        // 50% overlap (sqrt(5000) ≈ 70.71)
        let ctx = FigurePageContext::with_data(
            vec![make_image(0.0, 0.0, 100.0, 100.0)],
            vec![
                [0.0, 0.0, 71.0, 71.0], // ~50.4% coverage (>50%)
            ],
        );
        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 0, ">=50% overlap should NOT be classified as figure");
    }

    #[test]
    fn test_classify_figure_sort_order() {
        // Multiple figures should be sorted by top y (highest first)
        let ctx = FigurePageContext::with_data(
            vec![
                make_image(0.0, 100.0, 100.0, 200.0), // Lower
                make_image(0.0, 300.0, 100.0, 400.0), // Higher
                make_image(0.0, 200.0, 100.0, 300.0), // Middle
            ],
            vec![], // No text
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 3);
        assert_eq!(figures[0].bbox[3], 400.0); // Highest
        assert_eq!(figures[1].bbox[3], 300.0); // Middle
        assert_eq!(figures[2].bbox[3], 200.0); // Lowest
    }

    #[test]
    fn test_classify_figure_empty_context() {
        let ctx = FigurePageContext::new();
        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 0);
    }

    #[test]
    fn test_classify_figure_no_images() {
        let ctx = FigurePageContext::with_data(
            vec![],
            vec![[0.0, 0.0, 100.0, 100.0]],
        );
        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 0);
    }

    #[test]
    fn test_classify_figure_no_glyphs() {
        // Images with no glyphs at all should all be figures
        let ctx = FigurePageContext::with_data(
            vec![
                make_image(0.0, 0.0, 100.0, 100.0),
                make_image(200.0, 200.0, 300.0, 300.0),
            ],
            vec![],
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 2);
    }

    #[test]
    fn test_compute_text_overlap_area_multiple_glyphs() {
        // Multiple overlapping glyphs should produce a union area
        let image_bbox = [0.0, 0.0, 100.0, 100.0];
        let glyph_bboxes = vec![
            [0.0, 0.0, 40.0, 40.0],   // Bottom-left
            [60.0, 0.0, 100.0, 40.0],  // Bottom-right
            [0.0, 60.0, 40.0, 100.0], // Top-left
            [60.0, 60.0, 100.0, 100.0], // Top-right
        ];

        let overlap = compute_text_overlap_area(&image_bbox, &glyph_bboxes);
        // All 4 corners, disjoint, so area = 4 * 40*40 = 6400
        assert!((overlap - 6400.0).abs() < 1.0);
    }

    #[test]
    fn test_compute_text_overlap_area_union() {
        // Overlapping glyphs should produce union (not sum)
        let image_bbox = [0.0, 0.0, 100.0, 100.0];
        let glyph_bboxes = vec![
            [0.0, 0.0, 60.0, 60.0],   // Large area
            [40.0, 40.0, 100.0, 100.0], // Overlaps with first
        ];

        let overlap = compute_text_overlap_area(&image_bbox, &glyph_bboxes);
        // Union of [0,0,60,60] and [40,40,100,100] = 6800 (not 7200 sum due to overlap)
        // The overlapping region [40,40,60,60] is counted only once
        let expected = 6800.0;
        assert!((overlap - expected).abs() < 1.0, "Union area should be {}, got {}", expected, overlap);
        assert!(overlap < 10000.0, "Union should not exceed image bounds");
    }

    #[test]
    fn test_figure_block_properties() {
        let ctx = FigurePageContext::with_data(
            vec![make_image(100.0, 400.0, 300.0, 600.0)],
            vec![],
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 1);

        let figure = &figures[0];
        assert_eq!(figure.kind, "figure");
        assert_eq!(figure.text, "");
        assert_eq!(figure.median_font_size, 0.0);
        assert_eq!(figure.column, 0);
    }

    #[test]
    fn test_five_figures_no_text() {
        // Test case from acceptance criteria:
        // PDF with 5 figures (images, no text overlay) → 5 Figure blocks in output
        let ctx = FigurePageContext::with_data(
            vec![
                make_image(0.0, 100.0, 100.0, 200.0),
                make_image(100.0, 100.0, 200.0, 200.0),
                make_image(200.0, 100.0, 300.0, 200.0),
                make_image(300.0, 100.0, 400.0, 200.0),
                make_image(400.0, 100.0, 500.0, 200.0),
            ],
            vec![], // No text overlay
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 5);
    }

    #[test]
    fn test_text_covered_image_not_figure() {
        // Test case from acceptance criteria:
        // PDF with 1 image fully covered by text (screenshot with text annotation)
        // → no Figure block (text overlap >= 50%)
        let ctx = FigurePageContext::with_data(
            vec![make_image(0.0, 0.0, 100.0, 100.0)],
            vec![
                [0.0, 0.0, 100.0, 100.0], // Text fully covers image (100% overlap)
            ],
        );

        let figures = classify_figure(&ctx);
        assert_eq!(figures.len(), 0);
    }
}
