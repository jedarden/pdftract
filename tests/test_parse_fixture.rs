use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let pdf_path = Path::new("/home/coding/pdftract/tests/fixtures/test-minimal.pdf");
    match parse_pdf_file(pdf_path) {
        Ok((fingerprint, catalog, pages, resolver)) => {
            println!("PDF parsed successfully");
            println!("Fingerprint: {}", fingerprint);
            println!("Pages: {}", pages.len());
        }
        Err(e) => {
            println!("Error parsing PDF: {}", e);
            for cause in e.chain() {
                println!("  caused by: {}", cause);
            }
        }
    }
}
