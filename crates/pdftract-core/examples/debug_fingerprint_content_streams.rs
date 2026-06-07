// Debug script to check content stream hashing
use pdftract_core::document::parse_pdf_file;

fn main() {
    let v1_path = std::path::Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = std::path::Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("=== V1 ===");
    let (fp1, _cat1, pages1, _res1) = parse_pdf_file(v1_path).unwrap();
    println!("Fingerprint: {}", fp1);
    println!("Pages: {}", pages1.len());
    for (i, page) in pages1.iter().enumerate() {
        println!("Page {} content streams: {:?}", i, page.contents);
    }

    println!("\n=== V2 ===");
    let (fp2, _cat2, pages2, _res2) = parse_pdf_file(v2_path).unwrap();
    println!("Fingerprint: {}", fp2);
    println!("Pages: {}", pages2.len());
    for (i, page) in pages2.iter().enumerate() {
        println!("Page {} content streams: {:?}", i, page.contents);
    }

    println!("\n=== Fingerprints match: {} ===", fp1 == fp2);
}
