//! Diagnostic-contract tests for /Root resolution from the parsed trailer
//! dict (bead pdftract-5a1b5973).
//!
//! `document::resolve_root_ref` is the single helper behind the parse /
//! extract / remote / hash entry points. It must keep the two failure modes
//! distinct:
//!
//! - xref section without a trailer            → "No trailer in xref section"
//! - trailer present but without a /Root entry → "No /Root reference in trailer"
//!   (exact message; also when /Root holds a non-reference value)
//! - trailer present with a /Root reference    → resolved `ObjRef`
//!
//! The end-to-end tests craft minimal PDFs and pin the same contract through
//! `sdk::extract` / `sdk::extract_text`: a trailer that genuinely lacks
//! /Root must still fail with the exact "No /Root reference in trailer"
//! message, while a missing trailer is no longer conflated into it.

use pdftract_core::document::resolve_root_ref;
use pdftract_core::options::ExtractionOptions;
use pdftract_core::parser::object::{ObjRef, PdfDict, PdfObject};
use pdftract_core::parser::xref::XrefSection;
use pdftract_core::sdk;
use std::path::Path;

// --- helper-level contract ---

fn section_with_trailer(dict: PdfDict) -> XrefSection {
    let mut section = XrefSection::new();
    section.trailer = Some(dict);
    section
}

#[test]
fn helper_missing_trailer_reports_no_trailer() {
    let err = resolve_root_ref(&XrefSection::new()).unwrap_err();
    assert_eq!(err.root_cause().to_string(), "No trailer in xref section");
}

#[test]
fn helper_trailer_without_root_reports_exact_message() {
    let mut dict = PdfDict::new();
    dict.insert("Size".into(), PdfObject::Integer(2));
    let err = resolve_root_ref(&section_with_trailer(dict)).unwrap_err();
    assert_eq!(
        err.root_cause().to_string(),
        "No /Root reference in trailer"
    );
}

#[test]
fn helper_non_ref_root_reports_exact_message() {
    let mut dict = PdfDict::new();
    dict.insert("Root".into(), PdfObject::Null);
    let err = resolve_root_ref(&section_with_trailer(dict)).unwrap_err();
    assert_eq!(
        err.root_cause().to_string(),
        "No /Root reference in trailer"
    );
}

#[test]
fn helper_resolves_ref_root() {
    let mut dict = PdfDict::new();
    dict.insert("Root".into(), PdfObject::Ref(ObjRef::new(1, 0)));
    let root = resolve_root_ref(&section_with_trailer(dict)).unwrap();
    assert_eq!(root, ObjRef::new(1, 0));
}

// --- end-to-end contract through sdk::extract on crafted PDFs ---

/// Craft a minimal one-object PDF whose xref table is spec-shaped (20-byte
/// entries, correct startxref). `trailer_dict` is the raw dictionary body
/// placed under the `trailer` keyword; `None` omits the trailer entirely.
fn craft_pdf(trailer_dict: Option<&str>) -> tempfile::NamedTempFile {
    let mut body = String::from("%PDF-1.4\n");
    let obj_offset = body.len();
    body.push_str("1 0 obj\n<< /Type /Catalog >>\nendobj\n");
    let xref_offset = body.len();
    body.push_str("xref\n0 2\n0000000000 65535 f \n");
    body.push_str(&format!("{:010} 00000 n \n", obj_offset));
    if let Some(t) = trailer_dict {
        body.push_str(&format!("trailer\n<< {} >>\n", t));
    }
    body.push_str(&format!("startxref\n{}\n%%EOF\n", xref_offset));

    let file = tempfile::NamedTempFile::new().expect("create tempfile");
    std::fs::write(file.path(), body).expect("write crafted pdf");
    file
}

fn extract_root_err(path: &Path) -> String {
    match sdk::extract(path, &ExtractionOptions::default()) {
        Ok(_) => panic!("expected extract to fail for {}", path.display()),
        Err(e) => e.root_cause().to_string(),
    }
}

#[test]
fn extract_trailer_without_root_still_exact_message() {
    let pdf = craft_pdf(Some("Size 2"));
    assert_eq!(
        extract_root_err(pdf.path()),
        "No /Root reference in trailer"
    );
}

#[test]
fn extract_text_trailer_without_root_still_exact_message() {
    let pdf = craft_pdf(Some("Size 2"));
    match sdk::extract_text(pdf.path(), &ExtractionOptions::default()) {
        Ok(_) => panic!("expected extract_text to fail"),
        Err(e) => {
            assert_eq!(e.root_cause().to_string(), "No /Root reference in trailer");
        }
    }
}

#[test]
fn extract_missing_trailer_reports_no_trailer() {
    let pdf = craft_pdf(None);
    assert_eq!(extract_root_err(pdf.path()), "No trailer in xref section");
}
