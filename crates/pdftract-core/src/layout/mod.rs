//! Layout analysis for Phase 4.
//!
//! This module implements block-level layout analysis including:
//! - Caption classification (caption.rs)
//!
//! Phase 4 organizes extracted text into semantic blocks (paragraphs,
//! headings, figures, captions, etc.) based on spatial and font metrics.

pub mod caption;

pub use caption::{Block, PageContext, classify_caption, classify_page_captions};
