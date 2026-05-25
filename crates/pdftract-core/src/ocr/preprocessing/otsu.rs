//! Otsu global thresholding for OCR preprocessing (Phase 5.3.3b).
//!
//! This module implements Otsu's method for automatic image thresholding.
//! It finds the optimal threshold value that maximizes the inter-class variance
//! between foreground (text) and background (paper).
//!
//! # Algorithm
//!
//! Otsu's method assumes the image contains two classes of pixels (foreground
//! and background) and finds the threshold that minimizes the intra-class variance
//! (or equivalently, maximizes the inter-class variance).
//!
//! 1. Compute a 256-bin histogram of the input grayscale image
//! 2. For each possible threshold (0-255), compute the inter-class variance
//! 3. Select the threshold with maximum inter-class variance
//! 4. Apply the threshold to create a binary image
//!
//! # When to Use
//!
//! Otsu is optimal for images with globally consistent illumination:
//! - Digital-origin PDFs (rendered from electronic documents)
//! - Screenshots
//! - Scans with uniform lighting
//!
//! For scans with uneven lighting or shadows, use Sauvola local adaptive
//! thresholding instead (see `contrast::sauvola_binarize`).
//!
//! # Performance
//!
//! - O(N) for histogram computation + O(256) for threshold search
//! - ~30 ms for a 1080p image on a typical CPU
//! - Faster than Sauvola (single global threshold vs per-pixel computation)

use image::{GrayImage, Luma};

/// Apply Otsu global thresholding to binarize a grayscale image.
///
/// This function finds the optimal threshold value using Otsu's method
/// and applies it to create a binary image (black text on white background).
///
/// # Arguments
///
/// * `image` - The grayscale image to binarize
///
/// # Returns
///
/// A new binary image where each pixel is either 0 (black) or 255 (white).
///
/// # Algorithm
///
/// Uses `imageproc::contrast::otsu_level` to find the optimal threshold,
/// then applies `imageproc::contrast::threshold` to binarize the image.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::ocr::preprocessing::otsu::otsu_binarize;
/// use image::GrayImage;
///
/// let gray_img: GrayImage = // ... load grayscale image ...
/// let binary_img = otsu_binarize(&gray_img);
/// // binary_img contains only 0 (black) and 255 (white) pixels
/// ```
///
/// # Performance
///
/// - 1080p grayscale image (1920×1080): ~30 ms
/// - Significantly faster than Sauvola for uniformly-lit images
pub fn otsu_binarize(image: &GrayImage) -> GrayImage {
    use imageproc::contrast::{otsu_level, threshold};

    // Find the optimal threshold using Otsu's method
    let level = otsu_level(image);

    // Apply the threshold to create a binary image
    threshold(image, level)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Otsu on a digital-origin (uniform-lit) page produces clean binary
    ///
    /// Creates a synthetic image with distinct foreground (dark) and background
    /// (light) regions, simulating a digital-origin document with uniform lighting.
    #[test]
    fn test_otsu_digital_origin_clean_binary() {
        // Create a 200x200 image with distinct foreground/background
        let mut img = GrayImage::new(200, 200);

        // Left half: dark gray (simulating text)
        for y in 0..200 {
            for x in 0..100 {
                img.put_pixel(x, y, Luma([60]));
            }
        }

        // Right half: light gray (simulating paper)
        for y in 0..200 {
            for x in 100..200 {
                img.put_pixel(x, y, Luma([200]));
            }
        }

        // Apply Otsu binarization
        let binary = otsu_binarize(&img);

        // Verify binary output: all pixels are 0 or 255
        for y in 0..200 {
            for x in 0..200 {
                let pixel = binary.get_pixel(x, y)[0];
                assert!(
                    pixel == 0 || pixel == 255,
                    "Pixel at ({}, {}) should be 0 or 255, got {}",
                    x,
                    y,
                    pixel
                );
            }
        }

        // Verify foreground/background separation:
        // - Left half (dark) should become 0 (black)
        // - Right half (light) should become 255 (white)
        let left_half_is_black = (0..100).all(|x| binary.get_pixel(x, 100)[0] == 0);
        let right_half_is_white = (100..200).all(|x| binary.get_pixel(x, 100)[0] == 255);

        assert!(
            left_half_is_black,
            "Left half (foreground) should be black (0)"
        );
        assert!(
            right_half_is_white,
            "Right half (background) should be white (255)"
        );
    }

    /// Test: Output pixels are exactly 0 or 255 (no intermediate values)
    ///
    /// Verifies that the binarization produces a true binary image with
    /// no intermediate gray values.
    #[test]
    fn test_otsu_binary_output_only() {
        // Create a gradient image (0 to 255)
        let mut img = GrayImage::new(256, 1);
        for x in 0..256 {
            img.put_pixel(x, 0, Luma([x as u8]));
        }

        let binary = otsu_binarize(&img);

        // All pixels should be exactly 0 or 255
        for x in 0..256 {
            let pixel = binary.get_pixel(x, 0)[0];
            assert_eq!(
                pixel, 0,
                "Pixel at x={} should be 0 or 255, got {}",
                x, pixel
            );
        }
    }

    /// Test: Otsu on a nearly-uniform image (edge case)
    ///
    /// Verifies that Otsu handles edge cases gracefully:
    /// - Uniform dark image
    /// - Uniform light image
    /// - Very narrow histogram
    #[test]
    fn test_otsu_uniform_image() {
        // Test 1: Uniform dark image
        let mut dark_img = GrayImage::new(100, 100);
        for pixel in dark_img.pixels_mut() {
            *pixel = Luma([30]);
        }
        let binary_dark = otsu_binarize(&dark_img);
        // Should still produce binary output (all 0 or all 255)
        for pixel in binary_dark.pixels() {
            let val = pixel[0];
            assert!(
                val == 0 || val == 255,
                "Uniform dark image should binarize to 0 or 255, got {}",
                val
            );
        }

        // Test 2: Uniform light image
        let mut light_img = GrayImage::new(100, 100);
        for pixel in light_img.pixels_mut() {
            *pixel = Luma([225]);
        }
        let binary_light = otsu_binarize(&light_img);
        for pixel in binary_light.pixels() {
            let val = pixel[0];
            assert!(
                val == 0 || val == 255,
                "Uniform light image should binarize to 0 or 255, got {}",
                val
            );
        }

        // Test 3: Very narrow histogram (values in [100, 101])
        let mut narrow_img = GrayImage::new(100, 100);
        for pixel in narrow_img.pixels_mut() {
            *pixel = Luma([100]);
        }
        // Add a few pixels at 101 to create a tiny bit of variance
        for y in 0..10 {
            for x in 0..10 {
                narrow_img.put_pixel(x, y, Luma([101]));
            }
        }
        let binary_narrow = otsu_binarize(&narrow_img);
        // Should still produce binary output without panic
        for pixel in binary_narrow.pixels() {
            let val = pixel[0];
            assert!(
                val == 0 || val == 255,
                "Narrow histogram image should binarize to 0 or 255, got {}",
                val
            );
        }
    }

    /// Test: Otsu on a tri-modal histogram (edge case - suboptimal but no panic)
    ///
    /// Verifies that Otsu fails gracefully on tri-modal histograms
    /// (e.g., document with watermark or three distinct gray levels).
    /// The output should still be binary, even if the threshold is suboptimal.
    #[test]
    fn test_otsu_tri_modal_no_panic() {
        // Create a tri-modal histogram: dark (50), medium (128), light (200)
        let mut img = GrayImage::new(300, 100);

        // Third 1: dark (50)
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, Luma([50]));
            }
        }

        // Third 2: medium (128) - simulating watermark or gray background
        for y in 0..100 {
            for x in 100..200 {
                img.put_pixel(x, y, Luma([128]));
            }
        }

        // Third 3: light (200)
        for y in 0..100 {
            for x in 200..300 {
                img.put_pixel(x, y, Luma([200]));
            }
        }

        // Should not panic and should still produce binary output
        let binary = otsu_binarize(&img);

        // Verify binary output
        for y in 0..100 {
            for x in 0..300 {
                let pixel = binary.get_pixel(x, y)[0];
                assert!(
                    pixel == 0 || pixel == 255,
                    "Tri-modal image should still produce binary output, got {} at ({}, {})",
                    pixel,
                    x,
                    y
                );
            }
        }
    }

    /// Test: Otsu on a real-world-like text image
    ///
    /// Simulates a document page with text lines on white background.
    #[test]
    fn test_otsu_text_like_image() {
        let mut img = GrayImage::new(400, 300);

        // White background (240)
        for y in 0..300 {
            for x in 0..400 {
                img.put_pixel(x, y, Luma([240]));
            }
        }

        // Add dark horizontal lines (simulating text)
        for line in 0..10 {
            let y = 30 + line * 25;
            for x in 50..350 {
                img.put_pixel(x, y, Luma([40])); // Dark text
                img.put_pixel(x, y + 1, Luma([40]));
                img.put_pixel(x, y + 2, Luma([40]));
            }
        }

        let binary = otsu_binarize(&img);

        // Verify binary output
        for y in 0..300 {
            for x in 0..400 {
                let pixel = binary.get_pixel(x, y)[0];
                assert!(
                    pixel == 0 || pixel == 255,
                    "Text-like image should produce binary output, got {} at ({}, {})",
                    pixel,
                    x,
                    y
                );
            }
        }

        // Verify text lines became black (0) and background became white (255)
        // Check a text line pixel
        assert_eq!(binary.get_pixel(100, 31)[0], 0, "Text line should be black");
        // Check background pixel
        assert_eq!(
            binary.get_pixel(100, 20)[0],
            255,
            "Background should be white"
        );
    }

    /// Test: Otsu on small image (edge case for dimensions)
    ///
    /// Verifies that Otsu handles very small images correctly.
    #[test]
    fn test_otsu_small_image() {
        // 1x1 image
        let mut img1 = GrayImage::new(1, 1);
        img1.put_pixel(0, 0, Luma([128]));
        let binary1 = otsu_binarize(&img1);
        assert!(binary1.get_pixel(0, 0)[0] == 0 || binary1.get_pixel(0, 0)[0] == 255);

        // 2x2 image
        let mut img2 = GrayImage::new(2, 2);
        img2.put_pixel(0, 0, Luma([50]));
        img2.put_pixel(1, 0, Luma([200]));
        img2.put_pixel(0, 1, Luma([50]));
        img2.put_pixel(1, 1, Luma([200]));
        let binary2 = otsu_binarize(&img2);
        for y in 0..2 {
            for x in 0..2 {
                let pixel = binary2.get_pixel(x, y)[0];
                assert!(pixel == 0 || pixel == 255);
            }
        }
    }

    /// Benchmark: Verify Otsu runs in reasonable time
    ///
    /// This test ensures Otsu completes within the expected time bound
    /// for a 1080p image. The acceptance criteria specifies < 50ms.
    ///
    /// Note: This is a sanity check, not a precise benchmark. CI environments
    /// may have variable performance, so we use a generous timeout.
    #[test]
    #[ignore] // Ignored by default; run with: cargo test --features ocr -- --ignored otsu
    fn test_otsu_benchmark_1080p() {
        use std::time::Instant;

        // Create a 1920x1080 (1080p) image
        let mut img = GrayImage::new(1920, 1080);

        // Fill with synthetic content
        for y in 0..1080 {
            for x in 0..1920 {
                let val = if x < 960 { 60 } else { 200 };
                img.put_pixel(x, y, Luma([val]));
            }
        }

        let start = Instant::now();
        let _binary = otsu_binarize(&img);
        let duration = start.elapsed();

        // Acceptance criteria: < 50ms (use 100ms for CI variability)
        assert!(
            duration.as_millis() < 100,
            "Otsu on 1080p took {} ms, expected < 100 ms",
            duration.as_millis()
        );
    }
}
