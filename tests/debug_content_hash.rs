//! Debug script to check content stream normalization

use pdftract_core::document::parse_pdf_file;
use pdftract_core::fingerprint::{hash_content_streams, ContentStreamData};
use pdftract_core::parser::xref::XrefResolver;
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    // Parse both PDFs
    let (fp1, _cat1, _pages1, resolver1) = parse_pdf_file(v1_path).unwrap();
    let (fp2, _cat2, _pages2, resolver2) = parse_pdf_file(v2_path).unwrap();

    println!("v1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    println!("Fingerprints match: {}", fp1 == fp2);

    // Now let's manually check the content stream hash
    // We need to get the content stream references and source
    let source = Box::new(pdftract_core::parser::stream::ParserFileSource::open(v1_path).unwrap());

    // Get the page content streams
    let pages1 = &_pages1;
    let pages2 = &_pages2;

    if let Some(page1) = pages1.first() {
        let streams1: Vec<ContentStreamData> = page1.contents
            .iter()
            .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
            .collect();

        let hash1 = hash_content_streams(&streams1, &resolver1, Some(&*source));
        println!("v1 content hash: {:?}", hex::encode(hash1));
    }

    let source2 = Box::new(pdftract_core::parser::stream::ParserFileSource::open(v2_path).unwrap());
    if let Some(page2) = pages2.first() {
        let streams2: Vec<ContentStreamData> = page2.contents
            .iter()
            .map(|&obj_ref| ContentStreamData::Indirect(obj_ref))
            .collect();

        let hash2 = hash_content_streams(&streams2, &resolver2, Some(&*source2));
        println!("v2 content hash: {:?}", hex::encode(hash2));
    }
}
