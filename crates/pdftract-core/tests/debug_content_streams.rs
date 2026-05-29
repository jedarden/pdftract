//! Debug test to print normalized content streams for fixture PDFs.
//!
//! This helps diagnose why content_edit_one_glyph and content_edit_one_paragraph
//! fixtures produce identical fingerprints despite having different content.

use pdftract_core::document::PdfExtractor;
use std::path::Path;

fn print_normalized_content(path: &Path) {
    println!("\n=== {} ===", path.display());

    match PdfExtractor::open(path) {
        Ok(mut extractor) => {
            // Get the document and fingerprint
            let fingerprint = extractor.fingerprint();
            println!("Fingerprint: {}", fingerprint);

            // Try to get the first page
            if let Ok(pages) = extractor.materialize_pages() {
                if let Some(page) = pages.first() {
                    println!("Page 0 resources: {:?}", page.resources);

                    // Get content streams
                    for (i, stream_ref) in page.contents.iter().enumerate() {
                        println!("Content stream {}: ref={:?}", i, stream_ref);
                    }
                }
            }
        }
        Err(e) => {
            println!("Failed to open: {:?}", e);
        }
    }
}

fn main() {
    let fixtures = [
        "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v2.pdf",
    ];

    for fixture in fixtures {
        print_normalized_content(Path::new(fixture));
    }
}
