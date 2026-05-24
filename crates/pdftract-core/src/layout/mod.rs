//! Layout analysis for Phase 4.
//!
//! This module implements block-level layout analysis including:
//! - Caption classification (caption.rs)
//! - Line formation (line.rs)
//! - Readability aggregation (readability.rs)
//!
//! Phase 4 organizes extracted text into semantic blocks (paragraphs,
//! headings, figures, captions, etc.) based on spatial and font metrics.

pub mod caption;
pub mod line;
pub mod readability;

pub use caption::{classify_caption, classify_page_captions, Block, PageContext};
pub use line::{compute_baseline, union_bboxes, HasBBox, Line, LineDirection};
pub use readability::{aggregate_page_readability, ScoredSpan};
