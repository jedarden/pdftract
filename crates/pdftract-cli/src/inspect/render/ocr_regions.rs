//! OCR regions layer renderer for the inspector.
//!
//! This module renders SVG diagonal-stripe overlays for text spans that
//! were extracted via OCR (Tesseract). This distinguishes vector-text spans
//! from OCR-derived spans visually.
//!
//! Each overlay includes data-* attributes for tooltip and click consumption:
//! - data-ocr-source: the confidence source (ocr, ocr-assisted, ocr-fallback)
//! - data-confidence: the OCR confidence score (0.0-1.0)
//! - data-text: the extracted text content (for tooltip display)
//! - data-span-index: the span's index in the page (for JSON-tree navigation)

use pdftract_core::schema::SpanJson;

/// Render SVG diagonal-stripe overlays for OCR-derived spans.
///
/// # Arguments
///
/// * `spans` - Slice of spans to filter and render
///
/// # Returns
///
/// A vector of SVG strings. The first element (if any OCR spans exist)
/// is a `<defs>` element containing the diagonal-stripe pattern definition.
/// Subsequent elements are `<rect>` overlays for each OCR span, using the
/// pattern as fill.
///
/// # Visual style
///
/// - Cyan (#00d9ff) diagonal stripes at 45° angle
/// - 4px stripe width, 8px spacing
/// - Translucent background (opacity 0.15)
/// - Thin cyan stroke (1px, opacity 0.5)
///
/// # Data attributes
///
/// Each rect includes:
/// - `data-ocr-source`: the span's confidence_source (XML-escaped)
/// - `data-confidence`: OCR confidence score or empty string
/// - `data-text`: the span's text content, truncated to 100 chars (XML-escaped)
/// - `data-span-index`: the span's index in the page (for JSON-tree navigation)
///
/// # CSS class
///
/// Each rect has class `ocr-region-rect` for styling and frontend toggling.
pub fn render_ocr_regions(spans: &[SpanJson]) -> Vec<String> {
    // Filter OCR spans
    let ocr_spans: Vec<(usize, &SpanJson)> = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| is_ocr_span(span))
        .collect();

    if ocr_spans.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();

    // Add pattern definition
    result.push(PATTERN_DEF.to_string());

    // Add overlay rects for each OCR span
    for (index, span) in ocr_spans {
        let [x0, y0, x1, y1] = span.bbox;
        let width = x1 - x0;
        let height = y1 - y0;
        let data_source = escape_xml_attr(
            span.confidence_source.as_deref().unwrap_or("")
        );
        let confidence_str = span.confidence.map(|c| c.to_string()).unwrap_or_default();
        let data_confidence = escape_xml_attr(&confidence_str);

        // Truncate text for tooltip (max ~100 chars)
        let tooltip_text = if span.text.len() > 99 {
            format!("{}...", &span.text[..99])
        } else {
            span.text.clone()
        };
        let data_text = escape_xml_attr(&tooltip_text);

        result.push(format!(
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="url(#ocr-diagonal-stripes)" fill-opacity="0.15" stroke="#00d9ff" stroke-width="1" stroke-opacity="0.5" class="ocr-region-rect" data-ocr-source="{}" data-confidence="{}" data-text="{}" data-span-index="{}" />"##,
            x0, y0, width, height, data_source, data_confidence, data_text, index
        ));
    }

    result
}

/// Check if a span was extracted via OCR.
///
/// Returns true if the span's confidence_source contains "ocr"
/// (matches: "ocr", "ocr-assisted", "ocr-fallback").
fn is_ocr_span(span: &SpanJson) -> bool {
    span.confidence_source
        .as_ref()
        .map(|s| s.contains("ocr"))
        .unwrap_or(false)
}

/// SVG pattern definition for cyan diagonal stripes.
///
/// 45° diagonal stripes, 4px wide, 8px spacing, cyan (#00d9ff).
const PATTERN_DEF: &str = r##"<defs>
  <pattern id="ocr-diagonal-stripes" patternUnits="userSpaceOnUse" width="8" height="8" patternTransform="rotate(45)">
    <rect width="8" height="8" fill="#00d9ff" fill-opacity="0" />
    <line x1="0" y1="0" x2="0" y2="8" stroke="#00d9ff" stroke-width="4" stroke-opacity="0.3" />
  </pattern>
</defs>"##;

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

    fn make_test_span(text: &str, bbox: [f64; 4], confidence_source: Option<&str>) -> SpanJson {
        SpanJson {
            text: text.to_string(),
            bbox,
            font: "Arial".to_string(),
            size: 12.0,
            color: None,
            rendering_mode: None,
            confidence: Some(0.85),
            confidence_source: confidence_source.map(|s| s.to_string()),
            lang: None,
            flags: vec![],
            receipt: None,
            column: None,
        }
    }

    #[test]
    fn test_render_ocr_regions_empty() {
        let spans: Vec<SpanJson> = vec![];
        let output = render_ocr_regions(&spans);
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_ocr_regions_no_ocr_spans() {
        let spans = vec![
            make_test_span("Vector text", [100.0, 200.0, 300.0, 220.0], Some("vector")),
            make_test_span("Native text", [100.0, 230.0, 300.0, 250.0], Some("native")),
        ];
        let output = render_ocr_regions(&spans);
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_ocr_regions_single() {
        let spans = vec![make_test_span(
            "OCR text",
            [100.0, 200.0, 300.0, 220.0],
            Some("ocr"),
        )];

        let output = render_ocr_regions(&spans);
        assert!(!output.is_empty());

        // First element should be pattern definition
        assert!(output[0].contains("<defs>"));
        assert!(output[0].contains("ocr-diagonal-stripes"));

        // Second element should be overlay rect
        assert!(output.len() >= 2);
        let rect = &output[1];

        // Check basic SVG structure
        assert!(rect.contains("<rect"));
        assert!(rect.contains(r#"x="100.00""#));
        assert!(rect.contains(r#"y="200.00""#));
        assert!(rect.contains(r#"width="200.00""#)); // 300 - 100
        assert!(rect.contains(r#"height="20.00""#)); // 220 - 200

        // Check fill uses pattern
        assert!(rect.contains(r#"fill="url(#ocr-diagonal-stripes)""#));

        // Check stroke
        assert!(rect.contains(r##"stroke="#00d9ff""##));
        assert!(rect.contains(r##"stroke-width="1""##));
        assert!(rect.contains(r##"stroke-opacity="0.5""##));

        // Check data attributes
        assert!(rect.contains(r#"data-ocr-source="ocr""#));
        assert!(rect.contains(r#"data-confidence="0.85""#));
        assert!(rect.contains(r#"data-text="OCR text""#));
        assert!(rect.contains(r#"data-span-index="0""#));
    }

    #[test]
    fn test_render_ocr_regions_multiple() {
        let spans = vec![
            make_test_span("OCR 1", [50.0, 100.0, 150.0, 120.0], Some("ocr")),
            make_test_span("Vector", [50.0, 130.0, 150.0, 150.0], Some("vector")),
            make_test_span("OCR 2", [50.0, 160.0, 200.0, 180.0], Some("ocr-assisted")),
        ];

        let output = render_ocr_regions(&spans);

        // Should have pattern def + 2 overlay rects (not the vector span)
        assert!(output.len() >= 3);

        // Check first OCR span
        assert!(output[1].contains(r#"data-ocr-source="ocr""#));
        assert!(output[1].contains(r#"data-text="OCR 1""#));

        // Check second OCR span
        assert!(output[2].contains(r#"data-ocr-source="ocr-assisted""#));
        assert!(output[2].contains(r#"data-text="OCR 2""#));
    }

    #[test]
    fn test_render_ocr_regions_all_ocr_sources() {
        let test_cases = [
            ("ocr", true),
            ("ocr-assisted", true),
            ("ocr-fallback", true),
            ("vector", false),
            ("native", false),
            ("heuristic", false),
        ];

        for (source, expected_render) in test_cases {
            let spans = vec![make_test_span("Test", [0.0, 0.0, 100.0, 20.0], Some(source))];
            let output = render_ocr_regions(&spans);

            if expected_render {
                assert!(!output.is_empty(), "Source '{}' should render", source);
                assert!(output[1].contains(&format!("data-ocr-source=\"{}\"", source)));
            } else {
                assert!(output.is_empty(), "Source '{}' should not render", source);
            }
        }
    }

    #[test]
    fn test_render_ocr_regions_text_truncation() {
        let long_text = "a".repeat(200);
        let spans = vec![make_test_span(
            &long_text,
            [0.0, 0.0, 100.0, 20.0],
            Some("ocr"),
        )];

        let output = render_ocr_regions(&spans);
        let rect = &output[1];

        // Text should be truncated with "..." suffix
        assert!(rect.contains("data-text=\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...\""));
    }

    #[test]
    fn test_render_ocr_regions_xml_escaping() {
        let spans = vec![make_test_span(
            "Text & <tags>",
            [0.0, 0.0, 100.0, 20.0],
            Some("ocr"),
        )];

        let output = render_ocr_regions(&spans);
        let rect = &output[1];

        // Check XML escaping in data attributes
        assert!(rect.contains("data-text=\"Text &amp; &lt;tags&gt;\""));
    }

    #[test]
    fn test_render_ocr_regions_confidence_none() {
        let mut span = make_test_span("OCR", [0.0, 0.0, 100.0, 20.0], Some("ocr"));
        span.confidence = None;

        let output = render_ocr_regions(&[span]);
        let rect = &output[1];

        // Should have empty confidence string
        assert!(rect.contains(r#"data-confidence="""#));
    }

    #[test]
    fn test_render_ocr_regions_css_class() {
        let spans = vec![make_test_span("OCR", [0.0, 0.0, 100.0, 20.0], Some("ocr"))];

        let output = render_ocr_regions(&spans);
        let rect = &output[1];

        assert!(rect.contains(r#"class="ocr-region-rect""#));
    }

    #[test]
    fn test_is_ocr_span() {
        let mut span = make_test_span("Test", [0.0, 0.0, 100.0, 20.0], Some("ocr"));
        assert!(is_ocr_span(&span));

        span.confidence_source = Some("ocr-assisted".to_string());
        assert!(is_ocr_span(&span));

        span.confidence_source = Some("ocr-fallback".to_string());
        assert!(is_ocr_span(&span));

        span.confidence_source = Some("vector".to_string());
        assert!(!is_ocr_span(&span));

        span.confidence_source = None;
        assert!(!is_ocr_span(&span));
    }

    #[test]
    fn test_escape_xml_attr() {
        assert_eq!(escape_xml_attr("hello"), "hello");
        assert_eq!(escape_xml_attr("a&b"), "a&amp;b");
        assert_eq!(escape_xml_attr("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml_attr("\"quote\""), "&quot;quote&quot;");
        assert_eq!(escape_xml_attr("'apos'"), "&apos;apos&apos;");
    }

    #[test]
    fn test_render_ocr_regions_pattern_def() {
        let spans = vec![make_test_span("OCR", [0.0, 0.0, 100.0, 20.0], Some("ocr"))];
        let output = render_ocr_regions(&spans);

        // Check pattern definition structure
        assert!(output[0].contains("<pattern id=\"ocr-diagonal-stripes\""));
        assert!(output[0].contains("patternUnits=\"userSpaceOnUse\""));
        assert!(output[0].contains("width=\"8\""));
        assert!(output[0].contains("height=\"8\""));
        assert!(output[0].contains("patternTransform=\"rotate(45)\""));
        assert!(output[0].contains("stroke=\"#00d9ff\""));
        assert!(output[0].contains("stroke-width=\"4\""));
        assert!(output[0].contains("stroke-opacity=\"0.3\""));
    }

    #[test]
    fn test_render_ocr_regions_output_is_valid_svg() {
        let spans = vec![make_test_span("OCR", [0.0, 0.0, 100.0, 20.0], Some("ocr"))];
        let output = render_ocr_regions(&spans);

        // Pattern def should be valid XML
        assert!(output[0].starts_with("<defs>"));
        assert!(output[0].ends_with("</defs>"));

        // Rect should be valid XML
        assert!(output[1].starts_with("<rect"));
        assert!(output[1].ends_with(" />"));
    }

    #[test]
    fn test_render_ocr_regions_float_bbox() {
        let spans = vec![make_test_span(
            "OCR",
            [10.567, 20.891, 100.234, 110.567],
            Some("ocr"),
        )];

        let output = render_ocr_regions(&spans);
        let rect = &output[1];

        // Check that coordinates are rounded to 2 decimal places
        assert!(rect.contains(r#"x="10.57""#));
        assert!(rect.contains(r#"y="20.89""#));
        assert!(rect.contains(r#"width="89.67""#)); // 100.234 - 10.567
        assert!(rect.contains(r#"height="89.68""#)); // 110.567 - 20.891
    }

    #[test]
    fn test_render_ocr_regions_span_index_tracking() {
        let spans = vec![
            make_test_span("Vector", [0.0, 0.0, 50.0, 10.0], Some("vector")),
            make_test_span("OCR 1", [0.0, 20.0, 50.0, 30.0], Some("ocr")),
            make_test_span("Vector 2", [0.0, 40.0, 50.0, 50.0], Some("vector")),
            make_test_span("OCR 2", [0.0, 60.0, 50.0, 70.0], Some("ocr")),
        ];

        let output = render_ocr_regions(&spans);

        // Should have 2 OCR rects
        assert_eq!(output.len(), 3); // pattern def + 2 rects

        // Check span indices (should be 1 and 3, not 0 and 1)
        assert!(output[1].contains(r#"data-span-index="1""#)); // OCR 1 is at index 1
        assert!(output[2].contains(r#"data-span-index="3""#)); // OCR 2 is at index 3
    }
}
