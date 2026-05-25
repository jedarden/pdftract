//! Reading order layer renderer for the inspector.
//!
//! This module renders curved arrows between consecutive blocks in reading order.
//! Arrows are numbered 1..N to show the sequence in which blocks are read.
//!
//! Each arrow includes data-* attributes for tooltip consumption:
//! - data-from-block: index of the source block
//! - data-to-block: index of the target block
//! - data-reading-index: the sequence number (1, 2, 3, ...)

use pdftract_core::schema::BlockJson;

/// Render SVG curved arrows between consecutive blocks in reading order.
///
/// # Arguments
///
/// * `blocks` - Slice of blocks in the document (indexed by position)
/// * `order` - Slice of block indices in reading order (e.g., &[5, 2, 7, 3])
///
/// # Returns
///
/// A vector of SVG element strings containing:
/// - `<path>` elements for curved arrows from block center to next block center
/// - `<text>` elements for numeric labels at arrow midpoints
///
/// # Arrow style
///
/// - Stroke: blue (#3b82f6) with 1.5px width
/// - Marker-end: arrowhead (defined in parent SVG `<defs>`)
/// - Control point: (mid_x, mid_y + 10pt) for visible curve
///
/// # Data attributes
///
/// Each arrow path includes:
/// - `data-from-block`: index of the source block
/// - `data-to-block`: index of the target block
/// - `data-reading-index`: the sequence number (1, 2, 3, ...)
///
/// # Performance
///
/// Limits arrows to the first 50 blocks to avoid visual clutter. Additional
/// blocks are silently ignored (a warning could be logged in debug mode).
pub fn render_reading_order(blocks: &[BlockJson], order: &[usize]) -> Vec<String> {
    const MAX_ARROWS: usize = 50;

    let mut elements = Vec::new();

    // Limit to first N arrows to prevent visual clutter
    let order_limited = if order.len() > MAX_ARROWS {
        &order[..MAX_ARROWS]
    } else {
        order
    };

    // Draw arrows from each block to the next in reading order
    for (idx, window) in order_limited.windows(2).enumerate() {
        let from_idx = window[0];
        let to_idx = window[1];

        // Skip if either block index is out of bounds
        if from_idx >= blocks.len() || to_idx >= blocks.len() {
            continue;
        }

        let from_block = &blocks[from_idx];
        let to_block = &blocks[to_idx];

        // Calculate center points of each block bbox
        let from_center = block_center(from_block);
        let to_center = block_center(to_block);

        // Calculate bezier control point (midpoint + 10pt downward)
        let mid_x = (from_center.0 + to_center.0) / 2.0;
        let mid_y = (from_center.1 + to_center.1) / 2.0;
        let control_x = mid_x;
        let control_y = mid_y + 10.0;

        // Generate the SVG path for the curved arrow
        let path_d = format!(
            "M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}",
            from_center.0, from_center.1, control_x, control_y, to_center.0, to_center.1
        );

        elements.push(format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"#3b82f6\" stroke-width=\"1.5\" marker-end=\"url(#arrowhead)\" class=\"reading-order-arrow\" data-from-block=\"{}\" data-to-block=\"{}\" data-reading-index=\"{}\" />",
            path_d, from_idx, to_idx, idx + 1
        ));

        // Add numeric label at the midpoint
        elements.push(format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" fill=\"#3b82f6\" font-size=\"10\" font-weight=\"bold\" text-anchor=\"middle\" class=\"reading-order-label\" data-reading-index=\"{}\">{}</text>",
            mid_x, mid_y - 5.0, idx + 1, idx + 1
        ));
    }

    elements
}

/// Calculate the center point of a block's bounding box.
///
/// # Arguments
///
/// * `block` - The block whose center to calculate
///
/// # Returns
///
/// A tuple `(x, y)` representing the center point in PDF user-space units.
fn block_center(block: &BlockJson) -> (f64, f64) {
    let [x0, y0, x1, y1] = block.bbox;
    let center_x = (x0 + x1) / 2.0;
    let center_y = (y0 + y1) / 2.0;
    (center_x, center_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_reading_order_empty() {
        let blocks: Vec<BlockJson> = vec![];
        let order: Vec<usize> = vec![];
        let result = render_reading_order(&blocks, &order);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_reading_order_single_block() {
        // Need at least 2 blocks for an arrow
        let blocks = vec![BlockJson {
            kind: "paragraph".to_string(),
            text: "First".to_string(),
            bbox: [0.0, 100.0, 50.0, 120.0],
            level: None,
            table_index: None,
            spans: vec![],
            receipt: None,
        }];
        let order = vec![0];
        let result = render_reading_order(&blocks, &order);
        assert!(result.is_empty()); // No arrows with only 1 block
    }

    #[test]
    fn test_render_reading_order_two_blocks() {
        let blocks = vec![
            BlockJson {
                kind: "paragraph".to_string(),
                text: "First".to_string(),
                bbox: [0.0, 100.0, 50.0, 120.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Second".to_string(),
                bbox: [60.0, 80.0, 110.0, 100.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
        ];
        let order = vec![0, 1];
        let result = render_reading_order(&blocks, &order);

        // Should have 2 elements: 1 path + 1 text label
        assert_eq!(result.len(), 2);

        // Check that the path is a curved arrow
        let path = &result[0];
        assert!(path.contains("<path"));
        assert!(path.contains("d=\"M"));
        assert!(path.contains("Q")); // Quadratic bezier curve
        assert!(path.contains("stroke=\"#3b82f6\""));
        assert!(path.contains("marker-end=\"url(#arrowhead)\""));
        assert!(path.contains("data-from-block=\"0\""));
        assert!(path.contains("data-to-block=\"1\""));
        assert!(path.contains("data-reading-index=\"1\""));

        // Check that the text label is present
        let text = &result[1];
        assert!(text.contains("<text"));
        assert!(text.contains(">1<")); // Number 1
        assert!(text.contains("data-reading-index=\"1\""));
    }

    #[test]
    fn test_render_reading_order_three_blocks() {
        let blocks = vec![
            BlockJson {
                kind: "paragraph".to_string(),
                text: "First".to_string(),
                bbox: [0.0, 100.0, 50.0, 120.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Second".to_string(),
                bbox: [60.0, 80.0, 110.0, 100.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Third".to_string(),
                bbox: [120.0, 60.0, 170.0, 80.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
        ];
        let order = vec![0, 1, 2];
        let result = render_reading_order(&blocks, &order);

        // Should have 4 elements: 2 paths + 2 text labels
        assert_eq!(result.len(), 4);

        // Check first arrow (0 -> 1)
        assert!(result[0].contains("data-from-block=\"0\""));
        assert!(result[0].contains("data-to-block=\"1\""));
        assert!(result[0].contains("data-reading-index=\"1\""));

        // Check second arrow (1 -> 2)
        assert!(result[2].contains("data-from-block=\"1\""));
        assert!(result[2].contains("data-to-block=\"2\""));
        assert!(result[2].contains("data-reading-index=\"2\""));
    }

    #[test]
    fn test_render_reading_order_non_sequential() {
        // Test non-sequential reading order (e.g., columns read left-to-right)
        let blocks = vec![
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Col1".to_string(),
                bbox: [0.0, 100.0, 50.0, 120.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Col2".to_string(),
                bbox: [100.0, 100.0, 150.0, 120.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Col1-Second".to_string(),
                bbox: [0.0, 80.0, 50.0, 100.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Col2-Second".to_string(),
                bbox: [100.0, 80.0, 150.0, 100.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
        ];
        // Reading order: left column first, then right column
        let order = vec![0, 2, 1, 3];
        let result = render_reading_order(&blocks, &order);

        // Should have 6 elements: 3 paths + 3 text labels
        assert_eq!(result.len(), 6);

        // Verify arrows follow the reading order, not spatial order
        assert!(result[0].contains("data-from-block=\"0\""));
        assert!(result[0].contains("data-to-block=\"2\"")); // 0 -> 2 (down left column)

        assert!(result[2].contains("data-from-block=\"2\""));
        assert!(result[2].contains("data-to-block=\"1\"")); // 2 -> 1 (jump to right column)
    }

    #[test]
    fn test_render_reading_order_max_arrows_limit() {
        // Test that arrows are limited to 50 to prevent visual clutter
        let blocks: Vec<BlockJson> = (0..100)
            .map(|i| BlockJson {
                kind: "paragraph".to_string(),
                text: format!("Block{}", i),
                bbox: [0.0, 100.0 - i as f64, 50.0, 120.0 - i as f64],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            })
            .collect();

        let order: Vec<usize> = (0..100).collect();
        let result = render_reading_order(&blocks, &order);

        // With 100 blocks, we'd have 99 arrows, but we limit to 50 blocks in the order
        // windows(2) on 50 elements produces 49 arrows
        // Each arrow produces 2 elements (path + text), so 49 * 2 = 98 elements
        assert_eq!(result.len(), 98); // 49 arrows * 2 elements each
    }

    #[test]
    fn test_block_center() {
        let block = BlockJson {
            kind: "paragraph".to_string(),
            text: "Test".to_string(),
            bbox: [100.0, 200.0, 300.0, 250.0],
            level: None,
            table_index: None,
            spans: vec![],
            receipt: None,
        };

        let center = block_center(&block);
        assert_eq!(center.0, 200.0); // (100 + 300) / 2
        assert_eq!(center.1, 225.0); // (200 + 250) / 2
    }

    #[test]
    fn test_block_center_fractional() {
        let block = BlockJson {
            kind: "paragraph".to_string(),
            text: "Test".to_string(),
            bbox: [0.0, 0.0, 1.0, 1.0],
            level: None,
            table_index: None,
            spans: vec![],
            receipt: None,
        };

        let center = block_center(&block);
        assert_eq!(center.0, 0.5);
        assert_eq!(center.1, 0.5);
    }

    #[test]
    fn test_render_reading_order_css_class() {
        let blocks = vec![
            BlockJson {
                kind: "paragraph".to_string(),
                text: "A".to_string(),
                bbox: [0.0, 100.0, 50.0, 120.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "B".to_string(),
                bbox: [60.0, 80.0, 110.0, 100.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
        ];
        let order = vec![0, 1];
        let result = render_reading_order(&blocks, &order);

        let path = &result[0];
        assert!(path.contains("class=\"reading-order-arrow\""));

        let text = &result[1];
        assert!(text.contains("class=\"reading-order-label\""));
    }

    #[test]
    fn test_render_reading_order_out_of_bounds_indices() {
        let blocks = vec![
            BlockJson {
                kind: "paragraph".to_string(),
                text: "First".to_string(),
                bbox: [0.0, 100.0, 50.0, 120.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
            BlockJson {
                kind: "paragraph".to_string(),
                text: "Second".to_string(),
                bbox: [60.0, 80.0, 110.0, 100.0],
                level: None,
                table_index: None,
                spans: vec![],
                receipt: None,
            },
        ];

        // Include an out-of-bounds index in the reading order
        let order = vec![0, 5, 1];
        let result = render_reading_order(&blocks, &order);

        // The arrow from 0 -> 5 should be skipped (out of bounds)
        // Only the arrow from 5 -> 1 should also be skipped
        // So we should have no arrows since the first window is [0, 5] which is invalid
        assert!(result.is_empty());
    }
}
