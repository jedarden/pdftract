//! Regression coverage for the MCP plain-text encoding path.

use pdftract_core::{extract_pdf, ExtractionOptions};
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/encoding")
        .join(name)
}

#[test]
fn no_tounicode_encoding_fallback_does_not_leak_control_bytes() {
    let result = extract_pdf(
        &fixture_path("unmapped-comprehensive.pdf"),
        &ExtractionOptions::default(),
    )
    .expect("the encoding regression fixture should extract");

    let text: String = result.pages[0]
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect();

    assert_eq!(text.chars().filter(|&ch| ch == '\u{FFFD}').count(), 7);
    assert_eq!(
        text.chars()
            .filter(|&ch| ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t')
            .count(),
        0
    );
    assert!(text.contains('A'));
    assert!(text.contains('B'));
}
