use pdftract_core::{extract_pdf, ExtractionOptions};

fn main() {
    let result = extract_pdf(
        "tests/sdk-conformance/fixtures/mixed/mixed.pdf",
        &ExtractionOptions::default()
    );
    match result {
        Ok(doc) => println!("Success! Pages: {}", doc.pages.len()),
        Err(e) => println!("Error: {}", e),
    }
}
