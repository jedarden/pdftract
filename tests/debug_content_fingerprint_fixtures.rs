//! Debug content fingerprint fixtures
use pdftract_core::document::parse_pdf_file;

fn main() {
    env_logger::init();

    let test_cases = vec![
        ("content_edit_one_glyph", "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf", "tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf"),
        ("content_edit_one_paragraph", "tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf", "tests/fingerprint/fixtures/content_edit_one_paragraph/v2.pdf"),
    ];

    for (name, v1_path, v2_path) in test_cases {
        println!("\n=== {} ===", name);

        let (fp1, catalog1, pages1, _) = parse_pdf_file(std::path::Path::new(v1_path))
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", v1_path, e));

        let (fp2, catalog2, pages2, _) = parse_pdf_file(std::path::Path::new(v2_path))
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", v2_path, e));

        println!("v1 fingerprint: {}", fp1);
        println!("v2 fingerprint: {}", fp2);
        println!("Fingerprints match: {}", fp1 == fp2);

        println!("v1 page count: {}", pages1.len());
        println!("v2 page count: {}", pages2.len());

        for (i, (p1, p2)) in pages1.iter().zip(pages2.iter()).enumerate() {
            println!("\nPage {}:", i);
            println!("  v1 contents refs: {:?}", p1.contents);
            println!("  v2 contents refs: {:?}", p2.contents);
            println!("  v1 mediabox: {:?}", p1.media_box);
            println!("  v2 mediabox: {:?}", p2.media_box);
            println!("  v1 rotate: {}", p1.rotate);
            println!("  v2 rotate: {}", p2.rotate);

            // Check if content refs are the same
            if p1.contents == p2.contents {
                println!("  WARNING: Contents refs are IDENTICAL");
            } else {
                println!("  Contents refs are DIFFERENT");
            }
        }
    }
}
