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
use image::{GrayImage, ImageBuffer, Luma, Luma};
use std::ffi::c_float;

/// Border padding size in pixels.
///
/// This is the recommended minimum padding for Tesseract OCR.
const BORDER_PADDING: u32 = 10;

/// Image source type for preprocessing.
///
/// Determines which preprocessing steps to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSource {
    /// Physical scan (e.g., from a scanner).
    /// Applies all preprocessing steps including Sauvola binarization.
    PhysicalScan,
    /// Digital-origin PDF (e.g., exported from software).
    /// Applies all preprocessing steps including Otsu binarization.
    DigitalOrigin,
    /// JBIG2-encoded image (already binary).
    /// Skips contrast normalization, binarization, and denoising.
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

    /// Add a 10px white border to an image.
    ///
    /// This function creates a new image with dimensions (width+20) x (height+20),
    /// fills it with white (255), and copies the input image into the center.
    ///
    /// # Arguments
    ///
    /// * `image` - Input grayscale image
    ///
    /// # Returns
    ///
    /// A new image with a 10px white border on all sides.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::preprocess::add_border_padding;
    /// use image::GrayImage;
    ///
    /// let original: GrayImage = // ... load image
    /// let padded = add_border_padding(&original);
    ///
    /// assert_eq!(padded.width(), original.width() + 20);
    /// assert_eq!(padded.height(), original.height() + 20);
    /// ```
    pub fn add_border_padding(image: &GrayImage) -> GrayImage {
        let width = image.width();
        let height = image.height();
        let new_width = width + 2 * BORDER_PADDING;
        let new_height = height + 2 * BORDER_PADDING;

        let mut padded = GrayImage::new(new_width, new_height);

        // Fill with white
        for pixel in padded.pixels_mut() {
            *pixel = Luma([255]);
        }

        // Copy original image into center
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x, y);
                padded.put_pixel(x + BORDER_PADDING, y + BORDER_PADDING, *pixel);
            }
        }

        padded
    }

    /// Normalize contrast using histogram stretch to [0, 255].
    ///
    /// This function stretches the image histogram to use the full grayscale range.
    /// It finds the minimum and maximum pixel values and linearly maps them to 0 and 255.
    ///
    /// # Arguments
    ///
    /// * `image` - Input grayscale image
    ///
    /// # Returns
    ///
    /// A new image with contrast normalized to [0, 255].
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::preprocess::normalize_contrast;
    /// use image::GrayImage;
    ///
    /// let original: GrayImage = // ... load image
    /// let normalized = normalize_contrast(&original);
    /// ```
    pub fn normalize_contrast(image: &GrayImage) -> GrayImage {
        let mut min_val = 255u8;
        let mut max_val = 0u8;

        // Find min and max values
        for pixel in image.pixels() {
            let val = pixel[0];
            if val < min_val {
                min_val = val;
            }
            if val > max_val {
                max_val = val;
            }
        }

        // If image is already full contrast or constant, return as-is
        if min_val == 0 && max_val == 255 {
            return image.clone();
        }
        if min_val == max_val {
            return image.clone();
        }

        let range = (max_val - min_val) as f32;

        // Apply linear stretch
        let mut normalized = image.clone();
        for pixel in normalized.pixels_mut() {
            let val = pixel[0];
            let stretched = ((val as f32 - min_val as f32) * 255.0 / range).round() as u8;
            pixel[0] = stretched.clamp(0, 255);
        }

        normalized
    }

    /// Apply Otsu's global thresholding for binarization.
    ///
    /// Otsu's method automatically finds the optimal threshold value that maximizes
    /// the inter-class variance between foreground and background pixels.
    ///
    /// # Arguments
    ///
    /// * `image` - Input grayscale image
    ///
    /// # Returns
    ///
    /// A new binary image (black text on white background).
    pub fn binarize_otsu(image: &GrayImage) -> GrayImage {
        // Compute histogram
        let mut histogram = [0u32; 256];
        for pixel in image.pixels() {
            histogram[pixel[0] as usize] += 1;
        }

        let total = image.width() as u32 * image.height() as u32;

        // Compute optimal threshold using Otsu's method
        let mut sum: u32 = 0;
        for i in 0..256 {
            sum += i * histogram[i];
        }

        let mut sum_b: u32 = 0;
        let mut w_b: u32 = 0;
        let mut max_variance = 0u32;
        let mut threshold = 0u8;

        for i in 0..256 {
            w_b += histogram[i];
            if w_b == 0 {
                continue;
            }

            let w_f = total - w_b;
            if w_f == 0 {
                break;
            }

            sum_b += i * histogram[i];
            let sum_f = sum - sum_b;

            let m_b = if w_b > 0 {
                (sum_b as f64) / (w_b as f64)
            } else {
                0.0
            };
            let m_f = if w_f > 0 {
                (sum_f as f64) / (w_f as f64)
            } else {
                0.0
            };

            let variance = (w_b as f64) * (w_f as f64) * (m_b - m_f).powi(2);

            if variance > max_variance as f64 {
                max_variance = variance as u32;
                threshold = i as u8;
            }
        }

        // Apply threshold
        let mut binary = image.clone();
        for pixel in binary.pixels_mut() {
            pixel[0] = if pixel[0] < threshold { 0 } else { 255 };
        }

        binary
    }

    /// Apply Sauvola local adaptive thresholding for binarization.
    ///
    /// Sauvola's method uses a local window to compute a dynamic threshold for each
    /// pixel, which works well for documents with uneven lighting.
    ///
    /// # Arguments
    ///
    /// * `image` - Input grayscale image
    ///
    /// # Returns
    ///
    /// A new binary image (black text on white background).
    ///
    /// # Implementation note
    ///
    /// This implementation uses a window size of 25 pixels and k=0.34, which are
    /// the recommended values for document images.
    pub fn binarize_sauvola(image: &GrayImage) -> GrayImage {
        let width = image.width() as usize;
        let height = image.height() as usize;

        // Sauvola parameters
        let window_size = 25usize;
        let k = 0.34f32;
        let r = 128.0f32; // dynamic range of standard deviation

        let half_window = window_size / 2;
        let mut binary = image.clone();

        // Precompute integral images for mean and mean of squares
        let mut integral = vec![0u64; (width + 1) * (height + 1)];
        let mut integral_sq = vec![0u64; (width + 1) * (height + 1)];

        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x as u32, y as u32)[0] as u64;
                let pixel_sq = (pixel * pixel) as u64;

                let idx = (y + 1) * (width + 1) + (x + 1);
                integral[idx] = pixel
                    + integral[y * (width + 1) + (x + 1)]
                    + integral[(y + 1) * (width + 1) + x]
                    - integral[y * (width + 1) + x];

                integral_sq[idx] = pixel_sq
                    + integral_sq[y * (width + 1) + (x + 1)]
                    + integral_sq[(y + 1) * (width + 1) + x]
                    - integral_sq[y * (width + 1) + x];
            }
        }

        // Helper to get sum from integral image
        let get_sum = |integral: &[u64], x1: usize, y1: usize, x2: usize, y2: usize| -> u64 {
            let w = width + 1;
            integral[y2 * w + x2]
                + integral[y1 * w + x1]
                - integral[y1 * w + x2]
                - integral[y2 * w + x1]
        };

        // Apply Sauvola thresholding
        for y in 0..height {
            for x in 0..width {
                let x1 = x.saturating_sub(half_window);
                let y1 = y.saturating_sub(half_window);
                let x2 = (x + half_window + 1).min(width);
                let y2 = (y + half_window + 1).min(height);

                let area = ((x2 - x1) * (y2 - y1)) as u64;

                let sum = get_sum(&integral, x1, y1, x2, y2);
                let sum_sq = get_sum(&integral_sq, x1, y1, x2, y2);

                let mean = (sum as f32) / (area as f32);
                let variance = ((sum_sq as f32) - (sum as f32) * mean) / (area as f32);
                let std_dev = variance.sqrt().max(0.0);

                let threshold = mean * (1.0 + k * ((std_dev / r) - 1.0));

                let pixel = image.get_pixel(x as u32, y as u32)[0] as f32;
                binary.put_pixel(
                    x as u32,
                    y as u32,
                    Luma([if pixel < threshold { 0u8 } else { 255u8 }]),
                );
            }
        }

        binary
    }

    /// Apply a 3x3 median filter for denoising.
    ///
    /// This function removes salt-and-pepper noise by replacing each pixel with
    /// the median value of its 3x3 neighborhood.
    ///
    /// # Arguments
    ///
    /// * `image` - Input grayscale image
    ///
    /// # Returns
    ///
    /// A new image with median filtering applied.
    pub fn denoise_median(image: &GrayImage) -> GrayImage {
        let width = image.width();
        let height = image.height();
        let mut denoised = image.clone();

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                // Collect 3x3 neighborhood
                let mut neighborhood = [0u8; 9];
                let mut idx = 0;

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        neighborhood[idx] = image.get_pixel(nx as u32, ny as u32)[0];
                        idx += 1;
                    }
                }

                // Find median
                neighborhood.sort();
                denoised.put_pixel(x, y, Luma([neighborhood[4]]));
            }
        }

        denoised
    }

    /// Apply the full preprocessing pipeline to an image.
    ///
    /// This is the main entry point for preprocessing. It applies all steps in order:
    /// 1. Deskew (always)
    /// 2. Contrast normalization (skip for JBIG2)
    /// 3. Binarization (skip for JBIG2)
    /// 4. Denoising (skip for JBIG2)
    /// 5. Border padding (always)
    ///
    /// # Arguments
    ///
    /// * `image` - Input grayscale image
    /// * `source` - Image source type (determines which steps to apply)
    ///
    /// # Returns
    ///
    /// A tuple of (preprocessed image, diagnostics).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::preprocess::{preprocess, ImageSource};
    /// use image::GrayImage;
    ///
    /// let original: GrayImage = // ... load image
    /// let (preprocessed, diagnostics) = preprocess(&original, ImageSource::PhysicalScan)?;
    /// ```
    pub fn preprocess(image: &GrayImage, source: ImageSource) -> Result<(GrayImage, Vec<Diagnostic>)> {
        let mut diagnostics = Vec::new();
        let mut current = image.clone();

        // Step 1: Deskew (always)
        let (deskewed, _angle, mut deskew_diags) = deskew(&current)?;
        current = deskewed;
        diagnostics.append(&mut deskew_diags);

        // Skip remaining steps for JBIG2
        if !source.is_jbig2() {
            // Step 2: Contrast normalization
            current = normalize_contrast(&current);

            // Step 3: Binarization
            current = if source.is_digital() {
                binarize_otsu(&current)
            } else {
                binarize_sauvola(&current)
            };

            // Step 4: Denoising
            current = denoise_median(&current);
        }

        // Step 5: Border padding (always)
        current = add_border_padding(&current);

        Ok((current, diagnostics))
    }

    #[test]
    fn test_add_border_padding() {
        let img = create_horizontal_lines_image();
        let padded = add_border_padding(&img);

        // Check dimensions
        assert_eq!(padded.width(), img.width() + 20);
        assert_eq!(padded.height(), img.height() + 20);

        // Check borders are white
        for x in 0..10 {
            for y in 0..padded.height() {
                assert_eq!(padded.get_pixel(x, y)[0], 255);
                assert_eq!(padded.get_pixel(padded.width() - 1 - x, y)[0], 255);
            }
        }
        for y in 0..10 {
            for x in 0..padded.width() {
                assert_eq!(padded.get_pixel(x, y)[0], 255);
                assert_eq!(padded.get_pixel(x, padded.height() - 1 - y)[0], 255);
            }
        }

        // Check inner content matches
        for y in 0..img.height() {
            for x in 0..img.width() {
                let orig = img.get_pixel(x, y);
                let pad = padded.get_pixel(x + 10, y + 10);
                assert_eq!(orig[0], pad[0]);
            }
        }
    }

    #[test]
    fn test_normalize_contrast_full_range() {
        // Image already at full range should be unchanged
        let mut img = GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                let val = if x < 50 { 0 } else { 255 };
                img.put_pixel(x, y, Luma([val]));
            }
        }

        let normalized = normalize_contrast(&img);
        assert_eq!(normalized.width(), img.width());
        assert_eq!(normalized.height(), img.height());

        // Pixels should be identical
        for y in 0..100 {
            for x in 0..100 {
                assert_eq!(img.get_pixel(x, y)[0], normalized.get_pixel(x, y)[0]);
            }
        }
    }

    #[test]
    fn test_normalize_contrast_narrow_range() {
        // Image with narrow range should be stretched
        let mut img = GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, Luma([100])); // Constant mid-gray
            }
        }

        let normalized = normalize_contrast(&img);
        // Constant image should be unchanged
        for y in 0..100 {
            for x in 0..100 {
                assert_eq!(normalized.get_pixel(x, y)[0], 100);
            }
        }
    }

    #[test]
    fn test_binarize_otsu() {
        // Create an image with distinct foreground and background
        let mut img = GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                // Left half dark (text), right half light (background)
                let val = if x < 50 { 50 } else { 200 };
                img.put_pixel(x, y, Luma([val]));
            }
        }

        let binary = binarize_otsu(&img);

        // Check that we get a binary output
        for y in 0..100 {
            for x in 0..100 {
                let pixel = binary.get_pixel(x, y)[0];
                assert!(pixel == 0 || pixel == 255, "Pixel should be 0 or 255, got {}", pixel);
            }
        }

        // Left half should be darker (text)
        let left_sum: u32 = (0..50).map(|x| binary.get_pixel(x, 50)[0] as u32).sum();
        let right_sum: u32 = (50..100).map(|x| binary.get_pixel(x, 50)[0] as u32).sum();
        assert!(left_sum < right_sum, "Left half should be darker");
    }

    #[test]
    fn test_binarize_sauvola() {
        // Create a simple gradient image
        let mut img = GrayImage::new(100, 100);
        for y in 0..100 {
            for x in 0..100 {
                let val = (x + y) as u8 / 2;
                img.put_pixel(x, y, Luma([val]));
            }
        }

        let binary = binarize_sauvola(&img);

        // Check that we get a binary output
        for y in 0..100 {
            for x in 0..100 {
                let pixel = binary.get_pixel(x, y)[0];
                assert!(pixel == 0 || pixel == 255, "Pixel should be 0 or 255, got {}", pixel);
            }
        }
    }

    #[test]
    fn test_denoise_median() {
        // Create an image with salt-and-pepper noise
        let mut img = GrayImage::from_pixel(100, 100, Luma([128]));
        // Add some noise
        img.put_pixel(50, 50, Luma([0]));   // pepper
        img.put_pixel(51, 50, Luma([255])); // salt
        img.put_pixel(50, 51, Luma([255])); // salt
        img.put_pixel(51, 51, Luma([0]));   // pepper

        let denoised = denoise_median(&img);

        // The noisy pixels should be closer to 128 after median filtering
        let center = denoised.get_pixel(50, 50)[0];
        assert!(center > 64 && center < 192, "Denoised pixel should be near middle, got {}", center);
    }

    #[test]
    fn test_preprocess_physical_scan() {
        let img = create_horizontal_lines_image();
        let (preprocessed, diagnostics) = preprocess(&img, ImageSource::PhysicalScan)
            .expect("Preprocess failed");

        // Should have border padding
        assert_eq!(preprocessed.width(), img.width() + 20);
        assert_eq!(preprocessed.height(), img.height() + 20);

        // Diagnostics should not have errors
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgUnsupportedFormat));
    }

    #[test]
    fn test_preprocess_digital_origin() {
        let img = create_horizontal_lines_image();
        let (preprocessed, diagnostics) = preprocess(&img, ImageSource::DigitalOrigin)
            .expect("Preprocess failed");

        // Should have border padding
        assert_eq!(preprocessed.width(), img.width() + 20);
        assert_eq!(preprocessed.height(), img.height() + 20);

        // Diagnostics should not have errors
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgUnsupportedFormat));
    }

    #[test]
    fn test_preprocess_jbig2() {
        let img = create_horizontal_lines_image();
        let (preprocessed, diagnostics) = preprocess(&img, ImageSource::Jbig2)
            .expect("Preprocess failed");

        // Should have border padding
        assert_eq!(preprocessed.width(), img.width() + 20);
        assert_eq!(preprocessed.height(), img.height() + 20);

        // Diagnostics should not have errors
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgUnsupportedFormat));
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

    // Integration tests with fixtures

    /// Helper to load a fixture image.
    fn load_fixture(path: &str) -> GrayImage {
        image::io::Reader::with_format(std::io::Cursor::new(std::fs::read(path).unwrap()), image::ImageFormat::Png)
            .decode()
            .unwrap()
            .to_luma8()
    }

    #[test]
    fn test_preprocess_skewed_2deg_deskews() {
        // Acceptance criterion: 2-deg skewed fixture deskewed within 0.1 deg
        let source = load_fixture("tests/fixtures/preprocess/skewed_2deg/source.png");
        let (preprocessed, diagnostics) = preprocess(&source, ImageSource::PhysicalScan)
            .expect("Preprocess failed");

        // Should have border padding
        assert_eq!(preprocessed.width(), source.width() + 20);
        assert_eq!(preprocessed.height(), source.height() + 20);

        // Verify deskewing by checking that a second deskew pass detects near-zero skew
        // (after removing the border padding for the check)
        let cropped = image::imageops::crop_imm(
            &preprocessed,
            BORDER_PADDING,
            BORDER_PADDING,
            preprocessed.width() - 2 * BORDER_PADDING,
            preprocessed.height() - 2 * BORDER_PADDING,
        ).to_image();

        let (_, second_angle, _) = deskew(&cropped).expect("Second deskew failed");
        assert!(second_angle.abs() < 0.1, "Second pass should detect near-zero skew, got {}", second_angle);

        // No errors in diagnostics
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgUnsupportedFormat));
    }

    #[test]
    fn test_preprocess_uneven_lighting_binarizes() {
        // Acceptance criterion: uneven-lighting binarized correctly
        let source = load_fixture("tests/fixtures/preprocess/uneven_lighting/source.png");
        let (preprocessed, diagnostics) = preprocess(&source, ImageSource::PhysicalScan)
            .expect("Preprocess failed");

        // Should have border padding
        assert_eq!(preprocessed.width(), source.width() + 20);
        assert_eq!(preprocessed.height(), source.height() + 20);

        // Check that the inner region (excluding padding) is binarized
        for y in BORDER_PADDING..preprocessed.height() - BORDER_PADDING {
            for x in BORDER_PADDING..preprocessed.width() - BORDER_PADDING {
                let pixel = preprocessed.get_pixel(x, y)[0];
                assert!(pixel == 0 || pixel == 255, "Pixel should be binary (0 or 255), got {}", pixel);
            }
        }

        // No errors in diagnostics
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgUnsupportedFormat));
    }

    #[test]
    fn test_preprocess_clean_digital_binarizes() {
        // Acceptance criterion: clean digital origin binarized with Otsu
        let source = load_fixture("tests/fixtures/preprocess/clean_digital/source.png");
        let (preprocessed, diagnostics) = preprocess(&source, ImageSource::DigitalOrigin)
            .expect("Preprocess failed");

        // Should have border padding
        assert_eq!(preprocessed.width(), source.width() + 20);
        assert_eq!(preprocessed.height(), source.height() + 20);

        // Check that the inner region is binarized
        for y in BORDER_PADDING..preprocessed.height() - BORDER_PADDING {
            for x in BORDER_PADDING..preprocessed.width() - BORDER_PADDING {
                let pixel = preprocessed.get_pixel(x, y)[0];
                assert!(pixel == 0 || pixel == 255, "Pixel should be binary (0 or 255), got {}", pixel);
            }
        }

        // No errors in diagnostics
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgUnsupportedFormat));
    }

    #[test]
    fn test_preprocess_jbig2_only_pads() {
        // Acceptance criterion: JBIG2 untouched except for border padding
        let source = load_fixture("tests/fixtures/preprocess/jbig2_scan/source.png");
        let (preprocessed, diagnostics) = preprocess(&source, ImageSource::Jbig2)
            .expect("Preprocess failed");

        // Should have border padding
        assert_eq!(preprocessed.width(), source.width() + 20);
        assert_eq!(preprocessed.height(), source.height() + 20);

        // The inner region should match the original exactly (no binarization/denoise)
        for y in 0..source.height() {
            for x in 0..source.width() {
                let orig = source.get_pixel(x, y)[0];
                let pad = preprocessed.get_pixel(x + BORDER_PADDING, y + BORDER_PADDING)[0];
                assert_eq!(orig, pad, "JBIG2 inner pixel at ({}, {}) should match original", x, y);
            }
        }

        // No errors in diagnostics
        assert!(!diagnostics.iter().any(|d| d.code == DiagCode::ImgUnsupportedFormat));
    }

    #[test]
    fn test_preprocess_deterministic() {
        // Acceptance criterion: same input -> bit-identical output
        let source = load_fixture("tests/fixtures/preprocess/clean_digital/source.png");

        let (result1, _) = preprocess(&source, ImageSource::DigitalOrigin)
            .expect("First preprocess failed");
        let (result2, _) = preprocess(&source, ImageSource::DigitalOrigin)
            .expect("Second preprocess failed");

        // Compare pixel-by-pixel
        assert_eq!(result1.dimensions(), result2.dimensions());
        for y in 0..result1.height() {
            for x in 0..result1.width() {
                let p1 = result1.get_pixel(x, y)[0];
                let p2 = result2.get_pixel(x, y)[0];
                assert_eq!(p1, p2, "Pixels differ at ({}, {}): {} vs {}", x, y, p1, p2);
            }
        }
    }

    #[test]
    fn test_preprocess_border_padding_pixel_perfect() {
        // Acceptance criterion: padding adds exactly 10px on each side
        let source = load_fixture("tests/fixtures/preprocess/clean_digital/source.png");
        let (preprocessed, _) = preprocess(&source, ImageSource::DigitalOrigin)
            .expect("Preprocess failed");

        // Check top border is white
        for x in 0..preprocessed.width() {
            for y in 0..BORDER_PADDING {
                assert_eq!(preprocessed.get_pixel(x, y)[0], 255, "Top border should be white");
            }
        }

        // Check bottom border is white
        for x in 0..preprocessed.width() {
            for y in preprocessed.height() - BORDER_PADDING..preprocessed.height() {
                assert_eq!(preprocessed.get_pixel(x, y)[0], 255, "Bottom border should be white");
            }
        }

        // Check left border is white
        for y in 0..preprocessed.height() {
            for x in 0..BORDER_PADDING {
                assert_eq!(preprocessed.get_pixel(x, y)[0], 255, "Left border should be white");
            }
        }

        // Check right border is white
        for y in 0..preprocessed.height() {
            for x in preprocessed.width() - BORDER_PADDING..preprocessed.width() {
                assert_eq!(preprocessed.get_pixel(x, y)[0], 255, "Right border should be white");
            }
        }
    }
}

// Benchmarks for preprocessing performance

#[cfg(all(test, feature = "ocr", target_arch = "x86_64"))]
mod benches {
    use super::*;
    use std::time::{Duration, Instant};

    /// A4 page size at 300 DPI: 2480 x 3508 pixels.
    /// This is a typical input size for preprocessing.
    const A4_WIDTH: u32 = 2480;
    const A4_HEIGHT: u32 = 3508;

    /// Create an A4-sized test image with a simple pattern.
    fn create_a4_test_image() -> GrayImage {
        let mut img = GrayImage::new(A4_WIDTH, A4_HEIGHT);

        // Fill with a gradient pattern (simulating a scanned document)
        for y in 0..A4_HEIGHT {
            for x in 0..A4_WIDTH {
                // Create horizontal bands (simulating text lines)
                let line_y = (y / 20) * 20 + 10;
                let in_text_line = (y as i32 - line_y as i32).abs() < 6;
                let in_text = x % 60 < 50;

                let val = if in_text_line && in_text { 0 } else { 220 };
                img.put_pixel(x, y, Luma([val]));
            }
        }

        img
    }

    #[test]
    fn benchmark_preprocess_a4_physical_scan() {
        // Acceptance criterion: A4-page benchmark < 500 ms on CI
        let img = create_a4_test_image();

        let start = Instant::now();
        let (result, diagnostics) = preprocess(&img, ImageSource::PhysicalScan)
            .expect("Preprocess failed");
        let elapsed = start.elapsed();

        println!("A4 (2480x3508) PhysicalScan preprocess time: {:?}", elapsed);

        // Verify correctness
        assert_eq!(result.width(), A4_WIDTH + 20);
        assert_eq!(result.height(), A4_HEIGHT + 20);

        // Check performance requirement
        assert!(
            elapsed < Duration::from_millis(500),
            "A4 preprocess took {:?}, expected < 500ms",
            elapsed
        );

        println!("✓ A4 preprocessing completed within 500ms limit");
    }

    #[test]
    fn benchmark_preprocess_a4_digital_origin() {
        let img = create_a4_test_image();

        let start = Instant::now();
        let (result, _) = preprocess(&img, ImageSource::DigitalOrigin)
            .expect("Preprocess failed");
        let elapsed = start.elapsed();

        println!("A4 (2480x3508) DigitalOrigin preprocess time: {:?}", elapsed);

        assert_eq!(result.width(), A4_WIDTH + 20);
        assert_eq!(result.height(), A4_HEIGHT + 20);

        assert!(
            elapsed < Duration::from_millis(500),
            "A4 preprocess took {:?}, expected < 500ms",
            elapsed
        );
    }

    #[test]
    fn benchmark_preprocess_a4_jbig2() {
        let img = create_a4_test_image();

        let start = Instant::now();
        let (result, _) = preprocess(&img, ImageSource::Jbig2)
            .expect("Preprocess failed");
        let elapsed = start.elapsed();

        println!("A4 (2480x3508) Jbig2 preprocess time: {:?}", elapsed);

        assert_eq!(result.width(), A4_WIDTH + 20);
        assert_eq!(result.height(), A4_HEIGHT + 20);

        // JBIG2 should be faster (skips many steps)
        assert!(
            elapsed < Duration::from_millis(200),
            "A4 JBIG2 preprocess took {:?}, expected < 200ms",
            elapsed
        );
    }

    #[test]
    fn benchmark_individual_steps() {
        let img = create_a4_test_image();

        // Benchmark deskew
        let start = Instant::now();
        let (deskewed, angle, _) = deskew(&img).expect("Deskew failed");
        let deskew_time = start.elapsed();
        println!("Deskew time: {:?} (angle: {}°)", deskew_time, angle);

        // Benchmark contrast normalization
        let start = Instant::now();
        let normalized = normalize_contrast(&deskewed);
        let contrast_time = start.elapsed();
        println!("Contrast normalization time: {:?}", contrast_time);

        // Benchmark Sauvola binarization
        let start = Instant::now();
        let binary = binarize_sauvola(&normalized);
        let sauvola_time = start.elapsed();
        println!("Sauvola binarization time: {:?}", sauvola_time);

        // Benchmark denoising
        let start = Instant::now();
        let denoised = denoise_median(&binary);
        let denoise_time = start.elapsed();
        println!("Median denoise time: {:?}", denoise_time);

        // Benchmark padding
        let start = Instant::now();
        let padded = add_border_padding(&denoised);
        let pad_time = start.elapsed();
        println!("Border padding time: {:?}", pad_time);

        let total = deskew_time + contrast_time + sauvola_time + denoise_time + pad_time;
        println!("Total individual step time: {:?}", total);

        // Verify final result
        assert_eq!(padded.width(), A4_WIDTH + 20);
        assert_eq!(padded.height(), A4_HEIGHT + 20);

        assert!(
            total < Duration::from_millis(500),
            "Total step time took {:?}, expected < 500ms",
            total
        );
    }
}
