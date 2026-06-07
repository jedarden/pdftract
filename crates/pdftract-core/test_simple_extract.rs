use pdftract_core::sdk;
use pdftract_core::options::ExtractionOptions;

fn main() {
    let path = std::path::Path::new("tests/sdk-conformance/fixtures/scientific_paper/01.pdf");
    let options = ExtractionOptions::default();
    
    match sdk::extract(path, &options) {
        Ok(result) => {
            println!("Extracted {} pages", result.pages.len());
            if let Some(first_page) = result.pages.first() {
                println!("First page index: {:?}", first_page.index);
                println!("First page width: {:?}", first_page.width);
                println!("First page height: {:?}", first_page.height);
                println!("First page rotation: {:?}", first_page.rotation);
                println!("First page spans: {}", first_page.spans.len());
                println!("First page blocks: {}", first_page.blocks.len());
            }
        }
        Err(e) => {
            eprintln!("Extract failed: {}", e);
        }
    }
    
    // Test metadata
    match sdk::get_metadata(path) {
        Ok(metadata) => {
            println!("Metadata page_count: {}", metadata.page_count);
        }
        Err(e) => {
            eprintln!("Get metadata failed: {}", e);
        }
    }
}
