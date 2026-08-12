use pdftract_core::extract::extract_pdf;
use pdftract_core::options::ExtractionOptions;
use std::path::PathBuf;

fn main() {
    let fixture = PathBuf::from("tests/fixtures/security/embedded-js.pdf");
    let options = ExtractionOptions::default();
    let result = extract_pdf(&fixture, &options);
    
    match result {
        Ok(extraction_result) => {
            println!("Extraction succeeded!");
            println!("JavaScript actions found: {}", extraction_result.javascript_actions.len());
            for action in &extraction_result.javascript_actions {
                println!("  - Location: {}, Code excerpt: {}", action.location, action.code_excerpt);
            }
            println!("Diagnostics: {:?}", extraction_result.metadata.diagnostics);
        }
        Err(e) => {
            println!("Extraction failed: {:?}", e);
        }
    }
}
