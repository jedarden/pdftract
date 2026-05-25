//! Span layer renderer for the inspector.
//!
//! This module renders SVG outline rectangles for each text span,
//! color-coded by extraction confidence. Red (< 0.5), yellow (0.5-0.8),
//! and green (> 0.8) indicate low, medium, and high confidence respectively.
//!
//! Each rect includes data-* attributes for tooltip and click consumption:
//! - data-text: the extracted text content
//! - data-confidence: the confidence score (0.0-1.0)
//! - data-font: the font name
//! - data-size: the font size in points
//! - data-span-index: the span's index in the page (for JSON-tree navigation)

use pdftract_core::schema::SpanJson;

/// Render SVG outline rectangles for each span.
///
/// # Arguments
///
/// * `spans` - Slice of spans to render
///
/// # Returns
///
/// A vector of SVG `<rect>` element strings. Each rect is positioned at
/// the span's bbox with stroke color indicating confidence.
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
/// - `data-text`: the span's text content (XML-escaped)
/// - `data-confidence`: confidence score or empty string
/// - `data-font`: font name (XML-escaped)
/// - `data-size`: font size in points
/// - `data-span-index`: the span's index in the page (for JSON-tree navigation)
pub fn render_spans(spans: &[SpanJson]) -> Vec<String> {
    spans.iter().enumerate().map(|(index, span)| {
        let [x0, y0, x1, y1] = span.bbox;
        let width = x1 - x0;
        let height = y1 - y0;
        let stroke = confidence_to_color(span.confidence);
        let data_text = escape_xml_attr(&span.text);
        let data_font = escape_xml_attr(&span.font);
        let confidence_str = span.confidence.map(|c| c.to_string()).unwrap_or_default();
        let data_confidence = escape_xml_attr(&confidence_str);

        format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="{}" stroke-width="1" class="span-rect" data-text="{}" data-confidence="{}" data-font="{}" data-size="{}" data-span-index="{}" />"#,
            x0, y0, width, height, stroke, data_text, data_confidence, data_font, span.size, index
        )
    }).collect()
}

/// Convert a confidence score to an SVG stroke color.
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
    fn test_render_spans_empty() {
        let spans: Vec<SpanJson> = vec![];
        let output = render_spans(&spans);
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_spans_single() {
        let spans = vec![SpanJson {
            text: "Hello".to_string(),
            bbox: [100.0, 200.0, 200.0, 220.0],
            font: "Helvetica".to_string(),
            size: 12.0,
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let output = render_spans(&spans);
        assert_eq!(output.len(), 1);
        let rect = &output[0];

        // Check basic SVG structure
        assert!(rect.contains("<rect"));
        assert!(rect.contains(r#"x="100.00""#));
        assert!(rect.contains(r#"y="200.00""#));
        assert!(rect.contains(r#"width="100.00""#)); // 200 - 100
        assert!(rect.contains(r#"height="20.00""#)); // 220 - 200
        assert!(rect.contains(r#"fill="none""#));
        assert!(rect.contains(r#"stroke-width="1""#));

        // Check gray stroke for no confidence (direct extraction)
        assert!(rect.contains("stroke=\"#94a3b8\""));

        // Check data attributes
        assert!(rect.contains(r#"data-text="Hello""#));
        assert!(rect.contains(r#"data-font="Helvetica""#));
        assert!(rect.contains(r#"data-size="12""#));
        assert!(rect.contains(r#"data-confidence="""#)); // empty string for None
        assert!(rect.contains(r#"data-span-index="0""#));
    }

    #[test]
    fn test_render_spans_confidence_colors() {
        let test_cases = [
            (None, "#94a3b8"),       // gray - no confidence
            (Some(0.3), "#ef4444"),  // red - low
            (Some(0.5), "#eab308"),  // yellow - medium (boundary)
            (Some(0.6), "#eab308"),  // yellow - medium
            (Some(0.79), "#eab308"), // yellow - medium (boundary)
            (Some(0.8), "#22c55e"),  // green - high (boundary)
            (Some(0.95), "#22c55e"), // green - high
            (Some(1.0), "#22c55e"),  // green - perfect
        ];

        for (confidence, expected_color) in test_cases {
            let spans = vec![SpanJson {
                text: "Test".to_string(),
                bbox: [0.0, 0.0, 10.0, 10.0],
                font: "Arial".to_string(),
                size: 10.0,
                color: None,
                rendering_mode: None,
                confidence,
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            }];

            let output = render_spans(&spans);
            assert_eq!(output.len(), 1);
            assert!(
                output[0].contains(&format!("stroke=\"{}\"", expected_color)),
                "Confidence {:?} should produce color {}, got: {}",
                confidence,
                expected_color,
                output[0]
            );
        }
    }

    #[test]
    fn test_render_spans_data_attributes() {
        let spans = vec![SpanJson {
            text: "Test & <quote>".to_string(),
            bbox: [50.0, 100.0, 150.0, 120.0],
            font: "Times \"Roman\"".to_string(),
            size: 14.0,
            color: None,
            rendering_mode: None,
            confidence: Some(0.85),
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let output = render_spans(&spans);
        let rect = &output[0];

        // Check XML escaping in data attributes
        assert!(rect.contains("data-text=\"Test &amp; &lt;quote&gt;\""));
        assert!(rect.contains("data-font=\"Times &quot;Roman&quot;\""));
        assert!(rect.contains("data-confidence=\"0.85\""));
        assert!(rect.contains("data-size=\"14\""));
        assert!(rect.contains("data-span-index=\"0\""));
    }

    #[test]
    fn test_render_spans_span_index() {
        let spans = vec![
            SpanJson {
                text: "First".to_string(),
                bbox: [0.0, 0.0, 50.0, 10.0],
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
            },
            SpanJson {
                text: "Second".to_string(),
                bbox: [60.0, 0.0, 120.0, 10.0],
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
            },
            SpanJson {
                text: "Third".to_string(),
                bbox: [130.0, 0.0, 180.0, 10.0],
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
            },
        ];

        let output = render_spans(&spans);
        assert_eq!(output.len(), 3);

        // Check that each span has the correct index
        assert!(output[0].contains("data-span-index=\"0\""));
        assert!(output[1].contains("data-span-index=\"1\""));
        assert!(output[2].contains("data-span-index=\"2\""));
    }

    #[test]
    fn test_render_spans_multiple() {
        let spans = vec![
            SpanJson {
                text: "First".to_string(),
                bbox: [0.0, 0.0, 50.0, 10.0],
                font: "Arial".to_string(),
                size: 10.0,
                color: None,
                rendering_mode: None,
                confidence: Some(0.9), // green
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            },
            SpanJson {
                text: "Second".to_string(),
                bbox: [60.0, 0.0, 120.0, 10.0],
                font: "Arial".to_string(),
                size: 10.0,
                color: None,
                rendering_mode: None,
                confidence: Some(0.6), // yellow
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            },
            SpanJson {
                text: "Third".to_string(),
                bbox: [130.0, 0.0, 180.0, 10.0],
                font: "Arial".to_string(),
                size: 10.0,
                color: None,
                rendering_mode: None,
                confidence: Some(0.3), // red
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            },
        ];

        let output = render_spans(&spans);
        assert_eq!(output.len(), 3);

        // Check that each has the correct color
        assert!(output[0].contains("stroke=\"#22c55e\"")); // green
        assert!(output[1].contains("stroke=\"#eab308\"")); // yellow
        assert!(output[2].contains("stroke=\"#ef4444\"")); // red
    }

    #[test]
    fn test_render_spans_css_class() {
        let spans = vec![SpanJson {
            text: "Test".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Arial".to_string(),
            size: 12.0,
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let output = render_spans(&spans);
        assert!(output[0].contains(r#"class="span-rect""#));
    }

    #[test]
    fn test_confidence_to_color_boundaries() {
        // Test exact boundary conditions
        assert_eq!(confidence_to_color(None), "#94a3b8");
        assert_eq!(confidence_to_color(Some(0.0)), "#ef4444");
        assert_eq!(confidence_to_color(Some(0.49)), "#ef4444");
        assert_eq!(confidence_to_color(Some(0.5)), "#eab308");
        assert_eq!(confidence_to_color(Some(0.79)), "#eab308");
        assert_eq!(confidence_to_color(Some(0.8)), "#22c55e");
        assert_eq!(confidence_to_color(Some(1.0)), "#22c55e");
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
    fn test_render_spans_float_bbox() {
        let spans = vec![SpanJson {
            text: "Float".to_string(),
            bbox: [10.567, 20.891, 100.234, 110.567],
            font: "Arial".to_string(),
            size: 12.5,
            color: None,
            rendering_mode: None,
            confidence: None,
            confidence_source: None,
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let output = render_spans(&spans);
        let rect = &output[0];

        // Check that coordinates are rounded to 2 decimal places
        assert!(rect.contains(r#"x="10.57""#));
        assert!(rect.contains(r#"y="20.89""#));
        assert!(rect.contains(r#"width="89.67""#)); // 100.234 - 10.567
        assert!(rect.contains(r#"height="89.68""#)); // 110.567 - 20.891
    }

    #[test]
    fn test_render_spans_output_is_valid_svg() {
        let spans = vec![SpanJson {
            text: "Valid".to_string(),
            bbox: [0.0, 0.0, 100.0, 20.0],
            font: "Arial".to_string(),
            size: 12.0,
            color: None,
            rendering_mode: None,
            confidence: Some(0.95),
            confidence_source: Some("vector".to_string()),
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }];

        let output = render_spans(&spans);
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
        assert!(rect.contains("stroke="));
        assert!(rect.contains("stroke-width="));
        assert!(rect.contains("class="));
    }
}
