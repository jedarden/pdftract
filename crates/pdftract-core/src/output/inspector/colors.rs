//! Color constants for inspector overlay layers.
//!
//! Centralized color definitions matching plan section 7.9.5 (lines 2852-2863).
//! All colors are specified as CSS hex strings for direct SVG embedding.

/// Convert an f64 confidence score to a color encoding.
///
/// Per plan spec:
/// - red (< 0.5): low confidence
/// - yellow (0.5-0.8): medium confidence
/// - green (> 0.8): high confidence
///
/// # Arguments
///
/// * `confidence` - Confidence score in range [0.0, 1.0]
///
/// # Returns
///
/// CSS hex color string (e.g., "#ff0000" for red).
pub fn confidence_to_color(confidence: f64) -> &'static str {
    if confidence < 0.5 {
        "#ff4444" // red
    } else if confidence < 0.8 {
        "#ffcc00" // yellow
    } else {
        "#44cc44" // green
    }
}

/// Convert a block kind string to its corresponding fill color.
///
/// Per plan spec (line 2857):
/// - heading: blue
/// - paragraph: gray
/// - table: teal
/// - list: purple
/// - code: orange
/// - header/footer: light gray
/// - figure: brown
/// - caption: pink
///
/// # Arguments
///
/// * `kind` - Block kind string (e.g., "paragraph", "heading")
///
/// # Returns
///
/// CSS hex color string with opacity for translucent effect.
pub fn kind_to_color(kind: &str) -> &'static str {
    match kind {
        "heading" => "#4a90e2",     // blue
        "paragraph" => "#808080",   // gray
        "table" => "#50c8c8",       // teal
        "list" => "#9b59b6",        // purple
        "code" => "#f39c12",        // orange
        "header_footer" => "#d3d3d3", // light gray
        "figure" => "#8b4513",      // brown
        "caption" => "#ff69b4",     // pink
        _ => "#cccccc",             // default gray
    }
}

/// Get the stroke color for a block kind (darker version of fill).
///
/// Used for block outline borders to provide better contrast.
pub fn kind_to_stroke_color(kind: &str) -> &'static str {
    match kind {
        "heading" => "#2a5a8a",     // darker blue
        "paragraph" => "#505050",   // darker gray
        "table" => "#30a0a0",       // darker teal
        "list" => "#6b3a86",        // darker purple
        "code" => "#c47c0a",        // darker orange
        "header_footer" => "#a3a3a3", // darker light gray
        "figure" => "#5a2a0a",      // darker brown
        "caption" => "#d43984",     // darker pink
        _ => "#999999",             // default darker gray
    }
}

/// SVG pattern definition for OCR region diagonal stripes.
///
/// Returns the SVG `<pattern>` element that renders cyan diagonal stripes
/// on OCR-sourced text regions.
pub fn ocr_pattern_definition() -> &'static str {
    r##"<pattern id="ocr-diagonal-stripes" patternUnits="userSpaceOnUse" width="8" height="8" patternTransform="rotate(45)">
    <rect width="8" height="8" fill="#00ffff" fill-opacity="0.15"/>
    <line x1="0" y1="0" x2="0" y2="8" stroke="#00ffff" stroke-width="2" stroke-opacity="0.3"/>
</pattern>"##
}

/// Column label color for the "Col N" text at page top.
pub const COLUMN_LABEL_COLOR: &str = "#666666";

/// Reading order arrow color.
pub const READING_ORDER_ARROW_COLOR: &str = "#ff6600";

/// Reading order label color (numbered 1, 2, 3, ...).
pub const READING_ORDER_LABEL_COLOR: &str = "#ff6600";

/// MCID label color (numeric MCID in corners).
pub const MCID_LABEL_COLOR: &str = "#00ccff";

/// Anchor label color (block-id at top-left).
pub const ANCHOR_LABEL_COLOR: &str = "#999999";

/// Column boundary line color (dashed vertical lines).
pub const COLUMN_LINE_COLOR: &str = "#aaaaaa";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_to_color_low() {
        assert_eq!(confidence_to_color(0.2), "#ff4444");
        assert_eq!(confidence_to_color(0.49), "#ff4444");
        assert_eq!(confidence_to_color(0.0), "#ff4444");
    }

    #[test]
    fn test_confidence_to_color_medium() {
        assert_eq!(confidence_to_color(0.5), "#ffcc00");
        assert_eq!(confidence_to_color(0.65), "#ffcc00");
        assert_eq!(confidence_to_color(0.79), "#ffcc00");
    }

    #[test]
    fn test_confidence_to_color_high() {
        assert_eq!(confidence_to_color(0.8), "#44cc44");
        assert_eq!(confidence_to_color(0.9), "#44cc44");
        assert_eq!(confidence_to_color(1.0), "#44cc44");
    }

    #[test]
    fn test_kind_to_color() {
        assert_eq!(kind_to_color("heading"), "#4a90e2");
        assert_eq!(kind_to_color("paragraph"), "#808080");
        assert_eq!(kind_to_color("table"), "#50c8c8");
        assert_eq!(kind_to_color("list"), "#9b59b6");
        assert_eq!(kind_to_color("code"), "#f39c12");
        assert_eq!(kind_to_color("header_footer"), "#d3d3d3");
        assert_eq!(kind_to_color("figure"), "#8b4513");
        assert_eq!(kind_to_color("caption"), "#ff69b4");
        assert_eq!(kind_to_color("unknown"), "#cccccc");
    }

    #[test]
    fn test_kind_to_stroke_color() {
        assert_eq!(kind_to_stroke_color("heading"), "#2a5a8a");
        assert_eq!(kind_to_stroke_color("paragraph"), "#505050");
        assert_eq!(kind_to_stroke_color("unknown"), "#999999");
    }

    #[test]
    fn test_ocr_pattern_definition_is_valid_svg() {
        let pattern = ocr_pattern_definition();
        assert!(pattern.contains("<pattern"));
        assert!(pattern.contains("id=\"ocr-diagonal-stripes\""));
        assert!(pattern.contains("#00ffff"));
        assert!(pattern.contains("</pattern>"));
    }

    #[test]
    fn test_color_constants_are_hex() {
        // All color constants should be valid hex colors
        assert!(COLUMN_LABEL_COLOR.starts_with('#'));
        assert!(READING_ORDER_ARROW_COLOR.starts_with('#'));
        assert!(READING_ORDER_LABEL_COLOR.starts_with('#'));
        assert!(MCID_LABEL_COLOR.starts_with('#'));
        assert!(ANCHOR_LABEL_COLOR.starts_with('#'));
        assert!(COLUMN_LINE_COLOR.starts_with('#'));
    }
}
