use std::path::Path;
use pdftract_core::document::parse_pdf_file;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");
    
    let (v1_fp, v1_cat, v1_pages, _) = parse_pdf_file(v1_path).unwrap();
    let (v2_fp, v2_cat, v2_pages, _) = parse_pdf_file(v2_path).unwrap();
    
    println!("=== v1 ===");
    println!("Fingerprint: {}", v1_fp);
    println!("Pages: {}", v1_pages.len());
    for (i, page) in v1_pages.iter().enumerate() {
        println!("  Page {}: {} content streams, MediaBox {:?}", i, page.contents.len(), page.media_box);
    }
    
    println!();
    println!("=== v2 ===");
    println!("Fingerprint: {}", v2_fp);
    println!("Pages: {}", v2_pages.len());
    for (i, page) in v2_pages.iter().enumerate() {
        println!("  Page {}: {} content streams, MediaBox {:?}", i, page.contents.len(), page.media_box);
    }
    
    println!();
    println!("Fingerprints match: {}", v1_fp == v2_fp);
}
