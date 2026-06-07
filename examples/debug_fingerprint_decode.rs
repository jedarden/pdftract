use pdftract_core::document::compute_pdf_fingerprint;
use pdftract_core::parser::xref::XrefResolver;
use pdftract_core::parser::object::PdfObject;
use pdftract_core::parser::stream::{decode_stream, ExtractionOptions};
use pdftract_core::parser::PdfSource as ParserPdfSource;
use pdftract_core::parser::stream::PdfFileSource;
use sha2::{Digest, Sha256};

fn main() {
    let v1_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    // Parse v1
    let source1 = PdfFileSource::open(&v1_path).unwrap();
    let startxref1 = pdftract_core::document::find_startxref(&source1).unwrap();
    let xref_section1 = pdftract_core::document::load_xref_with_prev_chain(&source1, startxref1);
    let resolver1 = XrefResolver::from_section(xref_section1.clone());
    let root_ref1 = xref_section1.trailer.as_ref().unwrap().get("Root").unwrap().as_ref().unwrap();
    let catalog1 = pdftract_core::document::parse_catalog(&resolver1, root_ref1, Some(&source1 as &dyn ParserPdfSource)).unwrap();
    let pages1 = pdftract_core::document::flatten_page_tree(&resolver1, catalog1.pages_ref).unwrap();
    
    // Parse v2
    let source2 = PdfFileSource::open(&v2_path).unwrap();
    let startxref2 = pdftract_core::document::find_startxref(&source2).unwrap();
    let xref_section2 = pdftract_core::document::load_xref_with_prev_chain(&source2, startxref2);
    let resolver2 = XrefResolver::from_section(xref_section2.clone());
    let root_ref2 = xref_section2.trailer.as_ref().unwrap().get("Root").unwrap().as_ref().unwrap();
    let catalog2 = pdftract_core::document::parse_catalog(&resolver2, root_ref2, Some(&source2 as &dyn ParserPdfSource)).unwrap();
    let pages2 = pdftract_core::document::flatten_page_tree(&resolver2, catalog2.pages_ref).unwrap();

    // Get content streams for v1
    let page1 = &pages1[0];
    println!("v1 page contents: {:?}", page1.contents);
    
    let mut decompress_counter = 0u64;
    let opts = ExtractionOptions::default();
    
    for &stream_ref in &page1.contents {
        match resolver1.resolve(stream_ref) {
            Ok(PdfObject::Stream(stream)) => {
                println!("v1 stream dict: {:?}", stream.dict.keys().collect::<Vec<_>>());
                let decoded = decode_stream(&*stream, &source1, &opts, &mut decompress_counter);
                println!("v1 decoded stream ({} bytes): {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
                let hash = Sha256::digest(&decoded);
                println!("v1 SHA-256: {}", hex::encode(hash));
            }
            other => println!("v1 stream resolved to: {:?}", other),
        }
    }

    // Get content streams for v2
    let page2 = &pages2[0];
    println!("\nv2 page contents: {:?}", page2.contents);
    
    for &stream_ref in &page2.contents {
        match resolver2.resolve(stream_ref) {
            Ok(PdfObject::Stream(stream)) => {
                println!("v2 stream dict: {:?}", stream.dict.keys().collect::<Vec<_>>());
                let decoded = decode_stream(&*stream, &source2, &opts, &mut decompress_counter);
                println!("v2 decoded stream ({} bytes): {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
                let hash = Sha256::digest(&decoded);
                println!("v2 SHA-256: {}", hex::encode(hash));
            }
            other => println!("v2 stream resolved to: {:?}", other),
        }
    }

    // Compute fingerprints
    let fp1 = compute_pdf_fingerprint(&v1_path).unwrap();
    let fp2 = compute_pdf_fingerprint(&v2_path).unwrap();
    println!("\nv1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    println!("fingerprints match: {}", fp1 == fp2);
}
