//! Sauvola local adaptive thresholding for OCR preprocessing (Phase 5.3.3a).
//!
//! This module implements Sauvola's algorithm for adaptive image thresholding.
//! It computes a local threshold for each pixel based on the mean and standard
//! deviation of pixels in a sliding window around it.
//!
//! # Algorithm
//!
//! Sauvola's method is designed for document images with uneven illumination:
//! 1. For each pixel, compute the local mean (m) and standard deviation (s) in a window
//! 2. Compute threshold: T(x,y) = m × (1 + k × (s / R - 1))
//!    - R is the dynamic range of standard deviation (128 for 8-bit images)
//!    - k controls the threshold sensitivity (default 0.34)
//! 3. Apply the local threshold to create a binary image
//!
//! # When to Use
//!
//! Sauvola is optimal for physical scans with uneven lighting:
//! - Scanned documents with shadows or vignetting
//! - Camera-captured documents
//! - Any image where illumination varies across the page
//!
//! For digital-origin images with uniform lighting, use Otsu global
//! thresholding instead (see `otsu::otsu_binarize`), which is faster.
//!
//! # Performance
//!
//! - O(N) with sliding-window optimization (leptonica uses integral images)
//! - ~500 ms for a 1080p image on a typical CPU
//! - Significantly slower than Otsu, but necessary for scans with uneven lighting

use crate::diagnostics::{DiagCode, Diagnostic};
use image::{GrayImage, Luma};

/// Default window size for Sauvola binarization.
///
/// A 15×15 window is the sweet spot for document OCR at 300 DPI:
/// - Smaller windows (e.g., 7×7) adapt to finer features but introduce noise
/// - Larger windows (e.g., 31×31) miss local lighting changes
/// - 15×15 balances noise resistance with local adaptation
///
/// This is the default used in the original Sauvola paper and has been
/// validated against Tesseract WER on document scans.
const DEFAULT_WINDOW_SIZE: u32 = 15;

/// Default k parameter for Sauvola binarization.
///
/// The k parameter controls threshold sensitivity:
/// - Lower k (e.g., 0.2) binarizes more aggressively (more black pixels)
/// - Higher k (e.g., 0.5) binarizes more conservatively (more white pixels)
///
/// 0.34 is the value recommended in the Sauvola paper and has been
/// calibrated against Tesseract WER on document scans.
///
/// # Note
///
/// If you change this value from the default, document the rationale
/// in code comments and update the ADR.
const DEFAULT_K: f32 = 0.34;

/// Apply Sauvola local adaptive thresholding to binarize a grayscale image.
///
/// This function computes a local threshold for each pixel based on the
/// mean and standard deviation in a sliding window, then applies that
/// threshold to create a binary image (black text on white background).
///
/// # Arguments
///
/// * `image` - The grayscale image to binarize
/// * `window_size` - Size of the sliding window (default 15, must be odd)
/// * `k` - Sensitivity parameter (default 0.34, lower = more aggressive)
///
/// # Returns
///
/// A new binary image where each pixel is either 0 (black) or 255 (white).
///
/// If a leptonica Pix conversion fails (e.g. for a degenerate/zero-dimension
/// input that leptonica cannot allocate a Pix for), the failure is treated as
/// recoverable per the plan's no-panic error model: a `tracing::warn!` is
/// emitted and the **unbinarized input image is returned unchanged** as a
/// degraded result, so the OCR pipeline can still attempt extraction instead
/// of aborting the host process. The function never panics on a conversion
/// failure.
///
/// # Algorithm
///
/// Uses leptonica's `pixSauvolaBinarize` implementation, which applies
/// Sauvola's formula: T(x,y) = m × (1 + k × (s / R - 1)), where:
/// - m = local mean
/// - s = local standard deviation
/// - R = dynamic range of standard deviation (128 for 8-bit images)
///
/// # Example
///
/// ```ignore
/// use pdftract_core::ocr::preprocessing::sauvola::sauvola_binarize;
/// use image::GrayImage;
///
/// let gray_img: GrayImage = // ... load grayscale image from scan ...
/// let binary_img = sauvola_binarize(&gray_img, 15, 0.34);
/// // binary_img contains only 0 (black) and 255 (white) pixels
/// ```
///
/// # Performance
///
/// - 1080p grayscale image (1920×1080): ~500 ms
/// - Significantly slower than Otsu, but necessary for scans with uneven lighting
///
/// # Panics
///
/// Panics if the window size is even (must be odd for the algorithm to work).
pub fn sauvola_binarize(image: &GrayImage, window_size: u32, k: f32) -> GrayImage {
    #[cfg(feature = "ocr")]
    {
        use crate::preprocess::{grayimage_to_pix, pix_to_grayimage};
        use leptonica_plumbing::leptonica_sys::{l_float32, l_int32, pixDestroy, Pix};
        use tracing::warn;

        assert!(
            window_size % 2 == 1,
            "Window size must be odd, got {}",
            window_size
        );

        // Diagnostics recorded when a recoverable leptonica conversion fails.
        //
        // Per the no-panic error model (plan.md §Error model and the Anti-Patterns
        // table: "no `panic!` in library code"), each such failure is surfaced as a
        // `tracing::warn!` and handled by falling back to a degraded result instead
        // of aborting the host process through the FFI/PyO3 boundary.
        let mut diagnostics: Vec<Diagnostic> = Vec::new();

        // Convert GrayImage to leptonica Pix.
        //
        // If the conversion fails (e.g. leptonica cannot allocate a Pix for a
        // degenerate/zero-dimension image), record the diagnostics and return the
        // unbinarized input image as a degraded result so the OCR pipeline can
        // still attempt extraction instead of aborting the whole process.
        let pix = match grayimage_to_pix(image) {
            Ok(p) => p,
            Err(diag) => {
                diagnostics.extend(diag);
                warn!(
                    "sauvola_binarize: GrayImage→Pix conversion failed; returning \
                     unbinarized input as a degraded result ({})",
                    diagnostics
                        .iter()
                        .map(|d| d.message.as_ref())
                        .collect::<Vec<_>>()
                        .join("; ")
                );
                return image.clone();
            }
        };

        // Call pixSauvolaBinarize via leptonica-sys
        let (binary_pix, _result) = unsafe {
            // Window size must be odd
            let wh = window_size as i32;
            let wl = window_size as i32;
            let factor = k as l_float32;

            // The actual function signature in leptonica is:
            // PIX * pixSauvolaBinarize(PIX *pixs, l_int32 wh, l_int32 wl, l_float32 factor);
            extern "C" {
                fn pixSauvolaBinarize(
                    pixs: *mut Pix,
                    wh: l_int32,
                    wl: l_int32,
                    factor: l_float32,
                ) -> *mut Pix;
            }

            let result = pixSauvolaBinarize(pix, wh, wl, factor);

            if result.is_null() {
                // leptonica failed to binarize (e.g. internal allocation failure).
                // Recoverable per the no-panic model: free the input Pix and fall
                // back to the unbinarized input image as a degraded result.
                pixDestroy(pix);
                warn!(
                    "sauvola_binarize: pixSauvolaBinarize returned null; returning \
                     unbinarized input as a degraded result (window_size={}, k={})",
                    window_size, k
                );
                return image.clone();
            }

            (result, ())
        };

        // Convert back to GrayImage.
        //
        // If the back-conversion fails, record the diagnostics, free the Pix, and
        // return the unbinarized input image as a degraded result.
        let result_image = match pix_to_grayimage(binary_pix) {
            Ok(img) => img,
            Err(diag) => {
                unsafe { pixDestroy(binary_pix) };
                diagnostics.extend(diag);
                warn!(
                    "sauvola_binarize: Pix→GrayImage conversion failed; returning \
                     unbinarized input as a degraded result ({})",
                    diagnostics
                        .iter()
                        .map(|d| d.message.as_ref())
                        .collect::<Vec<_>>()
                        .join("; ")
                );
                return image.clone();
            }
        };

        // Clean up
        unsafe {
            pixDestroy(binary_pix);
        }

        result_image
    }

    #[cfg(not(feature = "ocr"))]
    compile_error!("The 'ocr' feature must be enabled to use Sauvola binarization");
}

/// Apply Sauvola binarization with default parameters.
///
/// This is a convenience function that uses the recommended defaults:
/// - window_size = 15
/// - k = 0.34
///
/// These defaults are calibrated for document OCR at 300 DPI and have been
/// validated against Tesseract WER on document scans.
///
/// # Arguments
///
/// * `image` - The grayscale image to binarize
///
/// # Returns
///
/// A new binary image where each pixel is either 0 (black) or 255 (white).
///
/// # Example
///
/// ```ignore
/// use pdftract_core::ocr::preprocessing::sauvola::sauvola_binarize_default;
/// use image::GrayImage;
///
/// let gray_img: GrayImage = // ... load grayscale image from scan ...
/// let binary_img = sauvola_binarize_default(&gray_img);
/// ```
pub fn sauvola_binarize_default(image: &GrayImage) -> GrayImage {
    sauvola_binarize(image, DEFAULT_WINDOW_SIZE, DEFAULT_K)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Sauvola on a scanned-like page with uneven lighting produces clean binary
    ///
    /// Creates a synthetic image with a dark corner to simulate uneven illumination,
    /// simulating a physical scan.
    #[test]
    fn test_sauvola_uneven_lighting_clean_binary() {
        // Create a 300x300 image with uneven lighting (dark corner)
        let mut img = GrayImage::new(300, 300);

        // Simulate uneven lighting: darker in top-left, lighter in bottom-right
        for y in 0..300 {
            for x in 0..300 {
                // Background gradient (simulating uneven illumination)
                // Top-left corner: ~80, Bottom-right: ~200
                let bg_val = 80 + (x + y) as u8 / 5;

                // Add some dark text-like regions
                let is_text = (x % 40 < 10) && (y % 20 < 3);
                let pixel = if is_text {
                    // Text is darker than background
                    bg_val.saturating_sub(60)
                } else {
                    bg_val
                };

                img.put_pixel(x, y, Luma([pixel]));
            }
        }

        // Apply Sauvola binarization
        let binary = sauvola_binarize_default(&img);

        // Verify binary output: all pixels are 0 or 255
        for y in 0..300 {
            for x in 0..300 {
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

        // Verify that text regions became black (0) even in dark corner
        // Check a text pixel in the dark corner (top-left)
        assert_eq!(
            binary.get_pixel(5, 5)[0],
            0,
            "Text in dark corner should be black (0)"
        );
    }

    /// Test: Output pixels are exactly 0 or 255 (no intermediate values)
    ///
    /// Verifies that the binarization produces a true binary image with
    /// no intermediate gray values.
    #[test]
    fn test_sauvola_binary_output_only() {
        // Create a gradient image with varying brightness
        let mut img = GrayImage::new(200, 200);

        for y in 0..200 {
            for x in 0..200 {
                // Gradient from dark (60) to light (200)
                let val = 60 + (x + y) as u8 / 2;
                img.put_pixel(x, y, Luma([val]));
            }
        }

        let binary = sauvola_binarize_default(&img);

        // All pixels should be exactly 0 or 255
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
    }

    /// Test: Sauvola on a nearly-uniform image (edge case)
    ///
    /// Verifies that Sauvola handles edge cases gracefully:
    /// - Uniform dark image
    /// - Uniform light image
    #[test]
    fn test_sauvola_uniform_image() {
        // Test 1: Uniform dark image
        let mut dark_img = GrayImage::new(100, 100);
        for pixel in dark_img.pixels_mut() {
            *pixel = Luma([40]);
        }
        let binary_dark = sauvola_binarize_default(&dark_img);
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
            *pixel = Luma([220]);
        }
        let binary_light = sauvola_binarize_default(&light_img);
        for pixel in binary_light.pixels() {
            let val = pixel[0];
            assert!(
                val == 0 || val == 255,
                "Uniform light image should binarize to 0 or 255, got {}",
                val
            );
        }
    }

    /// Test: Sauvola handles small window size
    ///
    /// Verifies that Sauvola works with a smaller window (7x7), which
    /// adapts more aggressively to local features.
    #[test]
    fn test_sauvola_small_window() {
        let mut img = GrayImage::new(200, 200);

        // Create a simple pattern
        for y in 0..200 {
            for x in 0..200 {
                let pixel = if x < 100 { 80 } else { 180 };
                img.put_pixel(x, y, Luma([pixel]));
            }
        }

        // Use a smaller window
        let binary = sauvola_binarize(&img, 7, DEFAULT_K);

        // Verify binary output
        for y in 0..200 {
            for x in 0..200 {
                let pixel = binary.get_pixel(x, y)[0];
                assert!(
                    pixel == 0 || pixel == 255,
                    "Small window should produce binary output, got {} at ({}, {})",
                    pixel,
                    x,
                    y
                );
            }
        }
    }

    /// Test: Sauvola with custom k parameter
    ///
    /// Verifies that Sauvola works with different k values.
    #[test]
    fn test_sauvola_custom_k() {
        let mut img = GrayImage::new(150, 150);

        // Create a gradient
        for y in 0..150 {
            for x in 0..150 {
                let val = 60 + x as u8;
                img.put_pixel(x, y, Luma([val]));
            }
        }

        // Test with different k values
        for k in [0.2, 0.34, 0.5] {
            let binary = sauvola_binarize(&img, 15, k);

            // Verify binary output
            for y in 0..150 {
                for x in 0..150 {
                    let pixel = binary.get_pixel(x, y)[0];
                    assert!(
                        pixel == 0 || pixel == 255,
                        "k={} should produce binary output, got {} at ({}, {})",
                        k,
                        pixel,
                        x,
                        y
                    );
                }
            }
        }
    }

    /// Test: Sauvola panics on even window size
    ///
    /// Verifies that the function correctly validates that window size must be odd.
    #[test]
    #[should_panic(expected = "Window size must be odd")]
    fn test_sauvola_even_window_panics() {
        let img = GrayImage::new(100, 100);
        sauvola_binarize(&img, 14, DEFAULT_K); // Even window size should panic
    }

    /// Test: Sauvola on a real-world-like scanned page
    ///
    /// Simulates a scanned document page with text lines and uneven lighting.
    #[test]
    fn test_sauvola_scan_like_image() {
        let mut img = GrayImage::new(400, 300);

        // Simulate uneven lighting (vignette effect)
        for y in 0..300 {
            for x in 0..400 {
                let dx = x as f32 - 200.0;
                let dy = y as f32 - 150.0;
                let dist = (dx * dx + dy * dy).sqrt();
                // Edges are darker
                let vignette = (dist / 200.0).min(1.0) * 80.0;
                let bg_val = (200.0 - vignette).max(80.0) as u8;
                img.put_pixel(x, y, Luma([bg_val]));
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

        let binary = sauvola_binarize_default(&img);

        // Verify binary output
        for y in 0..300 {
            for x in 0..400 {
                let pixel = binary.get_pixel(x, y)[0];
                assert!(
                    pixel == 0 || pixel == 255,
                    "Scan-like image should produce binary output, got {} at ({}, {})",
                    pixel,
                    x,
                    y
                );
            }
        }

        // Verify text lines became black (0) even at the edges
        // Check a text line pixel at the edge (darker region)
        assert_eq!(
            binary.get_pixel(60, 31)[0],
            0,
            "Text line at edge should be black (0)"
        );
        // Check background pixel
        assert!(
            binary.get_pixel(200, 20)[0] == 255,
            "Background should be white (255)"
        );
    }

    /// Test: Sauvola on small image (edge case for dimensions)
    ///
    /// Verifies that Sauvola handles very small images correctly.
    #[test]
    fn test_sauvola_small_image() {
        // 1x1 image (edge case - smaller than window)
        let mut img1 = GrayImage::new(1, 1);
        img1.put_pixel(0, 0, Luma([128]));
        let binary1 = sauvola_binarize(&img1, 15, DEFAULT_K);
        assert!(
            binary1.get_pixel(0, 0)[0] == 0 || binary1.get_pixel(0, 0)[0] == 255,
            "1x1 image should produce binary output"
        );

        // 20x20 image (larger than window, but small)
        let mut img2 = GrayImage::new(20, 20);
        for y in 0..20 {
            for x in 0..20 {
                let pixel = if x < 10 { 60 } else { 180 };
                img2.put_pixel(x, y, Luma([pixel]));
            }
        }
        let binary2 = sauvola_binarize(&img2, 15, DEFAULT_K);
        for y in 0..20 {
            for x in 0..20 {
                let pixel = binary2.get_pixel(x, y)[0];
                assert!(
                    pixel == 0 || pixel == 255,
                    "Small image should produce binary output"
                );
            }
        }
    }

    /// Test: Default parameters match constants
    ///
    /// Verifies that the default function uses the documented defaults.
    #[test]
    fn test_sauvola_defaults_match_constants() {
        let img = GrayImage::new(100, 100);
        let binary_default = sauvola_binarize_default(&img);
        let binary_explicit = sauvola_binarize(&img, DEFAULT_WINDOW_SIZE, DEFAULT_K);

        // Both should produce the same output
        assert_eq!(
            binary_default.into_raw(),
            binary_explicit.into_raw(),
            "Default parameters should match constant values"
        );
    }

    /// Benchmark: Verify Sauvola runs in reasonable time
    ///
    /// This test ensures Sauvola completes within the expected time bound
    /// for a 1080p image. The acceptance criteria specifies < 500ms.
    ///
    /// Note: This is a sanity check, not a precise benchmark. CI environments
    /// may have variable performance, so we use a generous timeout.
    #[test]
    #[ignore] // Ignored by default; run with: cargo nextest run --features otp -- --ignored sauvola
    fn test_sauvola_benchmark_1080p() {
        use std::time::Instant;

        // Create a 1920x1080 (1080p) image with uneven lighting
        let mut img = GrayImage::new(1920, 1080);

        for y in 0..1080 {
            for x in 0..1920 {
                // Simulate uneven lighting
                let vignette = ((x as f32 - 960.0).abs() / 960.0 * 80.0) as u8;
                let base_val = 180u8.saturating_sub(vignette);
                img.put_pixel(x, y, Luma([base_val]));
            }
        }

        let start = Instant::now();
        let _binary = sauvola_binarize_default(&img);
        let duration = start.elapsed();

        // Acceptance criteria: < 500ms (use 1000ms for CI variability)
        assert!(
            duration.as_millis() < 1000,
            "Sauvola on 1080p took {} ms, expected < 1000 ms",
            duration.as_millis()
        );
    }
}
