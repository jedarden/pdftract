//! OCR preprocessing operations (Phase 5.3).
//!
//! This module provides image preprocessing functions that prepare scanned
//! pages for OCR. Operations include contrast normalization, binarization,
//! and noise reduction.

pub mod contrast;
pub mod denoise;

pub use contrast::{histogram_stretch, histogram_stretch_if_needed, PreprocError};
pub use denoise::median_denoise;
