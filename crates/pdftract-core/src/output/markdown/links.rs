//! Markdown inline-link emission from Phase 7.6 link annotations.
//!
//! This module implements Phase 6.5.5b: inline-link emission in the Markdown sink.
//! Spans whose bbox falls under a Phase 7.6 link annotation rect get wrapped as
//! \[anchor text\](URL). The anchor text is the concatenated span text; the URL is from
//! the link annotation's /A /URI or /Dest resolved to a URL fragment.

use crate::annotation::links::{DestArray, FitType, LinkAnnotation};
use crate::schema::{LinkJson, SpanJson};

/// A resolved link target for Markdown emission.
///
/// Represents either an external URI or an internal page destination.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// External URI (https://..., http://..., etc.)
    External(String),
    /// Internal destination to a page (#page-N)
    InternalPage(usize),
    /// Internal named destination (dest name without page resolution)
    InternalNamed(String),
    /// No valid target (diagnostic placeholder)
    None,
}

/// Compute the center point of a bounding box.
///
/// Returns (center_x, center_y) for the bbox [x0, y0, x1, y1].
fn bbox_center(bbox: &[f64; 4]) -> (f64, f64) {
    ((bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0)
}

/// Check if a point is within a rectangle.
///
/// Point (px, py) is within rect [x0, y0, x1, y1] if x0 <= px <= x1 and y0 <= py <= y1.
fn point_in_rect(px: f64, py: f64, rect: &[f32; 4]) -> bool {
    px >= f64::from(rect[0])
        && px <= f64::from(rect[2])
        && py >= f64::from(rect[1])
        && py <= f64::from(rect[3])
}

/// Resolve a link annotation to a Markdown link target.
///
/// # Arguments
///
/// * `link` - The link annotation from Phase 7.6
///
/// # Returns
///
/// A `LinkTarget` representing the resolved destination.
pub fn resolve_link_target(link: &LinkAnnotation) -> LinkTarget {
    // Prefer URI for external links
    if let Some(uri) = &link.uri {
        // Filter out javascript: and other non-http schemes for safety
        if uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("mailto:") {
            return LinkTarget::External(uri.clone());
        }
        // For javascript: and other schemes, treat as no target
        return LinkTarget::None;
    }

    // Check for explicit destination array with page index
    if let Some(dest_array) = &link.dest_array {
        if let Some(page_index) = resolve_page_from_dest(dest_array) {
            return LinkTarget::InternalPage(page_index);
        }
    }

    // Fall back to named destination
    if let Some(dest) = &link.dest {
        return LinkTarget::InternalNamed(dest.clone());
    }

    LinkTarget::None
}

/// Resolve page index from a destination array.
///
/// Returns the page index if resolvable, None otherwise.
fn resolve_page_from_dest(dest: &DestArray) -> Option<usize> {
    // For now, return the page_index from dest if available
    // In a full implementation, this would handle all fit types
    Some(dest.page_index)
}

/// Escape special characters in Markdown link text.
///
/// Per CommonMark spec, square brackets and backslashes must be escaped in link text.
/// We process in a single pass to avoid double-escaping already-escaped sequences like `\[`.
fn escape_link_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    let mut chars = text.chars().peekable();
    let mut backslash_count = 0;

    while let Some(c) = chars.next() {
        if c == '\\' {
            backslash_count += 1;
            // Always escape backslashes in link text
            result.push_str("\\\\");
        } else if c == '[' || c == ']' {
            // Only escape brackets if NOT preceded by odd number of backslashes
            // (odd number means the bracket is already escaped like `\[`)
            if backslash_count % 2 == 0 {
                result.push('\\');
            }
            backslash_count = 0;
            result.push(c);
        } else {
            backslash_count = 0;
            result.push(c);
        }
    }

    result
}

/// Percent-encode a URL for Markdown link destination.
///
/// Encodes parentheses, whitespace, and other characters that would break Markdown parsing.
fn percent_encode_url(url: &str) -> String {
    let mut result = String::new();
    for byte in url.bytes() {
        let ch = byte as char;
        // Characters that must be encoded in Markdown link URLs
        if ch == '(' || ch == ')' || ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
            // Percent-encode
            result.push_str(&format!("%{:02X}", byte));
        } else {
            result.push(ch);
        }
    }
    result
}

/// Emit an inline Markdown link.
///
/// # Arguments
///
/// * `text` - The anchor text (already escaped)
/// * `target` - The resolved link target
///
/// # Returns
///
/// A Markdown inline link string, or empty text if no valid target.
pub fn emit_inline_link(text: &str, target: &LinkTarget) -> String {
    let escaped_text = escape_link_text(text);
    match target {
        LinkTarget::External(url) => {
            let encoded_url = percent_encode_url(url);
            format!("[{}]({})", escaped_text, encoded_url)
        }
        LinkTarget::InternalPage(page_index) => {
            // Zero-based to one-based for display
            format!("[{}](#page-{})", escaped_text, page_index + 1)
        }
        LinkTarget::InternalNamed(dest) => {
            // Emit as a named anchor without page resolution
            format!("[{}](#{})", escaped_text, dest)
        }
        LinkTarget::None => escaped_text, // No link, just emit the text
    }
}

/// Find spans whose bbox center falls within a link annotation's rect.
///
/// Returns the indices of spans that should be included in the link anchor text.
///
/// # Arguments
///
/// * `spans` - All spans on the page
/// * `link` - The link annotation
///
/// # Returns
///
/// A vector of span indices whose centers fall within the link rect.
pub fn find_spans_in_link(spans: &[SpanJson], link: &LinkAnnotation) -> Vec<usize> {
    let mut matched = Vec::new();

    let Some(link_rect) = link.common.rect else {
        return matched;
    };

    for (idx, span) in spans.iter().enumerate() {
        let (cx, cy) = bbox_center(&span.bbox);
        if point_in_rect(cx, cy, &link_rect) {
            matched.push(idx);
        }
    }

    // Sort by index to preserve document order
    matched.sort();
    matched
}

/// Concatenate span texts to form anchor text.
///
/// Spaces are inserted between spans when there's a gap in the x-coordinate
/// (typical for word breaks in PDF text extraction).
///
/// # Arguments
///
/// * `spans` - All spans on the page
/// * `span_indices` - Indices of spans to concatenate
///
/// # Returns
///
/// Concatenated text from the specified spans, with spaces inserted where appropriate.
pub fn concatenate_anchor_text(spans: &[SpanJson], span_indices: &[usize]) -> String {
    let mut result = String::new();

    for (i, &idx) in span_indices.iter().enumerate() {
        if let Some(span) = spans.get(idx) {
            // Add space before this span if there's a gap from the previous span
            if i > 0 {
                if let Some(&prev_idx) = span_indices.get(i - 1) {
                    if let Some(prev_span) = spans.get(prev_idx) {
                        // Check if there's a gap between spans (more than 2 points indicates a space)
                        let gap = span.bbox[0] - prev_span.bbox[2];
                        if gap > 2.0 {
                            result.push(' ');
                        }
                    }
                }
            }
            result.push_str(&span.text);
        }
    }

    result
}

/// Emit all inline links for a page's spans.
///
/// Returns a vector of (span_indices, link_markdown) tuples representing all
/// inline links to be emitted on this page. Each span index appears at most
/// once across all links (first link wins).
///
/// # Arguments
///
/// * `spans` - All spans on the page
/// * `links` - All link annotations on the page
///
/// # Returns
///
/// A vector of (span_indices, markdown_string) tuples.
pub fn emit_page_links(spans: &[SpanJson], links: &[LinkAnnotation]) -> Vec<(Vec<usize>, String)> {
    let mut results = Vec::new();
    let mut used_spans = std::collections::HashSet::new();

    for link in links {
        let span_indices = find_spans_in_link(spans, link);
        if span_indices.is_empty() {
            continue; // Skip links with no anchor text
        }

        let target = resolve_link_target(link);
        if target == LinkTarget::None {
            continue; // Skip links with no valid target
        }

        let anchor_text = concatenate_anchor_text(spans, &span_indices);
        if anchor_text.is_empty() {
            continue; // Skip links with empty anchor text
        }

        let markdown = emit_inline_link(&anchor_text, &target);

        // Filter out already-used spans (first link wins)
        let available_indices: Vec<usize> = span_indices
            .into_iter()
            .filter(|idx| !used_spans.contains(idx))
            .collect();

        if !available_indices.is_empty() {
            for &idx in &available_indices {
                used_spans.insert(idx);
            }
            results.push((available_indices, markdown));
        }
    }

    results
}

/// Resolve a LinkJson to a Markdown link target.
///
/// This is a variant of `resolve_link_target` that works with `LinkJson`
/// (the JSON-serializable type) instead of `LinkAnnotation` (the internal type).
///
/// # Arguments
///
/// * `link` - The link JSON from Phase 7.6
///
/// # Returns
///
/// A `LinkTarget` representing the resolved destination.
pub fn resolve_link_target_from_json(link: &LinkJson) -> LinkTarget {
    // Prefer URI for external links
    if let Some(uri) = &link.uri {
        // Filter out javascript: and other non-http schemes for safety
        if uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("mailto:") {
            return LinkTarget::External(uri.clone());
        }
        // For javascript: and other schemes, treat as no target
        return LinkTarget::None;
    }

    // Check for explicit destination array with page index
    if let Some(dest_array) = &link.dest_array {
        // Extract page_index from dest_array
        if let Some(page_index) = resolve_page_from_dest_json(&dest_array) {
            return LinkTarget::InternalPage(page_index);
        }
    }

    // Fall back to named destination
    if let Some(dest) = &link.dest {
        return LinkTarget::InternalNamed(dest.clone());
    }

    LinkTarget::None
}

/// Resolve page index from a destination array JSON.
///
/// Returns the page index if resolvable, None otherwise.
fn resolve_page_from_dest_json(dest: &crate::schema::DestArrayJson) -> Option<usize> {
    // For now, just return the page_index from dest
    // The dest field contains the fit type information
    Some(dest.page_index)
}

/// Find spans whose bbox center falls within a link JSON's rect.
///
/// This is a variant of `find_spans_in_link` that works with `LinkJson`
/// (the JSON-serializable type) instead of `LinkAnnotation` (the internal type).
///
/// Returns the indices of spans that should be included in the link anchor text.
///
/// # Arguments
///
/// * `spans` - All spans on the page
/// * `link` - The link JSON
///
/// # Returns
///
/// A vector of span indices whose centers fall within the link rect.
pub fn find_spans_in_link_json(spans: &[SpanJson], link: &LinkJson) -> Vec<usize> {
    let mut matched = Vec::new();

    let link_rect = link.rect; // LinkJson has rect directly

    for (idx, span) in spans.iter().enumerate() {
        let (cx, cy) = bbox_center(&span.bbox);
        if point_in_rect(cx, cy, &link_rect) {
            matched.push(idx);
        }
    }

    // Sort by index to preserve document order
    matched.sort();
    matched
}

/// Emit all inline links for a page's spans from LinkJson.
///
/// This is a variant of `emit_page_links` that works with `LinkJson`
/// (the JSON-serializable type) instead of `LinkAnnotation` (the internal type).
///
/// Returns a vector of (span_indices, link_markdown) tuples representing all
/// inline links to be emitted on this page. Each span index appears at most
/// once across all links (first link wins).
///
/// # Arguments
///
/// * `spans` - All spans on the page
/// * `links` - All link JSON objects for the page
///
/// # Returns
///
/// A vector of (span_indices, markdown_string) tuples.
pub fn emit_page_links_from_json(
    spans: &[SpanJson],
    links: &[LinkJson],
) -> Vec<(Vec<usize>, String)> {
    let mut results = Vec::new();
    let mut used_spans = std::collections::HashSet::new();

    for link in links {
        let span_indices = find_spans_in_link_json(spans, link);
        if span_indices.is_empty() {
            continue; // Skip links with no anchor text
        }

        let target = resolve_link_target_from_json(link);
        if target == LinkTarget::None {
            continue; // Skip links with no valid target
        }

        let anchor_text = concatenate_anchor_text(spans, &span_indices);
        if anchor_text.is_empty() {
            continue; // Skip links with empty anchor text
        }

        let markdown = emit_inline_link(&anchor_text, &target);

        // Filter out already-used spans (first link wins)
        let available_indices: Vec<usize> = span_indices
            .into_iter()
            .filter(|idx| !used_spans.contains(idx))
            .collect();

        if !available_indices.is_empty() {
            for &idx in &available_indices {
                used_spans.insert(idx);
            }
            results.push((available_indices, markdown));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::AnnotationCommon;

    fn make_test_span(text: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> SpanJson {
        SpanJson {
            text: text.to_string(),
            bbox: [x0, y0, x1, y1],
            font: "Helvetica".to_string(),
            size: 12.0,
            color: Some("#000000".to_string()),
            rendering_mode: Some(0),
            confidence: Some(1.0),
            confidence_source: Some("vector".to_string()),
            lang: Some("en".to_string()),
            flags: vec![],
            receipt: None,
            column: Some(0),
        }
    }

    fn make_test_link(rect: [f32; 4], uri: Option<&str>, dest: Option<&str>) -> LinkAnnotation {
        LinkAnnotation {
            common: AnnotationCommon {
                subtype: "Link".to_string(),
                rect: Some(rect),
                contents: None,
                author: None,
                modified: None,
                color: None,
                opacity: None,
                flags: 0,
                name_id: None,
                subject: None,
                page_index: 0,
            },
            uri: uri.map(|s| s.to_string()),
            dest: dest.map(|s| s.to_string()),
            dest_array: None,
        }
    }

    fn make_test_link_with_dest_array(rect: [f32; 4], page_index: usize) -> LinkAnnotation {
        LinkAnnotation {
            common: AnnotationCommon {
                subtype: "Link".to_string(),
                rect: Some(rect),
                contents: None,
                author: None,
                modified: None,
                color: None,
                opacity: None,
                flags: 0,
                name_id: None,
                subject: None,
                page_index: 0,
            },
            uri: None,
            dest: None,
            dest_array: Some(DestArray {
                page_index,
                fit: FitType::Fit,
            }),
        }
    }

    #[test]
    fn test_bbox_center() {
        let bbox = [100.0, 200.0, 300.0, 400.0];
        let (cx, cy) = bbox_center(&bbox);
        assert_eq!(cx, 200.0);
        assert_eq!(cy, 300.0);
    }

    #[test]
    fn test_point_in_rect() {
        let rect = [100.0, 200.0, 300.0, 400.0];

        // Point inside
        assert!(point_in_rect(200.0, 300.0, &rect));
        assert!(point_in_rect(100.0, 200.0, &rect)); // Corner inclusive
        assert!(point_in_rect(300.0, 400.0, &rect)); // Corner inclusive

        // Point outside
        assert!(!point_in_rect(99.0, 300.0, &rect));
        assert!(!point_in_rect(301.0, 300.0, &rect));
        assert!(!point_in_rect(200.0, 199.0, &rect));
        assert!(!point_in_rect(200.0, 401.0, &rect));
    }

    #[test]
    fn test_resolve_link_target_external_http() {
        let link = make_test_link([0.0, 0.0, 100.0, 20.0], Some("https://example.com"), None);
        let target = resolve_link_target(&link);
        assert_eq!(
            target,
            LinkTarget::External("https://example.com".to_string())
        );
    }

    #[test]
    fn test_resolve_link_target_external_mailto() {
        let link = make_test_link(
            [0.0, 0.0, 100.0, 20.0],
            Some("mailto:test@example.com"),
            None,
        );
        let target = resolve_link_target(&link);
        assert_eq!(
            target,
            LinkTarget::External("mailto:test@example.com".to_string())
        );
    }

    #[test]
    fn test_resolve_link_target_javascript_rejected() {
        let link = make_test_link([0.0, 0.0, 100.0, 20.0], Some("javascript:alert(1)"), None);
        let target = resolve_link_target(&link);
        assert_eq!(target, LinkTarget::None);
    }

    #[test]
    fn test_resolve_link_target_internal_named() {
        let link = make_test_link([0.0, 0.0, 100.0, 20.0], None, Some("Chapter1"));
        let target = resolve_link_target(&link);
        assert_eq!(target, LinkTarget::InternalNamed("Chapter1".to_string()));
    }

    #[test]
    fn test_resolve_link_target_internal_page() {
        let link = make_test_link_with_dest_array([0.0, 0.0, 100.0, 20.0], 5);
        let target = resolve_link_target(&link);
        assert_eq!(target, LinkTarget::InternalPage(5));
    }

    #[test]
    fn test_resolve_link_target_none() {
        let link = make_test_link([0.0, 0.0, 100.0, 20.0], None, None);
        let target = resolve_link_target(&link);
        assert_eq!(target, LinkTarget::None);
    }

    #[test]
    fn test_escape_link_text() {
        assert_eq!(escape_link_text("hello"), "hello");
        assert_eq!(escape_link_text("hello [world]"), r"hello \[world\]");
        assert_eq!(escape_link_text(r"hello \[world\]"), r"hello \\[world\\]");
    }

    #[test]
    fn test_percent_encode_url() {
        assert_eq!(
            percent_encode_url("https://example.com"),
            "https://example.com"
        );
        assert_eq!(
            percent_encode_url("https://example.com/path(with)parens"),
            "https://example.com/path%28with%29parens"
        );
        assert_eq!(
            percent_encode_url("https://example.com/path with spaces"),
            "https://example.com/path%20with%20spaces"
        );
    }

    #[test]
    fn test_emit_inline_link_external() {
        let markdown = emit_inline_link(
            "Example Site",
            &LinkTarget::External("https://example.com".to_string()),
        );
        assert_eq!(markdown, "[Example Site](https://example.com)");
    }

    #[test]
    fn test_emit_inline_link_internal_page() {
        let markdown = emit_inline_link("See Chapter 1", &LinkTarget::InternalPage(0));
        assert_eq!(markdown, "[See Chapter 1](#page-1)");
    }

    #[test]
    fn test_emit_inline_link_internal_named() {
        let markdown = emit_inline_link(
            "Appendix",
            &LinkTarget::InternalNamed("AppendixA".to_string()),
        );
        assert_eq!(markdown, "[Appendix](#AppendixA)");
    }

    #[test]
    fn test_emit_inline_link_none() {
        let markdown = emit_inline_link("No Link", &LinkTarget::None);
        assert_eq!(markdown, "No Link");
    }

    #[test]
    fn test_emit_inline_link_with_brackets() {
        let markdown = emit_inline_link(
            "See [Chapter 1] for details",
            &LinkTarget::External("https://example.com".to_string()),
        );
        assert_eq!(
            markdown,
            r"[See \[Chapter 1\] for details](https://example.com)"
        );
    }

    #[test]
    fn test_find_spans_in_link_single_span() {
        let spans = vec![
            make_test_span("Hello", 100.0, 720.0, 150.0, 730.0),
            make_test_span("World", 160.0, 720.0, 210.0, 730.0),
        ];
        let link = make_test_link(
            [90.0, 710.0, 160.0, 740.0],
            Some("https://example.com"),
            None,
        );

        let matched = find_spans_in_link(&spans, &link);
        assert_eq!(matched, vec![0]); // Only first span's center is in the link
    }

    #[test]
    fn test_find_spans_in_link_multiple_spans() {
        let spans = vec![
            make_test_span("Click", 100.0, 720.0, 140.0, 730.0),
            make_test_span("here", 145.0, 720.0, 180.0, 730.0),
            make_test_span("now", 185.0, 720.0, 210.0, 730.0),
        ];
        let link = make_test_link(
            [90.0, 710.0, 200.0, 740.0],
            Some("https://example.com"),
            None,
        );

        let matched = find_spans_in_link(&spans, &link);
        assert_eq!(matched, vec![0, 1, 2]); // All three spans
    }

    #[test]
    fn test_find_spans_in_link_empty_rect() {
        let spans = vec![make_test_span("Hello", 100.0, 720.0, 150.0, 730.0)];
        let link = LinkAnnotation {
            common: AnnotationCommon {
                subtype: "Link".to_string(),
                rect: None, // No rect
                contents: None,
                author: None,
                modified: None,
                color: None,
                opacity: None,
                flags: 0,
                name_id: None,
                subject: None,
                page_index: 0,
            },
            uri: Some("https://example.com".to_string()),
            dest: None,
            dest_array: None,
        };

        let matched = find_spans_in_link(&spans, &link);
        assert!(matched.is_empty());
    }

    #[test]
    fn test_concatenate_anchor_text() {
        let spans = vec![
            make_test_span("Hello", 100.0, 720.0, 140.0, 730.0),
            make_test_span(" ", 140.0, 720.0, 145.0, 730.0),
            make_test_span("World", 145.0, 720.0, 190.0, 730.0),
        ];

        let text = concatenate_anchor_text(&spans, &[0, 1, 2]);
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_emit_page_links_single_link() {
        let spans = vec![
            make_test_span("Click", 100.0, 720.0, 140.0, 730.0),
            make_test_span("here", 145.0, 720.0, 180.0, 730.0),
        ];
        let links = vec![make_test_link(
            [90.0, 710.0, 190.0, 740.0],
            Some("https://example.com"),
            None,
        )];

        let results = emit_page_links(&spans, &links);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, vec![0, 1]);
        assert_eq!(results[0].1, "[Click here](https://example.com)");
    }

    #[test]
    fn test_emit_page_links_internal_destination() {
        let spans = vec![make_test_span("Chapter 1", 100.0, 720.0, 180.0, 730.0)];
        let links = vec![make_test_link_with_dest_array(
            [90.0, 710.0, 190.0, 740.0],
            0,
        )];

        let results = emit_page_links(&spans, &links);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "[Chapter 1](#page-1)");
    }

    #[test]
    fn test_emit_page_links_no_anchor_text() {
        let spans = vec![make_test_span("Text", 100.0, 720.0, 140.0, 730.0)];
        let links = vec![make_test_link(
            [200.0, 720.0, 300.0, 730.0],
            Some("https://example.com"),
            None,
        )];

        let results = emit_page_links(&spans, &links);
        assert!(results.is_empty()); // No spans in link rect
    }

    #[test]
    fn test_emit_page_links_no_valid_target() {
        let spans = vec![make_test_span("Text", 100.0, 720.0, 140.0, 730.0)];
        let links = vec![make_test_link(
            [90.0, 710.0, 150.0, 740.0],
            Some("javascript:alert(1)"),
            None,
        )];

        let results = emit_page_links(&spans, &links);
        assert!(results.is_empty()); // JavaScript links rejected
    }

    #[test]
    fn test_emit_page_links_first_link_wins_for_overlap() {
        let spans = vec![make_test_span("Overlap", 100.0, 720.0, 160.0, 730.0)];

        // Two overlapping links
        let links = vec![
            make_test_link([90.0, 710.0, 150.0, 740.0], Some("https://first.com"), None),
            make_test_link(
                [110.0, 710.0, 170.0, 740.0],
                Some("https://second.com"),
                None,
            ),
        ];

        let results = emit_page_links(&spans, &links);
        assert_eq!(results.len(), 1);
        // First link wins
        assert_eq!(results[0].1, "[Overlap](https://first.com)");
    }
}
