//! Debug test to see actual content stream bytes for content_edit fixtures.

use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let fixtures = [
        "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v2.pdf",
    ];

    for path in fixtures {
        println!("\n=== {} ===", path);
        match parse_pdf_file(Path::new(path)) {
            Ok((fingerprint, catalog, pages, _resolver)) => {
                println!("Fingerprint: {}", fingerprint);
                println!("Page count: {}", pages.len());
                for (i, page) in pages.iter().enumerate() {
                    println!("  Page {} content streams: {} streams", i, page.content_streams.len());
                    for (j, stream) in page.content_streams.iter().enumerate() {
                        match stream {
                            pdftract_core::fingerprint::ContentStreamData::Indirect(ref_) => {
                                println!("    Stream {}: Indirect {:?}", j, ref_);
                            }
                            pdftract_core::fingerprint::ContentStreamData::Direct(bytes) => {
                                println!("    Stream {}: Direct, {} bytes", j, bytes.len());
                                println!("      Bytes: {:?}", String::from_utf8_lossy(bytes));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}
