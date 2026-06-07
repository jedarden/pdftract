//! Debug test for fingerprint fixture content hashing

use pdftract_core::document::parse_pdf_file;
use pdftract_core::fingerprint::{compute_fingerprint, FingerprintInput, PageFingerprintData, ContentStreamData};
use pdftract_core::parser::pages::{flatten_page_tree};
use std::path::PathBuf;

#[test]
fn debug_fixture_content_difference() {
    let v1_path = PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("=== Parsing v1.pdf ===");
    let (fp1, catalog1, pages1, resolver1) = parse_pdf_file(&v1_path)
        .expect("Failed to parse v1.pdf");

    println!("Fingerprint v1: {}", fp1);
    println!("Page count: {}", pages1.len());

    if let Some(page) = pages1.first() {
        println!("First page contents: {:?}", page.contents);
        println!("First page MediaBox: {:?}", page.media_box);
    }

    println!("\n=== Parsing v2.pdf ===");
    let (fp2, catalog2, pages2, resolver2) = parse_pdf_file(&v2_path)
        .expect("Failed to parse v2.pdf");

    println!("Fingerprint v2: {}", fp2);
    println!("Page count: {}", pages2.len());

    if let Some(page) = pages2.first() {
        println!("First page contents: {:?}", page.contents);
        println!("First page MediaBox: {:?}", page.media_box);
    }

    println!("\n=== Content stream comparison ===");
    // Let's check the actual content stream data
    if let (Some(page1), Some(page2)) = (pages1.first(), pages2.first()) {
        if let (Some(&content_ref1), Some(&content_ref2)) = (page1.contents.first(), page2.contents.first()) {
            println!("Content ref v1: {:?}", content_ref1);
            println!("Content ref v2: {:?}", content_ref2);

            // Resolve and print the streams
            if let Ok(obj1) = resolver1.resolve(content_ref1) {
                println!("Content obj v1 type: {:?}", std::mem::discriminant(&obj1));
                if let pdftract_core::parser::object::PdfObject::Stream(stream1) = obj1 {
                    println!("Stream v1 dict keys: {:?}", stream1.dict.keys().collect::<Vec<_>>());
                    println!("Stream v1 raw bytes (first 100): {:?}...", &stream1.data[..stream1.data.len().min(100)]);
                }
            }

            if let Ok(obj2) = resolver2.resolve(content_ref2) {
                println!("Content obj v2 type: {:?}", std::mem::discriminant(&obj2));
                if let pdftract_core::parser::object::PdfObject::Stream(stream2) = obj2 {
                    println!("Stream v2 dict keys: {:?}", stream2.dict.keys().collect::<Vec<_>>());
                    println!("Stream v2 raw bytes (first 100): {:?}...", &stream2.data[..stream2.data.len().min(100)]);
                }
            }
        }
    }

    println!("\n=== Fingerprints are {} ===", if fp1 == fp2 { "IDENTICAL" } else { "DIFFERENT" });
}
