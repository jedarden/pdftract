use std::path::Path;

fn main() {
    let path = Path::new("tests/sdk-conformance/fixtures/scientific_paper/01.pdf");
    
    // Try to extract with pdftract_core::sdk::extract
    let options = pdftract_core::options::ExtractionOptions::default();
    
    match pdftract_core::sdk::extract(path, &options) {
        Ok(result) => {
            eprintln!("Extraction succeeded!");
            eprintln!("Pages: {}", result.pages.len());
            if let Some(first_page) = result.pages.first() {
                eprintln!("First page: {}x{}", first_page.width, first_page.height);
            }
        }
        Err(e) => {
            eprintln!("Extraction failed: {}", e);
        }
    }
}
