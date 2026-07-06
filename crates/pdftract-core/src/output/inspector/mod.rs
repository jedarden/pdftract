//! SVG overlay layer renderers for the PDF inspector UI.
//!
//! This module implements the 8 toggleable overlay layers that visualize
//! extraction metadata in the inspector web interface:
//! - Spans (confidence-colored outlines)
//! - Blocks (kind-colored translucent fills)
//! - Columns (dashed vertical boundary lines)
//! - Reading order (curved numbered arrows)
//! - Confidence heatmap (per-glyph color cells)
//! - OCR regions (cyan diagonal stripes)
//! - MCID labels (marked-content identifiers)
//! - Anchor labels (block ID for Markdown links)
//!
//! Per plan section 7.9.5 (lines 2852-2863), each layer is independently
//! toggleable via CSS classes, and all 8 layer groups are present in every
//! page SVG output (CSS-only visibility toggling, no re-render needed).

pub mod colors;
pub mod layers;

pub use layers::{render_all, LayerGroup};
