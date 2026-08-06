//! Rendering support for pdftract.
//!
//! This module provides rendering capabilities for PDF content, including
//! both PDFium-based high-fidelity rendering and path-based bitmap rasterization.

// Image compositing for scanned pages (Phase 5.2.1)
// Only available when the `ocr` feature is enabled
#[cfg(feature = "ocr")]
pub mod image_compositing;

// PDFium rendering path (Phase 5.2.2) - only available with full-render feature
#[cfg(all(feature = "ocr", feature = "full-render"))]
pub mod pdfium_path;

pub mod path;

pub mod scanline;

// Re-exports
#[cfg(feature = "ocr")]
pub use image_compositing::{collect_image_placements, ImageBytesRef, ImagePlacement, ImageSource, ImageXObject, InlineImageHeader};

#[cfg(all(feature = "ocr", feature = "full-render"))]
pub use pdfium_path::has_full_render;

pub use path::{CurrentPath, PathCommand, Point};
pub use scanline::{fill_polygon, fill_polygon_from_tuples, Bitmap, Edge};
