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

#[test]
fn extraction_publishes_page_tree_diagnostics_from_valid_page_yields() {
    let objects = [
        "<</Type/Catalog/Pages 2 0 R>>",
        "<</Type/Pages/Kids[3 0 R]/Count 1>>",
        "<</Type/Page/Parent 2 0 R/Contents 4 0 R>>",
        "<</Length 3>>stream\nq Q\nendstream",
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj{object}endobj\n", index + 1).as_bytes());
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer<</Size 5/Root 1 0 R>>\nstartxref\n{xref_offset}\n%%EOF\n")
            .as_bytes(),
    );

    let temp_dir = tempfile::tempdir().expect("temporary directory");
    let fixture = temp_dir.path().join("missing-mediabox.pdf");
    std::fs::write(&fixture, pdf).expect("write PDF fixture");
    let result = extract_pdf(&fixture, &ExtractionOptions::default())
        .expect("missing MediaBox should recover with the default page size");

    let diagnostics: Vec<_> = result
        .metadata
        .diagnostics_detailed
        .iter()
        .filter(|diagnostic| diagnostic.code == "STRUCT_MISSING_KEY")
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics.iter().map(|d| d.page_index).collect::<Vec<_>>(),
        vec![Some(0)]
    );
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.location.is_some()));
    assert_eq!(
        result.metadata.diagnostics.len(),
        result.metadata.diagnostics_detailed.len()
    );
}
