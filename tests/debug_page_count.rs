//! Debug script to understand page count issues

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::xref::XrefResolver;
use std::path::Path;

fn main() {
    let fixtures = [
        ("tests/document_model/fixtures/encrypted_rc4_test.pdf", "encrypted_rc4_test"),
        ("tests/document_model/fixtures/ocg_default_off.pdf", "ocg_default_off"),
        ("tests/document_model/fixtures/missing_mediabox.pdf", "missing_mediabox"),
    ];

    for (fixture_path, fixture_name) in fixtures {
        println!("\n=== Testing: {} ===", fixture_path);
        let path = Path::new(fixture_path);

        match parse_pdf_file(path) {
            Ok((_fingerprint, catalog, pages, resolver)) => {
                println!("Page count: {}", pages.len());
                println!("Catalog pages_ref: {:?}", catalog.pages_ref);
                println!("Catalog diagnostics: {:?}", catalog.diagnostics);

                // Check if the pages_ref resolves correctly
                if let Some(pages_ref) = catalog.pages_ref {
                    match resolver.resolve(pages_ref) {
                        Ok(pages_obj) => {
                            println!("Resolved pages object: {:?}", pages_obj);
                            if let Some(dict) = pages_obj.as_dict() {
                                println!("Pages dict keys: {:?}", dict.keys().collect::<Vec<_>>());
                                if let Some(count) = dict.get("Count") {
                                    println!("Count from /Pages: {:?}", count);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Failed to resolve pages_ref: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("FAILED: {}", e);
            }
        }
    }
}
