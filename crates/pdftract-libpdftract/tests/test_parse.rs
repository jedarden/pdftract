use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let pdf_path = Path::new("/home/coding/pdftract/tests/fixtures/valid-minimal.pdf");
    match parse_pdf_file(pdf_path) {
        Ok((fingerprint, catalog, pages, resolver)) => {
            println!("Successfully parsed PDF");
            println!("Fingerprint: {}", fingerprint);
            println!("Pages: {}", pages.len());
        }
        Err(e) => {
            println!("Failed to parse PDF: {}", e);
        }
    }
}
