//! OCR fallback for SVG receipt generation.
//!
//! This module implements PNG raster fallback for SVG receipts when font
//! outlines are unavailable (OCR-sourced glyphs or Type 3 fonts).
//!
//! # Feature Gate
//!
//! This module is only available when both `receipts` and `full-render`
//! features are enabled.


#[cfg(all(feature = "receipts", feature = "full-render"))]
use crate::render::pdfium_path::render_page_via_pdfium;

#[cfg(all(feature = "receipts", feature = "full-render"))]
use image::GrayImage;

#[cfg(all(feature = "receipts", feature = "full-render"))]
use image::imageops;

#[cfg(all(feature = "receipts", feature = "full-render"))]
use std::fmt::Write;

/// DPI for OCR raster fallback receipts.
///
/// 150 DPI is the sweet spot between file size and audit clarity.
/// At 150 DPI, a typical 12pt-text bbox of 200×30 PDF points becomes
/// a 416×62-pixel PNG — about 5-15 KB base64-encoded.
pub const SVG_OCR_RASTER_DPI: u32 = 150;

/// Result type for OCR fallback operations.
pub type Result<T> = std::result::Result<T, String>;

/// OCR fallback generator for SVG receipts.
///
/// Produces base64-encoded PNG rasters of bbox regions for OCR-sourced
/// glyphs or Type 3 fonts where ttf-parser cannot extract outlines.
#[cfg(all(feature = "receipts", feature = "full-render"))]
pub struct OcrFallbackGenerator {
    /// PDF bytes for rendering.
    pdf_bytes: Vec<u8>,
    /// Page index for this receipt.
    page_index: usize,
    /// Per-page render cache (reused across multiple receipts on same page).
    page_render: Option<GrayImage>,
}

#[cfg(all(feature = "receipts", feature = "full-render"))]
impl OcrFallbackGenerator {
    /// Create a new OCR fallback generator for a page.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - Complete PDF document bytes
    /// * `page_index` - Zero-based page index
    pub fn new(pdf_bytes: Vec<u8>, page_index: usize) -> Self {
        Self {
            pdf_bytes,
            page_index,
            page_render: None,
        }
    }

    /// Generate an SVG receipt with OCR raster fallback.
    ///
    /// Renders the bbox region as a base64-encoded PNG image embedded
    /// in an SVG with `data-source="ocr"` attribute.
    ///
    /// # Arguments
    ///
    /// * `bbox` - Bounding box in PDF points [x0, y0, x1, y1]
    ///
    /// # Returns
    ///
    /// A self-contained SVG document as a string.
    pub fn generate(&mut self, bbox: [f64; 4]) -> Result<String> {
        // Render the page (cached for multiple receipts on same page)
        if self.page_render.is_none() {
            self.page_render = Some(
                render_page_via_pdfium(&self.pdf_bytes, self.page_index, SVG_OCR_RASTER_DPI)
                    .map_err(|e| format!("PDFium render failed: {:?}", e))?,
            );
        }

        let page_render = self.page_render.as_ref().unwrap();

        // Crop the bbox region
        let crop = self.crop_bbox(page_render, bbox)?;

        // Encode as PNG base64
        let png_base64 = self.encode_png_base64(&crop)?;

        // Build SVG with data-source="ocr" attribute
        let width = bbox[2] - bbox[0];
        let height = bbox[3] - bbox[1];

        let mut svg = String::new();
        write!(
            svg,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" data-source="ocr">"#,
            round_coord(width),
            round_coord(height)
        )
        .unwrap();
        write!(
            svg,
            r#"<image href="data:image/png;base64,{}" x="0" y="0" width="{}" height="{}"/>"#,
            png_base64,
            round_coord(width),
            round_coord(height)
        )
        .unwrap();
        svg.push_str("</svg>");

        Ok(svg)
    }

    /// Crop the bbox region from the rendered page.
    ///
    /// Converts PDF bbox coordinates to rendered image pixel coordinates
    /// and extracts the sub-image.
    ///
    /// # Coordinate Transform
    ///
    /// PDF uses bottom-left origin (y increases upward). The rendered image
    /// uses top-left origin (y increases downward). We flip the y-axis:
    /// - image_y = page_height - pdf_y * scale_factor
    fn crop_bbox(&self, page_render: &GrayImage, bbox: [f64; 4]) -> Result<GrayImage> {
        let page_width = page_render.width() as f64;
        let page_height = page_render.height() as f64;

        // Get PDF page dimensions (assumed US Letter at 72 DPI for scale calculation)
        // The actual scale is: scale_factor = dpi / 72.0
        let scale_factor = SVG_OCR_RASTER_DPI as f64 / 72.0;

        // Transform bbox to image coordinates
        let x0 = bbox[0] * scale_factor;
        let y1 = bbox[1] * scale_factor; // bottom y in PDF
        let x1 = bbox[2] * scale_factor;
        let y0 = bbox[3] * scale_factor; // top y in PDF

        // Flip y-axis: image_y = page_height - pdf_y
        let img_y0 = page_height - y0; // top in image (flipped)
        let img_y1 = page_height - y1; // bottom in image (flipped)

        // Clamp to image bounds
        let ix0 = x0.floor().max(0.0) as u32;
        let iy0 = img_y0.floor().max(0.0) as u32;
        let ix1 = x1.ceil().min(page_width) as u32;
        let iy1 = img_y1.ceil().min(page_height) as u32;

        if ix0 >= ix1 || iy0 >= iy1 {
            return Err(format!(
                "Invalid bbox after transform: [{}, {}, {}, {}]",
                ix0, iy0, ix1, iy1
            ));
        }

        let width = ix1 - ix0;
        let height = iy1 - iy0;

        // Extract sub-image
        let crop = imageops::crop(page_render, ix0, iy0, width, height).to_image();

        Ok(crop)
    }

    /// Encode a grayscale image as base64-encoded PNG.
    ///
    /// Uses compression level 6 (default) and strips metadata chunks
    /// to minimize size.
    fn encode_png_base64(&self, image: &GrayImage) -> Result<String> {
        let mut buffer = Vec::new();

        // Encode as PNG with default compression
        let encoder = image::codecs::png::PngEncoder::new_with_quality(
            &mut buffer,
            image::codecs::png::CompressionType::Default,
            image::codecs::png::FilterType::Default,
        );

        encoder
            .write_image(
                image.as_raw().as_slice(),
                image.width(),
                image.height(),
                image::ExtendedColorType::L8,
            )
            .map_err(|e| format!("PNG encoding failed: {}", e))?;

        // Encode to base64 (URL-safe NOT required for data: URLs)
        let base64_string = BASE64_STANDARD.encode(&buffer);

        Ok(base64_string)
    }
}

/// Generate an SVG receipt with OCR fallback (lite mode).
///
/// When `full-render` feature is not available, this function emits
/// a warning and returns an error, indicating that OCR spans should
/// fall back to lite-mode receipts.
///
/// # Arguments
///
/// * `_pdf_bytes` - PDF document bytes (unused in lite mode)
/// * `_page_index` - Page index (unused in lite mode)
/// * `_bbox` - Bounding box (unused in lite mode)
///
/// # Returns
///
/// An error indicating OCR fallback is unavailable without full-render.
#[cfg(not(all(feature = "receipts", feature = "full-render")))]
pub fn generate_ocr_fallback_svg(
    _pdf_bytes: &[u8],
    _page_index: usize,
    _bbox: [f64; 4],
) -> Result<String> {
    Err("SVG receipt for OCR span requires full-render feature; \
         emitting lite-mode receipt instead. Build with --features full-render to enable."
        .to_string())
}

/// Generate an SVG receipt with OCR fallback (full mode).
///
/// Convenience function that creates a generator and produces the SVG.
#[cfg(all(feature = "receipts", feature = "full-render"))]
pub fn generate_ocr_fallback_svg(
    pdf_bytes: &[u8],
    page_index: usize,
    bbox: [f64; 4],
) -> Result<String> {
    let mut generator = OcrFallbackGenerator::new(pdf_bytes.to_vec(), page_index);
    generator.generate(bbox)
}

/// Round a coordinate to 2 decimal places for SVG output.
fn round_coord(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(feature = "receipts", feature = "full-render"))]
    fn test_ocr_fallback_generates_valid_svg() {
        // Minimal PDF with one page
        let pdf_bytes = b"%PDF-1.4\n\
1 0 obj\n\
<<\n/Type /Catalog\n/Pages 2 0 R\n\
>>\n\
endobj\n\
2 0 obj\n\
<<\n/Type /Pages\n/Kids [ 3 0 R ]\n/Count 1\n\
>>\n\
endobj\n\
3 0 obj\n\
<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [ 0 0 612 792 ]\n\
>>\n\
endobj\n\
xref\n\
0 4\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
trailer\n\
<<\n/Size 4\n/Root 1 0 R\n\
>>\n\
startxref\n\
202\n\
%%EOF";

        let mut generator = OcrFallbackGenerator::new(pdf_bytes.to_vec(), 0);

        // Generate SVG for a bbox in the middle of the page
        let bbox = [100.0, 100.0, 300.0, 200.0];
        let svg = generator.generate(bbox);

        assert!(svg.is_ok(), "OCR fallback generation should succeed");
        let svg = svg.unwrap();

        // Verify SVG structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains(r#"data-source="ocr""#));
        assert!(svg.contains("<image"));
        assert!(svg.contains("data:image/png;base64,"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    #[cfg(all(feature = "receipts", feature = "full-render"))]
    fn test_ocr_fallback_includes_base64_png() {
        let pdf_bytes = b"%PDF-1.4\n\
1 0 obj\n\
<<\n/Type /Catalog\n/Pages 2 0 R\n\
>>\n\
endobj\n\
2 0 obj\n\
<<\n/Type /Pages\n/Kids [ 3 0 R ]\n/Count 1\n\
>>\n\
endobj\n\
3 0 obj\n\
<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [ 0 0 612 792 ]\n\
>>\n\
endobj\n\
xref\n\
0 4\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
trailer\n\
<<\n/Size 4\n/Root 1 0 R\n\
>>\n\
startxref\n\
202\n\
%%EOF";

        let mut generator = OcrFallbackGenerator::new(pdf_bytes.to_vec(), 0);
        let bbox = [100.0, 100.0, 300.0, 200.0];
        let svg = generator.generate(bbox).unwrap();

        // Verify base64 data is present and valid
        assert!(svg.contains("data:image/png;base64,"));
        let base64_start = svg.find("data:image/png;base64,").unwrap() + 22;
        let base64_end = svg[base64_start..].find("\"").unwrap();
        let base64_data = &svg[base64_start..base64_start + base64_end];

        // Base64 should be non-empty and contain valid characters
        assert!(!base64_data.is_empty());
        assert!(base64_data
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));

        // Decode base64 to verify it's valid PNG
        let decoded = BASE64_STANDARD.decode(base64_data).unwrap();
        assert!(decoded.len() > 8);
        assert_eq!(
            &decoded[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        ); // PNG signature
    }

    #[test]
    #[cfg(all(feature = "receipts", feature = "full-render"))]
    fn test_ocr_fallback_reuses_page_render() {
        let pdf_bytes = b"%PDF-1.4\n\
1 0 obj\n\
<<\n/Type /Catalog\n/Pages 2 0 R\n\
>>\n\
endobj\n\
2 0 obj\n\
<<\n/Type /Pages\n/Kids [ 3 0 R ]\n/Count 1\n\
>>\n\
endobj\n\
3 0 obj\n\
<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [ 0 0 612 792 ]\n\
>>\n\
endobj\n\
xref\n\
0 4\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
trailer\n\
<<\n/Size 4\n/Root 1 0 R\n\
>>\n\
startxref\n\
202\n\
%%EOF";

        let mut generator = OcrFallbackGenerator::new(pdf_bytes.to_vec(), 0);

        // First render should cache the page
        let bbox1 = [100.0, 100.0, 200.0, 150.0];
        let svg1 = generator.generate(bbox1).unwrap();
        assert!(generator.page_render.is_some());

        // Second render should reuse the cached page
        let bbox2 = [200.0, 200.0, 300.0, 250.0];
        let svg2 = generator.generate(bbox2).unwrap();

        // Both should succeed
        assert!(svg1.contains(r#"data-source="ocr""#));
        assert!(svg2.contains(r#"data-source="ocr""#));
    }

    #[test]
    #[cfg(all(feature = "receipts", feature = "full-render"))]
    fn test_ocr_fallback_convenience_function() {
        let pdf_bytes = b"%PDF-1.4\n\
1 0 obj\n\
<<\n/Type /Catalog\n/Pages 2 0 R\n\
>>\n\
endobj\n\
2 0 obj\n\
<<\n/Type /Pages\n/Kids [ 3 0 R ]\n/Count 1\n\
>>\n\
endobj\n\
3 0 obj\n\
<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [ 0 0 612 792 ]\n\
>>\n\
endobj\n\
xref\n\
0 4\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
trailer\n\
<<\n/Size 4\n/Root 1 0 R\n\
>>\n\
startxref\n\
202\n\
%%EOF";

        let bbox = [100.0, 100.0, 300.0, 200.0];
        let svg = generate_ocr_fallback_svg(pdf_bytes, 0, bbox);

        assert!(svg.is_ok());
        let svg = svg.unwrap();
        assert!(svg.contains(r#"data-source="ocr""#));
    }

    #[test]
    #[cfg(not(all(feature = "receipts", feature = "full-render")))]
    fn test_ocr_fallback_returns_error_without_full_render() {
        let pdf_bytes = b"%PDF-1.4\n...";
        let bbox = [100.0, 100.0, 300.0, 200.0];
        let result = generate_ocr_fallback_svg(pdf_bytes, 0, bbox);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("full-render feature"));
        assert!(err.contains("lite-mode receipt"));
    }

    #[test]
    #[cfg(all(feature = "receipts", feature = "full-render"))]
    fn test_ocr_fallback_invalid_bbox() {
        let pdf_bytes = b"%PDF-1.4\n\
1 0 obj\n\
<<\n/Type /Catalog\n/Pages 2 0 R\n\
>>\n\
endobj\n\
2 0 obj\n\
<<\n/Type /Pages\n/Kids [ 3 0 R ]\n/Count 1\n\
>>\n\
endobj\n\
3 0 obj\n\
<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [ 0 0 612 792 ]\n\
>>\n\
endobj\n\
xref\n\
0 4\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
trailer\n\
<<\n/Size 4\n/Root 1 0 R\n\
>>\n\
startxref\n\
202\n\
%%EOF";

        let mut generator = OcrFallbackGenerator::new(pdf_bytes.to_vec(), 0);

        // Bbox outside page bounds should fail
        let invalid_bbox = [10000.0, 10000.0, 10100.0, 10100.0];
        let result = generator.generate(invalid_bbox);

        assert!(result.is_err());
    }

    #[test]
    #[cfg(all(feature = "receipts", feature = "full-render"))]
    fn test_round_coord() {
        assert_eq!(round_coord(12.345), 12.35);
        assert_eq!(round_coord(12.344), 12.34);
        assert_eq!(round_coord(0.0), 0.0);
        assert_eq!(round_coord(-5.678), -5.68);
    }

    #[test]
    #[cfg(all(feature = "receipts", feature = "full-render"))]
    fn test_ocr_fallback_svg_size_estimate() {
        // Verify that OCR fallback SVG output size is reasonable
        // Plan acceptance criterion: 100 OCR receipts <= 1.5 MB
        let pdf_bytes = b"%PDF-1.4\n\
1 0 obj\n\
<<\n/Type /Catalog\n/Pages 2 0 R\n\
>>\n\
endobj\n\
2 0 obj\n\
<<\n/Type /Pages\n/Kids [ 3 0 R ]\n/Count 1\n\
>>\n\
endobj\n\
3 0 obj\n\
<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [ 0 0 612 792 ]\n\
>>\n\
endobj\n\
xref\n\
0 4\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
trailer\n\
<<\n/Size 4\n/Root 1 0 R\n\
>>\n\
startxref\n\
202\n\
%%EOF";

        let mut generator = OcrFallbackGenerator::new(pdf_bytes.to_vec(), 0);

        // Simulate 100 receipts with typical bbox size
        let receipts: Vec<String> = (0..100)
            .map(|_| {
                let bbox = [100.0, 400.0, 300.0, 430.0]; // 200×30 point bbox
                generator.generate(bbox).unwrap()
            })
            .collect();

        let total_bytes: usize = receipts.iter().map(|r| r.len()).sum();

        // 1.5 MB = 1,572,864 bytes
        assert!(
            total_bytes <= 1_572_864,
            "100 OCR receipts should be <= 1.5 MB, got {} bytes",
            total_bytes
        );

        // Also verify individual receipt size is reasonable
        let avg_size = total_bytes / 100;
        assert!(
            avg_size < 15_000,
            "Average OCR receipt should be < 15 KB, got {} bytes",
            avg_size
        );
    }
}
