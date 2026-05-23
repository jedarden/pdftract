//! Hybrid page handling (Phase 5.2.4).
//!
//! This module implements the hybrid page pipeline for pages with mixed
//! vector and scanned content:
//! 1. Consume PageClassification::hybrid_cells (set of scanned cell indices)
//! 2. Render only the image-heavy cells (not the whole page)
//! 3. Run OCR per cell
//! 4. Merge OCR spans with Phase 3 vector spans using bbox overlap rule
//!
//! # Cell Rendering Strategy
//!
//! Render the full page once at the selected DPI, then crop per cell from
//! the rendered raster. This is cheaper than re-rendering per cell.
//!
//! # Merge Rule
//!
//! For each OCR span O:
//! - Find any vector span V with IoU(O.bbox, V.bbox) > 0.5
//! - If found AND vector confidence >= 0.5: drop O (vector wins)
//! - If found AND vector confidence < 0.5: keep O (OCR preferred over bad vector)
//! - If not found: keep O
//!
//! IoU = area(A ∩ B) / area(A ∪ B)

use crate::classify::{CellIndex, PageClassification, PageClass};
use image::{GrayImage, ImageBuffer, Luma};
use std::collections::BTreeSet;

/// Internal span representation for merge operations.
///
/// This is a minimal span type used during the merge operation.
/// The actual extraction pipeline uses SpanJson from the schema module.
#[derive(Debug, Clone)]
pub struct Span {
    /// Bounding box [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f64; 4],
    /// Confidence score [0.0, 1.0].
    pub confidence: f32,
    /// Source of this span: "vector" or "ocr".
    pub source: SpanSource,
    /// The extracted text.
    pub text: String,
}

/// Source of a span - either vector extraction or OCR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanSource {
    /// Text extracted from content stream (Phase 3).
    Vector,
    /// Text extracted via OCR (Phase 5).
    Ocr,
}

impl Span {
    /// Create a new span.
    pub fn new(bbox: [f64; 4], confidence: f32, source: SpanSource, text: String) -> Self {
        Self {
            bbox,
            confidence,
            source,
            text,
        }
    }

    /// Create a span with vector source.
    pub fn vector(bbox: [f64; 4], confidence: f32, text: String) -> Self {
        Self::new(bbox, confidence, SpanSource::Vector, text)
    }

    /// Create a span with OCR source.
    pub fn ocr(bbox: [f64; 4], confidence: f32, text: String) -> Self {
        Self::new(bbox, confidence, SpanSource::Ocr, text)
    }

    /// Get the width of the span's bbox.
    #[inline]
    pub fn width(&self) -> f64 {
        self.bbox[2] - self.bbox[0]
    }

    /// Get the height of the span's bbox.
    #[inline]
    pub fn height(&self) -> f64 {
        self.bbox[3] - self.bbox[1]
    }

    /// Get the area of the span's bbox.
    #[inline]
    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }
}

/// Compute the Intersection over Union (IoU) of two bounding boxes.
///
/// IoU = area(A ∩ B) / area(A ∪ B)
///
/// # Arguments
///
/// * `a` - First bbox [x0, y0, x1, y1]
/// * `b` - Second bbox [x0, y0, x1, y1]
///
/// # Returns
///
/// IoU value in [0.0, 1.0]. Returns 0.0 if bboxes don't intersect.
#[inline]
pub fn compute_iou(a: [f64; 4], b: [f64; 4]) -> f64 {
    // Compute intersection
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = a[2].min(b[2]);
    let y1 = a[3].min(b[3]);

    // No intersection if x1 < x0 or y1 < y0
    if x1 < x0 || y1 < y0 {
        return 0.0;
    }

    let intersection_area = (x1 - x0) * (y1 - y0);

    // Compute union
    let a_area = (a[2] - a[0]) * (a[3] - a[1]);
    let b_area = (b[2] - b[0]) * (b[3] - b[1]);
    let union_area = a_area + b_area - intersection_area;

    if union_area <= 0.0 {
        return 0.0;
    }

    intersection_area / union_area
}

/// Merge vector and OCR spans using the bbox overlap rule.
///
/// For each OCR span O:
/// 1. Find any vector span V with IoU(O.bbox, V.bbox) > 0.5
/// 2. If found AND V.confidence >= 0.5: drop O (vector wins)
/// 3. If found AND V.confidence < 0.5: keep O (OCR preferred over bad vector)
/// 4. If not found: keep O
/// 5. Return all V + retained O sorted by reading order
///
/// # Arguments
///
/// * `vector_spans` - Spans from Phase 3 content stream extraction
/// * `ocr_spans` - Spans from Phase 5 OCR
///
/// # Returns
///
/// Merged span list with no duplicate text from overlapping regions.
///
/// # Reading Order
///
/// The returned spans are sorted by top-to-bottom, left-to-right order
/// (reading order). Note: Phase 4.5 recomputes the final reading order;
/// this task only produces the merged list.
pub fn merge_vector_and_ocr_spans(vector_spans: &[Span], ocr_spans: &[Span]) -> Vec<Span> {
    let mut result = Vec::new();

    // Add all vector spans (they're always kept unless overlapping with higher-confidence OCR)
    for v in vector_spans {
        result.push(v.clone());
    }

    // For each OCR span, check if it overlaps with any vector span
    for ocr_span in ocr_spans {
        let mut should_keep = true;

        for vector_span in vector_spans {
            let iou = compute_iou(ocr_span.bbox, vector_span.bbox);

            if iou > 0.5 {
                // Overlap detected
                if vector_span.confidence >= 0.5 {
                    // Vector wins - drop OCR span
                    should_keep = false;
                    break;
                }
                // else: vector confidence < 0.5, keep OCR span
            }
        }

        if should_keep {
            result.push(ocr_span.clone());
        }
    }

    // Sort by reading order (top-to-bottom, left-to-right)
    result.sort_by(|a, b| {
        let a_center_y = (a.bbox[1] + a.bbox[3]) / 2.0;
        let b_center_y = (b.bbox[1] + b.bbox[3]) / 2.0;

        // Primary sort: Y (top to bottom = descending Y in PDF coordinates)
        // Note: In PDF coordinates, Y=0 is at the bottom, so higher Y means higher on page
        b_center_y.partial_cmp(&a_center_y).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let a_center_x = (a.bbox[0] + a.bbox[2]) / 2.0;
                let b_center_x = (b.bbox[0] + b.bbox[2]) / 2.0;
                a_center_x.partial_cmp(&b_center_x).unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    result
}

/// Crop a cell from a rendered page image.
///
/// # Arguments
///
/// * `page_image` - The full rendered page (grayscale)
/// * `page_width_pt` - Page width in PDF points
/// * `page_height_pt` - Page height in PDF points
/// * `cell` - The cell index to crop
/// * `dpi` - DPI used for rendering
///
/// # Returns
///
/// The cropped cell image, padded with white if the crop extends beyond bounds.
pub fn crop_cell_from_page(
    page_image: &GrayImage,
    page_width_pt: f64,
    page_height_pt: f64,
    cell: CellIndex,
    dpi: u32,
) -> GrayImage {
    // Calculate cell dimensions in pixels
    let scale = dpi as f64 / 72.0;
    let page_width_px = (page_width_pt * scale).ceil() as u32;
    let page_height_px = (page_height_pt * scale).ceil() as u32;

    // Cell size in pixels (8x8 grid)
    let cell_width_px = page_width_px / 8;
    let cell_height_px = page_height_px / 8;

    // Cell origin in pixels
    let x0 = cell.col as u32 * cell_width_px;
    let y0 = (7 - cell.row) as u32 * cell_height_px; // Row 0 is at top (Y=max in PDF)

    // Cell extent (clamp to page bounds)
    let x1 = (x0 + cell_width_px).min(page_width_px);
    let y1 = (y0 + cell_height_px).min(page_height_px);

    // Handle edge cases: if crop extends beyond page, pad with white
    let actual_width = x1 - x0;
    let actual_height = y1 - y0;

    if actual_width == 0 || actual_height == 0 {
        // Cell is outside page bounds - return minimal white image
        return GrayImage::new(cell_width_px.max(1), cell_height_px.max(1));
    }

    // Create target image (white background)
    let mut cell_image = GrayImage::new(cell_width_px.max(1), cell_height_px.max(1));
    for pixel in cell_image.pixels_mut() {
        *pixel = Luma([255]);
    }

    // Copy pixels from page image to cell image
    for y in 0..actual_height {
        for x in 0..actual_width {
            let page_x = x0 + x;
            let page_y = y0 + y;

            if page_x < page_width_px && page_y < page_height_px {
                let pixel = page_image.get_pixel(page_x, page_y);
                cell_image.put_pixel(x, y, *pixel);
            }
        }
    }

    cell_image
}

/// Get the list of cell indices from a Hybrid page classification.
///
/// Returns an empty vec for non-Hybrid pages.
pub fn get_hybrid_cells(classification: &PageClassification) -> Vec<CellIndex> {
    if classification.class != crate::classify::PageClass::Hybrid {
        return Vec::new();
    }

    match &classification.hybrid_cells {
        Some(cells) => {
            cells.iter()
                .map(|&flat| CellIndex::from_flat(flat))
                .collect()
        }
        None => Vec::new(),
    }
}

/// Cell crop coordinates in PDF user space.
///
/// Represents the bounding box of a cell in PDF point coordinates.
#[derive(Debug, Clone)]
pub struct CellCrop {
    /// Cell row (0-7, 0 = top)
    pub row: u8,
    /// Cell column (0-7, 0 = left)
    pub col: u8,
    /// Bounding box [x0, y0, x1, y1] in PDF points
    pub bbox: [f64; 4],
}

/// Compute cell crop coordinates for all hybrid cells.
///
/// Returns the list of cell crops in PDF user space coordinates.
///
/// # Arguments
///
/// * `classification` - Page classification with hybrid_cells
/// * `page_width` - Page width in PDF points
/// * `page_height` - Page height in PDF points
///
/// # Returns
///
/// List of cell crops, sorted by flat index (deterministic order).
pub fn compute_cell_crops(
    classification: &PageClassification,
    page_width: f64,
    page_height: f64,
) -> Vec<CellCrop> {
    let cells = get_hybrid_cells(classification);
    let cell_width = page_width / 8.0;
    let cell_height = page_height / 8.0;

    cells.iter()
        .map(|cell| {
            // Cell coordinates in PDF space
            // col 0 = left, row 0 = top
            let x0 = cell.col as f64 * cell_width;
            let y1 = page_height - (cell.row as f64 * cell_height); // Y is flipped in PDF
            let x1 = x0 + cell_width;
            let y0 = y1 - cell_height;

            CellCrop {
                row: cell.row,
                col: cell.col,
                bbox: [x0, y0, x1, y1],
            }
        })
        .collect()
}

/// OCR callback trait for hybrid page processing.
///
/// This trait abstracts the OCR implementation (Phase 5.3 preprocessing + 5.4 Tesseract)
/// to allow testing and future implementation.
pub trait OcrCallback: Send + Sync {
    /// Run OCR on a single cell image.
    ///
    /// # Arguments
    ///
    /// * `cell_image` - The cropped cell image (grayscale)
    /// * `cell` - The cell index
    /// * `dpi` - The DPI used for rendering
    ///
    /// # Returns
    ///
    /// A vector of OCR spans found in this cell, or an error if OCR fails.
    fn ocr_cell(&self, cell_image: &GrayImage, cell: CellIndex, dpi: u32) -> Result<Vec<Span>, String>;
}

/// Mock OCR callback for testing that tracks call counts.
#[cfg(test)]
struct MockOcrCallback {
    call_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    output_spans: Vec<Span>,
}

#[cfg(test)]
impl OcrCallback for MockOcrCallback {
    fn ocr_cell(&self, _cell_image: &GrayImage, _cell: CellIndex, _dpi: u32) -> Result<Vec<Span>, String> {
        self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(self.output_spans.clone())
    }
}

/// Process a hybrid page by running OCR on image-heavy cells and merging with vector spans.
///
/// This is the main entry point for hybrid page handling (Phase 5.2.4):
/// 1. Render the full page once at the selected DPI
/// 2. For each hybrid cell: crop from the rendered page and run OCR
/// 3. Merge OCR spans with vector spans using the bbox overlap rule
///
/// # Arguments
///
/// * `page_image` - The full rendered page (grayscale) at the selected DPI
/// * `page_width_pt` - Page width in PDF points
/// * `page_height_pt` - Page height in PDF points
/// * `classification` - Page classification with hybrid_cells set
/// * `vector_spans` - Spans from Phase 3 content stream extraction
/// * `dpi` - DPI used for rendering
/// * `ocr_callback` - Callback to run OCR on each cell image
///
/// # Returns
///
/// Merged span list with no duplicate text from overlapping regions.
///
/// # Example
///
/// ```
/// use pdftract_core::hybrid::{process_hybrid_page, Span, SpanSource};
/// use pdftract_core::classify::{PageClassification, CellIndex};
/// use std::collections::BTreeSet;
/// use image::GrayImage;
///
/// // Create a mock classification with hybrid cells (bottom 6 rows)
/// let mut cells = BTreeSet::new();
/// for row in 2..8 {
///     for col in 0..8 {
///         cells.insert(CellIndex::new(row, col).flat());
///     }
/// }
/// let classification = PageClassification::hybrid(0.75, cells);
///
/// // Process the page (with mock OCR)
/// let result = process_hybrid_page(
///     &page_image,
///     612.0,
///     792.0,
///     &classification,
///     &vector_spans,
///     300,
///     &mock_ocr,
/// );
/// ```
pub fn process_hybrid_page(
    page_image: &GrayImage,
    page_width_pt: f64,
    page_height_pt: f64,
    classification: &PageClassification,
    vector_spans: &[Span],
    dpi: u32,
    ocr_callback: &dyn OcrCallback,
) -> Vec<Span> {
    let mut all_ocr_spans = Vec::new();

    // Get the list of hybrid cells (scanned cells only)
    let hybrid_cells = get_hybrid_cells(classification);

    // For each hybrid cell: crop and run OCR
    for cell in hybrid_cells {
        // Crop the cell from the rendered page
        let cell_image = crop_cell_from_page(
            page_image,
            page_width_pt,
            page_height_pt,
            cell,
            dpi,
        );

        // Run OCR on this cell
        match ocr_callback.ocr_cell(&cell_image, cell, dpi) {
            Ok(mut spans) => {
                all_ocr_spans.append(&mut spans);
            }
            Err(_) => {
                // OCR failed for this cell - skip it
                // In production, we might want to emit a diagnostic
                continue;
            }
        }
    }

    // Merge vector and OCR spans using the bbox overlap rule
    merge_vector_and_ocr_spans(vector_spans, &all_ocr_spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_iou_identical() {
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [0.0, 0.0, 100.0, 100.0];
        assert!((compute_iou(a, b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_iou_no_overlap() {
        let a = [0.0, 0.0, 10.0, 10.0];
        let b = [20.0, 20.0, 30.0, 30.0];
        assert_eq!(compute_iou(a, b), 0.0);
    }

    #[test]
    fn test_compute_iou_half_overlap() {
        // Two 100x100 squares, offset by 50 in X
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [50.0, 0.0, 150.0, 100.0];
        // Intersection: 50x100 = 5000
        // Union: 10000 + 10000 - 5000 = 15000
        // IoU = 5000 / 15000 = 1/3
        let iou = compute_iou(a, b);
        assert!((iou - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_iou_contained() {
        // Small box completely inside large box
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [25.0, 25.0, 75.0, 75.0];
        // Intersection = area of b = 50x50 = 2500
        // Union = area of a = 100x100 = 10000
        // IoU = 2500 / 10000 = 0.25
        let iou = compute_iou(a, b);
        assert!((iou - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_span_new() {
        let span = Span::new([10.0, 20.0, 50.0, 40.0], 0.9, SpanSource::Vector, "test".to_string());
        assert_eq!(span.bbox, [10.0, 20.0, 50.0, 40.0]);
        assert_eq!(span.confidence, 0.9);
        assert_eq!(span.source, SpanSource::Vector);
        assert_eq!(span.text, "test");
    }

    #[test]
    fn test_span_vector() {
        let span = Span::vector([0.0, 0.0, 100.0, 20.0], 0.95, "vector text".to_string());
        assert_eq!(span.source, SpanSource::Vector);
        assert_eq!(span.confidence, 0.95);
    }

    #[test]
    fn test_span_ocr() {
        let span = Span::ocr([0.0, 0.0, 100.0, 20.0], 0.85, "ocr text".to_string());
        assert_eq!(span.source, SpanSource::Ocr);
        assert_eq!(span.confidence, 0.85);
    }

    #[test]
    fn test_span_dimensions() {
        let span = Span::vector([10.0, 20.0, 60.0, 50.0], 1.0, "test".to_string());
        assert_eq!(span.width(), 50.0);
        assert_eq!(span.height(), 30.0);
        assert_eq!(span.area(), 1500.0);
    }

    #[test]
    fn test_merge_no_overlap() {
        let vector = vec![
            Span::vector([0.0, 0.0, 10.0, 10.0], 0.9, "vector".to_string()),
        ];
        let ocr = vec![
            Span::ocr([20.0, 20.0, 30.0, 30.0], 0.8, "ocr".to_string()),
        ];

        let result = merge_vector_and_ocr_spans(&vector, &ocr);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_merge_iou_06_vector_kept() {
        // IoU = 0.6 > 0.5, vector confidence >= 0.5 -> vector kept, OCR dropped
        let vector = vec![
            Span::vector([0.0, 0.0, 100.0, 100.0], 0.9, "vector text".to_string()),
        ];
        let ocr = vec![
            // OCR overlaps by 60%: intersection 60x100, union (10000 + 10000 - 6000) = 14000
            // bbox [40, 0, 100, 100] overlaps [0, 0, 100, 100] by 60x100
            Span::ocr([40.0, 0.0, 100.0, 100.0], 0.7, "ocr text".to_string()),
        ];

        let result = merge_vector_and_ocr_spans(&vector, &ocr);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].source, SpanSource::Vector);
        assert_eq!(result[0].text, "vector text");
    }

    #[test]
    fn test_merge_iou_03_both_kept() {
        // IoU = 0.3 < 0.5 -> both kept
        let vector = vec![
            Span::vector([0.0, 0.0, 100.0, 100.0], 0.9, "vector".to_string()),
        ];
        let ocr = vec![
            // OCR overlaps by 30%: [70, 0, 100, 100] overlaps [0, 0, 100, 100] by 30x100
            Span::ocr([70.0, 0.0, 100.0, 100.0], 0.7, "ocr".to_string()),
        ];

        let result = merge_vector_and_ocr_spans(&vector, &ocr);
        assert_eq!(result.len(), 2);
        // Check that both spans are present
        assert!(result.iter().any(|s| s.source == SpanSource::Vector));
        assert!(result.iter().any(|s| s.source == SpanSource::Ocr));
    }

    #[test]
    fn test_merge_iou_06_low_vector_confidence_ocr_kept() {
        // IoU = 0.6 > 0.5, but vector confidence < 0.5 -> OCR kept
        let vector = vec![
            Span::vector([0.0, 0.0, 100.0, 100.0], 0.2, "bad vector".to_string()),
        ];
        let ocr = vec![
            Span::ocr([40.0, 0.0, 100.0, 100.0], 0.7, "ocr text".to_string()),
        ];

        let result = merge_vector_and_ocr_spans(&vector, &ocr);
        assert_eq!(result.len(), 2); // Both kept because vector confidence is low
        // Verify both are present
        assert!(result.iter().any(|s| s.source == SpanSource::Vector));
        assert!(result.iter().any(|s| s.source == SpanSource::Ocr));
    }

    #[test]
    fn test_merge_sorting() {
        let vector = vec![
            Span::vector([0.0, 100.0, 50.0, 120.0], 0.9, "top".to_string()),
            Span::vector([0.0, 0.0, 50.0, 20.0], 0.9, "bottom".to_string()),
        ];
        let ocr = vec![];

        let result = merge_vector_and_ocr_spans(&vector, &ocr);
        // Should be sorted by Y descending (top to bottom in PDF coordinates)
        assert_eq!(result[0].text, "top"); // Higher Y comes first
        assert_eq!(result[1].text, "bottom");
    }

    #[test]
    fn test_get_hybrid_cells_non_hybrid() {
        let classification = PageClassification::new(
            crate::classify::PageClass::Vector,
            0.9,
        );
        assert!(get_hybrid_cells(&classification).is_empty());
    }

    #[test]
    fn test_get_hybrid_cells_with_cells() {
        let mut cells = BTreeSet::new();
        cells.insert(16);
        cells.insert(17);
        cells.insert(18);

        let classification = PageClassification::hybrid(0.75, cells);
        let result = get_hybrid_cells(&classification);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].row, 2); // flat 16 = row 2, col 0
        assert_eq!(result[0].col, 0);
        assert_eq!(result[1].row, 2); // flat 17 = row 2, col 1
        assert_eq!(result[1].col, 1);
    }

    #[test]
    fn test_compute_cell_crops() {
        let mut cells = BTreeSet::new();
        cells.insert(0);  // row 0, col 0 (top-left)
        cells.insert(63); // row 7, col 7 (bottom-right)

        let classification = PageClassification::hybrid(0.75, cells);
        let crops = compute_cell_crops(&classification, 612.0, 792.0);

        assert_eq!(crops.len(), 2);

        // First cell: row 0, col 0 (top-left)
        assert_eq!(crops[0].row, 0);
        assert_eq!(crops[0].col, 0);
        // Cell width = 612 / 8 = 76.5
        // Cell height = 792 / 8 = 99
        // Top-left cell: x=[0, 76.5], y=[693, 792] (Y is flipped)
        assert!((crops[0].bbox[0] - 0.0).abs() < 0.1);
        assert!((crops[0].bbox[1] - 693.0).abs() < 0.1);
        assert!((crops[0].bbox[2] - 76.5).abs() < 0.1);
        assert!((crops[0].bbox[3] - 792.0).abs() < 0.1);

        // Second cell: row 7, col 7 (bottom-right)
        assert_eq!(crops[1].row, 7);
        assert_eq!(crops[1].col, 7);
        assert!((crops[1].bbox[0] - 535.5).abs() < 0.1); // 7 * 76.5
        assert!((crops[1].bbox[1] - 0.0).abs() < 0.1);
        assert!((crops[1].bbox[2] - 612.0).abs() < 0.1);
        assert!((crops[1].bbox[3] - 99.0).abs() < 0.1);
    }

    #[test]
    fn test_crop_cell_from_page() {
        // Create a simple 800x600 page image (white background)
        // Match page dimensions to the image size for this test
        let page_image = GrayImage::new(800, 600);

        // Page is 800x600 points (matching image), rendered at 72 DPI
        // 800 pt * 72 / 72 = 800 px wide
        // 600 pt * 72 / 72 = 600 px tall

        // Crop cell at row 0, col 0 (top-left)
        let cell = crop_cell_from_page(&page_image, 800.0, 600.0, CellIndex::new(0, 0), 72);

        // Cell should be 1/8 of page dimensions
        assert_eq!(cell.width(), 100); // 800 / 8
        assert_eq!(cell.height(), 75);  // 600 / 8
    }

    #[test]
    fn test_merge_reading_order() {
        let vector = vec![
            Span::vector([0.0, 50.0, 50.0, 70.0], 0.9, "middle".to_string()),
            Span::vector([0.0, 100.0, 50.0, 120.0], 0.9, "top".to_string()),
            Span::vector([0.0, 0.0, 50.0, 20.0], 0.9, "bottom".to_string()),
        ];

        let result = merge_vector_and_ocr_spans(&vector, &[]);

        // Should be sorted: top, middle, bottom (descending Y)
        assert_eq!(result[0].text, "top");
        assert_eq!(result[1].text, "middle");
        assert_eq!(result[2].text, "bottom");
    }

    #[test]
    fn test_merge_multiple_ocr_spans() {
        let vector = vec![
            Span::vector([0.0, 0.0, 100.0, 100.0], 0.9, "vector".to_string()),
        ];
        let ocr = vec![
            Span::ocr([200.0, 0.0, 300.0, 100.0], 0.8, "ocr1".to_string()),
            Span::ocr([400.0, 0.0, 500.0, 100.0], 0.8, "ocr2".to_string()),
        ];

        let result = merge_vector_and_ocr_spans(&vector, &ocr);
        assert_eq!(result.len(), 3); // All three spans, no overlap
    }

    #[test]
    fn test_span_source_equality() {
        assert_eq!(SpanSource::Vector, SpanSource::Vector);
        assert_eq!(SpanSource::Ocr, SpanSource::Ocr);
        assert_ne!(SpanSource::Vector, SpanSource::Ocr);
    }

    // ============ Hybrid Page Processing Tests (Phase 5.2.4) ============

    #[test]
    fn test_process_hybrid_page_ocr_only_on_scanned_cells() {
        // Critical test: Hybrid page with text header (top 2 rows) + scanned body (bottom 6 rows)
        // Verify OCR runs only on bottom 6 rows, not on entire page

        // Create a mock classification with hybrid cells (bottom 6 rows = rows 2-7)
        let mut cells = BTreeSet::new();
        for row in 2..8 {
            for col in 0..8 {
                cells.insert(CellIndex::new(row, col).flat());
            }
        }
        let classification = PageClassification::hybrid(0.75, cells);

        // Create vector spans from the text header (top 2 rows)
        let vector_spans = vec![
            Span::vector([50.0, 700.0, 200.0, 720.0], 0.95, "Header Text".to_string()),
            Span::vector([50.0, 650.0, 200.0, 670.0], 0.95, "More Header".to_string()),
        ];

        // Create mock OCR callback that tracks call count
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mock_spans = vec![
            Span::ocr([50.0, 100.0, 200.0, 120.0], 0.8, "Scanned Text 1".to_string()),
            Span::ocr([50.0, 50.0, 200.0, 70.0], 0.8, "Scanned Text 2".to_string()),
        ];
        let mock_ocr = MockOcrCallback {
            call_count: call_count.clone(),
            output_spans: mock_spans,
        };

        // Create a simple page image (white background)
        let page_image = GrayImage::new(612, 792);

        // Process the hybrid page
        let result = process_hybrid_page(
            &page_image,
            612.0,
            792.0,
            &classification,
            &vector_spans,
            72,
            &mock_ocr,
        );

        // Verify OCR was called exactly 48 times (6 rows * 8 cols)
        // NOT 64 times (full page)
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 48,
            "OCR should run only on scanned cells (48), not entire page (64)");

        // Verify result contains both vector and OCR spans
        assert!(result.iter().any(|s| s.source == SpanSource::Vector));
        assert!(result.iter().any(|s| s.source == SpanSource::Ocr));

        // Verify vector spans are present
        assert!(result.iter().any(|s| s.text == "Header Text"));
        assert!(result.iter().any(|s| s.text == "More Header"));

        // Verify OCR spans are present (each cell produces the same mock output)
        assert!(result.iter().filter(|s| s.text == "Scanned Text 1").count() >= 1);
    }

    #[test]
    fn test_process_hybrid_page_no_duplicate_text_from_overlap() {
        // Critical test: End-to-end hybrid extraction produces no duplicate text
        // from overlapping vector + OCR regions

        // Create a classification with one scanned cell
        let mut cells = BTreeSet::new();
        cells.insert(CellIndex::new(7, 0).flat()); // Bottom-left cell
        let classification = PageClassification::hybrid(0.75, cells);

        // Create vector spans that overlap with OCR region
        let vector_spans = vec![
            Span::vector([50.0, 50.0, 150.0, 70.0], 0.9, "Vector Text".to_string()),
        ];

        // Create mock OCR that produces overlapping text (IoU > 0.5)
        // OCR bbox [40, 40, 160, 80] overlaps vector bbox [50, 50, 150, 70]
        // Intersection = [50, 50, 150, 70] = 100 * 20 = 2000
        // Union = (120*40) + (100*20) - 2000 = 4800 + 2000 - 2000 = 4800
        // IoU = 2000 / 4800 = 0.417 (not > 0.5, so both kept)
        // Let's create stronger overlap:
        // OCR bbox [45, 45, 155, 75] overlaps vector bbox [50, 50, 150, 70]
        // Intersection = [50, 50, 150, 70] = 100 * 20 = 2000
        // Union = (110*30) + (100*20) - 2000 = 3300 + 2000 - 2000 = 3300
        // IoU = 2000 / 3300 = 0.606 > 0.5
        let mock_spans = vec![
            Span::ocr([45.0, 45.0, 155.0, 75.0], 0.7, "OCR Text".to_string()),
        ];
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mock_ocr = MockOcrCallback {
            call_count,
            output_spans: mock_spans,
        };

        // Create a simple page image
        let page_image = GrayImage::new(612, 792);

        // Process the hybrid page
        let result = process_hybrid_page(
            &page_image,
            612.0,
            792.0,
            &classification,
            &vector_spans,
            72,
            &mock_ocr,
        );

        // With IoU > 0.5 and vector confidence >= 0.5, vector should win
        // Result should have only 1 span (the vector span)
        assert_eq!(result.len(), 1, "Should have only 1 span after merge (vector wins)");
        assert_eq!(result[0].source, SpanSource::Vector);
        assert_eq!(result[0].text, "Vector Text");
    }

    #[test]
    fn test_process_hybrid_page_low_vector_confidence_ocr_wins() {
        // Test that OCR is preferred when vector confidence is low (< 0.5)
        // even with IoU > 0.5

        let mut cells = BTreeSet::new();
        cells.insert(CellIndex::new(7, 0).flat());
        let classification = PageClassification::hybrid(0.75, cells);

        // Vector span with low confidence
        let vector_spans = vec![
            Span::vector([50.0, 50.0, 150.0, 70.0], 0.2, "Bad Vector".to_string()),
        ];

        // OCR span with high confidence, overlapping vector
        let mock_spans = vec![
            Span::ocr([45.0, 45.0, 155.0, 75.0], 0.7, "Good OCR".to_string()),
        ];
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mock_ocr = MockOcrCallback {
            call_count,
            output_spans: mock_spans,
        };

        let page_image = GrayImage::new(612, 792);

        let result = process_hybrid_page(
            &page_image,
            612.0,
            792.0,
            &classification,
            &vector_spans,
            72,
            &mock_ocr,
        );

        // With IoU > 0.5 but vector confidence < 0.5, OCR should be kept
        // Result should have 2 spans (both vector and OCR kept)
        assert_eq!(result.len(), 2, "Both vector and OCR should be kept when vector confidence is low");
        assert!(result.iter().any(|s| s.source == SpanSource::Vector));
        assert!(result.iter().any(|s| s.source == SpanSource::Ocr));
    }

    #[test]
    fn test_process_hybrid_page_non_hybrid_classification() {
        // Test that non-hybrid classifications return only vector spans

        let classification = PageClassification::new(PageClass::Vector, 0.9);
        let vector_spans = vec![
            Span::vector([50.0, 50.0, 150.0, 70.0], 0.9, "Vector Only".to_string()),
        ];

        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mock_ocr = MockOcrCallback {
            call_count: call_count.clone(),
            output_spans: vec![],
        };

        let page_image = GrayImage::new(612, 792);

        let result = process_hybrid_page(
            &page_image,
            612.0,
            792.0,
            &classification,
            &vector_spans,
            72,
            &mock_ocr,
        );

        // OCR should not be called for non-hybrid pages
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        // Result should have only vector spans
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].source, SpanSource::Vector);
        assert_eq!(result[0].text, "Vector Only");
    }

    #[test]
    fn test_process_hybrid_page_empty_hybrid_cells() {
        // Test hybrid classification with empty hybrid_cells

        let classification = PageClassification::hybrid(0.75, BTreeSet::new());
        let vector_spans = vec![
            Span::vector([50.0, 50.0, 150.0, 70.0], 0.9, "Vector".to_string()),
        ];

        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mock_ocr = MockOcrCallback {
            call_count: call_count.clone(),
            output_spans: vec![],
        };

        let page_image = GrayImage::new(612, 792);

        let result = process_hybrid_page(
            &page_image,
            612.0,
            792.0,
            &classification,
            &vector_spans,
            72,
            &mock_ocr,
        );

        // OCR should not be called when hybrid_cells is empty
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        // Result should have only vector spans
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].source, SpanSource::Vector);
    }
}
