//! Regression tests for structural `startxref` discovery.
//!
//! The marker is a keyword token, not an arbitrary byte substring. These
//! malformed fixtures append text containing `startxref` inside a name and a
//! generic keyword after the real marker. The old tail scanners selected the
//! embedded text and failed before the xref trailer could be parsed.

use std::path::{Path, PathBuf};

use pdftract_core::extract::extract_pdf;
use pdftract_core::options::ExtractionOptions;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/fixtures/malformed/{name}"))
}

#[test]
fn extracts_when_startxref_is_followed_by_name_embedded_text() {
    let path = fixture("startxref_inside_name.pdf");
    let result = extract_pdf(&path, &ExtractionOptions::default())
        .unwrap_or_else(|error| panic!("{}: {error:#}", path.display()));
    assert_eq!(result.pages.len(), 1);
}

#[test]
fn extracts_when_startxref_is_followed_by_keyword_embedded_text() {
    let path = fixture("startxref_inside_keyword.pdf");
    let result = extract_pdf(&path, &ExtractionOptions::default())
        .unwrap_or_else(|error| panic!("{}: {error:#}", path.display()));
    assert_eq!(result.pages.len(), 1);
}
