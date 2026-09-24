//! Focused checks for page, classification, and extraction diagnostic context.

use std::path::Path;

use pdftract_core::classify::{apply_broken_vector_escalation_with_diagnostics, PageClass};
use pdftract_core::content_stream::{process_with_mode_and_diagnostics, ProcessingMode};
use pdftract_core::diagnostics::{DiagCode, Diagnostic, Severity};
use pdftract_core::extract_pdf;
use pdftract_core::options::ExtractionOptions;
use pdftract_core::pages::parse_pages;
use pdftract_core::parser::resources::ResourceDict;
use pdftract_core::schema::DiagnosticJson;

fn json_for(diagnostic: &Diagnostic) -> serde_json::Value {
    serde_json::to_value(DiagnosticJson::from(diagnostic)).expect("diagnostic JSON serializes")
}

#[test]
fn page_range_diagnostic_retains_structured_page_context() {
    let mut diagnostics = Vec::new();
    let selected = parse_pages("12", 10, &mut diagnostics).expect("range is recoverable");

    assert!(selected.is_empty());
    let diagnostic = diagnostics.first().expect("out-of-range page is diagnosed");
    assert_eq!(diagnostic.code, DiagCode::PageOutOfRange);
    assert_eq!(diagnostic.page_index, Some(11));
    assert_eq!(diagnostic.severity(), Severity::Error);
    assert_eq!(diagnostic.hint(), DiagCode::PageOutOfRange.policy().hint);

    let value = json_for(diagnostic);
    assert_eq!(value["code"], "PAGE_OUT_OF_RANGE");
    assert_eq!(value["page_index"], 11);
    assert_eq!(value["severity"], "error");
    assert_eq!(value["hint"], diagnostic.hint().unwrap());
}

#[cfg(not(feature = "ocr"))]
#[test]
fn classification_diagnostic_retains_page_context_and_policy() {
    let mut diagnostics = Vec::new();
    let result = apply_broken_vector_escalation_with_diagnostics(
        PageClass::Vector,
        0.25,
        4,
        &mut diagnostics,
    );

    assert_eq!(result, PageClass::BrokenVector);
    let diagnostic = diagnostics
        .first()
        .expect("broken-vector escalation should be diagnosed");
    assert_eq!(diagnostic.code, DiagCode::OcrBrokenVectorUnavailable);
    assert_eq!(diagnostic.page_index, Some(4));
    assert_eq!(diagnostic.severity(), Severity::Warning);
    assert_eq!(
        diagnostic.hint(),
        DiagCode::OcrBrokenVectorUnavailable.policy().hint
    );
    assert!(diagnostic.message.contains("Page 4"));
}

#[test]
fn content_recovery_retains_glyphs_alongside_diagnostics() {
    let result = process_with_mode_and_diagnostics(
        b"BT (Hello) Tj BT (World) Tj ET ET",
        &ResourceDict::new(),
        ProcessingMode::PositionHint,
        None,
        None,
        None,
    );

    assert_eq!(result.glyphs.len(), 2);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagCode::BtNested));
}

#[test]
fn extraction_mirrors_page_diagnostics_to_legacy_and_structured_surfaces() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-minimal.pdf");
    let mut options = ExtractionOptions::default();
    options.pages = Some("2".to_string());

    let result = extract_pdf(&fixture, &options).expect("out-of-range page is recoverable");
    let diagnostic = result
        .metadata
        .diagnostics_detailed
        .iter()
        .find(|diagnostic| diagnostic.code == "PAGE_OUT_OF_RANGE")
        .expect("extraction should expose the page-range diagnostic");

    assert_eq!(diagnostic.page_index, Some(1));
    assert_eq!(diagnostic.severity, "error");
    assert!(diagnostic.hint.is_some());
    assert_eq!(
        result.metadata.diagnostics,
        vec![diagnostic.message.clone()]
    );
}
