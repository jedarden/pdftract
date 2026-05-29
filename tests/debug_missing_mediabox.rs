use pdftract_core::document::parse_pdf_file;

#[test]
fn debug_missing_mediabox() {
    let result = parse_pdf_file(std::path::Path::new("tests/document_model/fixtures/missing_mediabox.pdf"));
    println!("Result: {:?}", result);
}
