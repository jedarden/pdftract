//! DPI selection logic for OCR rendering (Phase 5.2.3).
//!
//! This module implements the DPI selector that picks the rendering DPI per page
//! from font-size signals (Phase 4 spans) plus image-filter signals (Phase 1.5).
//!
//! # DPI Selection Table
//!
//! | Signal                     | DPI | Rationale                              |
//! |----------------------------|-----|----------------------------------------|
//! | JBIG2Decode filter present | 200 | Already binary; higher DPI wastes CPU  |
//! | Median font_size < 7.0 pt  | 400 | Fine print needs higher resolution     |
//! | Median font_size ≥ 7.0 pt | 300 | Standard body text sweet spot          |
//! | No font signals            | 300 | Default for scanned pages              |
//! | Override set               | *   | User-specified DPI overrides all signals |
//!
//! # Why DPI matters for OCR
//!
//! DPI is the single biggest correctness lever for OCR. 300 DPI is the sweet spot
//! for 10pt body text; below that, character recognition WER spikes. Fine-print
//! (legal documents, footnotes) needs 400 DPI to avoid character collisions. JBIG2
//! images are already binary at scan resolution; rendering at 300 DPI throws away
//! no data but wastes ~9x the CPU.

use crate::classify::PageContext;
use crate::options::ExtractionOptions;

/// PDF 1.x filter name for image streams.
///
/// These are the filter names that appear in PDF stream dictionaries
/// (e.g., `/Filter /DCTDecode` or `/Filter [/FlateDecode /DCTDecode]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pdf1Filter {
    /// JBIG2 bilevel image compression (already binary)
    Jbig2Decode,
    /// DCT (JPEG) compression
    DctDecode,
    /// JPX (JPEG 2000) compression
    JpxDecode,
    /// CCITT fax compression
    CcittFaxDecode,
    /// Flate (zlib) compression
    FlateDecode,
    /// LZW compression
    LzwDecode,
    /// Run-length encoding
    RunLengthDecode,
    /// ASCII85 encoding
    Ascii85Decode,
    /// ASCII hexadecimal encoding
    AsciiHexDecode,
    /// Unknown or unsupported filter
    Unknown(String),
}

impl Pdf1Filter {
    /// Parse a filter name from a PDF stream dictionary.
    ///
    /// Accepts both abbreviated and full names per PDF spec 7.4.2 Table 6.
    pub fn from_name(name: &str) -> Self {
        // Strip leading slash if present
        let name = name.strip_prefix('/').unwrap_or(name);

        match name {
            "JBIG2Decode" => Pdf1Filter::Jbig2Decode,
            "DCTDecode" | "DCT" => Pdf1Filter::DctDecode,
            "JPXDecode" => Pdf1Filter::JpxDecode,
            "CCITTFaxDecode" | "CCF" => Pdf1Filter::CcittFaxDecode,
            "FlateDecode" | "Fl" => Pdf1Filter::FlateDecode,
            "LZWDecode" | "LZW" => Pdf1Filter::LzwDecode,
            "RunLengthDecode" | "RL" => Pdf1Filter::RunLengthDecode,
            "ASCII85Decode" | "A85" => Pdf1Filter::Ascii85Decode,
            "ASCIIHexDecode" | "AHx" => Pdf1Filter::AsciiHexDecode,
            other => Pdf1Filter::Unknown(other.to_string()),
        }
    }

    /// Check if this filter indicates a JBIG2 image.
    #[inline]
    pub fn is_jbig2(&self) -> bool {
        matches!(self, Pdf1Filter::Jbig2Decode)
    }
}

/// Font size span from Phase 4 text assembly.
///
/// This represents a text element with its font size, used for DPI selection.
#[derive(Debug, Clone, Copy)]
pub struct FontSizeSpan {
    /// Font size in points (1/72 inch).
    pub font_size: f32,
}

impl FontSizeSpan {
    /// Create a new font size span.
    #[inline]
    pub fn new(font_size: f32) -> Self {
        Self { font_size }
    }

    /// Create a font size span, clamping to reasonable bounds.
    ///
    /// Font sizes outside [4.0, 72.0] are clamped to prevent outliers
    /// (drop caps, footers, corrupted data) from skewing the median.
    #[inline]
    pub fn new_clamped(font_size: f32) -> Self {
        Self {
            font_size: font_size.clamp(4.0, 72.0),
        }
    }
}

/// Select the DPI for rendering a page based on available signals.
///
/// This function implements the DPI selection algorithm:
/// 1. If override is set, use it
/// 2. If any JBIG2 filter is present, return 200
/// 3. If font size spans are available, compute median and select 300 or 400
/// 4. Default to 300
///
/// # Arguments
///
/// * `page` - Page context with classification metrics
/// * `image_filters` - List of filters from image XObjects on the page
/// * `font_sizes` - Optional list of font sizes from Phase 4 spans
/// * `options` - Extraction options with optional DPI override
///
/// # Returns
///
/// The DPI to use for rendering (always a valid u32).
///
/// # Examples
///
/// ```
/// use pdftract_core::dpi::{select_dpi, Pdf1Filter};
/// use pdftract_core::classify::PageContext;
/// use pdftract_core::options::ExtractionOptions;
///
/// let page = PageContext::new();
/// let filters = vec![Pdf1Filter::DctDecode];
/// let options = ExtractionOptions::default();
///
/// // Default: no JBIG2, no font data -> 300 DPI
/// let dpi = select_dpi(&page, &filters, None, &options);
/// assert_eq!(dpi, 300);
///
/// // JBIG2 present -> 200 DPI
/// let filters = vec![Pdf1Filter::Jbig2Decode];
/// let dpi = select_dpi(&page, &filters, None, &options);
/// assert_eq!(dpi, 200);
///
/// // Override takes precedence
/// let options = ExtractionOptions { ocr_dpi_override: Some(150), ..Default::default() };
/// let dpi = select_dpi(&page, &filters, None, &options);
/// assert_eq!(dpi, 150);
/// ```
pub fn select_dpi(
    _page: &PageContext,
    image_filters: &[Pdf1Filter],
    font_sizes: Option<&[f32]>,
    options: &ExtractionOptions,
) -> u32 {
    // Step 0: Check override first (highest priority)
    if let Some(override_dpi) = options.ocr_dpi_override {
        return override_dpi;
    }

    // Step 1: Check for JBIG2 filter
    for filter in image_filters {
        if filter.is_jbig2() {
            return 200;
        }
    }

    // Step 2: If font size spans available, compute median
    if let Some(sizes) = font_sizes {
        if !sizes.is_empty() {
            let median = compute_median_font_size(sizes);
            // Threshold from plan: < 7.0 pt -> 400 (fine print)
            if median < 7.0 {
                return 400;
            } else {
                return 300;
            }
        }
    }

    // Step 3: Default for scanned pages with no font signals
    300
}

/// Compute the median font size from a list of font sizes.
///
/// Uses linear-time median selection (nth_element) rather than full sorting
/// for performance on pages with many spans.
///
/// # Arguments
///
/// * `font_sizes` - Slice of font sizes in points
///
/// # Returns
///
/// The median font size in points.
fn compute_median_font_size(font_sizes: &[f32]) -> f32 {
    if font_sizes.is_empty() {
        return 10.0; // Default fallback
    }

    // Clamp font sizes to reasonable bounds to prevent outliers
    let mut clamped: Vec<f32> = font_sizes.iter().map(|&s| s.clamp(4.0, 72.0)).collect();

    // Use nth_element for O(n) median selection
    let len = clamped.len();
    let mid = len / 2;

    if len % 2 == 0 {
        // Even length: average of two middle elements
        let (left, median, _right) = clamped.select_nth_unstable_by(mid, |a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
        // Find the maximum of the left partition
        let max_left = left.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        (max_left + *median) / 2.0
    } else {
        // Odd length: middle element
        let (_left, median, _right) = clamped.select_nth_unstable_by(mid, |a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });
        *median
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf1_filter_from_name() {
        assert_eq!(
            Pdf1Filter::from_name("JBIG2Decode"),
            Pdf1Filter::Jbig2Decode
        );
        assert_eq!(
            Pdf1Filter::from_name("/JBIG2Decode"),
            Pdf1Filter::Jbig2Decode
        );
        assert_eq!(Pdf1Filter::from_name("DCTDecode"), Pdf1Filter::DctDecode);
        assert_eq!(Pdf1Filter::from_name("DCT"), Pdf1Filter::DctDecode);
        assert_eq!(Pdf1Filter::from_name("Fl"), Pdf1Filter::FlateDecode);
        assert_eq!(Pdf1Filter::from_name("CCF"), Pdf1Filter::CcittFaxDecode);
        assert_eq!(
            Pdf1Filter::from_name("UnknownFilter"),
            Pdf1Filter::Unknown("UnknownFilter".to_string())
        );
    }

    #[test]
    fn test_pdf1_filter_is_jbig2() {
        assert!(Pdf1Filter::Jbig2Decode.is_jbig2());
        assert!(!Pdf1Filter::DctDecode.is_jbig2());
        assert!(!Pdf1Filter::JpxDecode.is_jbig2());
        assert!(!Pdf1Filter::FlateDecode.is_jbig2());
    }

    #[test]
    fn test_font_size_span_new() {
        let span = FontSizeSpan::new(12.0);
        assert_eq!(span.font_size, 12.0);
    }

    #[test]
    fn test_font_size_span_new_clamped() {
        // Within bounds
        assert_eq!(FontSizeSpan::new_clamped(10.0).font_size, 10.0);
        // Below minimum
        assert_eq!(FontSizeSpan::new_clamped(2.0).font_size, 4.0);
        // Above maximum
        assert_eq!(FontSizeSpan::new_clamped(100.0).font_size, 72.0);
    }

    #[test]
    fn test_compute_median_font_size_empty() {
        let sizes: Vec<f32> = vec![];
        assert_eq!(compute_median_font_size(&sizes), 10.0);
    }

    #[test]
    fn test_compute_median_font_size_single() {
        let sizes = vec![10.0];
        assert_eq!(compute_median_font_size(&sizes), 10.0);
    }

    #[test]
    fn test_compute_median_font_size_odd() {
        let sizes = vec![6.0, 8.0, 10.0, 12.0, 14.0];
        assert_eq!(compute_median_font_size(&sizes), 10.0);
    }

    #[test]
    fn test_compute_median_font_size_even() {
        let sizes = vec![6.0, 8.0, 10.0, 12.0];
        assert_eq!(compute_median_font_size(&sizes), 9.0); // (8 + 10) / 2
    }

    #[test]
    fn test_compute_median_font_size_clamps_outliers() {
        // Drop cap (huge) and footer (tiny) should be clamped
        let sizes = vec![1.0, 8.0, 10.0, 12.0, 100.0];
        // After clamping: [4.0, 8.0, 10.0, 12.0, 72.0] -> median 10.0
        assert_eq!(compute_median_font_size(&sizes), 10.0);
    }

    #[test]
    fn test_select_dpi_default() {
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::DctDecode];
        let options = ExtractionOptions::default();

        let dpi = select_dpi(&page, &filters, None, &options);
        assert_eq!(dpi, 300);
    }

    #[test]
    fn test_select_dpi_jbig2() {
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::Jbig2Decode];
        let options = ExtractionOptions::default();

        let dpi = select_dpi(&page, &filters, None, &options);
        assert_eq!(dpi, 200);
    }

    #[test]
    fn test_select_dpi_mixed_filters_with_jbig2() {
        let page = PageContext::new();
        // Mixed page with JBIG2 + DCT should pick 200
        let filters = vec![Pdf1Filter::DctDecode, Pdf1Filter::Jbig2Decode];
        let options = ExtractionOptions::default();

        let dpi = select_dpi(&page, &filters, None, &options);
        assert_eq!(dpi, 200);
    }

    #[test]
    fn test_select_dpi_fine_print() {
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::DctDecode];
        let options = ExtractionOptions::default();

        // Legal document with lots of 6pt footnotes -> median < 7.0
        let font_sizes = vec![6.0, 6.5, 7.0, 8.0, 10.0]; // median 7.0
        let dpi = select_dpi(&page, &filters, Some(&font_sizes), &options);
        // median = 7.0, threshold is < 7.0, so should be 300
        assert_eq!(dpi, 300);

        // Actually below threshold
        let font_sizes = vec![5.5, 6.0, 6.5, 8.0, 10.0]; // median 6.5
        let dpi = select_dpi(&page, &filters, Some(&font_sizes), &options);
        assert_eq!(dpi, 400);
    }

    #[test]
    fn test_select_dpi_standard_textbook() {
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::DctDecode];
        let options = ExtractionOptions::default();

        // Standard textbook with 10pt body text
        let font_sizes = vec![10.0, 10.5, 11.0, 12.0, 14.0];
        let dpi = select_dpi(&page, &filters, Some(&font_sizes), &options);
        assert_eq!(dpi, 300);
    }

    #[test]
    fn test_select_dpi_override() {
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::Jbig2Decode];
        let options = ExtractionOptions {
            ocr_dpi_override: Some(150),
            ..Default::default()
        };

        // Override should take precedence over JBIG2
        let dpi = select_dpi(&page, &filters, None, &options);
        assert_eq!(dpi, 150);
    }

    #[test]
    fn test_select_dpi_empty_font_sizes() {
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::DctDecode];
        let options = ExtractionOptions::default();

        // Empty font sizes should fall back to default
        let font_sizes: Vec<f32> = vec![];
        let dpi = select_dpi(&page, &filters, Some(&font_sizes), &options);
        assert_eq!(dpi, 300);
    }

    #[test]
    fn test_select_dpi_integration_legal_document() {
        // Critical test: legal-document fixture (lots of 6pt footnotes) -> 400 DPI
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::DctDecode];
        let options = ExtractionOptions::default();

        // Legal document: mostly 10pt body, but many 6pt footnotes
        // With 30 footnotes vs 20 body text, median should be in fine-print range
        let mut font_sizes: Vec<f32> = (0..30).map(|_| 6.0).collect(); // footnotes
        font_sizes.extend((0..20).map(|_| 10.0)); // body text
                                                  // Sorted: 30x 6.0, then 20x 10.0 -> median is at index 25 (0-indexed)
                                                  // That's the 26th element, which is 6.0
        let dpi = select_dpi(&page, &filters, Some(&font_sizes), &options);
        assert_eq!(dpi, 400);
    }

    #[test]
    fn test_select_dpi_integration_textbook() {
        // Critical test: standard textbook -> 300 DPI
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::DctDecode];
        let options = ExtractionOptions::default();

        // Textbook: mostly 10-12pt body text
        let font_sizes: Vec<f32> = vec![10.0, 10.5, 11.0, 11.5, 12.0, 10.5, 11.0];
        let dpi = select_dpi(&page, &filters, Some(&font_sizes), &options);
        assert_eq!(dpi, 300);
    }

    #[test]
    fn test_select_dpi_integration_pure_jbig2() {
        // Critical test: pure JBIG2 fixture -> 200 DPI
        let page = PageContext::new();
        let filters = vec![Pdf1Filter::Jbig2Decode];
        let options = ExtractionOptions::default();

        let dpi = select_dpi(&page, &filters, None, &options);
        assert_eq!(dpi, 200);
    }
}
