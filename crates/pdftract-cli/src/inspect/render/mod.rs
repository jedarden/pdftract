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
pub mod colors;
pub mod columns;
pub mod confidence_heatmap;
pub mod mcid;
pub mod ocr_regions;
pub mod reading_order;
pub mod spans;

pub use colors::{
    confidence_to_color, kind_to_color, column_boundary_color,
    // Confidence colors
    RED_LOW, YELLOW_MEDIUM, GREEN_HIGH, GRAY_NEUTRAL,
    // Block kind colors
    BLUE_HEADING, GRAY_PARAGRAPH, TEAL_TABLE, PURPLE_LIST,
    ORANGE_CODE, GRAY_LIGHT_HEADER, BROWN_FIGURE, PINK_CAPTION,
    GRAY_DEFAULT,
    // Special layer colors
    BLUE_READING_ORDER, PURPLE_MCID, BLACK_ANCHOR, CYAN_OCR,
};

use pdftract_core::schema::{BlockJson, SpanJson};
use std::collections::HashMap;

/// A single overlay layer group containing SVG elements.
///
/// Each layer represents a specific debugging view (spans, blocks, columns, etc.)
/// and can be toggled on/off via CSS classes in the frontend inspector.
#[derive(Debug, Clone)]
pub struct LayerGroup {
    /// CSS class name for this layer (e.g., "layer-spans", "layer-blocks")
    pub class: String,
    /// SVG elements for this layer
    pub elements: Vec<String>,
    /// Whether this layer is currently visible
    pub visible: bool,
}

impl LayerGroup {
    /// Create a new layer group.
    pub fn new(class: impl Into<String>, elements: Vec<String>) -> Self {
        Self {
            class: class.into(),
            elements,
            visible: false, // Layers are hidden by default
        }
    }

    /// Create a new visible layer group.
    pub fn new_visible(class: impl Into<String>, elements: Vec<String>) -> Self {
        Self {
            class: class.into(),
            elements,
            visible: true,
        }
    }

    /// Create an empty layer group (no elements to render).
    pub fn empty(class: impl Into<String>) -> Self {
        Self {
            class: class.into(),
            elements: Vec::new(),
            visible: false,
        }
    }

    /// Check if this layer has any elements to render.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Render this layer as an SVG group element.
    ///
    /// Returns an SVG `<g>` element string containing all layer elements.
    pub fn render_as_svg_group(&self) -> String {
        if self.is_empty() {
            format!(r#"<g class="{}"></g>"#, self.class)
        } else {
            let style = if self.visible {
                ""
            } else {
                r#" style="display: none;""#
            };
            format!(
                r#"<g class="{}"{}>{}</g>"#,
                self.class,
                style,
                self.elements.join("")
            )
        }
    }
}

/// Render all 8 overlay layers for a page.
///
/// This function orchestrates all layer renderers and returns the complete
/// set of layer groups for a page. Each layer can be independently toggled
/// via CSS classes in the frontend inspector.
///
/// # Arguments
///
/// * `page_index` - Zero-based page index
/// * `page_number` - One-based page number (for display)
/// * `page_height` - Page height in points (for column rendering)
/// * `spans` - Text spans on the page
/// * `blocks` - Semantic blocks on the page
/// * `reading_order` - Optional reading order (block indices in sequence)
/// * `mcid_map` - Optional MCID mapping (Phase 3.4)
///
/// # Returns
///
/// A vector of `LayerGroup` objects, one for each layer. Layers are returned
/// in a consistent order: spans, blocks, columns, reading_order,
/// confidence_heatmap, ocr_regions, mcid, anchors.
///
/// # Example
///
/// ```rust
/// let layers = render_all(
///     0,  // page_index
///     1,  // page_number
///     792.0,  // page_height
///     &spans,
///     &blocks,
///     &reading_order,
///     &mcid_map,
/// );
///
/// for layer in layers {
///     if !layer.is_empty() {
///         println!("{}", layer.render_as_svg_group());
///     }
/// }
/// ```
pub fn render_all(
    page_index: usize,
    page_number: u32,
    page_height: f32,
    spans: &[SpanJson],
    blocks: &[BlockJson],
    reading_order: &[usize],
    mcid_map: &Option<HashMap<u32, usize>>,
) -> Vec<LayerGroup> {
    let mut layers = Vec::new();

    // 1. Spans layer - thin outline rectangles per span, color-coded by confidence
    if !spans.is_empty() {
        let span_elements = spans::render_spans(spans, blocks);
        layers.push(LayerGroup::new("layer-spans", span_elements));
    } else {
        layers.push(LayerGroup::empty("layer-spans"));
    }

    // 2. Blocks layer - translucent block rects, color-coded by kind
    if !blocks.is_empty() {
        let block_elements = blocks::render_blocks(blocks);
        layers.push(LayerGroup::new("layer-blocks", block_elements));
    } else {
        layers.push(LayerGroup::empty("layer-blocks"));
    }

    // 3. Columns layer - dashed vertical lines at column boundaries
    // Extract column information from spans
    let detected_columns = extract_columns_from_spans(spans, page_height);
    if !detected_columns.is_empty() {
        let column_elements = columns::render_columns(&detected_columns, page_height);
        layers.push(LayerGroup::new("layer-columns", column_elements));
    } else {
        layers.push(LayerGroup::empty("layer-columns"));
    }

    // 4. Reading order layer - curved arrows with numeric labels
    if blocks.len() > 1 && !reading_order.is_empty() {
        let reading_order_elements = reading_order::render_reading_order(blocks, reading_order);
        if !reading_order_elements.is_empty() {
            layers.push(LayerGroup::new("layer-reading-order", reading_order_elements));
        } else {
            layers.push(LayerGroup::empty("layer-reading-order"));
        }
    } else {
        layers.push(LayerGroup::empty("layer-reading-order"));
    }

    // 5. Confidence heatmap layer - per-glyph color cells
    if !spans.is_empty() {
        let heatmap_elements = confidence_heatmap::render_confidence_heatmap(spans);
        if !heatmap_elements.is_empty() {
            layers.push(LayerGroup::new("layer-confidence-heatmap", heatmap_elements));
        } else {
            layers.push(LayerGroup::empty("layer-confidence-heatmap"));
        }
    } else {
        layers.push(LayerGroup::empty("layer-confidence-heatmap"));
    }

    // 6. OCR layer - cyan diagonal-stripe overlay on OCR'd regions
    let ocr_elements = ocr_regions::render_ocr_regions(spans);
    if !ocr_elements.is_empty() {
        layers.push(LayerGroup::new("layer-ocr", ocr_elements));
    } else {
        layers.push(LayerGroup::empty("layer-ocr"));
    }

    // 7. MCID layer - numeric MCID labels for marked-content blocks
    // Only render if MCID map is present and non-empty
    if let Some(map) = mcid_map {
        if !map.is_empty() && !blocks.is_empty() {
            let mcid_elements = mcid::render_mcid_labels(&Some(map.clone()), blocks);
            if !mcid_elements.is_empty() {
                layers.push(LayerGroup::new("layer-mcid", mcid_elements));
            } else {
                layers.push(LayerGroup::empty("layer-mcid"));
            }
        } else {
            layers.push(LayerGroup::empty("layer-mcid"));
        }
    } else {
        layers.push(LayerGroup::empty("layer-mcid"));
    }

    // 8. Anchors layer - block-ID labels at top-left of each block
    if !blocks.is_empty() {
        let anchor_elements = anchors::render_anchors(page_index, page_number, blocks);
        layers.push(LayerGroup::new("layer-anchors", anchor_elements));
    } else {
        layers.push(LayerGroup::empty("layer-anchors"));
    }

    layers
}

/// Extract column information from spans.
///
/// Groups spans by their column field and creates Column objects
/// for rendering column boundaries.
fn extract_columns_from_spans(spans: &[SpanJson], _page_height: f32) -> Vec<pdftract_core::layout::columns::Column> {
    use pdftract_core::layout::columns::Column;
    use std::collections::HashMap;

    // Group spans by column
    let mut column_spans: HashMap<u32, Vec<&SpanJson>> = HashMap::new();

    for span in spans {
        if let Some(col) = span.column {
            column_spans.entry(col).or_default().push(span);
        }
    }

    // Create Column objects from grouped spans
    column_spans
        .into_iter()
        .map(|(col_index, col_spans)| {
            // Find the x-range for this column
            let x0 = col_spans.iter().map(|s| s.bbox[0]).fold(f64::INFINITY, f64::min);
            let x1 = col_spans.iter().map(|s| s.bbox[2]).fold(f64::NEG_INFINITY, f64::max);

            Column {
                index: col_index,
                x_range: [x0 as f32, x1 as f32],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdftract_core::schema::{BlockJson, SpanJson};

    fn make_test_span(text: &str, bbox: [f64; 4], column: Option<u32>) -> SpanJson {
        SpanJson {
            text: text.to_string(),
            bbox,
            font: "Arial".to_string(),
            size: 12.0,
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column,
        }
    }

    fn make_test_block(kind: &str, text: &str, bbox: [f64; 4]) -> BlockJson {
        BlockJson {
            kind: kind.to_string(),
            text: text.to_string(),
            bbox,
            level: None,
            table_index: None,
            spans: vec![],
            receipt: None,
        }
    }

    #[test]
    fn test_layer_group_new() {
        let layer = LayerGroup::new("test-layer", vec!["<rect />".to_string()]);
        assert_eq!(layer.class, "test-layer");
        assert_eq!(layer.elements.len(), 1);
        assert_eq!(layer.visible, false);
    }

    #[test]
    fn test_layer_group_new_visible() {
        let layer = LayerGroup::new_visible("test-layer", vec!["<rect />".to_string()]);
        assert_eq!(layer.visible, true);
    }

    #[test]
    fn test_layer_group_empty() {
        let layer = LayerGroup::empty("empty-layer");
        assert_eq!(layer.class, "empty-layer");
        assert!(layer.is_empty());
        assert_eq!(layer.visible, false);
    }

    #[test]
    fn test_layer_group_is_empty() {
        let empty = LayerGroup::new("empty", vec![]);
        assert!(empty.is_empty());

        let non_empty = LayerGroup::new("non-empty", vec!["<rect />".to_string()]);
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_layer_group_render_as_svg_group() {
        let layer = LayerGroup::new("test-layer", vec![
            r#"<rect x="10" y="20" width="100" height="50" />"#.to_string(),
        ]);

        let svg = layer.render_as_svg_group();
        assert!(svg.contains(r#"class="test-layer""#));
        assert!(svg.contains(r#"style="display: none;""#));
        assert!(svg.contains(r#"<rect x="10" y="20" width="100" height="50" />"#));
    }

    #[test]
    fn test_layer_group_render_as_svg_group_visible() {
        let layer = LayerGroup::new_visible("test-layer", vec![
            r#"<rect x="10" y="20" width="100" height="50" />"#.to_string(),
        ]);

        let svg = layer.render_as_svg_group();
        assert!(svg.contains(r#"class="test-layer""#));
        // Visible layers should NOT have display: none
        assert!(!svg.contains("display: none"));
    }

    #[test]
    fn test_layer_group_render_as_svg_group_empty() {
        let layer = LayerGroup::empty("empty-layer");
        let svg = layer.render_as_svg_group();
        assert_eq!(svg, r#"<g class="empty-layer"></g>"#);
    }

    #[test]
    fn test_render_all_empty_page() {
        let layers = render_all(
            0,  // page_index
            1,  // page_number
            792.0,  // page_height
            &[],
            &[],
            &[],
            &None,
        );

        assert_eq!(layers.len(), 8);

        // All layers should be empty
        for layer in &layers {
            assert!(layer.is_empty());
        }

        // Check layer names are correct
        assert_eq!(layers[0].class, "layer-spans");
        assert_eq!(layers[1].class, "layer-blocks");
        assert_eq!(layers[2].class, "layer-columns");
        assert_eq!(layers[3].class, "layer-reading-order");
        assert_eq!(layers[4].class, "layer-confidence-heatmap");
        assert_eq!(layers[5].class, "layer-ocr");
        assert_eq!(layers[6].class, "layer-mcid");
        assert_eq!(layers[7].class, "layer-anchors");
    }

    #[test]
    fn test_render_all_with_spans_and_blocks() {
        let spans = vec![
            make_test_span("Hello", [100.0, 200.0, 200.0, 220.0], Some(0)),
            make_test_span("World", [100.0, 230.0, 200.0, 250.0], Some(0)),
        ];
        let blocks = vec![
            make_test_block("paragraph", "Hello World", [100.0, 200.0, 200.0, 250.0]),
        ];

        let layers = render_all(
            0, 1, 792.0,
            &spans,
            &blocks,
            &[0],
            &None,
        );

        assert_eq!(layers.len(), 8);

        // Spans layer should have content
        assert!(!layers[0].is_empty());
        assert_eq!(layers[0].class, "layer-spans");

        // Blocks layer should have content
        assert!(!layers[1].is_empty());
        assert_eq!(layers[1].class, "layer-blocks");

        // Columns layer should have content (from span.column)
        assert!(!layers[2].is_empty());
        assert_eq!(layers[2].class, "layer-columns");

        // Anchors layer should have content
        assert!(!layers[7].is_empty());
        assert_eq!(layers[7].class, "layer-anchors");
    }

    #[test]
    fn test_render_all_with_mcid_map() {
        let blocks = vec![
            make_test_block("paragraph", "Block 1", [100.0, 200.0, 300.0, 250.0]),
            make_test_block("paragraph", "Block 2", [100.0, 260.0, 300.0, 310.0]),
        ];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(10, 0);
        mcid_map.insert(20, 1);

        let layers = render_all(
            0, 1, 792.0,
            &[],
            &blocks,
            &[0, 1],
            &Some(mcid_map),
        );

        // MCID layer should have content
        assert!(!layers[6].is_empty());
        assert_eq!(layers[6].class, "layer-mcid");
    }

    #[test]
    fn test_render_all_layers_order() {
        let layers = render_all(0, 1, 792.0, &[], &[], &[], &None);

        // Verify consistent layer order
        let expected_order = vec![
            "layer-spans",
            "layer-blocks",
            "layer-columns",
            "layer-reading-order",
            "layer-confidence-heatmap",
            "layer-ocr",
            "layer-mcid",
            "layer-anchors",
        ];

        for (i, expected) in expected_order.iter().enumerate() {
            assert_eq!(layers[i].class, *expected);
        }
    }

    #[test]
    fn test_extract_columns_from_spans() {
        let spans = vec![
            make_test_span("Col 1", [50.0, 100.0, 200.0, 120.0], Some(0)),
            make_test_span("Col 2", [250.0, 100.0, 400.0, 120.0], Some(1)),
        ];

        let columns = extract_columns_from_spans(&spans, 792.0);

        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].index, 0);
        assert_eq!(columns[1].index, 1);
    }
}
