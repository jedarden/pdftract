// Debug test to understand why content_edit fixtures produce same fingerprint
use pdftract_core::document::compute_pdf_fingerprint;
use std::path::PathBuf;

fn main() {
    let fixtures_dir = PathBuf::from("tests/fingerprint/fixtures");

    // Test content_edit_one_glyph
    let dir = fixtures_dir.join("content_edit_one_glyph");
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    println!("=== Testing content_edit_one_glyph ===");

    let fp1 = compute_pdf_fingerprint(&v1).expect("Failed to compute fingerprint for v1");
    let fp2 = compute_pdf_fingerprint(&v2).expect("Failed to compute fingerprint for v2");

    println!("v1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    println!("Are they equal? {}", fp1 == fp2);

    // Test content_edit_one_paragraph
    let dir2 = fixtures_dir.join("content_edit_one_paragraph");
    let v1p = dir2.join("v1.pdf");
    let v2p = dir2.join("v2.pdf");

    println!("\n=== Testing content_edit_one_paragraph ===");

    let fp1p = compute_pdf_fingerprint(&v1p).expect("Failed to compute fingerprint for v1p");
    let fp2p = compute_pdf_fingerprint(&v2p).expect("Failed to compute fingerprint for v2p");

    println!("v1 fingerprint: {}", fp1p);
    println!("v2 fingerprint: {}", fp2p);
    println!("Are they equal? {}", fp1p == fp2p);
}
