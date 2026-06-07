//! Quick test to verify SDK extraction works with fixtures

use pdftract_core::sdk;
use pdftract_core::options::ExtractionOptions;

#[test]
fn test_simple_extract() {
    let fixture_path = std::path::Path::new("../../../tests/sdk-conformance/fixtures/scientific_paper/01.pdf");

    println!("Testing extraction with: {:?}", fixture_path);

    // Check if file exists
    if !fixture_path.exists() {
        println!("ERROR: Fixture file does not exist at {:?}", fixture_path);
        panic!("Fixture not found");
    }

    println!("File exists");

    // Try to get metadata first (lighter operation)
    println!("Trying get_metadata...");
    match sdk::get_metadata(fixture_path) {
        Ok(metadata) => {
            println!("SUCCESS: get_metadata returned:");
            println!("  page_count: {}", metadata.page_count);
            println!("  is_encrypted: {}", metadata.is_encrypted);
            println!("  is_tagged: {}", metadata.is_tagged);
        }
        Err(e) => {
            println!("ERROR: get_metadata failed: {}", e);
        }
    }

    // Try full extraction
    println!("Trying extract...");
    let options = ExtractionOptions::default();
    match sdk::extract(fixture_path, &options) {
        Ok(result) => {
            println!("SUCCESS: extract returned:");
            println!("  fingerprint: {}", result.fingerprint);
            println!("  pages: {}", result.pages.len());
            if !result.pages.is_empty() {
                let page = &result.pages[0];
                println!("  First page:");
                println!("    index: {}", page.index);
                println!("    width: {:?}", page.width);
                println!("    height: {:?}", page.height);
                println!("    rotation: {:?}", page.rotation);
                println!("    spans: {}", page.spans.len());
                println!("    blocks: {}", page.blocks.len());
            }
        }
        Err(e) => {
            println!("ERROR: extract failed: {}", e);
            panic!("Extract failed");
        }
    }
}
