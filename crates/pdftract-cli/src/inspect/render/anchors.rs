//! Anchor layer renderer for the inspector.
//!
//! This module renders SVG text labels at the top-left corner of each block,
//! showing the Markdown anchor IDs that downstream output (Phase 6.5 --md-anchors)
//! will produce.
//!
//! Each text label includes data-* attributes for tooltip and click consumption:
//! - data-page-index: the page index (0-based)
//! - data-page-number: the page number (1-based, for display)
//! - data-block-index: the block's index in the page
//! - data-bbox: the block's bounding box
//! - data-kind: the block kind

use pdftract_core::schema::BlockJson;

/// Render SVG text labels at the top-left corner of each block.
///
/// # Arguments
///
/// * `page_index` - Zero-based page index
/// * `page_number` - One-based page number (for display)
/// * `blocks` - Slice of blocks to render
///
/// # Returns
///
/// A vector of SVG `<text>` element strings. Each text is positioned at
/// the top-left corner of the block's bbox with the anchor ID as content.
///
/// # Anchor format
///
/// The anchor ID format is: `p{page_number}-b{block_index}`
/// - page_number: 1-based page number for human readability
/// - block_index: 0-based block index within the page
///
/// This matches the Phase 6.5 Markdown anchor comment format:
/// `<!-- pdftract: page={page_number} block={block_index} bbox=[...] kind=... -->`
///
/// # Data attributes
///
/// Each text element includes:
/// - `data-page-index`: the page's 0-based index
/// - `data-page-number`: the page's 1-based number (for display)
/// - `data-block-index`: the block's index in the page
/// - `data-bbox`: the block's bounding box as "[x0,y0,x1,y1]"
/// - `data-kind`: the block's kind string (XML-escaped)
pub fn render_anchors(page_index: usize, page_number: u32, blocks: &[BlockJson]) -> Vec<String> {
    blocks.iter().enumerate().map(|(block_index, block)| {
        let [x0, _y0, x1, y1] = block.bbox;
        let data_kind = escape_xml_attr(&block.kind);
        let data_bbox = format!("[{:.2},{:.2},{:.2},{:.2}]", x0, block.bbox[1], x1, y1);

        // Position text at top-left corner with a small offset
        // In PDF coordinates, y1 is the top (higher y value)
        let x = x0 + 2.0; // Small offset from left edge
        let y = y1 - 4.0; // Small offset from top edge (text baseline)

        let anchor_id = format!("p{}-b{}", page_number, block_index);

        format!(
            r##"<text x="{:.2}" y="{:.2}" class="anchor-label" fill="{}" font-size="10" font-family="monospace" data-page-index="{}" data-page-number="{}" data-block-index="{}" data-bbox="{}" data-kind="{}">{}</text>"##,
            x, y, "#000000", page_index, page_number, block_index, data_bbox, data_kind, anchor_id
        )
    }).collect()
}

/// Escape a string for use in an XML attribute value.
///
/// Replaces special XML characters with their entity references:
/// - `&` → `&amp;`
/// - `<` → `&lt;`
/// - `>` → `&gt;`
/// - `"` → `&quot;`
/// - `'` → `&apos;`
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_block(kind: &str, text: &str, bbox: [f64; 4]) -> BlockJson {
        BlockJson {
            kind: kind.to_string(),
            text: text.to_string(),
            bbox,
            level: None,
            table_index: None,
            receipt: None,
        }
    }

    #[test]
    fn test_render_anchors_empty() {
        let blocks: Vec<BlockJson> = vec![];
        let output = render_anchors(0, 1, &blocks);
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_anchors_single() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test paragraph",
            [100.0, 200.0, 400.0, 250.0],
        )];

        let output = render_anchors(0, 1, &blocks);
        assert_eq!(output.len(), 1);
        let text = &output[0];

        // Check basic SVG structure
        assert!(text.contains("<text"));
        assert!(text.contains(r#"x="102.00""#)); // x0 + 2
        assert!(text.contains(r#"y="246.00""#)); // y1 - 4

        // Check anchor ID content
        assert!(text.contains(">p1-b0</text>"));

        // Check data attributes
        assert!(text.contains(r#"data-page-index="0""#));
        assert!(text.contains(r#"data-page-number="1""#));
        assert!(text.contains(r#"data-block-index="0""#));
        assert!(text.contains(r#"data-kind="paragraph""#));
    }

    #[test]
    fn test_render_anchors_multiple() {
        let blocks = vec![
            make_test_block("heading", "Title", [50.0, 50.0, 300.0, 80.0]),
            make_test_block("paragraph", "Para 1", [50.0, 90.0, 300.0, 150.0]),
            make_test_block("list", "Item 1", [70.0, 160.0, 280.0, 180.0]),
        ];

        let output = render_anchors(2, 3, &blocks);
        assert_eq!(output.len(), 3);

        // Check anchor IDs
        assert!(output[0].contains(">p3-b0</text>"));
        assert!(output[1].contains(">p3-b1</text>"));
        assert!(output[2].contains(">p3-b2</text>"));

        // Check page indices
        assert!(output[0].contains(r#"data-page-index="2""#));
        assert!(output[1].contains(r#"data-page-index="2""#));
        assert!(output[2].contains(r#"data-page-index="2""#));
    }

    #[test]
    fn test_render_anchors_bbox_format() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test",
            [10.567, 20.891, 100.234, 110.567],
        )];

        let output = render_anchors(0, 1, &blocks);
        let text = &output[0];

        // Check bbox format in data attribute
        assert!(text.contains(r#"data-bbox="[10.57,20.89,100.23,110.57]""#));
    }

    #[test]
    fn test_render_anchors_xml_escaping() {
        let blocks = vec![make_test_block(
            "code & <script>",
            "Text",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let output = render_anchors(0, 1, &blocks);
        let text = &output[0];

        // Check XML escaping in data-kind attribute
        assert!(text.contains(r#"data-kind="code &amp; &lt;script&gt;""#));
    }

    #[test]
    fn test_render_anchors_block_index_incrementing() {
        let blocks = vec![
            make_test_block("paragraph", "First", [0.0, 0.0, 50.0, 10.0]),
            make_test_block("paragraph", "Second", [60.0, 0.0, 120.0, 10.0]),
            make_test_block("paragraph", "Third", [130.0, 0.0, 180.0, 10.0]),
            make_test_block("paragraph", "Fourth", [190.0, 0.0, 240.0, 10.0]),
        ];

        let output = render_anchors(5, 6, &blocks);
        assert_eq!(output.len(), 4);

        // Check block indices increment correctly
        assert!(output[0].contains(r#"data-block-index="0""#));
        assert!(output[0].contains(">p6-b0</text>"));

        assert!(output[1].contains(r#"data-block-index="1""#));
        assert!(output[1].contains(">p6-b1</text>"));

        assert!(output[2].contains(r#"data-block-index="2""#));
        assert!(output[2].contains(">p6-b2</text>"));

        assert!(output[3].contains(r#"data-block-index="3""#));
        assert!(output[3].contains(">p6-b3</text>"));
    }

    #[test]
    fn test_render_anchors_positioning() {
        // Test that text is positioned at top-left with offset
        let blocks = vec![make_test_block(
            "paragraph",
            "Test",
            [100.0, 200.0, 500.0, 300.0],
        )];

        let output = render_anchors(0, 1, &blocks);
        let text = &output[0];

        // x should be x0 + 2 = 102
        assert!(text.contains(r#"x="102.00""#));
        // y should be y1 - 4 = 296
        assert!(text.contains(r#"y="296.00""#));
    }

    #[test]
    fn test_render_anchors_different_pages() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test",
            [0.0, 0.0, 100.0, 20.0],
        )];

        // Page 1 (index 0, number 1)
        let output1 = render_anchors(0, 1, &blocks);
        assert!(output1[0].contains(">p1-b0</text>"));
        assert!(output1[0].contains(r#"data-page-index="0""#));
        assert!(output1[0].contains(r#"data-page-number="1""#));

        // Page 5 (index 4, number 5)
        let output5 = render_anchors(4, 5, &blocks);
        assert!(output5[0].contains(">p5-b0</text>"));
        assert!(output5[0].contains(r#"data-page-index="4""#));
        assert!(output5[0].contains(r#"data-page-number="5""#));
    }

    #[test]
    fn test_render_anchors_output_is_valid_svg() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Valid",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let output = render_anchors(0, 1, &blocks);
        let text = &output[0];

        // Verify basic XML structure
        assert!(text.starts_with("<text"));
        assert!(text.ends_with("</text>"));

        // Check that all required attributes are present
        assert!(text.contains("x="));
        assert!(text.contains("y="));
        assert!(text.contains("fill="));
        assert!(text.contains("font-size="));
        assert!(text.contains("font-family="));
        assert!(text.contains("class="));
        assert!(text.contains("data-page-index="));
        assert!(text.contains("data-page-number="));
        assert!(text.contains("data-block-index="));
        assert!(text.contains("data-bbox="));
        assert!(text.contains("data-kind="));
    }

    #[test]
    fn test_render_anchors_css_class() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let output = render_anchors(0, 1, &blocks);
        assert!(output[0].contains(r#"class="anchor-label""#));
    }

    #[test]
    fn test_escape_xml_attr() {
        assert_eq!(escape_xml_attr("hello"), "hello");
        assert_eq!(escape_xml_attr("a&b"), "a&amp;b");
        assert_eq!(escape_xml_attr("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml_attr("\"quote\""), "&quot;quote&quot;");
        assert_eq!(escape_xml_attr("'apos'"), "&apos;apos&apos;");
        assert_eq!(
            escape_xml_attr("All & <special> \"chars'"),
            "All &amp; &lt;special&gt; &quot;chars&apos;"
        );
    }

    #[test]
    fn test_render_anchors_all_kinds() {
        let kinds = [
            "heading",
            "paragraph",
            "table",
            "list",
            "code",
            "header",
            "footer",
            "figure",
            "caption",
        ];

        for kind in kinds {
            let blocks = vec![make_test_block(kind, "Test", [0.0, 0.0, 100.0, 20.0])];
            let output = render_anchors(0, 1, &blocks);
            assert_eq!(output.len(), 1);
            assert!(output[0].contains(&format!("data-kind=\"{}\"", kind)));
        }
    }
}
