//! Layer renderers for the inspector debug viewer.
//!
//! Each renderer generates SVG elements for a specific debugging layer.
//! All renderers follow a common pattern:
//!
//! ```rust
//! pub fn render_<name>(input: &[InputType]) -> Vec<String>
//! ```
//!
//! The returned Vec<String> contains SVG elements that are placed inside
//! a `<g class="layer-<name>">` group in the final output.

pub mod anchors;
pub mod blocks;
pub mod columns;
pub mod confidence_heatmap;
pub mod ocr_regions;
pub mod reading_order;
pub mod spans;
