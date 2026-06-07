//! Debug test for content_edit fixtures.

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::stream::{FileSource, PdfSource as ParserPdfSource};
use std::path::PathBuf;

#[test]
fn debug_content_edit_one_glyph() {
    let mut fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixtures_dir.push("../../tests/fingerprint/fixtures");

    // Load v1.pdf
    let v1_path = fixtures_dir.join("content_edit_one_glyph/v1.pdf");
    let v1_source = FileSource::open(&v1_path).unwrap();

    // Parse to get fingerprint input
    let (fp1, _, pages1, resolver1) = parse_pdf_file(&v1_path).unwrap();
    println!("v1 fingerprint: {}", fp1);

    // Check page 0 content stream
    let page1 = &pages1[0];
    println!("Page 0 content streams: {} streams", page1.contents.len());

    // Load v2.pdf
    let v2_path = fixtures_dir.join("content_edit_one_glyph/v2.pdf");
    let v2_source = FileSource::open(&v2_path).unwrap();
    let (fp2, _, pages2, resolver2) = parse_pdf_file(&v2_path).unwrap();
    println!("v2 fingerprint: {}", fp2);

    // Check page 0 content stream
    let page2 = &pages2[0];
    println!("Page 0 content streams: {} streams", page2.contents.len());

    // Try to read and decode the content streams
    for (i, content_ref) in page1.contents.iter().enumerate() {
        let obj = resolver1.resolve(*content_ref).unwrap();
        if let pdftract_core::parser::object::PdfObject::Stream(stream) = obj {
            println!("v1 stream {} len_hint: {:?}", i, stream.len_hint);
            println!("v1 stream filter: {:?}", stream.dict.get("/Filter"));

            // Try to decode
            use pdftract_core::parser::stream::{ExtractionOptions, decode_stream};
            let mut decompress_counter = 0u64;
            let decoded = decode_stream(&*stream, &v1_source, &ExtractionOptions::default(), &mut decompress_counter);
            println!("v1 decoded stream (first 100 bytes): {:?}", &decoded[..decoded.len().min(100)]);
            println!("v1 decoded as text: {:?}", String::from_utf8_lossy(&decoded));
        }
    }

    for (i, content_ref) in page2.contents.iter().enumerate() {
        let obj = resolver2.resolve(*content_ref).unwrap();
        if let pdftract_core::parser::object::PdfObject::Stream(stream) = obj {
            println!("v2 stream {} len_hint: {:?}", i, stream.len_hint);
            println!("v2 stream filter: {:?}", stream.dict.get("/Filter"));

            // Try to decode
            use pdftract_core::parser::stream::{ExtractionOptions, decode_stream};
            let mut decompress_counter = 0u64;
            let decoded = decode_stream(&*stream, &v2_source, &ExtractionOptions::default(), &mut decompress_counter);
            println!("v2 decoded stream (first 100 bytes): {:?}", &decoded[..decoded.len().min(100)]);
            println!("v2 decoded as text: {:?}", String::from_utf8_lossy(&decoded));
        }
    }

    assert_ne!(fp1, fp2, "Fingerprints should differ");
}
