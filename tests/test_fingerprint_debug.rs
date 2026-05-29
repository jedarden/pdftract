use pdftract_core::document::compute_pdf_fingerprint;

#[test]
fn test_debug_fingerprints() {
    let v1_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");
    
    let fp1 = compute_pdf_fingerprint(&v1_path).unwrap();
    let fp2 = compute_pdf_fingerprint(&v2_path).unwrap();
    
    println!("v1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    println!("Equal: {}", fp1 == fp2);
    
    // This should fail
    assert_ne!(fp1, fp2, "Content edits should produce different fingerprints");
}
