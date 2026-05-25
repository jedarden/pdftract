//! Confidence heatmap layer renderer for the inspector.
//!
//! This module renders per-glyph translucent colored cells representing
//! extraction confidence. Red (< 0.5), yellow (0.5-0.8), and green (> 0.8)
//! indicate low, medium, and high confidence respectively.
//!
//! Each cell includes data-* attributes for tooltip consumption:
//! - data-char: the character
//! - data-confidence: the confidence score
//! - data-span-index: the parent span's index

use pdftract_core::schema::SpanJson;

/// Render SVG filled rectangles for each glyph in each span.
///
/// # Arguments
///
/// * `spans` - Slice of spans to render
///
/// # Returns
///
/// A vector of SVG `<rect>` element strings. Each rect is a translucent
/// colored cell positioned at the estimated glyph position.
///
/// # Color coding
///
/// - Red (#ef4444): confidence < 0.5 (low)
/// - Yellow (#eab308): 0.5 <= confidence < 0.8 (medium)
/// - Green (#22c55e): confidence >= 0.8 (high)
/// - Gray (#94a3b8): no confidence value (direct extraction)
///
/// # Data attributes
///
/// Each rect includes:
/// - `data-char`: the character
/// - `data-confidence`: confidence score or empty string
/// - `data-span-index`: the parent span's index
pub fn render_confidence_heatmap(spans: &[SpanJson]) -> Vec<String> {
    let mut cells = Vec::new();

    for (span_index, span) in spans.iter().enumerate() {
        let [x0, y0, x1, y1] = span.bbox;
        let span_width = x1 - x0;
        let span_height = y1 - y0;

        // Estimate character positions within the span
        let char_count = span.text.chars().count();
        if char_count == 0 {
            continue;
        }

        // Use font size to estimate glyph width and height
        let glyph_width = span_width / char_count as f64;
        let glyph_height = span.size.min(span_height);

        // Calculate vertical centering offset
        let y_offset = (span_height - glyph_height) / 2.0;

        let fill = confidence_to_color(span.confidence);
        let confidence_str = span.confidence.map(|c| c.to_string()).unwrap_or_default();
        let data_confidence = escape_xml_attr(&confidence_str);

        for (char_idx, ch) in span.text.chars().enumerate() {
            let char_x = x0 + (char_idx as f64 * glyph_width);
            let char_y = y0 + y_offset;
            let data_char = escape_xml_attr(&ch.to_string());

            cells.push(format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="0.3" class="heatmap-cell" data-char="{}" data-confidence="{}" data-span-index="{}" />"#,
                char_x, char_y, glyph_width, glyph_height, fill, data_char, data_confidence, span_index
            ));
        }
    }

    cells
}

/// Convert a confidence score to an SVG fill color.
///
/// # Arguments
///
/// * `confidence` - Optional confidence score (0.0 to 1.0)
///
/// # Returns
///
/// A CSS hex color string.
///
/// # Color mapping
///
/// - `None`: gray (#94a3b8) - direct extraction without OCR
/// - `Some(c) where c < 0.5`: red (#ef4444) - low confidence
/// - `Some(c) where 0.5 <= c < 0.8`: yellow (#eab308) - medium confidence
/// - `Some(c) where c >= 0.8`: green (#22c55e) - high confidence
fn confidence_to_color(confidence: Option<f64>) -> &'static str {
    match confidence {
        None => "#94a3b8",               // gray - direct extraction
        Some(c) if c < 0.5 => "#ef4444", // red - low confidence
        Some(c) if c < 0.8 => "#eab308", // yellow - medium confidence
        Some(_) => "#22c55e",            // green - high confidence
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

    #[test]
    fn test_confidence_to_color() {
        assert_eq!(confidence_to_color(None), "#94a3b8");
        assert_eq!(confidence_to_color(Some(0.3)), "#ef4444");
        assert_eq!(confidence_to_color(Some(0.6)), "#eab308");
        assert_eq!(confidence_to_color(Some(0.9)), "#22c55e");
    }

    #[test]
    fn test_escape_xml_attr() {
        assert_eq!(escape_xml_attr("hello"), "hello");
        assert_eq!(escape_xml_attr("a&b"), "a&amp;b");
        assert_eq!(escape_xml_attr("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml_attr("\"quote\""), "&quot;quote&quot;");
    }

    #[test]
    fn test_render_confidence_heatmap_empty() {
        let result = render_confidence_heatmap(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_confidence_heatmap_single_span() {
        let spans = vec![SpanJson {
            text: "ABC".to_string(),
            bbox: [100.0, 200.0, 400.0, 220.0],
            font: "Helvetica".to_string(),
            size: 20.0,
            color: None,
            rendering_mode: None,
            confidence: Some(0.9),
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let result = render_confidence_heatmap(&spans);
        assert_eq!(result.len(), 3); // 3 characters

        // Check that each cell has the expected attributes
        for cell in &result {
            assert!(cell.contains("class=\"heatmap-cell\""));
            assert!(cell.contains("fill=\"#22c55e\"")); // green for high confidence
            assert!(cell.contains("fill-opacity=\"0.3\""));
            assert!(cell.contains("data-span-index=\"0\""));
        }
    }

    #[test]
    fn test_render_confidence_heatmap_low_confidence() {
        let spans = vec![SpanJson {
            text: "X".to_string(),
            bbox: [0.0, 0.0, 10.0, 10.0],
            font: "Arial".to_string(),
            size: 10.0,
            color: None,
            rendering_mode: None,
            confidence: Some(0.3),
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let result = render_confidence_heatmap(&spans);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("fill=\"#ef4444\"")); // red for low confidence
    }

    #[test]
    fn test_render_confidence_heatmap_no_confidence() {
        let spans = vec![SpanJson {
            text: "Y".to_string(),
            bbox: [0.0, 0.0, 10.0, 10.0],
            font: "Arial".to_string(),
            size: 10.0,
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let result = render_confidence_heatmap(&spans);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("fill=\"#94a3b8\"")); // gray for no confidence
    }
}
