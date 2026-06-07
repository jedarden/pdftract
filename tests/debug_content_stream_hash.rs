// Debug test to understand why content streams produce same hash
use pdftract_core::parser::lexer::Lexer;
use pdftract_core::parser::stream::{decode_stream, ExtractionOptions};
use pdftract_core::parser::xref::XrefResolver;
use pdftract_core::parser::object::PdfObject;
use pdftract_core::document::parse_pdf_file;
use pdftract_core::fingerprint::normalize_content_bytes;
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("=== Debugging content stream hashing ===");

    // Parse both PDFs
    let (_fp1, _cat1, pages1, resolver1) = parse_pdf_file(v1_path).expect("Failed to parse v1");
    let (_fp2, _cat2, pages2, resolver2) = parse_pdf_file(v2_path).expect("Failed to parse v2");

    println!("v1 pages: {}", pages1.len());
    println!("v2 pages: {}", pages2.len());

    // Get page 0 from each
    let page1 = &pages1[0];
    let page2 = &pages2[0];

    println!("\nv1 page contents refs: {:?}", page1.contents);
    println!("v2 page contents refs: {:?}", page2.contents);

    // Resolve and decode the content streams
    if let Some(&content_ref) = page1.contents.first() {
        match resolver1.resolve(content_ref) {
            Ok(PdfObject::Stream(stream)) => {
                println!("\n=== v1 content stream ===");
                println!("Stream dict: {:?}", stream.dict);

                // Decode the stream
                let source = pdftract_core::parser::stream::ParserFileSource::open(v1_path).unwrap();
                let opts = ExtractionOptions::default();
                let mut decompress_counter = 0u64;
                let decoded = decode_stream(&*stream, &source, &opts, &mut decompress_counter);
                println!("Decoded bytes: {:?}", decoded);
                println!("Decoded hex: {}", hex::encode(&decoded));

                // Normalize
                let normalized = normalize_content_bytes(&decoded);
                println!("Normalized bytes: {:?}", normalized);
                println!("Normalized hex: {}", hex::encode(&normalized));
            }
            Ok(other) => println!("v1 content is not a stream: {:?}", other),
            Err(e) => println!("v1 failed to resolve: {:?}", e),
        }
    }

    if let Some(&content_ref) = page2.contents.first() {
        match resolver2.resolve(content_ref) {
            Ok(PdfObject::Stream(stream)) => {
                println!("\n=== v2 content stream ===");
                println!("Stream dict: {:?}", stream.dict);

                // Decode the stream
                let source = pdftract_core::parser::stream::ParserFileSource::open(v2_path).unwrap();
                let opts = ExtractionOptions::default();
                let mut decompress_counter = 0u64;
                let decoded = decode_stream(&*stream, &source, &opts, &mut decompress_counter);
                println!("Decoded bytes: {:?}", decoded);
                println!("Decoded hex: {}", hex::encode(&decoded));

                // Normalize
                let normalized = normalize_content_bytes(&decoded);
                println!("Normalized bytes: {:?}", normalized);
                println!("Normalized hex: {}", hex::encode(&normalized));
            }
            Ok(other) => println!("v2 content is not a stream: {:?}", other),
            Err(e) => println!("v2 failed to resolve: {:?}", e),
        }
    }

    println!("\n=== Testing normalize_content_bytes directly ===");
    let test1 = b"BT /F1 12 Tf 50 700 Td (Hello World) Tj ET";
    let test2 = b"BT /F1 12 Tf 50 700 Td (Hello Worl) Tj ET";
    println!("Input 1: {:?}", test1);
    println!("Input 2: {:?}", test2);
    println!("Normalized 1: {:?}", normalize_content_bytes(test1));
    println!("Normalized 2: {:?}", normalize_content_bytes(test2));
    println!("Are they equal? {}", normalize_content_bytes(test1) == normalize_content_bytes(test2));
}
