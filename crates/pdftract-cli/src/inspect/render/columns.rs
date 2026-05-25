//! Column layer renderer for the inspector.
//!
//! This module renders SVG dashed vertical lines at column boundaries.
//! Each column boundary uses a different color for visual distinction.
//!
//! Each line includes data-* attributes for tooltip and click consumption:
//! - data-column-index: the column's index
//! - data-boundary: "left" or "right" indicating which boundary this line represents
//! - data-x0: the left boundary x-coordinate
//! - data-x1: the right boundary x-coordinate

use pdftract_core::layout::columns::Column;

/// Render SVG dashed vertical lines at column boundaries.
///
/// # Arguments
///
/// * `columns` - Slice of columns to render
/// * `page_height` - Page height in points (for line extent)
///
/// # Returns
///
/// A vector of SVG `<line>` element strings. Each line is a vertical dashed
/// line at a column boundary (x0 or x1).
///
/// # Color coding
///
/// Each column boundary uses a distinct color from the palette:
/// - Left boundaries cycle through: cyan, magenta, yellow, green, orange
/// - Right boundaries use darker variants of the corresponding left boundary
///
/// # Data attributes
///
/// Each line includes:
/// - `data-column-index`: the column's index (0-based)
/// - `data-boundary`: "left" or "right" indicating which boundary
/// - `data-x0`: the column's left x-coordinate
/// - `data-x1`: the column's right x-coordinate
pub fn render_columns(columns: &[Column], page_height: f32) -> Vec<String> {
    columns.iter().enumerate().flat_map(|(idx, col)| {
        let left_color = boundary_color(idx, true);
        let right_color = boundary_color(idx, false);

        vec![
            render_left_boundary(col, page_height, left_color),
            render_right_boundary(col, page_height, right_color),
        ]
    }).collect()
}

/// Render the left boundary (x0) of a column.
fn render_left_boundary(column: &Column, page_height: f32, color: &str) -> String {
    let x = column.x_range[0];
    format!(
        r#"<line x1="{:.2}" y1="0" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5" stroke-dasharray="5,3" class="column-boundary column-left" data-column-index="{}" data-boundary="left" data-x0="{:.2}" data-x1="{:.2}" />"#,
        x, x, page_height, color, column.index, column.x_range[0], column.x_range[1]
    )
}

/// Render the right boundary (x1) of a column.
fn render_right_boundary(column: &Column, page_height: f32, color: &str) -> String {
    let x = column.x_range[1];
    format!(
        r#"<line x1="{:.2}" y1="0" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5" stroke-dasharray="8,4" class="column-boundary column-right" data-column-index="{}" data-boundary="right" data-x0="{:.2}" data-x1="{:.2}" />"#,
        x, x, page_height, color, column.index, column.x_range[0], column.x_range[1]
    )
}

/// Get a color for a column boundary.
///
/// Left boundaries use lighter colors, right boundaries use darker variants.
/// Colors cycle through a palette to distinguish adjacent columns.
fn boundary_color(column_index: usize, is_left: bool) -> &'static str {
    const PALETTE: &[(&str, &str)] = &[
        ("#06b6d4", "#0891b2"), // cyan (light, dark)
        ("#d946ef", "#c026d3"), // magenta (light, dark)
        ("#facc15", "#ca8a04"), // yellow (light, dark)
        ("#22c55e", "#16a34a"), // green (light, dark)
        ("#f97316", "#ea580c"), // orange (light, dark)
        ("#3b82f6", "#2563eb"), // blue (light, dark)
        ("#a855f7", "#9333ea"), // purple (light, dark)
        ("#f43f5e", "#e11d48"), // red (light, dark)
    ];

    let (light, dark) = PALETTE[column_index % PALETTE.len()];
    if is_left { light } else { dark }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_columns_empty() {
        let columns: Vec<Column> = Vec::new();
        let result = render_columns(&columns, 792.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_columns_single() {
        let columns = vec![Column::new(0, [50.0, 300.0])];
        let result = render_columns(&columns, 792.0);
        assert_eq!(result.len(), 2); // left and right boundary

        // Check left boundary
        assert!(result[0].contains("x1=\"50.00\""));
        assert!(result[0].contains("y1=\"0\""));
        assert!(result[0].contains("y2=\"792.00\""));
        assert!(result[0].contains("data-column-index=\"0\""));
        assert!(result[0].contains("data-boundary=\"left\""));
        assert!(result[0].contains("data-x0=\"50.00\""));
        assert!(result[0].contains("data-x1=\"300.00\""));
        assert!(result[0].contains("stroke-dasharray=\"5,3\""));

        // Check right boundary
        assert!(result[1].contains("x1=\"300.00\""));
        assert!(result[1].contains("data-boundary=\"right\""));
        assert!(result[1].contains("stroke-dasharray=\"8,4\""));
    }

    #[test]
    fn test_render_columns_multiple() {
        let columns = vec![
            Column::new(0, [50.0, 300.0]),
            Column::new(1, [320.0, 570.0]),
        ];
        let result = render_columns(&columns, 792.0);
        assert_eq!(result.len(), 4); // 2 boundaries per column

        // First column (cyan colors)
        assert!(result[0].contains("#06b6d4")); // left - light cyan
        assert!(result[1].contains("#0891b2")); // right - dark cyan

        // Second column (magenta colors)
        assert!(result[2].contains("#d946ef")); // left - light magenta
        assert!(result[3].contains("#c026d3")); // right - dark magenta
    }

    #[test]
    fn test_render_columns_colors_cycle() {
        let columns = vec![
            Column::new(0, [0.0, 100.0]),
            Column::new(1, [100.0, 200.0]),
            Column::new(2, [200.0, 300.0]),
            Column::new(3, [300.0, 400.0]),
            Column::new(4, [400.0, 500.0]),
            Column::new(5, [500.0, 600.0]),
            Column::new(6, [600.0, 700.0]),
            Column::new(7, [700.0, 800.0]),
            Column::new(8, [800.0, 900.0]), // Should cycle back to first color
        ];
        let result = render_columns(&columns, 792.0);

        // Check that colors cycle correctly
        let left_colors: Vec<&str> = result.iter()
            .step_by(2)
            .filter(|s| s.contains("column-left"))
            .map(|s| {
                if s.contains("#06b6d4") { "#06b6d4" }
                else if s.contains("#d946ef") { "#d946ef" }
                else if s.contains("#facc15") { "#facc15" }
                else if s.contains("#22c55e") { "#22c55e" }
                else if s.contains("#f97316") { "#f97316" }
                else if s.contains("#3b82f6") { "#3b82f6" }
                else if s.contains("#a855f7") { "#a855f7" }
                else if s.contains("#f43f5e") { "#f43f5e" }
                else { "unknown" }
            })
            .collect();

        // First 8 columns should have distinct colors
        assert_eq!(left_colors[0], "#06b6d4"); // cyan
        assert_eq!(left_colors[1], "#d946ef"); // magenta
        assert_eq!(left_colors[2], "#facc15"); // yellow
        assert_eq!(left_colors[3], "#22c55e"); // green
        assert_eq!(left_colors[4], "#f97316"); // orange
        assert_eq!(left_colors[5], "#3b82f6"); // blue
        assert_eq!(left_colors[6], "#a855f7"); // purple
        assert_eq!(left_colors[7], "#f43f5e"); // red

        // 9th column should cycle back to cyan
        assert_eq!(left_colors[8], "#06b6d4");
    }

    #[test]
    fn test_boundary_color_left_vs_right() {
        // Left boundaries are lighter
        assert_eq!(boundary_color(0, true), "#06b6d4");
        assert_eq!(boundary_color(1, true), "#d946ef");
        assert_eq!(boundary_color(2, true), "#facc15");

        // Right boundaries are darker
        assert_eq!(boundary_color(0, false), "#0891b2");
        assert_eq!(boundary_color(1, false), "#c026d3");
        assert_eq!(boundary_color(2, false), "#ca8a04");
    }

    #[test]
    fn test_render_columns_svg_validity() {
        let columns = vec![Column::new(0, [50.0, 300.0])];
        let result = render_columns(&columns, 792.0);

        // All results should be valid SVG line elements
        for line in &result {
            assert!(line.starts_with(r#"<line "#));
            assert!(line.ends_with(r#" />"#));
            assert!(line.contains("stroke="));
            assert!(line.contains("stroke-width="));
            assert!(line.contains("stroke-dasharray="));
        }
    }

    #[test]
    fn test_render_columns_class_attributes() {
        let columns = vec![Column::new(0, [50.0, 300.0])];
        let result = render_columns(&columns, 792.0);

        // Left boundary should have correct classes
        assert!(result[0].contains(r#"class="column-boundary column-left""#));

        // Right boundary should have correct classes
        assert!(result[1].contains(r#"class="column-boundary column-right""#));
    }

    #[test]
    fn test_render_columns_data_attributes() {
        let columns = vec![
            Column::new(0, [50.0, 300.0]),
            Column::new(1, [320.0, 570.0]),
        ];
        let result = render_columns(&columns, 792.0);

        // Check data attributes for first column
        assert!(result[0].contains(r#"data-column-index="0""#));
        assert!(result[0].contains(r#"data-boundary="left""#));
        assert!(result[0].contains(r#"data-x0="50.00""#));
        assert!(result[0].contains(r#"data-x1="300.00""#));

        // Check data attributes for second column
        assert!(result[2].contains(r#"data-column-index="1""#));
        assert!(result[2].contains(r#"data-boundary="left""#));
        assert!(result[2].contains(r#"data-x0="320.00""#));
        assert!(result[2].contains(r#"data-x1="570.00""#));
    }

    #[test]
    fn test_render_columns_dash_patterns() {
        let columns = vec![Column::new(0, [50.0, 300.0])];
        let result = render_columns(&columns, 792.0);

        // Left boundaries use "5,3" dash pattern
        assert!(result[0].contains(r#"stroke-dasharray="5,3""#));

        // Right boundaries use "8,4" dash pattern
        assert!(result[1].contains(r#"stroke-dasharray="8,4""#));
    }
}
