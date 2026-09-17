//! Regression coverage for the source-backed classic-xref/page-tree path.

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::object::{ObjRef, PdfObject};
use pdftract_core::sdk;
use pdftract_core::ExtractionOptions;
use std::path::Path;

const FIXTURE: &str = "tests/fixtures/test-minimal.pdf";

#[test]
fn w3c_dummy_exercises_trailer_root_catalog_and_page_tree() {
    let path = Path::new(FIXTURE);
    let bytes = std::fs::read(path).expect("smoke fixture should be readable");
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(
        bytes.windows(b"xref".len()).any(|window| window == b"xref"),
        "fixture must use a classic xref table"
    );

    let (_fingerprint, catalog, pages, resolver, trailer) =
        parse_pdf_file(path).expect("source-backed parsing should succeed");

    let root_ref = trailer
        .get("Root")
        .and_then(PdfObject::as_ref)
        .expect("classic trailer should contain an indirect /Root");
    assert_eq!(root_ref, ObjRef::new(14, 0));

    let catalog_object = resolver.resolve(root_ref).expect("catalog should resolve");
    let catalog_dict = catalog_object
        .as_dict()
        .expect("/Root should resolve to a dictionary");
    assert_eq!(
        catalog_dict.get("Type").and_then(PdfObject::as_name),
        Some("Catalog")
    );
    assert_eq!(
        catalog_dict.get("Pages").and_then(PdfObject::as_ref),
        Some(catalog.pages_ref)
    );
    assert_eq!(catalog.pages_ref, ObjRef::new(4, 0));

    let pages_object = resolver
        .resolve(catalog.pages_ref)
        .expect("page tree root should resolve");
    let pages_dict = pages_object
        .as_dict()
        .expect("/Pages should resolve to a dictionary");
    assert_eq!(
        pages_dict.get("Type").and_then(PdfObject::as_name),
        Some("Pages")
    );
    assert_eq!(
        pages_dict.get("Count").and_then(PdfObject::as_int),
        Some(1)
    );
    assert_eq!(pages.len(), 1);

    let page_object = resolver
        .resolve(ObjRef::new(1, 0))
        .expect("leaf page should resolve");
    assert_eq!(
        page_object
            .as_dict()
            .and_then(|dict| dict.get("Type"))
            .and_then(PdfObject::as_name),
        Some("Page")
    );
}

#[test]
fn core_sdk_extracts_the_fixture_text_and_page_count() {
    let path = Path::new(FIXTURE);
    let options = ExtractionOptions::default();

    let document = pdftract_core::Document::open(path).expect("core document should open");
    assert_eq!(document.page_count().expect("page count should resolve"), 1);

    let result = sdk::extract(path, &options).expect("SDK structured extraction should succeed");
    assert_eq!(result.pages.len(), 1);
    assert_eq!(result.metadata.page_count, 1);

    let text = sdk::extract_text(path, &options).expect("SDK text extraction should succeed");
    assert_eq!(text.trim(), "Dummy PDF file");
}
