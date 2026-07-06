//! Color encodings for inspector overlay layers.
//!
//! This module centralizes all color constants used by the overlay layer renderers.
//! Colors match the specification in plan §7.9.

/// Convert a confidence score to an SVG color.
///
/// # Arguments
///
/// * `confidence` - Optional confidence score (0.0 to 1.0)
///
/// # Returns
///
/// A CSS hex color string.
///
/// # Color mapping (per plan §7.9)
///
/// - `None`: gray (#94a3b8) - direct extraction without OCR
/// - `Some(c) where c < 0.5`: red (#ef4444) - low confidence
/// - `Some(c) where 0.5 <= c < 0.8`: yellow (#eab308) - medium confidence
/// - `Some(c) where c >= 0.8`: green (#22c55e) - high confidence
pub fn confidence_to_color(confidence: Option<f64>) -> &'static str {
    match confidence {
        None => GRAY_NEUTRAL,                // gray - direct extraction
        Some(c) if c < 0.5 => RED_LOW,       // red - low confidence
        Some(c) if c < 0.8 => YELLOW_MEDIUM, // yellow - medium confidence
        Some(_) => GREEN_HIGH,               // green - high confidence
    }
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
pub fn kind_to_color(kind: &str) -> &'static str {
    match kind {
        "heading" => BLUE_HEADING,
        "paragraph" => GRAY_PARAGRAPH,
        "table" => TEAL_TABLE,
        "list" => PURPLE_LIST,
        "code" => ORANGE_CODE,
        "header" | "footer" => GRAY_LIGHT_HEADER,
        "figure" => BROWN_FIGURE,
        "caption" => PINK_CAPTION,
        _ => GRAY_DEFAULT,
    }
}

/// Get a color for a column boundary.
///
/// Left boundaries use lighter colors, right boundaries use darker variants.
/// Colors cycle through a palette to distinguish adjacent columns.
///
/// # Arguments
///
/// * `column_index` - Zero-based column index
/// * `is_left` - True for left boundary, false for right boundary
///
/// # Returns
///
/// A CSS hex color string.
pub fn column_boundary_color(column_index: usize, is_left: bool) -> &'static str {
    const PALETTE: &[(&str, &str)] = &[
        (CYAN_COL_LEFT, CYAN_COL_RIGHT),
        (MAGENTA_COL_LEFT, MAGENTA_COL_RIGHT),
        (YELLOW_COL_LEFT, YELLOW_COL_RIGHT),
        (GREEN_COL_LEFT, GREEN_COL_RIGHT),
        (ORANGE_COL_LEFT, ORANGE_COL_RIGHT),
        (BLUE_COL_LEFT, BLUE_COL_RIGHT),
        (PURPLE_COL_LEFT, PURPLE_COL_RIGHT),
        (RED_COL_LEFT, RED_COL_RIGHT),
    ];

    let (light, dark) = PALETTE[column_index % PALETTE.len()];
    if is_left {
        light
    } else {
        dark
    }
}

// ============== Confidence Colors ==============

/// Red for low confidence (< 0.5)
pub const RED_LOW: &str = "#ef4444";

/// Yellow for medium confidence (0.5 - 0.8)
pub const YELLOW_MEDIUM: &str = "#eab308";

/// Green for high confidence (>= 0.8)
pub const GREEN_HIGH: &str = "#22c55e";

/// Gray for no confidence value (direct extraction)
pub const GRAY_NEUTRAL: &str = "#94a3b8";

// ============== Block Kind Colors ==============

/// Blue for headings
pub const BLUE_HEADING: &str = "#3b82f6";

/// Gray for paragraphs (default)
pub const GRAY_PARAGRAPH: &str = "#9ca3af";

/// Gray default for unknown block kinds
pub const GRAY_DEFAULT: &str = "#9ca3af";

/// Teal for tables
pub const TEAL_TABLE: &str = "#14b8a6";

/// Purple for lists
pub const PURPLE_LIST: &str = "#a855f7";

/// Orange for code blocks
pub const ORANGE_CODE: &str = "#f97316";

/// Light gray for headers and footers
pub const GRAY_LIGHT_HEADER: &str = "#d1d5db";

/// Brown for figures
pub const BROWN_FIGURE: &str = "#a52a2a";

/// Pink for captions
pub const PINK_CAPTION: &str = "#ec4899";

// ============== Column Boundary Colors ==============

/// Cyan left boundary
pub const CYAN_COL_LEFT: &str = "#06b6d4";

/// Cyan right boundary (darker)
pub const CYAN_COL_RIGHT: &str = "#0891b2";

/// Magenta left boundary
pub const MAGENTA_COL_LEFT: &str = "#d946ef";

/// Magenta right boundary (darker)
pub const MAGENTA_COL_RIGHT: &str = "#c026d3";

/// Yellow left boundary
pub const YELLOW_COL_LEFT: &str = "#facc15";

/// Yellow right boundary (darker)
pub const YELLOW_COL_RIGHT: &str = "#ca8a04";

/// Green left boundary
pub const GREEN_COL_LEFT: &str = "#22c55e";

/// Green right boundary (darker)
pub const GREEN_COL_RIGHT: &str = "#16a34a";

/// Orange left boundary
pub const ORANGE_COL_LEFT: &str = "#f97316";

/// Orange right boundary (darker)
pub const ORANGE_COL_RIGHT: &str = "#ea580c";

/// Blue left boundary
pub const BLUE_COL_LEFT: &str = "#3b82f6";

/// Blue right boundary (darker)
pub const BLUE_COL_RIGHT: &str = "#2563eb";

/// Purple left boundary
pub const PURPLE_COL_LEFT: &str = "#a855f7";

/// Purple right boundary (darker)
pub const PURPLE_COL_RIGHT: &str = "#9333ea";

/// Red left boundary
pub const RED_COL_LEFT: &str = "#f43f5e";

/// Red right boundary (darker)
pub const RED_COL_RIGHT: &str = "#e11d48";

// ============== Special Layer Colors ==============

/// Blue for reading order arrows
pub const BLUE_READING_ORDER: &str = "#3b82f6";

/// Purple for MCID labels
pub const PURPLE_MCID: &str = "#9333ea";

/// Black for anchor labels
pub const BLACK_ANCHOR: &str = "#000000";

/// Cyan for OCR regions overlay
pub const CYAN_OCR: &str = "#00d9ff";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_to_color_boundaries() {
        assert_eq!(confidence_to_color(None), GRAY_NEUTRAL);
        assert_eq!(confidence_to_color(Some(0.0)), RED_LOW);
        assert_eq!(confidence_to_color(Some(0.49)), RED_LOW);
        assert_eq!(confidence_to_color(Some(0.5)), YELLOW_MEDIUM);
        assert_eq!(confidence_to_color(Some(0.79)), YELLOW_MEDIUM);
        assert_eq!(confidence_to_color(Some(0.8)), GREEN_HIGH);
        assert_eq!(confidence_to_color(Some(1.0)), GREEN_HIGH);
    }

    #[test]
    fn test_kind_to_color_all_kinds() {
        assert_eq!(kind_to_color("heading"), BLUE_HEADING);
        assert_eq!(kind_to_color("paragraph"), GRAY_PARAGRAPH);
        assert_eq!(kind_to_color("table"), TEAL_TABLE);
        assert_eq!(kind_to_color("list"), PURPLE_LIST);
        assert_eq!(kind_to_color("code"), ORANGE_CODE);
        assert_eq!(kind_to_color("header"), GRAY_LIGHT_HEADER);
        assert_eq!(kind_to_color("footer"), GRAY_LIGHT_HEADER);
        assert_eq!(kind_to_color("figure"), BROWN_FIGURE);
        assert_eq!(kind_to_color("caption"), PINK_CAPTION);
        assert_eq!(kind_to_color("unknown"), GRAY_DEFAULT);
    }

    #[test]
    fn test_column_boundary_color_cycles() {
        // Test that colors cycle through the palette
        assert_eq!(column_boundary_color(0, true), CYAN_COL_LEFT);
        assert_eq!(column_boundary_color(1, true), MAGENTA_COL_LEFT);
        assert_eq!(column_boundary_color(2, true), YELLOW_COL_LEFT);
        assert_eq!(column_boundary_color(8, true), CYAN_COL_LEFT); // cycles back

        // Test left vs right
        assert_eq!(column_boundary_color(0, true), CYAN_COL_LEFT);
        assert_eq!(column_boundary_color(0, false), CYAN_COL_RIGHT);
    }

    #[test]
    fn test_color_constants_are_valid_hex() {
        // All color constants should be valid 7-character hex codes
        let colors = [
            RED_LOW,
            YELLOW_MEDIUM,
            GREEN_HIGH,
            GRAY_NEUTRAL,
            BLUE_HEADING,
            GRAY_PARAGRAPH,
            TEAL_TABLE,
            PURPLE_LIST,
            ORANGE_CODE,
            GRAY_LIGHT_HEADER,
            BROWN_FIGURE,
            PINK_CAPTION,
            CYAN_COL_LEFT,
            CYAN_COL_RIGHT,
            MAGENTA_COL_LEFT,
            MAGENTA_COL_RIGHT,
            YELLOW_COL_LEFT,
            YELLOW_COL_RIGHT,
            GREEN_COL_LEFT,
            GREEN_COL_RIGHT,
            ORANGE_COL_LEFT,
            ORANGE_COL_RIGHT,
            BLUE_COL_LEFT,
            BLUE_COL_RIGHT,
            PURPLE_COL_LEFT,
            PURPLE_COL_RIGHT,
            RED_COL_LEFT,
            RED_COL_RIGHT,
            BLUE_READING_ORDER,
            PURPLE_MCID,
            BLACK_ANCHOR,
            CYAN_OCR,
        ];

        for color in colors {
            assert!(color.starts_with('#'), "{} should start with #", color);
            assert!(color.len() == 7, "{} should be 7 characters", color);
            // All chars after # should be hex digits
            assert!(
                color[1..].chars().all(|c| c.is_ascii_hexdigit()),
                "{} should be valid hex",
                color
            );
        }
    }
}
