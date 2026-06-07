//! Debug fingerprint content streams.
//!
//! This test helps debug why content_edit fixtures have identical fingerprints.

use pdftract_core::document::parse_pdf_file;
use std::path::PathBuf;

fn main() {
    let fixtures = vec![
        "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v2.pdf",
    ];

    for fixture in fixtures {
        let path = PathBuf::from(fixture);
        println!("\n=== {} ===", fixture);

        match parse_pdf_file(&path) {
            Ok((fingerprint, _catalog, pages, _resolver)) => {
                println!("Fingerprint: {}", fingerprint);
                println!("Pages: {}", pages.len());

                for (i, page) in pages.iter().enumerate() {
                    println!("  Page {}:", i);
                    println!("    MediaBox: {:?}", page.media_box);
                    println!("    Contents: {} stream refs", page.contents.len());
                    for (j, content_ref) in page.contents.iter().enumerate() {
                        println!("      Content {}: {:?}", j, content_ref);
                    }
                    println!("    Resources keys: {:?}", page.resources.keys().collect::<Vec<_>>());
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
