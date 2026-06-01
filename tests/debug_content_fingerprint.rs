//! Debug test for fingerprint content hashing

use pdftract_core::document::parse_pdf_file;
use std::path::Path;

#[test]
fn debug_content_edit_one_glyph() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("Testing content_edit_one_glyph fixture");

    let (fp1, catalog1, pages1, _resolver1) = parse_pdf_file(v1_path).unwrap();
    let (fp2, catalog2, pages2, _resolver2) = parse_pdf_file(v2_path).unwrap();

    println!("v1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    println!("fingerprints match: {}", fp1 == fp2);

    println!("\nv1 pages: {}", pages1.len());
    println!("v2 pages: {}", pages2.len());

    for (i, (page1, page2)) in pages1.iter().zip(pages2.iter()).enumerate() {
        println!("\nPage {}:", i);
        println!("  v1 contents: {} refs", page1.contents.len());
        println!("  v2 contents: {} refs", page2.contents.len());
        println!("  v1 media_box: {:?}", page1.media_box);
        println!("  v2 media_box: {:?}", page2.media_box);

        if page1.contents.len() != page2.contents.len() {
            println!("  WARNING: Different number of content streams!");
        }
    }

    println!("\nv1 is_tagged: {}", catalog1.mark_info.is_tagged);
    println!("v2 is_tagged: {}", catalog2.mark_info.is_tagged);

    // This should fail - the content is different
    assert_ne!(fp1, fp2, "Content difference should produce different fingerprints");
}
