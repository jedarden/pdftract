//! Debug test to understand why content_edit fixtures produce same fingerprint

use pdftract_core::document::parse_pdf_file;

#[test]
fn debug_content_edit_fingerprints() {
    let fixtures_dir = std::path::PathBuf::from("tests/fingerprint/fixtures");

    // Test content_edit_one_glyph
    let dir = fixtures_dir.join("content_edit_one_glyph");
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    println!("=== Testing content_edit_one_glyph ===");

    let (fp1, catalog1, pages1, resolver1) = parse_pdf_file(&v1).expect("Failed to parse v1");
    let (fp2, catalog2, pages2, resolver2) = parse_pdf_file(&v2).expect("Failed to parse v2");

    println!("v1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    println!("Are they equal? {}", fp1 == fp2);

    println!("\nv1 pages: {}", pages1.len());
    println!("v2 pages: {}", pages2.len());

    // Check page contents
    for (i, page) in pages1.iter().enumerate() {
        println!("v1 page {} has {} content streams", i, page.contents.len());
        for (j, &obj_ref) in page.contents.iter().enumerate() {
            println!("  v1 page {} stream {}: {:?}", i, j, obj_ref);
        }
    }

    for (i, page) in pages2.iter().enumerate() {
        println!("v2 page {} has {} content streams", i, page.contents.len());
        for (j, &obj_ref) in page.contents.iter().enumerate() {
            println!("  v2 page {} stream {}: {:?}", i, j, obj_ref);
        }
    }

    // Decode and show actual content streams
    use pdftract_core::parser::stream::{FileSource as ParserFileSource, decode_stream, ExtractionOptions};
    use pdftract_core::parser::stream::PdfSource as ParserPdfSource;

    let source1 = ParserFileSource::open(&v1).expect("Failed to open v1");
    let source2 = ParserFileSource::open(&v2).expect("Failed to open v2");

    if !pages1.is_empty() && !pages1[0].contents.is_empty() {
        let obj_ref = pages1[0].contents[0];
        match resolver1.resolve(obj_ref) {
            Ok(pdftract_core::parser::object::PdfObject::Stream(stream)) => {
                let opts = ExtractionOptions::default();
                let mut decompress_counter = 0u64;
                let decoded = decode_stream(&*stream, &source1, &opts, &mut decompress_counter);
                println!("v1 decoded content ({} bytes): {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
            }
            Ok(other) => println!("v1 stream is not a stream: {:?}", other),
            Err(e) => println!("v1 stream failed to resolve: {:?}", e),
        }
    }

    if !pages2.is_empty() && !pages2[0].contents.is_empty() {
        let obj_ref = pages2[0].contents[0];
        match resolver2.resolve(obj_ref) {
            Ok(pdftract_core::parser::object::PdfObject::Stream(stream)) => {
                let opts = ExtractionOptions::default();
                let mut decompress_counter = 0u64;
                let decoded = decode_stream(&*stream, &source2, &opts, &mut decompress_counter);
                println!("v2 decoded content ({} bytes): {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
            }
            Ok(other) => println!("v2 stream is not a stream: {:?}", other),
            Err(e) => println!("v2 stream failed to resolve: {:?}", e),
        }
    }

    // Test content_edit_one_paragraph
    let dir2 = fixtures_dir.join("content_edit_one_paragraph");
    let v1p = dir2.join("v1.pdf");
    let v2p = dir2.join("v2.pdf");

    println!("\n=== Testing content_edit_one_paragraph ===");

    let (fp1p, _, _, _) = parse_pdf_file(&v1p).expect("Failed to parse v1p");
    let (fp2p, _, _, _) = parse_pdf_file(&v2p).expect("Failed to parse v2p");

    println!("v1 fingerprint: {}", fp1p);
    println!("v2 fingerprint: {}", fp2p);
    println!("Are they equal? {}", fp1p == fp2p);

    assert_ne!(fp1, fp2, "content_edit_one_glyph should produce different fingerprints");
    assert_ne!(fp1p, fp2p, "content_edit_one_paragraph should produce different fingerprints");
}
