//! Debug test to see what content streams are extracted for the content_edit fixtures

use pdftract_core::document::parse_pdf_file;
use std::path::PathBuf;

fn main() {
    let fixtures = vec![
        ("content_edit_one_glyph/v1.pdf", "Hello World"),
        ("content_edit_one_glyph/v2.pdf", "Hello Worl"),
    ];

    for (fixture, expected_text) in fixtures {
        let path = PathBuf::from("tests/fingerprint/fixtures").join(fixture);
        println!("=== {} ===", fixture);

        match parse_pdf_file(&path) {
            Ok((fingerprint, catalog, pages, resolver)) => {
                println!("Fingerprint: {}", fingerprint);
                println!("Page count: {}", pages.len());

                for (i, page) in pages.iter().enumerate() {
                    println!("Page {}:", i);
                    println!("  Contents refs: {:?}", page.contents);
                    println!("  MediaBox: {:?}", page.media_box);

                    // Try to resolve and decode content streams
                    use pdftract_core::parser::stream::{FileSource, decode_stream, ExtractionOptions};
                    use pdftract_core::fingerprint::ContentStreamData;

                    let source = FileSource::open(&path).unwrap();
                    let mut decompress_counter = 0u64;
                    let opts = ExtractionOptions::default();

                    for &obj_ref in &page.contents {
                        match resolver.resolve(obj_ref) {
                            Ok(pdf_obj) => {
                                println!("    Resolved obj: {:?}", pdf_obj);
                                if let pdftract_core::parser::object::PdfObject::Stream(stream) = pdf_obj {
                                    let decoded = decode_stream(&stream, &source, &opts, &mut decompress_counter);
                                    println!("    Decoded stream ({} bytes):", decoded.len());
                                    println!("      {}", String::from_utf8_lossy(&decoded));
                                }
                            }
                            Err(e) => println!("    Failed to resolve: {:?}", e),
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to parse: {:?}", e);
            }
        }
        println!();
    }
}
