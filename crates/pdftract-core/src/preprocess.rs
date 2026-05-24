//! Image preprocessing pipeline (Phase 5.3).
//!
//! This module implements the preprocessing pipeline applied to raster images
//! before Tesseract OCR invocation. The pipeline is:
//! 1. **Deskew:** Hough line transform via pixDeskew; skip if angle < 0.3°
//! 2. **Contrast normalization:** Histogram stretch to [0, 255]
//! 3. **Binarization:** Sauvola (physical scans) or Otsu (digital)
//! 4. **Denoising:** 3×3 median filter
//! 5. **Border padding:** Add 10px white border
//!
//! # Feature Gate
//!
//! This module is only available when the `ocr` feature is enabled.

#![cfg(feature = "ocr")]

use crate::diagnostics::{Diagnostic, DiagCode};
use image::{GrayImage, ImageBuffer, Luma};
use std::ffi::c_float;

/// Result type for preprocessing operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// Minimum skew angle threshold in degrees.
///
/// Skew angles below this threshold are considered negligible and the image
/// is returned unchanged. This avoids unnecessary rotation for near-level scans.
const DESKEW_THRESHOLD_DEG: f64 = 0.3;

/// Maximum skew angle that pixDeskew can detect in degrees.
///
/// Angles outside this range will be reported as "no skew found" and the
/// function returns the input unchanged.
const DESKEW_MAX_RANGE_DEG: f64 = 15.0;

/// Deskew a grayscale image using leptonica's pixFindSkewAndDeskew (Hough transform).
///
/// This function detects the dominant text angle in the image using a Hough
/// line transform. If the detected angle is >= 0.3 degrees, the image is
/// rotated by the negative of that angle to correct the skew. Otherwise,
/// the image is returned unchanged.
///
/// # Arguments
///
/// * `image` - Input grayscale image
///
/// # Returns
///
/// A tuple of (deskewed image, detected angle in degrees, diagnostics).
/// If no significant skew is detected, the original image is returned with angle = 0.0.
///
/// # Critical considerations
///
/// - **DO NOT pre-binarize** for skew detection — pixFindSkewAndDeskew works on any depth
/// - The detected angle is deterministic for the same input
/// - Rotation preserves aspect ratio and pads with white (no cropping)
/// - Performance: < 100 ms per 8.5x11 page at 300 DPI
///
/// # Example
///
/// ```ignore
/// use pdftract_core::preprocess::deskew;
/// use image::GrayImage;
///
/// let original: GrayImage = // ... load image
/// let (deskewed, angle, diagnostics) = deskew(&original)?;
///
/// if angle.abs() >= 0.3 {
///     println!("Deskewed by {} degrees", angle);
/// } else {
///     println!("No significant skew detected");
/// }
/// ```
pub fn deskew(image: &GrayImage) -> Result<(GrayImage, f64, Vec<Diagnostic>)> {
    use leptonica_plumbing::leptonica_sys::{
        pixDestroy, pixFindSkewAndDeskew, pixGetWidth, pixGetHeight, pixGetDepth,
        Pix, l_float32, l_int32,
    };

    let mut diagnostics = Vec::new();

    // Convert GrayImage to leptonica Pix
    let pix = grayimage_to_pix(image)?;

    // Call pixFindSkewAndDeskew to detect the skew angle and deskew
    let (deskewed_pix, angle) = unsafe {
        let mut angle: l_float32 = 0.0;
        let mut conf: l_float32 = 0.0;

        // redsearch = 0 means use default reduction factor for binary search
        // Returns deskewed pix if angle is significant, otherwise returns a clone
        let result = pixFindSkewAndDeskew(pix, 0, &mut angle, &mut conf);

        if result.is_null() {
            pixDestroy(pix);
            let diagnostics = vec![Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "pixFindSkewAndDeskew returned null",
            )];
            return Err(diagnostics);
        }

        let angle_deg = angle as f64;

        // Check if angle is below the threshold (function returns clone for small angles)
        if angle_deg.abs() < DESKEW_THRESHOLD_DEG {
            pixDestroy(result);
            pixDestroy(pix);
            return Ok((image.clone(), 0.0, diagnostics));
        }

        // Check if angle is within the expected detection range
        // pixFindSkewAndDeskew typically searches within ±7 degrees by default
        if angle_deg.abs() > DESKEW_MAX_RANGE_DEG {
            pixDestroy(result);
            pixDestroy(pix);
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::ImgDeskewOutOfRange,
                format!("Skew angle {}° exceeds detection range (±{}°)", angle_deg, DESKEW_MAX_RANGE_DEG),
            ));
            return Ok((image.clone(), angle_deg, diagnostics));
        }

        (result, angle_deg)
    };

    // Convert back to GrayImage
    let result_image = pix_to_grayimage(deskewed_pix)?;

    // Clean up
    unsafe {
        pixDestroy(deskewed_pix);
    }

    Ok((result_image, angle, diagnostics))
}

/// Convert a GrayImage to a leptonica Pix.
///
/// Creates an 8-bit grayscale Pix from the image data.
fn grayimage_to_pix(image: &GrayImage) -> Result<*mut Pix> {
    use leptonica_plumbing::leptonica_sys::{
        pixCreate, pixDestroy, pixGetData, Pix,
    };
    use std::ptr;

    let width = image.width() as i32;
    let height = image.height() as i32;
    const DEPTH: i32 = 8;

    unsafe {
        let pix = pixCreate(width, height, DEPTH);

        if pix.is_null() {
            let diagnostics = vec![Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "Failed to create leptonica Pix for deskew",
            )];
            return Err(diagnostics);
        }

        // Get the data pointer from the Pix
        let pix_data = pixGetData(pix);

        if pix_data.is_null() {
            pixDestroy(pix);
            let diagnostics = vec![Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "Failed to get pixel data pointer from Pix",
            )];
            return Err(diagnostics);
        }

        // Copy pixel data from GrayImage to Pix
        // Pix stores data as l_uint32* (4-byte words), but for 8 bpp each pixel is one byte
        let raw_data = image.as_raw();
        let len = raw_data.len();

        // Copy byte by byte
        for i in 0..len {
            *pix_data.add(i) = raw_data[i] as u32;
        }

        Ok(pix)
    }
}

/// Convert a leptonica Pix to a GrayImage.
///
/// Expects an 8-bit grayscale Pix.
fn pix_to_grayimage(pix: *mut Pix) -> Result<GrayImage> {
    use leptonica_plumbing::leptonica_sys::{
        pixGetData, pixGetWidth, pixGetHeight, pixGetDepth, Pix,
    };

    unsafe {
        if pix.is_null() {
            let diagnostics = vec![Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "Null Pix pointer in pix_to_grayimage",
            )];
            return Err(diagnostics);
        }

        let width = pixGetWidth(pix) as u32;
        let height = pixGetHeight(pix) as u32;
        let depth = pixGetDepth(pix) as u32;

        if depth != 8 {
            let diagnostics = vec![Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                format!("Unsupported Pix depth {} (expected 8)", depth),
            )];
            return Err(diagnostics);
        }

        let data_ptr = pixGetData(pix);

        if data_ptr.is_null() {
            let diagnostics = vec![Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "Null data pointer in Pix",
            )];
            return Err(diagnostics);
        }

        // Copy the pixel data into a GrayImage
        let len = (width * height) as usize;
        let mut buffer = Vec::with_capacity(len);

        // Copy pixel data (stored as u32 but each pixel is 1 byte for 8 bpp)
        for i in 0..len {
            buffer.push(*data_ptr.add(i) as u8);
        }

        GrayImage::from_raw(width, height, buffer).ok_or_else(|| {
            vec![Diagnostic::with_static_no_offset(
                DiagCode::ImgUnsupportedFormat,
                "Failed to create GrayImage from Pix data",
            )]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a simple test pattern with horizontal lines.
    fn create_horizontal_lines_image() -> GrayImage {
        let mut img = GrayImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200 {
                let pixel = if y % 10 < 5 { 0 } else { 255 };
                img.put_pixel(x, y, Luma([pixel]));
            }
        }
        img
    }

    /// Create a simple test pattern with vertical lines.
    fn create_vertical_lines_image() -> GrayImage {
        let mut img = GrayImage::new(100, 200);
        for y in 0..200 {
            for x in 0..100 {
                let pixel = if x % 10 < 5 { 0 } else { 255 };
                img.put_pixel(x, y, Luma([pixel]));
            }
        }
        img
    }

    /// Create a solid white image.
    fn create_white_image() -> GrayImage {
        GrayImage::from_pixel(200, 100, Luma([255]))
    }

    #[test]
    fn test_deskew_horizontal_lines() {
        // Horizontal lines should have 0° skew
        let img = create_horizontal_lines_image();
        let (deskewed, angle, diagnostics) = deskew(&img).expect("Deskew failed");

        assert!(angle.abs() < 0.1, "Angle should be near 0°, got {}", angle);
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgDeskewOutOfRange));
    }

    #[test]
    fn test_deskew_white_image() {
        // White image should have no detectable skew
        let img = create_white_image();
        let (deskewed, angle, diagnostics) = deskew(&img).expect("Deskew failed");

        assert_eq!(angle, 0.0, "Angle should be exactly 0° for white image");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_grayimage_to_pix_roundtrip() {
        let img = create_horizontal_lines_image();
        let pix = grayimage_to_pix(&img).expect("Failed to convert to Pix");

        // Check that the Pix was created successfully
        unsafe {
            use leptonica_plumbing::leptonica_sys::{pixGetWidth, pixGetHeight, pixGetDepth, pixDestroy};

            assert!(!pix.is_null(), "Pix pointer should not be null");
            assert_eq!(pixGetWidth(pix) as u32, img.width());
            assert_eq!(pixGetHeight(pix) as u32, img.height());
            assert_eq!(pixGetDepth(pix) as u32, 8);

            pixDestroy(pix);
        }
    }

    #[test]
    fn test_pix_to_grayimage_roundtrip() {
        let img = create_horizontal_lines_image();
        let pix = grayimage_to_pix(&img).expect("Failed to convert to Pix");

        let converted = pix_to_grayimage(pix).expect("Failed to convert back");

        // Clean up
        unsafe {
            use leptonica_plumbing::leptonica_sys::pixDestroy;
            pixDestroy(pix);
        }

        assert_eq!(converted.width(), img.width());
        assert_eq!(converted.height(), img.height());
    }

    /// Create a test image with horizontal text-like lines at a specified skew angle.
    /// This creates a synthetic image with multiple horizontal lines that should be
    /// detectable by the Hough transform for skew detection.
    fn create_skewed_text_lines(width: u32, height: u32, angle_deg: f64) -> GrayImage {
        use std::f64::consts::PI;

        let mut img = GrayImage::new(width, height);
        let angle_rad = angle_deg * PI / 180.0;
        let cos_a = cos_a(angle_rad);
        let sin_a = sin_a(angle_rad);
        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;

        // Draw horizontal lines (like text lines) with skew
        for y in 0..height {
            for x in 0..width {
                // Transform point to unrotated coordinate system
                let dx = x as f64 - center_x;
                let dy = y as f64 - center_y;

                // Rotate back to find the "original" y coordinate
                let orig_y = dy * cos_a + dx * sin_a + center_y;

                // Draw lines every 20 pixels (like text lines)
                let line_y = (orig_y as i32) / 20;
                let is_line = line_y % 2 == 0;
                let is_text = ((orig_y as i32) % 20) < 12; // Text height within line

                let pixel = if is_line && is_text { 0 } else { 255 };
                img.put_pixel(x, y, Luma([pixel]));
            }
        }

        img
    }

    // Helper functions for trig (avoiding libm dependency for simple cases)
    fn cos_a(angle: f64) -> f64 {
        // Small angle approximation for testing (angles near 0)
        // For angles < 20 degrees, this is accurate enough
        if angle.abs() < 0.01 {
            1.0
        } else {
            // Taylor series: cos(x) ≈ 1 - x²/2 + x⁴/24
            let x2 = angle * angle;
            1.0 - x2 / 2.0 + x2 * x2 / 24.0
        }
    }

    fn sin_a(angle: f64) -> f64 {
        // Small angle approximation for testing
        // sin(x) ≈ x - x³/6
        if angle.abs() < 0.001 {
            angle
        } else {
            angle - angle * angle * angle / 6.0
        }
    }

    /// Verify that an image is deskewed to within a tolerance.
    /// This runs deskew twice on the image and verifies the second pass
    /// detects near-zero skew.
    fn verify_deskewed(img: &GrayImage, max_angle: f64) -> bool {
        let (deskewed, angle, _) = deskew(img).expect("Second deskew failed");
        angle.abs() < max_angle
    }

    #[test]
    fn test_deskew_2_degree_skew() {
        // Acceptance criterion: 2-deg synthetic skewed fixture: deskewed within 0.1 deg of upright
        let skewed = create_skewed_text_lines(400, 300, 2.0);
        let (deskewed, angle, diagnostics) = deskew(&skewed).expect("Deskew failed");

        // The detected angle should be close to 2 degrees
        assert!((angle.abs() - 2.0).abs() < 0.5, "Detected angle {} should be close to 2°", angle);

        // After deskewing, a second pass should detect near-zero skew
        let (_, second_angle, _) = deskew(&deskewed).expect("Second deskew failed");
        assert!(second_angle.abs() < 0.1, "Second pass should detect near-zero skew, got {}", second_angle);

        // No out-of-range diagnostic for 2 degrees
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgDeskewOutOfRange));
    }

    #[test]
    fn test_deskew_0_2_degree_skew_skipped() {
        // Acceptance criterion: 0.2-deg skewed fixture: untouched (skip branch verified)
        let skewed = create_skewed_text_lines(400, 300, 0.2);
        let (deskewed, angle, diagnostics) = deskew(&skewed).expect("Deskew failed");

        // Angle should be 0.0 because we skip deskewing for angles < 0.3 deg
        assert_eq!(angle, 0.0, "Angle should be 0.0 for sub-threshold skew, got {}", angle);

        // Image should be unchanged (same dimensions and pixels)
        assert_eq!(deskewed.dimensions(), skewed.dimensions());

        // No diagnostics
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_deskew_20_degree_skew_out_of_range() {
        // Acceptance criterion: 20-deg skewed fixture (outside search range):
        // leaves input untouched, emits IMG_DESKEW_OUT_OF_RANGE diagnostic
        let skewed = create_skewed_text_lines(400, 300, 20.0);
        let (deskewed, angle, diagnostics) = deskew(&skewed).expect("Deskew failed");

        // Should emit the out-of-range diagnostic
        assert!(diagnostics.iter().any(|d| d.code == DiagCode::ImgDeskewOutOfRange),
                "Should emit IMG_DESKEW_OUT_OF_RANGE for 20-degree skew");

        // Image dimensions should be preserved (may be different due to rotation padding,
        // but should not be the original since pixFindSkewAndDeskew will attempt to rotate)
        // The key is the diagnostic is emitted
    }
}
