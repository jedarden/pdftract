//! Quick test to verify basic extraction works on known-good fixtures.

use pdftract_core::sdk;
use pdftract_core::options::ExtractionOptions;
use std::path::Path;

#[test]
fn test_extract_tagged_pdf() {
    let path = Path::new("/home/coding/pdftract/tests/fixtures/tagged-suspects-false.pdf");
    let options = ExtractionOptions::default();

    let result = sdk::extract(path, &options).unwrap();
    println!("Pages extracted: {}", result.pages.len());
    assert!(result.pages.len() > 0, "Should extract at least one page");
}

#[test]
fn test_extract_base_hello() {
    let path = Path::new("/home/coding/pdftract/tests/document_model/fixtures/base_hello.pdf");
    let options = ExtractionOptions::default();

    let result = sdk::extract(path, &options).unwrap();
    println!("Pages extracted: {}", result.pages.len());
    assert!(result.pages.len() > 0, "Should extract at least one page");
}

#[test]
fn test_extract_conformance_fixture() {
    let path = Path::new("/home/coding/pdftract/tests/sdk-conformance/fixtures/scientific_paper/01.pdf");
    let options = ExtractionOptions::default();

    let result = sdk::extract(path, &options).unwrap();
    println!("Pages extracted: {}", result.pages.len());
    assert!(result.pages.len() > 0, "Should extract at least one page");
}
