use pdftract_core::document::parse_pdf_file;

fn main() {
    let v1_path = std::path::Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = std::path::Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("=== Parsing v1.pdf ===");
    let (fp1, _cat1, pages1, _resolver1) = parse_pdf_file(v1_path).unwrap();
    println!("v1 fingerprint: {}", fp1);
    println!("v1 pages: {}", pages1.len());
    if let Some(p) = pages1.first() {
        println!("v1 page 0 contents: {:?} ({} streams)", p.contents, p.contents.len());
        println!("v1 page 0 media_box: {:?}", p.media_box);
    }

    println!("\n=== Parsing v2.pdf ===");
    let (fp2, _cat2, pages2, _resolver2) = parse_pdf_file(v2_path).unwrap();
    println!("v2 fingerprint: {}", fp2);
    println!("v2 pages: {}", pages2.len());
    if let Some(p) = pages2.first() {
        println!("v2 page 0 contents: {:?} ({} streams)", p.contents, p.contents.len());
        println!("v2 page 0 media_box: {:?}", p.media_box);
    }

    println!("\n=== Comparing content refs ===");
    println!("v1 content ref: {:?}", pages1[0].contents.get(0));
    println!("v2 content ref: {:?}", pages2[0].contents.get(0));
    println!("Content refs equal: {}", pages1[0].contents == pages2[0].contents);

    println!("\n=== Re-parsing to verify ===");
    let (fp1_re, _, _, _) = parse_pdf_file(v1_path).unwrap();
    let (fp2_re, _, _, _) = parse_pdf_file(v2_path).unwrap();
    println!("v1 fingerprint (re-parsed): {}", fp1_re);
    println!("v2 fingerprint (re-parsed): {}", fp2_re);
    println!("Fingerprints equal: {}", fp1 == fp2);
}
