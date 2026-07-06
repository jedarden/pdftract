//! Image-source dispatch for OCR preprocessing (Phase 5.3.2b).
//!
//! This module implements the dispatch policy that selects the binarization
//! algorithm per image based on the PDF filter chain from Phase 1.5.
//!
//! # Dispatch Policy
//!
//! | First Filter     | ImageSource    | BinarizerKind | Rationale                           |
//! |------------------|----------------|---------------|-------------------------------------|
//! | DCTDecode        | PhysicalScan   | Sauvola       | JPEG scans need local adaptive      |
//! | FlateDecode      | DigitalOrigin  | Otsu          | Lossless = digital origin            |
//! | JBIG2Decode      | Jbig2          | Skip          | Already binary                      |
//! | Other/Unknown    | PhysicalScan   | Sauvola       | Conservative default                |
//!
//! # Why this matters
//!
//! - **Sauvola** is slower but adapts to local lighting (good for physical scans
//!   where one corner may be darker than another).
//! - **Otsu** is faster but assumes globally consistent illumination (good for
//!   digitally-rendered images).
//! - **JBIG2** is already binary; binarizing again is wasteful and potentially
//!   destructive.
//!
//! # Per-image dispatch
//!
//! The dispatch decision is **per-image** (per Phase 1.5 image XObject), not
//! per-page. A single page may contain multiple images each with different filter
//! chains.

use crate::dpi::Pdf1Filter;

/// Image source type for preprocessing.
///
/// This enum represents the origin of an image in a PDF, determined from the
/// filter chain on the image XObject (Phase 1.5 filter inventory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSource {
    /// Physical scan (e.g., from a scanner).
    ///
    /// Typical indicator: DCTDecode (JPEG) filter.
    PhysicalScan,
    /// Digital-origin PDF (e.g., exported from software).
    ///
    /// Typical indicator: FlateDecode (lossless) filter.
    DigitalOrigin,
    /// JBIG2-encoded image (already binary).
    ///
    /// Indicator: JBIG2Decode filter.
    Jbig2,
}

impl ImageSource {
    /// Check if this is a JBIG2 image.
    #[inline]
    pub fn is_jbig2(self) -> bool {
        matches!(self, ImageSource::Jbig2)
    }

    /// Check if this is a digital-origin image.
    #[inline]
    pub fn is_digital(self) -> bool {
        matches!(self, ImageSource::DigitalOrigin)
    }

    /// Check if this is a physical scan.
    #[inline]
    pub fn is_physical_scan(self) -> bool {
        matches!(self, ImageSource::PhysicalScan)
    }
}

/// Binarization algorithm kind.
///
/// Represents the binarization strategy to apply to an image based on its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinarizerKind {
    /// Sauvola local adaptive thresholding.
    ///
    /// Used for physical scans where lighting may be uneven across the page.
    Sauvola,
    /// Otsu global thresholding.
    ///
    /// Used for digital-origin images with globally consistent illumination.
    Otsu,
    /// Skip binarization.
    ///
    /// Used for JBIG2 images that are already binary.
    Skip,
}

impl BinarizerKind {
    /// Check if this binarizer should be skipped (no binarization step).
    #[inline]
    pub fn is_skip(self) -> bool {
        matches!(self, BinarizerKind::Skip)
    }
}

/// Determine the image source from a filter chain.
///
/// This function inspects the **first filter** in the filter chain and maps it
/// to an `ImageSource`. The first filter is the most significant indicator:
/// - DCTDecode (JPEG) → typical physical scan
/// - FlateDecode (lossless) → typical digital origin
/// - JBIG2Decode → already binary
///
/// # Arguments
///
/// * `filters` - Slice of filters in the filter chain (first filter is primary)
///
/// # Returns
///
/// The `ImageSource` determined from the first filter, or `PhysicalScan` as the
/// conservative default for unknown filter chains.
///
/// # Examples
///
/// ```
/// use pdftract_core::ocr::preprocessing::dispatch::{image_source_from_filters, ImageSource};
/// use pdftract_core::dpi::Pdf1Filter;
///
/// // JPEG scan → PhysicalScan
/// let filters = vec![Pdf1Filter::DctDecode];
/// assert_eq!(image_source_from_filters(&filters), ImageSource::PhysicalScan);
///
/// // Lossless digital → DigitalOrigin
/// let filters = vec![Pdf1Filter::FlateDecode];
/// assert_eq!(image_source_from_filters(&filters), ImageSource::DigitalOrigin);
///
/// // JBIG2 → Jbig2
/// let filters = vec![Pdf1Filter::Jbig2Decode];
/// assert_eq!(image_source_from_filters(&filters), ImageSource::Jbig2);
///
/// // Unknown → PhysicalScan (conservative default)
/// let filters = vec![Pdf1Filter::Unknown("Crypt".to_string())];
/// assert_eq!(image_source_from_filters(&filters), ImageSource::PhysicalScan);
///
/// // Empty → PhysicalScan (default)
/// let filters: Vec<Pdf1Filter> = vec![];
/// assert_eq!(image_source_from_filters(&filters), ImageSource::PhysicalScan);
/// ```
pub fn image_source_from_filters(filters: &[Pdf1Filter]) -> ImageSource {
    match filters.first() {
        Some(Pdf1Filter::Jbig2Decode) => ImageSource::Jbig2,
        Some(Pdf1Filter::DctDecode) => ImageSource::PhysicalScan,
        Some(Pdf1Filter::FlateDecode) => ImageSource::DigitalOrigin,
        // Unknown, exotic, or empty filter chains default to PhysicalScan
        // (conservative: Sauvola is safer for unknown sources than skipping)
        _ => ImageSource::PhysicalScan,
    }
}

/// Select the binarization algorithm based on image source.
///
/// This is the core dispatch function that maps `ImageSource` to `BinarizerKind`:
/// - PhysicalScan → Sauvola (local adaptive, handles uneven lighting)
/// - DigitalOrigin → Otsu (global, faster for uniform lighting)
/// - Jbig2 → Skip (already binary)
///
/// # Arguments
///
/// * `source` - The image source type
///
/// # Returns
///
/// The `BinarizerKind` to use for this image.
///
/// # Examples
///
/// ```
/// use pdftract_core::ocr::preprocessing::dispatch::{select_binarizer, ImageSource, BinarizerKind};
///
/// assert_eq!(select_binarizer(ImageSource::PhysicalScan), BinarizerKind::Sauvola);
/// assert_eq!(select_binarizer(ImageSource::DigitalOrigin), BinarizerKind::Otsu);
/// assert_eq!(select_binarizer(ImageSource::Jbig2), BinarizerKind::Skip);
/// ```
pub fn select_binarizer(source: ImageSource) -> BinarizerKind {
    match source {
        ImageSource::PhysicalScan => BinarizerKind::Sauvola,
        ImageSource::DigitalOrigin => BinarizerKind::Otsu,
        ImageSource::Jbig2 => BinarizerKind::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_source_from_filters_dct_decode() {
        // DCTDecode (JPEG) → PhysicalScan
        let filters = vec![Pdf1Filter::DctDecode];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::PhysicalScan
        );
    }

    #[test]
    fn test_image_source_from_filters_flate_decode() {
        // FlateDecode (lossless) → DigitalOrigin
        let filters = vec![Pdf1Filter::FlateDecode];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::DigitalOrigin
        );
    }

    #[test]
    fn test_image_source_from_filters_jbig2_decode() {
        // JBIG2Decode → Jbig2
        let filters = vec![Pdf1Filter::Jbig2Decode];
        assert_eq!(image_source_from_filters(&filters), ImageSource::Jbig2);
    }

    #[test]
    fn test_image_source_from_filters_unknown() {
        // Unknown filter → PhysicalScan (conservative default)
        let filters = vec![Pdf1Filter::Unknown("Crypt".to_string())];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::PhysicalScan
        );
    }

    #[test]
    fn test_image_source_from_filters_empty() {
        // Empty filter chain → PhysicalScan (default)
        let filters: Vec<Pdf1Filter> = vec![];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::PhysicalScan
        );
    }

    #[test]
    fn test_image_source_from_filters_multi_filter_uses_first() {
        // Multi-filter chain: use FIRST filter only
        // DCTDecode as first → PhysicalScan (even if followed by JBIG2)
        let filters = vec![Pdf1Filter::DctDecode, Pdf1Filter::Jbig2Decode];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::PhysicalScan
        );

        // JBIG2 as first → Jbig2 (even if followed by FlateDecode)
        let filters = vec![Pdf1Filter::Jbig2Decode, Pdf1Filter::FlateDecode];
        assert_eq!(image_source_from_filters(&filters), ImageSource::Jbig2);
    }

    #[test]
    fn test_image_source_from_filters_other_known_filters() {
        // Other known filters default to PhysicalScan
        let filters = vec![Pdf1Filter::CcittFaxDecode];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::PhysicalScan
        );

        let filters = vec![Pdf1Filter::JpxDecode];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::PhysicalScan
        );

        let filters = vec![Pdf1Filter::LzwDecode];
        assert_eq!(
            image_source_from_filters(&filters),
            ImageSource::PhysicalScan
        );
    }

    #[test]
    fn test_select_binarizer_physical_scan() {
        assert_eq!(
            select_binarizer(ImageSource::PhysicalScan),
            BinarizerKind::Sauvola
        );
    }

    #[test]
    fn test_select_binarizer_digital_origin() {
        assert_eq!(
            select_binarizer(ImageSource::DigitalOrigin),
            BinarizerKind::Otsu
        );
    }

    #[test]
    fn test_select_binarizer_jbig2() {
        assert_eq!(select_binarizer(ImageSource::Jbig2), BinarizerKind::Skip);
    }

    #[test]
    fn test_image_source_is_jbig2() {
        assert!(ImageSource::Jbig2.is_jbig2());
        assert!(!ImageSource::PhysicalScan.is_jbig2());
        assert!(!ImageSource::DigitalOrigin.is_jbig2());
    }

    #[test]
    fn test_image_source_is_digital() {
        assert!(ImageSource::DigitalOrigin.is_digital());
        assert!(!ImageSource::PhysicalScan.is_digital());
        assert!(!ImageSource::Jbig2.is_digital());
    }

    #[test]
    fn test_image_source_is_physical_scan() {
        assert!(ImageSource::PhysicalScan.is_physical_scan());
        assert!(!ImageSource::DigitalOrigin.is_physical_scan());
        assert!(!ImageSource::Jbig2.is_physical_scan());
    }

    #[test]
    fn test_binarizer_kind_is_skip() {
        assert!(BinarizerKind::Skip.is_skip());
        assert!(!BinarizerKind::Sauvola.is_skip());
        assert!(!BinarizerKind::Otsu.is_skip());
    }

    #[test]
    fn test_dispatch_round_trip() {
        // Test full round-trip: filter chain → ImageSource → BinarizerKind

        // JPEG scan → PhysicalScan → Sauvola
        let filters = vec![Pdf1Filter::DctDecode];
        let source = image_source_from_filters(&filters);
        let binarizer = select_binarizer(source);
        assert_eq!(binarizer, BinarizerKind::Sauvola);

        // Lossless digital → DigitalOrigin → Otsu
        let filters = vec![Pdf1Filter::FlateDecode];
        let source = image_source_from_filters(&filters);
        let binarizer = select_binarizer(source);
        assert_eq!(binarizer, BinarizerKind::Otsu);

        // JBIG2 → Jbig2 → Skip
        let filters = vec![Pdf1Filter::Jbig2Decode];
        let source = image_source_from_filters(&filters);
        let binarizer = select_binarizer(source);
        assert_eq!(binarizer, BinarizerKind::Skip);
    }
}
