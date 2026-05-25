//! Block layer renderer for the inspector.
//!
//! This module renders SVG translucent rectangles for each structural block,
//! color-coded by block kind. Each block type has a distinct fill color for
//! easy visual identification of the document structure.
//!
//! Each rect includes data-* attributes for tooltip and click consumption:
//! - data-kind: the block kind (heading, paragraph, list, table, etc.)
//! - data-text: the block's text content (truncated for tooltip display)
//! - data-level: heading level (1-6) for heading blocks
//! - data-table-index: table index for table blocks
//! - data-block-index: the block's index in the page (for JSON-tree navigation)

use pdftract_core::schema::BlockJson;

/// Render SVG translucent rectangles for each block.
///
/// # Arguments
///
/// * `blocks` - Slice of blocks to render
///
/// # Returns
///
/// A vector of SVG `<rect>` element strings. Each rect is positioned at
/// the block's bbox with translucent fill color indicating kind.
///
/// # Color coding
///
/// - Blue (#3b82f6): heading
/// - Gray (#9ca3af): paragraph
/// - Teal (#14b8a6): table
/// - Purple (#a855f7): list
/// - Orange (#f97316): code
/// - Light gray (#d1d5db): header/footer
/// - Brown (#a52a2a): figure
/// - Pink (#ec4899): caption
/// - Default gray (#9ca3af): unknown kinds
///
/// # Data attributes
///
/// Each rect includes:
/// - `data-kind`: the block's kind string (XML-escaped)
/// - `data-text`: the block's text content, truncated to 100 chars (XML-escaped)
/// - `data-level`: heading level for heading blocks, or empty string
/// - `data-table-index`: table index for table blocks, or empty string
/// - `data-block-index`: the block's index in the page (for JSON-tree navigation)
pub fn render_blocks(blocks: &[BlockJson]) -> Vec<String> {
    blocks.iter().enumerate().map(|(index, block)| {
        let [x0, y0, x1, y1] = block.bbox;
        let width = x1 - x0;
        let height = y1 - y0;
        let fill = kind_to_color(&block.kind);
        let data_kind = escape_xml_attr(&block.kind);

        // Truncate text for tooltip (max ~100 chars total including "...")
        let tooltip_text = if block.text.len() > 99 {
            format!("{}...", &block.text[..99])
        } else {
            block.text.clone()
        };
        let data_text = escape_xml_attr(&tooltip_text);

        let data_level = block.level.map(|l| l.to_string()).unwrap_or_default();
        let data_table_index = block.table_index.map(|i| i.to_string()).unwrap_or_default();

        format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="0.3" stroke="{}" stroke-width="1" stroke-opacity="0.5" class="block-rect" data-kind="{}" data-text="{}" data-level="{}" data-table-index="{}" data-block-index="{}" />"#,
            x0, y0, width, height, fill, fill, data_kind, data_text, data_level, data_table_index, index
        )
    }).collect()
}

/// Convert a block kind string to an SVG fill color.
///
/// # Arguments
///
/// * `kind` - Block kind string (e.g., "heading", "paragraph", "list")
///
/// # Returns
///
/// A CSS hex color string.
///
/// # Color mapping (per plan §7.9)
///
/// - `"heading"`: blue (#3b82f6)
/// - `"paragraph"`: gray (#9ca3af)
/// - `"table"`: teal (#14b8a6)
/// - `"list"`: purple (#a855f7)
/// - `"code"`: orange (#f97316)
/// - `"header"`, `"footer"`: light gray (#d1d5db)
/// - `"figure"`: brown (#a52a2a)
/// - `"caption"`: pink (#ec4899)
/// - Other values: default gray (#9ca3af)
fn kind_to_color(kind: &str) -> &'static str {
    match kind {
        "heading" => "#3b82f6",           // blue
        "paragraph" => "#9ca3af",         // gray
        "table" => "#14b8a6",             // teal
        "list" => "#a855f7",              // purple
        "code" => "#f97316",              // orange
        "header" | "footer" => "#d1d5db", // light gray
        "figure" => "#a52a2a",            // brown
        "caption" => "#ec4899",           // pink
        _ => "#9ca3af",                   // default gray
    }
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
    fn test_render_blocks_empty() {
        let blocks: Vec<BlockJson> = vec![];
        let output = render_blocks(&blocks);
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_blocks_single() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test paragraph",
            [100.0, 200.0, 400.0, 250.0],
        )];

        let output = render_blocks(&blocks);
        assert_eq!(output.len(), 1);
        let rect = &output[0];

        // Check basic SVG structure
        assert!(rect.contains("<rect"));
        assert!(rect.contains(r#"x="100.00""#));
        assert!(rect.contains(r#"y="200.00""#));
        assert!(rect.contains(r#"width="300.00""#)); // 400 - 100
        assert!(rect.contains(r#"height="50.00""#)); // 250 - 200

        // Check fill color for paragraph (gray)
        assert!(rect.contains("fill=\"#9ca3af\""));
        assert!(rect.contains("fill-opacity=\"0.3\""));
        assert!(rect.contains("stroke=\"#9ca3af\""));
        assert!(rect.contains("stroke-opacity=\"0.5\""));

        // Check data attributes
        assert!(rect.contains(r#"data-kind="paragraph""#));
        assert!(rect.contains(r#"data-text="Test paragraph""#));
        assert!(rect.contains(r#"data-block-index="0""#));
    }

    #[test]
    fn test_render_blocks_heading() {
        let mut block = make_test_block("heading", "Chapter 1", [50.0, 100.0, 300.0, 140.0]);
        block.level = Some(1);

        let blocks = vec![block];
        let output = render_blocks(&blocks);
        assert_eq!(output.len(), 1);
        let rect = &output[0];

        // Check blue color for heading
        assert!(rect.contains("fill=\"#3b82f6\""));

        // Check level attribute
        assert!(rect.contains(r#"data-level="1""#));
    }

    #[test]
    fn test_render_blocks_table() {
        let mut block = make_test_block("table", "Table data", [100.0, 300.0, 500.0, 600.0]);
        block.table_index = Some(3);

        let blocks = vec![block];
        let output = render_blocks(&blocks);
        assert_eq!(output.len(), 1);
        let rect = &output[0];

        // Check teal color for table
        assert!(rect.contains("fill=\"#14b8a6\""));

        // Check table_index attribute
        assert!(rect.contains(r#"data-table-index="3""#));
    }

    #[test]
    fn test_render_blocks_all_kinds() {
        let test_cases = [
            ("heading", "#3b82f6"),
            ("paragraph", "#9ca3af"),
            ("table", "#14b8a6"),
            ("list", "#a855f7"),
            ("code", "#f97316"),
            ("header", "#d1d5db"),
            ("footer", "#d1d5db"),
            ("figure", "#a52a2a"),
            ("caption", "#ec4899"),
        ];

        for (kind, expected_color) in test_cases {
            let blocks = vec![make_test_block(kind, "Test", [0.0, 0.0, 100.0, 20.0])];
            let output = render_blocks(&blocks);
            assert_eq!(output.len(), 1);
            assert!(
                output[0].contains(&format!("fill=\"{}\"", expected_color)),
                "Kind '{}' should produce color {}, got: {}",
                kind,
                expected_color,
                output[0]
            );
        }
    }

    #[test]
    fn test_render_blocks_unknown_kind() {
        let blocks = vec![make_test_block(
            "unknown_kind",
            "Test",
            [0.0, 0.0, 100.0, 20.0],
        )];
        let output = render_blocks(&blocks);
        assert_eq!(output.len(), 1);
        // Unknown kinds should default to gray
        assert!(output[0].contains("fill=\"#9ca3af\""));
    }

    #[test]
    fn test_render_blocks_text_truncation() {
        let long_text = "a".repeat(200);
        let blocks = vec![make_test_block(
            "paragraph",
            &long_text,
            [0.0, 0.0, 100.0, 20.0],
        )];

        let output = render_blocks(&blocks);
        let rect = &output[0];

        // Text should be truncated to ~100 chars with "..." suffix
        assert!(rect.contains("data-text=\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...\""));
        assert!(rect.len() < long_text.len() + 200); // Output should be significantly shorter than input
    }

    #[test]
    fn test_render_blocks_xml_escaping() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Text with <tags> & \"quotes\" and 'apostrophes'",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let output = render_blocks(&blocks);
        let rect = &output[0];

        // Check XML escaping in data-text attribute
        assert!(rect.contains("data-text=\"Text with &lt;tags&gt; &amp; &quot;quotes&quot; and &apos;apostrophes&apos;\""));
    }

    #[test]
    fn test_render_blocks_css_class() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Test",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let output = render_blocks(&blocks);
        assert!(output[0].contains(r#"class="block-rect""#));
    }

    #[test]
    fn test_render_blocks_multiple() {
        let blocks = vec![
            make_test_block("heading", "Title", [50.0, 50.0, 300.0, 80.0]),
            make_test_block("paragraph", "Para 1", [50.0, 90.0, 300.0, 150.0]),
            make_test_block("list", "Item 1", [70.0, 160.0, 280.0, 180.0]),
        ];

        let output = render_blocks(&blocks);
        assert_eq!(output.len(), 3);

        // Check block indices
        assert!(output[0].contains("data-block-index=\"0\""));
        assert!(output[1].contains("data-block-index=\"1\""));
        assert!(output[2].contains("data-block-index=\"2\""));

        // Check colors
        assert!(output[0].contains("fill=\"#3b82f6\"")); // heading - blue
        assert!(output[1].contains("fill=\"#9ca3af\"")); // paragraph - gray
        assert!(output[2].contains("fill=\"#a855f7\"")); // list - purple
    }

    #[test]
    fn test_kind_to_color() {
        assert_eq!(kind_to_color("heading"), "#3b82f6");
        assert_eq!(kind_to_color("paragraph"), "#9ca3af");
        assert_eq!(kind_to_color("table"), "#14b8a6");
        assert_eq!(kind_to_color("list"), "#a855f7");
        assert_eq!(kind_to_color("code"), "#f97316");
        assert_eq!(kind_to_color("header"), "#d1d5db");
        assert_eq!(kind_to_color("footer"), "#d1d5db");
        assert_eq!(kind_to_color("figure"), "#a52a2a");
        assert_eq!(kind_to_color("caption"), "#ec4899");
        assert_eq!(kind_to_color("unknown"), "#9ca3af");
    }

    #[test]
    fn test_render_blocks_float_bbox() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Float",
            [10.567, 20.891, 100.234, 110.567],
        )];

        let output = render_blocks(&blocks);
        let rect = &output[0];

        // Check that coordinates are rounded to 2 decimal places
        assert!(rect.contains(r#"x="10.57""#));
        assert!(rect.contains(r#"y="20.89""#));
        assert!(rect.contains(r#"width="89.67""#)); // 100.234 - 10.567
        assert!(rect.contains(r#"height="89.68""#)); // 110.567 - 20.891
    }

    #[test]
    fn test_render_blocks_output_is_valid_svg() {
        let blocks = vec![make_test_block(
            "paragraph",
            "Valid",
            [0.0, 0.0, 100.0, 20.0],
        )];

        let output = render_blocks(&blocks);
        let rect = &output[0];

        // Verify basic XML structure
        assert!(rect.starts_with("<rect"));
        assert!(rect.ends_with(" />"));

        // Check that all required attributes are present
        assert!(rect.contains("x="));
        assert!(rect.contains("y="));
        assert!(rect.contains("width="));
        assert!(rect.contains("height="));
        assert!(rect.contains("fill="));
        assert!(rect.contains("fill-opacity="));
        assert!(rect.contains("stroke="));
        assert!(rect.contains("stroke-width="));
        assert!(rect.contains("stroke-opacity="));
        assert!(rect.contains("class="));
    }

    #[test]
    fn test_render_blocks_empty_level_and_table_index() {
        let block = make_test_block("paragraph", "Test", [0.0, 0.0, 100.0, 20.0]);
        // level and table_index are None (not heading or table)

        let output = render_blocks(&[block]);
        let rect = &output[0];

        // Should have empty strings for level and table_index
        assert!(rect.contains(r#"data-level="""#));
        assert!(rect.contains(r#"data-table-index="""#));
    }
}
