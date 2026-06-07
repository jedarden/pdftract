//! Debug test for content stream hashing.

use pdftract_core::document::parse_pdf_file;
use pdftract_core::fingerprint::{hash_content_streams, ContentStreamData};
use pdftract_core::parser::xref::XrefResolver;

#[test]
fn debug_content_edit_fixture_streams() {
    //! Debug: examine actual content stream hashes for content_edit fixtures.

    let v1_path = "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf";
    let v2_path = "tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf";

    let (_fp1, _catalog1, pages1, resolver1) = parse_pdf_file(std::path::Path::new(v1_path))
        .expect("Failed to parse v1");
    let (_fp2, _catalog2, pages2, resolver2) = parse_pdf_file(std::path::Path::new(v2_path))
        .expect("Failed to parse v2");

    println!("=== v1 ===");
    println!("v1 fingerprint: {}", _fp1);
    println!("v1 pages: {}", pages1.len());

    for (i, page) in pages1.iter().enumerate() {
        println!("  v1 page {} has {} content streams", i, page.contents.len());
        for (j, &obj_ref) in page.contents.iter().enumerate() {
            println!("    v1 page {} stream {}: {:?}", i, j, obj_ref);
        }
    }

    println!("\n=== v2 ===");
    println!("v2 fingerprint: {}", _fp2);
    println!("v2 pages: {}", pages2.len());

    for (i, page) in pages2.iter().enumerate() {
        println!("  v2 page {} has {} content streams", i, page.contents.len());
        for (j, &obj_ref) in page.contents.iter().enumerate() {
            println!("    v2 page {} stream {}: {:?}", i, j, obj_ref);
        }
    }

    // Now let's manually compute the content hashes
    println!("\n=== Manual content hash computation ===");

    let v1_streams: Vec<_> = pages1[0].contents
        .iter()
        .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
        .collect();

    let v2_streams: Vec<_> = pages2[0].contents
        .iter()
        .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
        .collect();

    // We need a source - let's open the files
    use pdftract_core::parser::stream::FileSource as ParserFileSource;
    use pdftract_core::parser::stream::PdfSource as ParserPdfSource;

    let source1 = ParserFileSource::open(std::path::Path::new(v1_path))
        .expect("Failed to open v1");
    let source2 = ParserFileSource::open(std::path::Path::new(v2_path))
        .expect("Failed to open v2");

    let v1_hash = hash_content_streams(&v1_streams, &resolver1, Some(&source1 as &dyn ParserPdfSource));
    let v2_hash = hash_content_streams(&v2_streams, &resolver2, Some(&source2 as &dyn ParserPdfSource));

    println!("v1 content hash: {:?}", v1_hash);
    println!("v2 content hash: {:?}", v2_hash);
    println!("Content hashes match: {}", v1_hash == v2_hash);

    // Also let's see the actual decoded content
    for (i, &obj_ref) in pages1[0].contents.iter().enumerate() {
        match resolver1.resolve(obj_ref) {
            Ok(pdftract_core::parser::object::PdfObject::Stream(stream)) => {
                use pdftract_core::parser::stream::{decode_stream, ExtractionOptions};
                let opts = ExtractionOptions::default();
                let mut decompress_counter = 0u64;
                let decoded = decode_stream(&*stream, &source1, &opts, &mut decompress_counter);
                println!("v1 stream {} decoded ({} bytes): {:?}", i, decoded.len(), String::from_utf8_lossy(&decoded));
            }
            Ok(other) => println!("v1 stream {} is not a stream: {:?}", i, other),
            Err(e) => println!("v1 stream {} failed to resolve: {:?}", i, e),
        }
    }

    for (i, &obj_ref) in pages2[0].contents.iter().enumerate() {
        match resolver2.resolve(obj_ref) {
            Ok(pdftract_core::parser::object::PdfObject::Stream(stream)) => {
                use pdftract_core::parser::stream::{decode_stream, ExtractionOptions};
                let opts = ExtractionOptions::default();
                let mut decompress_counter = 0u64;
                let decoded = decode_stream(&*stream, &source2, &opts, &mut decompress_counter);
                println!("v2 stream {} decoded ({} bytes): {:?}", i, decoded.len(), String::from_utf8_lossy(&decoded));
            }
            Ok(other) => println!("v2 stream {} is not a stream: {:?}", i, other),
            Err(e) => println!("v2 stream {} failed to resolve: {:?}", i, e),
        }
    }
}
