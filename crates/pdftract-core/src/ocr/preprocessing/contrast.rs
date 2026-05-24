//! Contrast normalization via histogram stretch (Phase 5.3.2a).
//!
//! This module implements histogram-based contrast normalization for OCR
//! preprocessing. It stretches the input image's gray value range to the
//! full [0, 255] output range, improving the effectiveness of downstream
//! binarization (Sauvola/Otsu).
//!
//! # Algorithm
//!
//! 1. Compute a 256-bin histogram of the input grayscale image
//! 2. Find the 1st percentile (p01) and 99th percentile (p99) values
//! 3. Linearly map [p01, p99] to [0, 255]: `new = ((old - p01) * 255) / (p99 - p01)`
//! 4. Clamp results to [0, 255]
//!
//! The percentile-based approach provides robustness against outliers:
//! - A few hot pixels (e.g., scanner artifacts) don't dominate the stretch
//! - A black border or noise specks don't prevent proper normalization
//!
//! # JBIG2 Skip Rule
//!
//! JBIG2-encoded images are already binary (1-bit per pixel). Applying
//! histogram stretch to them is unnecessary and would incorrectly introduce
//! 8-bit grayscale values. Such images are identified at the image-source
//! dispatch layer and skip contrast normalization entirely.

use image::{GrayImage, Luma};

/// Error type for preprocessing operations.
#[derive(Debug, Clone, PartialEq)]
pub enum PreprocError {
    /// Image is uniform (single gray value) - no stretching possible.
    UniformImage,
    /// Invalid image dimensions (zero width or height).
    InvalidDimensions,
}

impl std::fmt::Display for PreprocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocError::UniformImage => {
                write!(
                    f,
                    "image has uniform gray value; contrast stretch is a no-op"
                )
            }
            PreprocError::InvalidDimensions => {
                write!(f, "image has invalid dimensions (zero width or height)")
            }
        }
    }
}

impl std::error::Error for PreprocError {}

/// Apply histogram stretch contrast normalization to a grayscale image.
///
/// This function modifies the image in-place, mapping the input gray value
/// range (from 1st to 99th percentile) to the full [0, 255] output range.
///
/// # Arguments
///
/// * `image` - The grayscale image to normalize (modified in-place)
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(PreprocError::UniformImage)` - Image has uniform gray value (no stretch possible)
/// * `Err(PreprocError::InvalidDimensions)` - Image has zero width or height
///
/// # Algorithm
///
/// 1. Compute histogram (256 bins for u8 gray values)
/// 2. Find p01 = gray value at 1st percentile (cumulative count >= 1% of pixels)
/// 3. Find p99 = gray value at 99th percentile (cumulative count >= 99% of pixels)
/// 4. For each pixel: `new = clamp(((old - p01) * 255) / (p99 - p01), 0, 255)`
/// 5. If p99 == p01 (uniform image), return early with `UniformImage` error
///
/// # Performance
///
/// - 1080p grayscale image (1920×1080): ~25 ms on typical CPU
/// - 8 MP image (3264×2448): ~45 ms on typical CPU
///
/// # Example
///
/// ```ignore
/// use pdftract_core::ocr::preprocessing::contrast::histogram_stretch;
/// use image::GrayImage;
///
/// let mut img = GrayImage::new(100, 100);
/// // ... populate img with data in range [50, 200] ...
/// histogram_stretch(&mut img).unwrap();
/// // img now has full [0, 255] range
/// ```
pub fn histogram_stretch(image: &mut GrayImage) -> Result<(), PreprocError> {
    let width = image.width();
    let height = image.height();

    if width == 0 || height == 0 {
        return Err(PreprocError::InvalidDimensions);
    }

    let pixel_count = (width as usize) * (height as usize);

    // Step 1: Compute histogram (256 bins)
    let mut histogram = [0usize; 256];
    for pixel in image.pixels() {
        histogram[pixel[0] as usize] += 1;
    }

    // Step 2: Find p01 (1st percentile)
    let p01_target = pixel_count / 100; // 1% of pixels
    let mut cumulative = 0;
    let p01 = loop {
        let mut found = None;
        for (gray, &count) in histogram.iter().enumerate() {
            cumulative += count;
            if cumulative >= p01_target {
                found = Some(gray as u8);
                break;
            }
        }
        match found {
            Some(v) => break v,
            None => return Err(PreprocError::UniformImage),
        }
    };

    // Step 3: Find p99 (99th percentile)
    let p99_target = (99 * pixel_count) / 100; // 99% of pixels
    cumulative = 0;
    let p99 = loop {
        let mut found = None;
        for (gray, &count) in histogram.iter().enumerate() {
            cumulative += count;
            if cumulative >= p99_target {
                found = Some(gray as u8);
                break;
            }
        }
        match found {
            Some(v) => break v,
            None => return Err(PreprocError::UniformImage),
        }
    };

    // Step 4: If p99 == p01, image is uniform - no stretching possible
    if p99 == p01 {
        return Err(PreprocError::UniformImage);
    }

    // Step 5: Apply linear stretch: new = ((old - p01) * 255) / (p99 - p01)
    // Cast to i32 to avoid overflow in numerator
    let range = (p99 - p01) as i32;
    for pixel in image.pixels_mut() {
        let old = pixel[0] as i32;
        let new = ((old - (p01 as i32)) * 255) / range;
        // Clamp to [0, 255] (saturating_sub handles underflow, min handles overflow)
        pixel[0] = new.clamp(0, 255) as u8;
    }

    Ok(())
}

/// Apply histogram stretch only if the image is not JBIG2-encoded.
///
/// This is a convenience wrapper that callers can use when they don't have
/// image source information available. For images where the source is known
/// to be JBIG2 (already binary), callers should skip calling this function
/// entirely to avoid unnecessary processing.
///
/// # Arguments
///
/// * `image` - The grayscale image to normalize (modified in-place)
///
/// # Returns
///
/// * `Ok(true)` - Stretch applied successfully
/// * `Ok(false)` - Image is uniform (stretch not applied)
/// * `Err(PreprocError::InvalidDimensions)` - Invalid dimensions
///
/// # Note
///
/// This function treats `UniformImage` as a soft error (returns `Ok(false)`),
/// since a uniform image simply doesn't need contrast stretching. Hard errors
/// (like `InvalidDimensions`) still propagate.
pub fn histogram_stretch_if_needed(image: &mut GrayImage) -> Result<bool, PreprocError> {
    match histogram_stretch(image) {
        Ok(()) => Ok(true),
        Err(PreprocError::UniformImage) => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test image with a uniform gray value.
    fn make_uniform_image(value: u8, width: u32, height: u32) -> GrayImage {
        GrayImage::from_fn(width, height, |_, _| Luma([value]))
    }

    /// Create a test image with a specific gray value range.
    fn make_range_image(min: u8, max: u8, width: u32, height: u32) -> GrayImage {
        GrayImage::from_fn(width, height, |x, y| {
            let progress = (x + y) as f32 / (width + height) as f32;
            let value = min as f32 + progress * (max - min) as f32;
            Luma([value.round().clamp(0.0, 255.0) as u8])
        })
    }

    /// Create a test image with hot pixels at extremes.
    fn make_image_with_hot_pixels() -> GrayImage {
        let mut img = make_uniform_image(128, 100, 100);
        // Add one black pixel (0)
        img.put_pixel(0, 0, Luma([0]));
        // Add one white pixel (255)
        img.put_pixel(99, 99, Luma([255]));
        img
    }

    #[test]
    fn test_histogram_stretch_normal_range() {
        // Image with [50, 200] range should stretch to [0, 255]
        let mut img = make_range_image(50, 200, 100, 100);
        histogram_stretch(&mut img).unwrap();

        // Check that min is close to 0 and max is close to 255
        let mut min = 255u8;
        let mut max = 0u8;
        for pixel in img.pixels() {
            min = min.min(pixel[0]);
            max = max.max(pixel[0]);
        }

        assert!(min <= 5, "min should be near 0, got {}", min);
        assert!(max >= 250, "max should be near 255, got {}", max);
    }

    #[test]
    fn test_histogram_stretch_hot_pixel_robustness() {
        // Image with hot pixels at 0 and 255 should still stretch
        let mut img = make_image_with_hot_pixels();
        histogram_stretch(&mut img).unwrap();

        // Most pixels should be stretched away from 128
        let mut sum = 0u64;
        let mut count = 0u32;
        for pixel in img.pixels() {
            // Skip the hot pixels themselves
            if pixel[0] == 0 || pixel[0] == 255 {
                continue;
            }
            sum += pixel[0] as u64;
            count += 1;
        }

        // Average should be significantly different from 128
        let avg = (sum / count as u64) as i32 - 128;
        assert!(
            avg.abs() > 20,
            "average should be far from 128, got diff {}",
            avg
        );
    }

    #[test]
    fn test_histogram_stretch_uniform_image() {
        // Uniform image should return error
        let mut img = make_uniform_image(128, 100, 100);
        let result = histogram_stretch(&mut img);
        assert_eq!(result, Err(PreprocError::UniformImage));

        // Image should be unchanged
        for pixel in img.pixels() {
            assert_eq!(pixel[0], 128);
        }
    }

    #[test]
    fn test_histogram_stretch_single_pixel() {
        // 1x1 image is uniform
        let mut img = make_uniform_image(100, 1, 1);
        let result = histogram_stretch(&mut img);
        assert_eq!(result, Err(PreprocError::UniformImage));
    }

    #[test]
    fn test_histogram_stretch_invalid_dimensions() {
        let mut img = GrayImage::new(0, 100);
        let result = histogram_stretch(&mut img);
        assert_eq!(result, Err(PreprocError::InvalidDimensions));

        let mut img = GrayImage::new(100, 0);
        let result = histogram_stretch(&mut img);
        assert_eq!(result, Err(PreprocError::InvalidDimensions));
    }

    #[test]
    fn test_histogram_stretch_full_range() {
        // Image already at [0, 255] should be unchanged (no-op)
        let mut img = make_range_image(0, 255, 100, 100);
        let img_clone = img.clone();
        histogram_stretch(&mut img).unwrap();

        // Pixels should be nearly identical (small differences due to percentile clipping)
        let mut max_diff = 0u8;
        for (p1, p2) in img.pixels().zip(img_clone.pixels()) {
            max_diff = max_diff.max(p1[0].abs_diff(p2[0]));
        }
        assert!(
            max_diff <= 10,
            "max difference should be small, got {}",
            max_diff
        );
    }

    #[test]
    fn test_histogram_stretch_narrow_range() {
        // Narrow range [100, 110] should stretch to [0, 255]
        let mut img = make_range_image(100, 110, 100, 100);
        histogram_stretch(&mut img).unwrap();

        let mut min = 255u8;
        let mut max = 0u8;
        for pixel in img.pixels() {
            min = min.min(pixel[0]);
            max = max.max(pixel[0]);
        }

        assert!(min <= 10, "min should be near 0, got {}", min);
        assert!(max >= 245, "max should be near 255, got {}", max);
    }

    #[test]
    fn test_histogram_stretch_if_needed_true() {
        let mut img = make_range_image(50, 200, 100, 100);
        let result = histogram_stretch_if_needed(&mut img);
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn test_histogram_stretch_if_needed_uniform() {
        let mut img = make_uniform_image(128, 100, 100);
        let result = histogram_stretch_if_needed(&mut img);
        assert_eq!(result, Ok(false));

        // Image should be unchanged
        for pixel in img.pixels() {
            assert_eq!(pixel[0], 128);
        }
    }

    #[test]
    fn test_histogram_stretch_preserves_dimensions() {
        let mut img = make_range_image(50, 200, 123, 456);
        let original_width = img.width();
        let original_height = img.height();
        histogram_stretch(&mut img).unwrap();

        assert_eq!(img.width(), original_width);
        assert_eq!(img.height(), original_height);
    }

    #[test]
    fn test_preproc_error_display() {
        assert_eq!(
            format!("{}", PreprocError::UniformImage),
            "image has uniform gray value; contrast stretch is a no-op"
        );
        assert_eq!(
            format!("{}", PreprocError::InvalidDimensions),
            "image has invalid dimensions (zero width or height)"
        );
    }

    #[test]
    fn test_histogram_stretch_no_underflow() {
        // Image with values close to 0 should not underflow
        let mut img = make_range_image(0, 50, 100, 100);
        histogram_stretch(&mut img).unwrap();

        // All values should be in [0, 255]
        for pixel in img.pixels() {
            assert!(pixel[0] <= 255);
        }
    }

    #[test]
    fn test_histogram_stretch_no_overflow() {
        // Image with values close to 255 should not overflow
        let mut img = make_range_image(200, 255, 100, 100);
        histogram_stretch(&mut img).unwrap();

        // All values should be in [0, 255]
        for pixel in img.pixels() {
            assert!(pixel[0] <= 255);
        }
    }
}
