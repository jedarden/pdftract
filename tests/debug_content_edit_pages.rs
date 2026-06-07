#[test]
fn debug_content_edit_pages() {
    use pdftract_core::document::parse_pdf_file;
    
    let v1_path = "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf";
    let v2_path = "tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf";

    println!("Checking v1: {}", v1_path);
    let (_fp1, _cat1, pages1, _resolver1) = parse_pdf_file(std::path::Path::new(v1_path)).unwrap();
    println!("v1 pages: {}", pages1.len());

    println!("Checking v2: {}", v2_path);
    let (_fp2, _cat2, pages2, _resolver2) = parse_pdf_file(std::path::Path::new(v2_path)).unwrap();
    println!("v2 pages: {}", pages2.len());
    
    panic!("Debug info printed");
}
