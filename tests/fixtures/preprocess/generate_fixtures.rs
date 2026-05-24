//! Generate preprocessing test fixtures.
//!
//! This binary creates synthetic test images for the preprocessing pipeline.
//! Run with: cargo run --bin generate_preprocess_fixtures

use image::{GrayImage, ImageBuffer, Luma};

fn main() {
    println!("Generating preprocessing test fixtures...");

    create_skewed_2deg();
    create_uneven_lighting();
    create_clean_digital();
    create_jbig2_scan();

    println!("Done!");
}

/// Create a 2-degree skewed image for deskew testing.
fn create_skewed_2deg() {
    let width = 400u32;
    let height = 300u32;
    let angle_deg = 2.0f32;
    let angle_rad = angle_deg * std::f32::consts::PI / 180.0;

    // Create a deskewed image with horizontal text lines
    let mut img = GrayImage::new(width, height);

    // Fill with white background
    for pixel in img.pixels_mut() {
        *pixel = Luma([255]);
    }

    // Draw horizontal text-like lines (every 20 pixels)
    for y in 0..height {
        for x in 0..width {
            // Create a pattern of lines that look like text
            let line_y = (y / 20) * 20 + 10;
            let in_text_line = (y as i32 - line_y as i32).abs() < 6;
            let in_text = x % 40 < 30; // Text-like pattern

            if in_text_line && in_text {
                img.put_pixel(x, y, Luma([0]));
            }
        }
    }

    // Rotate by 2 degrees (manual rotation for simplicity)
    let mut skewed = GrayImage::new(width, height);

    // Fill with white background
    for pixel in skewed.pixels_mut() {
        *pixel = Luma([255]);
    }

    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;

    for y in 0..height {
        for x in 0..width {
            // Transform point to unrotated coordinate system
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;

            // Rotate back to find the "original" coordinates
            let orig_x = dx * cos_a + dy * sin_a + center_x;
            let orig_y = dy * cos_a - dx * sin_a + center_y;

            // Sample from original image (nearest neighbor)
            let ox = orig_x.round() as i32;
            let oy = orig_y.round() as i32;

            if ox >= 0 && ox < width as i32 && oy >= 0 && oy < height as i32 {
                let pixel = img.get_pixel(ox as u32, oy as u32);
                skewed.put_pixel(x, y, *pixel);
            }
        }
    }

    skewed
        .save("tests/fixtures/preprocess/skewed_2deg/source.png")
        .unwrap();
    println!("Created skewed_2deg/source.png");
}

/// Create an image with uneven lighting for Sauvola testing.
fn create_uneven_lighting() {
    let width = 400u32;
    let height = 300u32;

    let mut img = GrayImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Gradient from darker (left) to lighter (right)
            let val = 150u8 + (x as u32 * 80 / width) as u8;
            img.put_pixel(x, y, Luma([val]));
        }
    }

    // Draw text-like patterns on the uneven background
    for y in (50..250).step_by(25) {
        for line_y in y..y + 10 {
            for x in 50..350 {
                // Create a text-like pattern
                let word_start = x / 50 * 50;
                let in_word = (x as i32 - word_start as i32) < 35;
                if in_word {
                    img.put_pixel(x, line_y, Luma([0]));
                }
            }
        }
    }

    img.save("tests/fixtures/preprocess/uneven_lighting/source.png")
        .unwrap();
    println!("Created uneven_lighting/source.png");
}

/// Create a clean digital-origin image for Otsu testing.
fn create_clean_digital() {
    let width = 400u32;
    let height = 300u32;

    // Create a clean white background
    let mut img = GrayImage::new(width, height);

    for pixel in img.pixels_mut() {
        *pixel = Luma([255]);
    }

    // Draw crisp text (as if from a digital PDF)
    for y in (50..250).step_by(25) {
        for line_y in y..y + 10 {
            for x in 50..350 {
                // Create a text-like pattern
                let word_start = x / 50 * 50;
                let in_word = (x as i32 - word_start as i32) < 35;
                if in_word {
                    img.put_pixel(x, line_y, Luma([0]));
                }
            }
        }
    }

    img.save("tests/fixtures/preprocess/clean_digital/source.png")
        .unwrap();
    println!("Created clean_digital/source.png");
}

/// Create a binary image (simulating JBIG2).
fn create_jbig2_scan() {
    let width = 400u32;
    let height = 300u32;

    // Create a pure binary image
    let mut img = GrayImage::new(width, height);

    for pixel in img.pixels_mut() {
        *pixel = Luma([255]);
    }

    // Draw binary text
    for y in (50..250).step_by(25) {
        for line_y in y..y + 10 {
            for x in 50..350 {
                // Create a text-like pattern
                let word_start = x / 50 * 50;
                let in_word = (x as i32 - word_start as i32) < 35;
                if in_word {
                    img.put_pixel(x, line_y, Luma([0]));
                }
            }
        }
    }

    // Ensure it's truly binary (only 0 and 255)
    for pixel in img.pixels_mut() {
        let val = pixel[0];
        pixel[0] = if val < 128 { 0 } else { 255 };
    }

    img.save("tests/fixtures/preprocess/jbig2_scan/source.png")
        .unwrap();
    println!("Created jbig2_scan/source.png");
}
