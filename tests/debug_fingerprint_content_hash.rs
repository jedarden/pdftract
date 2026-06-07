//! Debug test to trace fingerprint content-sensitivity issue.
//!
//! This test prints the intermediate values in fingerprint computation
//! to diagnose why content_edit fixtures produce identical fingerprints.

use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("=== Content-Sensitivity Debug Test ===\n");

    // Parse v1.pdf
    println!("Parsing v1.pdf...");
    let (fp1, catalog1, pages1, resolver1) = parse_pdf_file(v1_path)
        .expect("Failed to parse v1.pdf");
    println!("  Fingerprint: {}", fp1);
    println!("  Page count: {}", pages1.len());

    // Parse v2.pdf
    println!("\nParsing v2.pdf...");
    let (fp2, catalog2, pages2, resolver2) = parse_pdf_file(v2_path)
        .expect("Failed to parse v2.pdf");
    println!("  Fingerprint: {}", fp2);
    println!("  Page count: {}", pages2.len());

    // Compare page contents
    println!("\n=== Page 0 Contents Comparison ===");
    if let (Some(page1), Some(page2)) = (pages1.first(), pages2.first()) {
        println!("  v1 contents refs: {:?}", page1.contents);
        println!("  v2 contents refs: {:?}", page2.contents);

        // Resolve and compare content streams
        for (i, (&ref1, &ref2)) in page1.contents.iter().zip(page2.contents.iter()).enumerate() {
            println!("\n  Content stream {}:", i);

            let obj1 = resolver1.resolve(ref1);
            let obj2 = resolver2.resolve(ref2);

            match (obj1, obj2) {
                (Ok(pdftract_core::parser::object::PdfObject::Stream(s1)),
                 Ok(pdftract_core::parser::object::PdfObject::Stream(s2))) => {
                    println!("    Both are streams");

                    // Get raw stream lengths
                    let raw1 = s1.stream_bytes.len();
                    let raw2 = s2.stream_bytes.len();
                    println!("    v1 raw stream length: {} bytes", raw1);
                    println!("    v2 raw stream length: {} bytes", raw2);

                    // Check if raw bytes differ
                    if s1.stream_bytes != s2.stream_bytes {
                        println!("    ✓ Raw streams are DIFFERENT");
                    } else {
                        println!("    ✗ Raw streams are IDENTICAL");
                    }

                    // Compare dictionaries
                    println!("    v1 dict keys: {:?}", s1.dict.keys().collect::<Vec<_>>());
                    println!("    v2 dict keys: {:?}", s2.dict.keys().collect::<Vec<_>>());
                }
                _ => {
                    println!("    ✗ One or both are not streams: obj1={:?}, obj2={:?}", obj1.is_ok(), obj2.is_ok());
                }
            }
        }

        println!("\n  v1 media_box: {:?}", page1.media_box);
        println!("  v2 media_box: {:?}", page2.media_box);
        println!("\n  v1 rotate: {}", page1.rotate);
        println!("  v2 rotate: {}", page2.rotate);
    }

    // Final comparison
    println!("\n=== Final Comparison ===");
    if fp1 == fp2 {
        println!("  ✗ FAIL: Fingerprints are IDENTICAL (should be DIFFERENT)");
    } else {
        println!("  ✓ PASS: Fingerprints are DIFFERENT (expected)");
    }
}
