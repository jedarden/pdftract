//! PDFium-based page rendering path (Phase 5.2.2).
//!
//! This module implements high-fidelity page rendering using PDFium,
//! which correctly handles:
//! - Overlapping images with proper blend modes
//! - Image masks and soft masks
//! - Transparency and alpha blending
//! - Shading patterns
//! - Complex color spaces
//!
//! # Feature Gate
//!
//! This module is only available when both `ocr` and `full-render` features are enabled.

use crate::diagnostics::{Diagnostic, DiagCode};
use image::{GrayImage, Luma};
use pdfium_render::prelude::*;
use std::sync::{Arc, Mutex};
use tracing::{debug, warn};
use std::thread::LocalKey;

/// Result type for PDFium rendering operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// Thread-local PDFium instance holder with lazy initialization.
///
/// PDFium initialization is expensive (~50-100ms per instance), so we
/// maintain one instance per thread. The `thread_local!` macro ensures
/// each thread gets its own instance, avoiding synchronization overhead.
///
/// This uses `Option` to handle initialization failures gracefully.
struct ThreadLocalPdfium {
    instance: Option<Arc<Pdfium>>,
    init_failed: bool,
}

impl ThreadLocalPdfium {
    fn new() -> Self {
        Self {
            instance: None,
            init_failed: false,
        }
    }

    fn get_or_init(&mut self) -> Option<Arc<Pdfium>> {
        if self.init_failed {
            return None;
        }

        if self.instance.is_none() {
            // Try to bind to the system PDFium library
            // This returns a Result, so we can handle errors gracefully
            match Pdfium::bind_to_system_library() {
                Ok(bindings) => {
                    debug!("PDFium initialized successfully");
                    let pdfium = Pdfium::new(bindings);
                    self.instance = Some(Arc::new(pdfium));
                }
                Err(e) => {
                    warn!("PDFium initialization failed: {:?}", e);
                    self.init_failed = true;
                    return None;
                }
            }
        }

        self.instance.clone()
    }
}

thread_local! {
    static PDFIUM_INSTANCE: Mutex<ThreadLocalPdfium> = Mutex::new(ThreadLocalPdfium::new());
}

/// Get the thread-local PDFium instance, if available.
///
/// Returns `None` if PDFium initialization failed (e.g., native library not found).
fn get_pdfium() -> Option<Arc<Pdfium>> {
    PDFIUM_INSTANCE.try_with(|instance| {
        let mut guard = instance.lock().unwrap();
        guard.get_or_init()
    }).ok().flatten()
}

/// Check if the full-render feature is available at runtime.
///
/// This function attempts to access PDFium and returns true if successful.
/// It's used by serve mode to validate `full_render` form-field requests.
///
/// # Returns
///
/// `true` if PDFium is available and can render pages, `false` otherwise.
pub fn has_full_render() -> bool {
    get_pdfium().is_some()
}

/// Render a PDF page using PDFium.
///
/// This function:
/// 1. Loads the PDF document from bytes
/// 2. Opens the specified page
/// 3. Renders the page at the specified DPI
/// 4. Converts the result to a grayscale image
///
/// # Arguments
///
/// * `pdf_bytes` - The complete PDF document bytes
/// * `page_index` - Zero-based page index to render
/// * `dpi` - Resolution in dots per inch (default 300)
///
/// # Returns
///
/// The rendered grayscale image, or diagnostics if rendering fails.
///
/// # Errors
///
/// Returns diagnostics if:
/// - PDFium is not available (full-render feature not compiled or initialization failed)
/// - PDFium fails to load the document
/// - The page index is out of bounds
/// - Rendering fails
pub fn render_page_via_pdfium(
    pdf_bytes: &[u8],
    page_index: usize,
    dpi: u32,
) -> Result<GrayImage> {
    let mut diagnostics = Vec::new();

    // Get the thread-local PDFium instance
    let pdfium = match get_pdfium() {
        Some(instance) => instance,
        None => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructMissingKey,
                "PDFium not available (full-render feature not compiled or initialization failed)",
            ));
            return Err(diagnostics);
        }
    };

    // Load the PDF document from memory
    let document = match pdfium.load_pdf_from_byte_slice(pdf_bytes, None) {
        Ok(doc) => doc,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("Failed to load PDF with PDFium: {:?}", e),
            ));
            return Err(diagnostics);
        }
    };

    // Check page count
    let page_count = document.pages().len();
    if page_index as i32 >= page_count {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructMissingKey,
            format!("Page index {} out of bounds (document has {} pages)", page_index, page_count),
        ));
        return Err(diagnostics);
    }

    // Open the page
    let page = match document.pages().get(page_index as i32) {
        Ok(p) => p,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructMissingKey,
                format!("Failed to open page {}: {:?}", page_index, e),
            ));
            return Err(diagnostics);
        }
    };

    // Get page dimensions in points (1 point = 1/72 inch)
    let page_width = page.width().value;
    let page_height = page.height().value;

    // Calculate rendering dimensions based on DPI
    // PDF uses 72 points per inch as the base unit
    let scale_factor = dpi as f32 / 72.0;
    let render_width = (page_width * scale_factor).ceil() as i32;
    let render_height = (page_height * scale_factor).ceil() as i32;

    // Create render configuration
    let render_config = PdfRenderConfig::new()
        .set_target_width(render_width)
        .set_target_height(render_height);

    // Render the page to a bitmap using the config
    let bitmap = match page.render_with_config(&render_config) {
        Ok(bitmap) => bitmap,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::ImgUnsupportedFormat,
                format!("Failed to render page with PDFium: {:?}", e),
            ));
            return Err(diagnostics);
        }
    };

    // Convert the bitmap to an image::DynamicImage
    // The as_image() method returns a DynamicImage
    let dynamic_image = match bitmap.as_image() {
        Ok(img) => img,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::ImgUnsupportedFormat,
                format!("Failed to convert PDFium bitmap to image: {:?}", e),
            ));
            return Err(diagnostics);
        }
    };

    // Convert to grayscale using luminance
    let gray_image = dynamic_image.to_luma8();

    Ok(gray_image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "full-render")]
    fn test_has_full_render() {
        // When full-render feature is enabled, this should return true
        // if PDFium native library is available
        let result = has_full_render();
        // We don't assert true/false because it depends on runtime environment
        // Just verify it doesn't panic
        let _ = result;
    }

    #[test]
    #[cfg(feature = "full-render")]
    fn test_render_minimal_pdf() {
        // Create a minimal valid PDF
        // This is a minimal PDF with one empty page
        let minimal_pdf = b"%PDF-1.4\n\
1 0 obj\n\
<<\n/Type /Catalog\n/Pages 2 0 R\n\
>>\n\
endobj\n\
2 0 obj\n\
<<\n/Type /Pages\n/Kids [ 3 0 R ]\n/Count 1\n\
>>\n\
endobj\n\
3 0 obj\n\
<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [ 0 0 612 792 ]\n/Contents 4 0 R\n\
>>\n\
endobj\n\
4 0 obj\n\
<<\n/Length 44\n\
>>\n\
stream\n\
BT\n/F1 12 Tf\n100 700 Td\n(Test) Tj\nET\n\
endstream\n\
endobj\n\
xref\n\
0 5\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
0000000214 00000 n\n\
trailer\n\
<<\n/Size 5\n/Root 1 0 R\n\
>>\n\
startxref\n\
310\n\
%%EOF";

        // Try to render the page
        // This test may fail if PDFium native library is not available
        let result = render_page_via_pdfium(minimal_pdf, 0, 72);

        // If PDFium is not available, we expect an error
        if !has_full_render() {
            assert!(result.is_err());
        } else {
            // If PDFium is available, we expect success
            assert!(result.is_ok());
        }
    }

    #[test]
    #[cfg(feature = "full-render")]
    fn test_render_invalid_page_index() {
        let minimal_pdf = b"%PDF-1.4\n\
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

        // If PDFium is not available, this test should be skipped
        if !has_full_render() {
            return;
        }

        let result = render_page_via_pdfium(minimal_pdf, 99, 72);
        assert!(result.is_err());
        let diags = result.unwrap_err();
        assert!(diags.iter().any(|d| d.code == DiagCode::StructMissingKey));
    }
}
