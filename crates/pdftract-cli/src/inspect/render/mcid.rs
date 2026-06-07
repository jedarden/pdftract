//! MCID layer renderer for the inspector.
//!
//! This module renders SVG text labels showing the Marked Content Identifier (MCID)
//! for blocks that are associated with marked content sequences (Phase 3.4).
//!
//! Each label includes data-* attributes for tooltip and click consumption:
//! - data-mcid: the MCID number
//! - data-block-index: the block's index in the page
//! - data-block-kind: the block's kind string

use pdftract_core::schema::BlockJson;
use std::collections::HashMap;

/// Render SVG text labels for MCID numbers on marked-content blocks.
///
/// # Arguments
///
/// * `mcid_map` - Optional mapping from MCID numbers to block indices.
///                None if the page has no marked content (Phase 3.4).
///                Some(HashMap) maps MCID -> block_index.
/// * `blocks` - Slice of blocks to render
///
/// # Returns
///
/// A vector of SVG `<text>` element strings. Each text is positioned at
/// the top-right corner of the block's bbox with the MCID number as content.
///
/// # MCID display
///
/// The MCID number is displayed in the top-right corner of each block
/// that has an associated MCID from the marked content tracking.
///
/// # Data attributes
///
/// Each text element includes:
/// - `data-mcid`: the MCID number
/// - `data-block-index`: the block's index in the page
/// - `data-block-kind`: the block's kind string (XML-escaped)
pub fn render_mcid_labels(
    mcid_map: &Option<HashMap<u32, usize>>,
    blocks: &[BlockJson],
) -> Vec<String> {
    let mcid_map = match mcid_map {
        Some(map) if !map.is_empty() => map,
        _ => return Vec::new(), // No MCIDs to render
    };

    let mut labels = Vec::new();

    // Iterate through MCID->block_index mappings
    for (&mcid, &block_index) in mcid_map {
        // Skip if block index is out of bounds
        if block_index >= blocks.len() {
            continue;
        }

        let block = &blocks[block_index];
        let [x0, _y0, x1, y1] = block.bbox;
        let data_kind = escape_xml_attr(&block.kind);

        // Position text at top-right corner with a small offset
        // In PDF coordinates, y1 is the top (higher y value)
        let x = x1 - 4.0; // Small offset from right edge (text-anchor: end)
        let y = y1 - 4.0;  // Small offset from top edge (text baseline)

        labels.push(format!(
            r##"<text x="{:.2}" y="{:.2}" class="mcid-label" fill="{}" font-size="10" font-family="monospace" font-weight="bold" text-anchor="end" data-mcid="{}" data-block-index="{}" data-block-kind="{}">{}</text>"##,
            x, y, "#f59e0b", mcid, block_index, data_kind, mcid
        ));
    }

    labels
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
            spans: vec![],
            receipt: None,
        }
    }

    #[test]
    fn test_render_mcid_labels_none_map() {
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];
        let result = render_mcid_labels(&None, &blocks);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_mcid_labels_empty_map() {
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];
        let empty_map: HashMap<u32, usize> = HashMap::new();
        let result = render_mcid_labels(&Some(empty_map), &blocks);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_mcid_labels_single() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test paragraph",
            [100.0, 200.0, 400.0, 250.0],
        )];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(47, 0); // MCID 47 maps to block 0

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        assert_eq!(result.len(), 1);
        let label = &result[0];

        // Check basic SVG structure
        assert!(label.contains("<text"));
        assert!(label.contains(r#"x="396.00""#)); // x1 - 4 = 400 - 4 = 396
        assert!(label.contains(r#"y="246.00""#)); // y1 - 4 = 250 - 4 = 246

        // Check MCID content
        assert!(label.contains(">47</text>"));

        // Check data attributes
        assert!(label.contains(r#"data-mcid="47""#));
        assert!(label.contains(r#"data-block-index="0""#));
        assert!(label.contains(r#"data-block-kind="paragraph""#));
    }

    #[test]
    fn test_render_mcid_labels_multiple() {
        let blocks = vec![
            make_test_block("heading", "Title", [50.0, 50.0, 300.0, 80.0]),
            make_test_block("paragraph", "Para 1", [50.0, 90.0, 300.0, 150.0]),
            make_test_block("list", "Item 1", [70.0, 160.0, 280.0, 180.0]),
        ];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(10, 0); // heading
        mcid_map.insert(47, 1); // paragraph
        mcid_map.insert(88, 2); // list

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        assert_eq!(result.len(), 3);

        // Check first MCID label
        assert!(result[0].contains(">10</text>"));
        assert!(result[0].contains(r#"data-mcid="10""#));
        assert!(result[0].contains(r#"data-block-kind="heading""#));

        // Check second MCID label
        assert!(result[1].contains(">47</text>"));
        assert!(result[1].contains(r#"data-mcid="47""#));
        assert!(result[1].contains(r#"data-block-kind="paragraph""#));

        // Check third MCID label
        assert!(result[2].contains(">88</text>"));
        assert!(result[2].contains(r#"data-mcid="88""#));
        assert!(result[2].contains(r#"data-block-kind="list""#));
    }

    #[test]
    fn test_render_mcid_labels_positioning() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test",
            [100.0, 200.0, 500.0, 300.0],
        )];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(5, 0);

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        let label = &result[0];

        // x should be x1 - 4 = 500 - 4 = 496
        assert!(label.contains(r#"x="496.00""#));
        // y should be y1 - 4 = 300 - 4 = 296
        assert!(label.contains(r#"y="296.00""#));
        // text-anchor should be "end" for right alignment
        assert!(label.contains(r#"text-anchor="end""#));
    }

    #[test]
    fn test_render_mcid_labels_xml_escaping() {
        let blocks = vec![make_test_block(
            "code & <script>",
            "Text",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(1, 0);

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        let label = &result[0];

        // Check XML escaping in data-block-kind attribute
        assert!(label.contains(r#"data-block-kind="code &amp; &lt;script&gt;""#));
    }

    #[test]
    fn test_render_mcid_labels_out_of_bounds() {
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(10, 0);  // Valid
        mcid_map.insert(20, 5);  // Out of bounds (only 1 block)

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        // Should only have one label (the valid one)
        assert_eq!(result.len(), 1);
        assert!(result[0].contains(r#"data-mcid="10""#));
    }

    #[test]
    fn test_render_mcid_labels_zero_mcid() {
        // MCID 0 is valid (per plan)
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(0, 0);

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains(">0</text>"));
        assert!(result[0].contains(r#"data-mcid="0""#));
    }

    #[test]
    fn test_render_mcid_labels_output_is_valid_svg() {
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(42, 0);

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        let label = &result[0];

        // Verify basic XML structure
        assert!(label.starts_with("<text"));
        assert!(label.ends_with("</text>"));

        // Check that all required attributes are present
        assert!(label.contains("x="));
        assert!(label.contains("y="));
        assert!(label.contains("fill="));
        assert!(label.contains("font-size="));
        assert!(label.contains("font-family="));
        assert!(label.contains("font-weight="));
        assert!(label.contains("text-anchor="));
        assert!(label.contains("class="));
        assert!(label.contains("data-mcid="));
        assert!(label.contains("data-block-index="));
        assert!(label.contains("data-block-kind="));
    }

    #[test]
    fn test_render_mcid_labels_css_class() {
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(7, 0);

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        assert!(result[0].contains(r#"class="mcid-label""#));
    }

    #[test]
    fn test_render_mcid_labels_color() {
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(3, 0);

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        // Check for the amber/orange color (#f59e0b)
        assert!(result[0].contains(r##"fill="#f59e0b""##));
    }

    #[test]
    fn test_render_mcid_labels_font_properties() {
        let blocks = vec![make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0])];

        let mut mcid_map: HashMap<u32, usize> = HashMap::new();
        mcid_map.insert(15, 0);

        let result = render_mcid_labels(&Some(mcid_map), &blocks);
        assert!(result[0].contains(r#"font-size="10""#));
        assert!(result[0].contains(r#"font-family="monospace""#));
        assert!(result[0].contains(r#"font-weight="bold""#));
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
}
