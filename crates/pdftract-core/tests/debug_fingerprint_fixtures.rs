//! Debug test to understand why fixture fingerprints are identical

use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");
    
    println!("=== Parsing v1 ===");
    let (fp1, cat1, pages1, _resolver1) = parse_pdf_file(v1_path).unwrap();
    println!("Fingerprint: {}", fp1);
    println!("Pages: {}", pages1.len());
    if let Some(page) = pages1.first() {
        println!("First page contents: {} objects", page.contents.len());
        println!("MediaBox: {:?}", page.media_box);
    }
    
    println!("\n=== Parsing v2 ===");
    let (fp2, cat2, pages2, _resolver2) = parse_pdf_file(v2_path).unwrap();
    println!("Fingerprint: {}", fp2);
    println!("Pages: {}", pages2.len());
    if let Some(page) = pages2.first() {
        println!("First page contents: {} objects", page.contents.len());
        println!("MediaBox: {:?}", page.media_box);
    }
    
    println!("\n=== Comparisons ===");
    println!("Fingerprints equal: {}", fp1 == fp2);
    println!("Page counts equal: {}", pages1.len() == pages2.len());
    
    if let (Some(p1), Some(p2)) = (pages1.first(), pages2.first()) {
        println!("MediaBox equal: {}", p1.media_box == p2.media_box);
        println!("Contents count equal: {}", p1.contents.len() == p2.contents.len());
        
        // Check if content object refs are different
        if p1.contents.len() > 0 && p2.contents.len() > 0 {
            println!("v1 content ref: {:?}", p1.contents[0]);
            println!("v2 content ref: {:?}", p2.contents[0]);
            println!("Content refs equal: {}", p1.contents[0] == p2.contents[0]);
        }
    }
}
