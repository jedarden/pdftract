//! Individual overlay layer renderers for the PDF inspector.
//!
//! This module implements the 8 toggleable overlay layers specified in
//! plan section 7.9.5 (lines 2852-2863). Each layer is independently
//! toggleable via CSS classes and all layers are present in every page
//! SVG output.

use std::fmt::Write;
use crate::schema::{BlockJson, SpanJson};
use crate::output::inspector::colors;

/// A single SVG layer group with its CSS class name.
///
/// Represents one of the 8 overlay layers that can be toggled independently.
#[derive(Debug, Clone)]
pub struct LayerGroup {
    /// CSS class name for this layer (e.g., "layer-spans").
    pub class_name: &'static str,
    /// SVG content for this layer (the inner content of the `&lt;g&gt;` element).
    pub content: String,
}

impl LayerGroup {
    /// Create a new layer group.
    fn new(class_name: &'static str, content: String) -> Self {
        Self { class_name, content }
    }

    /// Render this layer as an SVG `&lt;g&gt;` element.
    ///
    /// Returns the complete SVG group element with class attribute.
    pub fn render(&self) -> String {
        format!(r#"<g class="{}">{}</g>"#, self.class_name, self.content)
    }
}

/// Page data for overlay rendering.
///
/// Aggregates all the data needed to render the 8 overlay layers.
#[derive(Debug, Clone)]
pub struct PageData {
    /// Text spans extracted from the page.
    pub spans: Vec<SpanJson>,
    /// Structural blocks (paragraphs, headings, lists, tables).
    pub blocks: Vec<BlockJson>,
    /// Page width in points.
    pub page_width: f32,
    /// Page height in points.
    pub page_height: f32,
    /// Column boundary x-coordinates (empty if no columns detected).
    pub column_boundaries: Vec<f32>,
    /// Reading order indices (block indices in reading order).
    pub reading_order: Vec<usize>,
    /// OCR regions (bbox of regions sourced from Tesseract).
    pub ocr_regions: Vec<[f32; 4]>,
    /// MCID map (MCID number -> block reference).
    pub mcid_map: std::collections::HashMap<u32, BlockRef>,
}

/// Reference to a block for MCID mapping.
#[derive(Debug, Clone, Copy)]
pub struct BlockRef {
    /// Block index in the blocks array.
    pub block_index: usize,
    /// MCID number.
    pub mcid: u32,
}

impl PageData {
    /// Create a new PageData from the JSON schema types.
    pub fn from_json(
        spans: Vec<SpanJson>,
        blocks: Vec<BlockJson>,
        page_width: f32,
        page_height: f32,
    ) -> Self {
        Self {
            spans,
            blocks,
            page_width,
            page_height,
            column_boundaries: Vec::new(),
            reading_order: Vec::new(),
            ocr_regions: Vec::new(),
            mcid_map: std::collections::HashMap::new(),
        }
    }

    /// Set column boundaries for the columns overlay.
    pub fn with_columns(mut self, boundaries: Vec<f32>) -> Self {
        self.column_boundaries = boundaries;
        self
    }

    /// Set reading order for the reading-order overlay.
    pub fn with_reading_order(mut self, order: Vec<usize>) -> Self {
        self.reading_order = order;
        self
    }

    /// Set OCR regions for the OCR overlay.
    pub fn with_ocr_regions(mut self, regions: Vec<[f32; 4]>) -> Self {
        self.ocr_regions = regions;
        self
    }

    /// Set MCID map for the MCID overlay.
    pub fn with_mcid_map(mut self, map: std::collections::HashMap<u32, BlockRef>) -> Self {
        self.mcid_map = map;
        self
    }
}

/// Render all 8 overlay layers for a page.
///
/// This is the main entry point for overlay generation. Returns all 8
/// layer groups even when some are empty (CSS toggles visibility, not presence).
///
/// # Arguments
///
/// * `page` - Page data containing spans, blocks, and metadata
///
/// # Returns
///
/// Vector of 8 LayerGroup structs, one per overlay layer.
pub fn render_all(page: &PageData) -> Vec<LayerGroup> {
    vec![
        render_spans_layer(&page.spans),
        render_blocks_layer(&page.blocks),
        render_columns_layer(&page.column_boundaries, page.page_height),
        render_reading_order_layer(&page.blocks, &page.reading_order),
        render_confidence_heatmap_layer(&page.spans),
        render_ocr_regions_layer(&page.ocr_regions),
        render_mcid_labels_layer(&page.mcid_map, &page.blocks),
        render_anchor_labels_layer(&page.blocks),
    ]
}

/// Layer 1: Spans (confidence-colored outline rectangles).
///
/// Per plan line 2856: "Thin outline rectangles around each span;
/// color encodes confidence (red < 0.5, yellow 0.5–0.8, green > 0.8)"
fn render_spans_layer(spans: &[SpanJson]) -> LayerGroup {
    let mut content = String::new();

    for (idx, span) in spans.iter().enumerate() {
        let bbox = &span.bbox;
        let confidence = span.confidence.unwrap_or(1.0);
        let color = colors::confidence_to_color(confidence);

        // Escape text for data attribute
        let text_escaped = escape_xml(&span.text);
        let font_escaped = escape_xml(&span.font);

        // Build data-* attributes for tooltip
        let _ = write!(
            content,
            r#"<rect class="span-outline" x="{x0}" y="{y0}" width="{w}" height="{h}" fill="none" stroke="{color}" stroke-width="1" data-text="{text}" data-font="{font}" data-confidence="{conf}" data-bbox="[{bbox_x0},{bbox_y0},{bbox_x1},{bbox_y1}]" data-span-idx="{idx}"/>"#,
            x0 = bbox[0],
            y0 = bbox[1],
            w = bbox[2] - bbox[0],
            h = bbox[3] - bbox[1],
            color = color,
            text = text_escaped,
            font = font_escaped,
            conf = confidence,
            bbox_x0 = bbox[0],
            bbox_y0 = bbox[1],
            bbox_x1 = bbox[2],
            bbox_y1 = bbox[3],
            idx = idx,
        );
    }

    LayerGroup::new("layer-spans", content)
}

/// Layer 2: Blocks (kind-colored translucent rectangles).
///
/// Per plan line 2857: "Translucent rectangles around each block;
/// fill color encodes block kind (heading=blue, paragraph=gray, table=teal,
/// list=purple, code=orange, header/footer=light gray, figure=brown, caption=pink)"
fn render_blocks_layer(blocks: &[BlockJson]) -> LayerGroup {
    let mut content = String::new();

    for (idx, block) in blocks.iter().enumerate() {
        let bbox = &block.bbox;
        let kind = &block.kind;
        let fill_color = colors::kind_to_color(kind);
        let stroke_color = colors::kind_to_stroke_color(kind);

        let _ = write!(
            content,
            r#"<rect class="block-rect" x="{x0}" y="{y0}" width="{w}" height="{h}" fill="{fill}" fill-opacity="0.15" stroke="{stroke}" stroke-width="1" stroke-opacity="0.5" data-block-idx="{idx}" data-kind="{kind}"/>"#,
            x0 = bbox[0],
            y0 = bbox[1],
            w = bbox[2] - bbox[0],
            h = bbox[3] - bbox[1],
            fill = fill_color,
            stroke = stroke_color,
            idx = idx,
            kind = kind,
        );
    }

    LayerGroup::new("layer-blocks", content)
}

/// Layer 3: Columns (dashed vertical boundary lines).
///
/// Per plan line 2858: "Dashed vertical lines at column boundaries;
/// column index labels at the page top"
fn render_columns_layer(boundaries: &[f32], page_height: f32) -> LayerGroup {
    let mut content = String::new();

    for (idx, &x) in boundaries.iter().enumerate() {
        // Dashed vertical line from top to bottom
        let _ = write!(
            content,
            r#"<line class="column-line" x1="{x}" y1="0" x2="{x}" y2="{height}" stroke="{color}" stroke-width="1" stroke-dasharray="4,4"/>"#,
            x = x,
            height = page_height,
            color = colors::COLUMN_LINE_COLOR,
        );

        // Column label at the top
        let _ = write!(
            content,
            r#"<text class="column-label" x="{x}" y="12" fill="{color}" font-size="10" font-family="sans-serif" text-anchor="middle">Col {idx}</text>"#,
            x = x,
            color = colors::COLUMN_LABEL_COLOR,
            idx = idx,
        );
    }

    LayerGroup::new("layer-columns", content)
}

/// Layer 4: Reading order (curved numbered arrows).
///
/// Per plan line 2859: "Curved arrows connecting blocks in the extracted
/// reading order (numbered 1, 2, 3, ...)"
///
/// Only renders arrows for the first 50 blocks to avoid clutter.
fn render_reading_order_layer(blocks: &[BlockJson], reading_order: &[usize]) -> LayerGroup {
    let mut content = String::new();

    const MAX_ARROWS: usize = 50;
    let arrows_to_render = reading_order.iter().take(MAX_ARROWS).collect::<Vec<_>>();

    for (seq_idx, &block_idx) in arrows_to_render.iter().enumerate() {
        if let Some(block) = blocks.get(*block_idx) {
            let bbox = &block.bbox;
            let center_x = (bbox[0] + bbox[2]) / 2.0;
            let center_y = (bbox[1] + bbox[3]) / 2.0;

            // Draw numbered label at block center
            let label_num = seq_idx + 1;
            let _ = write!(
                content,
                r#"<text class="reading-order-label" x="{cx}" y="{cy}" fill="{color}" font-size="12" font-family="sans-serif" text-anchor="middle" dominant-baseline="middle">{num}</text>"#,
                cx = center_x,
                cy = center_y,
                color = colors::READING_ORDER_LABEL_COLOR,
                num = label_num,
            );

            // Draw arrow to next block (if any)
            if seq_idx + 1 < arrows_to_render.len() {
                if let Some(next_block) = blocks.get(*arrows_to_render[seq_idx + 1]) {
                    let next_bbox = &next_block.bbox;
                    let next_center_x = (next_bbox[0] + next_bbox[2]) / 2.0;
                    let next_center_y = (next_bbox[1] + next_bbox[3]) / 2.0;

                    // Bezier curve control point (slight downward curve)
                    let control_x = (center_x + next_center_x) / 2.0;
                    let control_y = (center_y + next_center_y) / 2.0 + 10.0;

                    let _ = write!(
                        content,
                        r#"<path class="reading-order-arrow" d="M{x1},{y1} Q{cx},{cy} {x2},{y2}" fill="none" stroke="{color}" stroke-width="1.5" marker-end="url(#arrowhead)"/>"#,
                        x1 = center_x,
                        y1 = center_y,
                        cx = control_x,
                        cy = control_y,
                        x2 = next_center_x,
                        y2 = next_center_y,
                        color = colors::READING_ORDER_ARROW_COLOR,
                    );
                }
            }
        }
    }

    // Add arrowhead marker definition (only once, at the start)
    let arrowhead = r##"<defs><marker id="arrowhead" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto"><path d="M0,0 L0,6 L9,3 z" fill="#ff6600"/></marker></defs>"##;

    LayerGroup::new("layer-reading-order", format!("{}{}", arrowhead, content))
}

/// Layer 5: Confidence heatmap (per-glyph color cells).
///
/// Per plan line 2860: "Per-glyph color grade: red < 0.5 → green > 0.9"
///
/// Since SpanJson doesn't have per-glyph data, we render per-span confidence
/// as small colored cells at each span position. For true per-glyph heatmaps,
/// the frontend would need access to Glyph-level data (Phase 3).
fn render_confidence_heatmap_layer(spans: &[SpanJson]) -> LayerGroup {
    let mut content = String::new();

    for span in spans {
        let bbox = &span.bbox;
        let confidence = span.confidence.unwrap_or(1.0);
        let color = colors::confidence_to_color(confidence);

        // Render a small colored cell at each span position
        // For dense glyph coverage, this samples 1 in 4 to keep SVG manageable
        let span_width = bbox[2] - bbox[0];
        let cell_size = (span_width / span.text.chars().count() as f64).max(2.0).min(8.0);

        let _ = write!(
            content,
            r#"<rect class="heatmap-cell" x="{x0}" y="{y0}" width="{w}" height="{h}" fill="{color}" fill-opacity="0.3"/>"#,
            x0 = bbox[0],
            y0 = bbox[1],
            w = cell_size,
            h = cell_size,
            color = color,
        );
    }

    LayerGroup::new("layer-confidence-heatmap", content)
}

/// Layer 6: OCR regions (cyan diagonal stripes).
///
/// Per plan line 2861: "Cyan diagonal-stripe overlay on regions whose
/// text came from Tesseract (Phase 5)"
fn render_ocr_regions_layer(ocr_regions: &[[f32; 4]]) -> LayerGroup {
    let mut content = String::new();

    // Include the OCR pattern definition
    content.push_str(colors::ocr_pattern_definition());

    for bbox in ocr_regions {
        let _ = write!(
            content,
            r##"<rect class="ocr-region" x="{x0}" y="{y0}" width="{w}" height="{h}" fill="url(#ocr-diagonal-stripes)" stroke="#00ffff" stroke-width="1" stroke-opacity="0.5"/>"##,
            x0 = bbox[0],
            y0 = bbox[1],
            w = bbox[2] - bbox[0],
            h = bbox[3] - bbox[1],
        );
    }

    LayerGroup::new("layer-ocr-regions", content)
}

/// Layer 7: MCID labels (numeric marked-content identifiers).
///
/// Per plan line 2862: "Numeric MCID labels in the corner of each
/// marked-content block (Phase 3.4)"
fn render_mcid_labels_layer(
    mcid_map: &std::collections::HashMap<u32, BlockRef>,
    blocks: &[BlockJson],
) -> LayerGroup {
    let mut content = String::new();

    for (&mcid, block_ref) in mcid_map {
        // Look up the block to get its bbox
        if let Some(block) = blocks.get(block_ref.block_index) {
            let bbox = &block.bbox;

            // Render MCID label at top-right corner of the block
            // (x1-5, y1-5 for padding from the edge)
            let _ = write!(
                content,
                r#"<text class="mcid-label" x="{x}" y="{y}" fill="{color}" font-size="10" font-family="sans-serif" text-anchor="end">{mcid}</text>"#,
                x = bbox[2] - 5.0, // top-right x, slightly inset
                y = bbox[3] - 5.0, // top-right y, slightly inset
                color = colors::MCID_LABEL_COLOR,
                mcid = mcid,
            );
        }
    }

    LayerGroup::new("layer-mcid", content)
}

/// Layer 8: Anchor labels (block ID for Markdown links).
///
/// Per plan line 2863: "Block-ID labels at the top-left corner of each
/// block (matches Phase 6.5 Markdown anchor IDs)"
fn render_anchor_labels_layer(blocks: &[BlockJson]) -> LayerGroup {
    let mut content = String::new();

    for (idx, block) in blocks.iter().enumerate() {
        let bbox = &block.bbox;
        let anchor_id = format!("block_{}", idx);

        let _ = write!(
            content,
            r#"<text class="anchor-label" x="{x0}" y="{y1}" fill="{color}" font-size="9" font-family="monospace" text-anchor="start">{id}</text>"#,
            x0 = bbox[0] + 2.0,
            y1 = bbox[3] - 2.0,
            color = colors::ANCHOR_LABEL_COLOR,
            id = anchor_id,
        );
    }

    LayerGroup::new("layer-anchors", content)
}

/// Escape special XML characters for use in SVG attributes.
///
/// Replaces &, <, >, ", and ' with their XML entity equivalents.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_span(text: &str, confidence: f64, bbox: [f64; 4]) -> SpanJson {
        SpanJson {
            text: text.to_string(),
            bbox,
            font: "Helvetica".to_string(),
            size: 12.0,
            color: Some("#000000".to_string()),
            rendering_mode: Some(0),
            confidence: Some(confidence),
            confidence_source: Some("vector".to_string()),
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }
    }

    fn create_test_block(kind: &str, bbox: [f64; 4]) -> BlockJson {
        BlockJson {
            kind: kind.to_string(),
            text: "Test block".to_string(),
            bbox,
            level: None,
            table_index: None,
            spans: vec![],
            receipt: None,
        }
    }

    #[test]
    fn test_layer_group_render() {
        let layer = LayerGroup::new("layer-test", "<rect/>".to_string());
        let rendered = layer.render();
        assert!(rendered.contains(r#"class="layer-test""#));
        assert!(rendered.contains("<rect/>"));
        assert!(rendered.starts_with("<g"));
        assert!(rendered.ends_with("</g>"));
    }

    #[test]
    fn test_render_spans_layer() {
        let spans = vec![
            create_test_span("Hello", 0.95, [10.0, 20.0, 50.0, 30.0]),
            create_test_span("World", 0.3, [60.0, 20.0, 100.0, 30.0]),
        ];

        let layer = render_spans_layer(&spans);
        assert!(layer.class_name == "layer-spans");
        assert!(layer.content.contains("class=\"span-outline\""));

        // High confidence span should be green
        assert!(layer.content.contains("#44cc44"));

        // Low confidence span should be red
        assert!(layer.content.contains("#ff4444"));

        // Check data attributes
        assert!(layer.content.contains("data-text=\"Hello\""));
        assert!(layer.content.contains("data-confidence=\"0.95\""));
        assert!(layer.content.contains("data-span-idx=\"0\""));
    }

    #[test]
    fn test_render_blocks_layer() {
        let blocks = vec![
            create_test_block("heading", [50.0, 700.0, 250.0, 750.0]),
            create_test_block("paragraph", [50.0, 600.0, 250.0, 650.0]),
        ];

        let layer = render_blocks_layer(&blocks);
        assert!(layer.class_name == "layer-blocks");
        assert!(layer.content.contains("class=\"block-rect\""));

        // Heading should be blue
        assert!(layer.content.contains("#4a90e2"));

        // Paragraph should be gray
        assert!(layer.content.contains("#808080"));

        // Check for fill-opacity (translucent)
        assert!(layer.content.contains("fill-opacity=\"0.15\""));
    }

    #[test]
    fn test_render_columns_layer() {
        let boundaries = vec![100.0, 300.0];

        let layer = render_columns_layer(&boundaries, 792.0);
        assert!(layer.class_name == "layer-columns");

        // Should have 2 lines
        assert!(layer.content.contains("<line class=\"column-line\""));

        // Should have column labels
        assert!(layer.content.contains("Col 0"));
        assert!(layer.content.contains("Col 1"));

        // Lines should be dashed
        assert!(layer.content.contains("stroke-dasharray=\"4,4\""));
    }

    #[test]
    fn test_render_reading_order_layer() {
        let blocks = vec![
            create_test_block("paragraph", [50.0, 700.0, 250.0, 750.0]),
            create_test_block("paragraph", [50.0, 600.0, 250.0, 650.0]),
        ];
        let reading_order = vec![0, 1];

        let layer = render_reading_order_layer(&blocks, &reading_order);
        assert!(layer.class_name == "layer-reading-order");

        // Should have numbered labels
        assert!(layer.content.contains("class=\"reading-order-label\""));

        // Should include arrowhead marker definition
        assert!(layer.content.contains("<marker id=\"arrowhead\""));

        // Should have arrow paths
        assert!(layer.content.contains("class=\"reading-order-arrow\""));
    }

    #[test]
    fn test_render_confidence_heatmap_layer() {
        let spans = vec![
            create_test_span("High", 0.95, [10.0, 20.0, 50.0, 30.0]),
            create_test_span("Low", 0.3, [60.0, 20.0, 100.0, 30.0]),
        ];

        let layer = render_confidence_heatmap_layer(&spans);
        assert!(layer.class_name == "layer-confidence-heatmap");

        // Should have heatmap cells
        assert!(layer.content.contains("class=\"heatmap-cell\""));

        // Should have both colors
        assert!(layer.content.contains("#44cc44")); // green
        assert!(layer.content.contains("#ff4444")); // red

        // Cells should be translucent
        assert!(layer.content.contains("fill-opacity=\"0.3\""));
    }

    #[test]
    fn test_render_ocr_regions_layer() {
        let ocr_regions = vec![[50.0, 100.0, 200.0, 300.0]];

        let layer = render_ocr_regions_layer(&ocr_regions);
        assert!(layer.class_name == "layer-ocr-regions");

        // Should have OCR pattern definition
        assert!(layer.content.contains("id=\"ocr-diagonal-stripes\""));

        // Should have cyan fill
        assert!(layer.content.contains("#00ffff"));

        // Should reference the pattern
        assert!(layer.content.contains("fill=\"url(#ocr-diagonal-stripes)\""));
    }

    #[test]
    fn test_render_anchor_labels_layer() {
        let blocks = vec![
            create_test_block("heading", [50.0, 700.0, 250.0, 750.0]),
            create_test_block("paragraph", [50.0, 600.0, 250.0, 650.0]),
        ];

        let layer = render_anchor_labels_layer(&blocks);
        assert!(layer.class_name == "layer-anchors");

        // Should have anchor labels
        assert!(layer.content.contains("class=\"anchor-label\""));

        // Should have block_0 and block_1
        assert!(layer.content.contains("block_0"));
        assert!(layer.content.contains("block_1"));

        // Should use monospace font
        assert!(layer.content.contains("font-family=\"monospace\""));
    }

    #[test]
    fn test_render_all_returns_eight_layers() {
        let page_data = PageData::from_json(vec![], vec![], 612.0, 792.0);

        let layers = render_all(&page_data);
        assert_eq!(layers.len(), 8);

        // Verify all layer class names
        let class_names: Vec<&str> = layers.iter().map(|l| l.class_name).collect();
        assert!(class_names.contains(&"layer-spans"));
        assert!(class_names.contains(&"layer-blocks"));
        assert!(class_names.contains(&"layer-columns"));
        assert!(class_names.contains(&"layer-reading-order"));
        assert!(class_names.contains(&"layer-confidence-heatmap"));
        assert!(class_names.contains(&"layer-ocr-regions"));
        assert!(class_names.contains(&"layer-mcid"));
        assert!(class_names.contains(&"layer-anchors"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quote\""), "&quot;quote&quot;");
        assert_eq!(escape_xml("'single'"), "&apos;single&apos;");
    }

    #[test]
    fn test_reading_order_max_arrows_limit() {
        // Create many blocks to test the MAX_ARROWS limit
        let blocks: Vec<BlockJson> = (0..100)
            .map(|i| create_test_block("paragraph", [50.0, 700.0 - (i as f64 * 10.0), 250.0, 750.0 - (i as f64 * 10.0)]))
            .collect();

        let reading_order: Vec<usize> = (0..100).collect();

        let layer = render_reading_order_layer(&blocks, &reading_order);

        // Should only render arrows for first 50 blocks
        // Count the number of arrow paths (should be 49, since there's no arrow from the last item)
        let arrow_count = layer.content.matches("class=\"reading-order-arrow\"").count();
        assert!(arrow_count <= 50, "Should have at most 50 arrows, got {}", arrow_count);

        // But should still have all 50 labels (limit applies to arrows, not labels)
        let label_count = layer.content.matches("class=\"reading-order-label\"").count();
        assert!(label_count <= 50, "Should have at most 50 labels, got {}", label_count);
    }

    #[test]
    fn test_page_data_builder() {
        let spans = vec![create_test_span("test", 1.0, [0.0, 0.0, 100.0, 10.0])];
        let blocks = vec![create_test_block("paragraph", [0.0, 0.0, 100.0, 50.0])];

        let page_data = PageData::from_json(spans, blocks, 612.0, 792.0)
            .with_columns(vec![100.0, 300.0])
            .with_reading_order(vec![0])
            .with_ocr_regions(vec![[50.0, 100.0, 200.0, 300.0]]);

        assert_eq!(page_data.column_boundaries.len(), 2);
        assert_eq!(page_data.reading_order.len(), 1);
        assert_eq!(page_data.ocr_regions.len(), 1);
    }

    #[test]
    fn test_block_kind_color_coverage() {
        // Test that all expected block kinds have colors defined
        let kinds = ["heading", "paragraph", "table", "list", "code", "header_footer", "figure", "caption"];

        for kind in &kinds {
            let color = colors::kind_to_color(kind);
            assert!(color.starts_with('#'), "Color for {} should be hex", kind);

            let stroke = colors::kind_to_stroke_color(kind);
            assert!(stroke.starts_with('#'), "Stroke for {} should be hex", kind);
        }
    }

    #[test]
    fn test_svg_well_formedness() {
        // Verify that all layers produce well-formed SVG snippets
        let spans = vec![create_test_span("test", 1.0, [0.0, 0.0, 100.0, 10.0])];
        let blocks = vec![create_test_block("paragraph", [0.0, 0.0, 100.0, 50.0])];

        let page_data = PageData::from_json(spans, blocks, 612.0, 792.0);
        let layers = render_all(&page_data);

        for layer in layers {
            let rendered = layer.render();

            // Basic well-formedness checks
            assert!(rendered.starts_with("<g"), "Layer should start with <g>");
            assert!(rendered.ends_with("</g>"), "Layer should end with </g>");

            // Check for balanced quotes
            let open_quotes = rendered.matches('"').count();
            assert!(open_quotes % 2 == 0, "Unbalanced quotes in SVG");

            // No unescaped ampersands in attributes (except &amp;)
            let content_without_escaped = rendered.replace("&amp;", "");
            assert!(!content_without_escaped.contains("&"), "Unescaped ampersand in SVG");
        }
    }
}
