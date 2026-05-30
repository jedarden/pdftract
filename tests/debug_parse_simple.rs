//! Debug script to understand PDF parsing failures

use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let fixtures = [
        "tests/document_model/fixtures/encrypted_rc4_test.pdf",
        "tests/document_model/fixtures/ocg_default_off.pdf",
        "tests/document_model/fixtures/tagged_3_level_outline.pdf",
    ];

    for fixture_path in fixtures {
        println!("\n=== Testing: {} ===", fixture_path);
        let path = Path::new(fixture_path);

        match parse_pdf_file(path) {
            Ok((fingerprint, catalog, pages, resolver)) => {
                println!("SUCCESS!");
                println!("  Fingerprint: {:?}", fingerprint);
                println!("  Page count: {}", pages.len());
                println!("  Diagnostics: {} diagnostics", catalog.diagnostics.len());
            }
            Err(e) => {
                println!("FAILED: {}", e);
            }
        }
    }
}
