//! Simple fingerprint test - single fixture to debug the hang

use pdftract_core::document::compute_pdf_fingerprint;
use std::path::Path;

#[test]
fn test_single_fixture_byte_identical() {
    let v1 = Path::new("tests/fingerprint/fixtures/byte_identical/v1.pdf");
    let v2 = Path::new("tests/fingerprint/fixtures/byte_identical/v2.pdf");

    println!("Testing byte_identical fixture...");
    let start = std::time::Instant::now();

    let fp1 = compute_pdf_fingerprint(v1).unwrap();
    println!("v1 fingerprint: {} (took {:?})", fp1, start.elapsed());

    let fp2 = compute_pdf_fingerprint(v2).unwrap();
    println!("v2 fingerprint: {} (took {:?})", fp2, start.elapsed());

    assert_eq!(fp1, fp2, "Byte-identical files must produce identical fingerprints");
}

#[test]
fn test_single_fixture_content_edit_one_glyph() {
    let v1 = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2 = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("Testing content_edit_one_glyph fixture...");
    let start = std::time::Instant::now();

    let fp1 = compute_pdf_fingerprint(v1).unwrap();
    println!("v1 fingerprint: {} (took {:?})", fp1, start.elapsed());

    let fp2 = compute_pdf_fingerprint(v2).unwrap();
    println!("v2 fingerprint: {} (took {:?})", fp2, start.elapsed());

    assert_ne!(fp1, fp2, "Single glyph removal must change fingerprint");
}
