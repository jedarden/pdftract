use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let pdf_path = Path::new("/tmp/valid_test.pdf");
    match parse_pdf_file(pdf_path) {
        Ok((fingerprint, catalog, pages, resolver)) => {
            println!("Success!");
            println!("Fingerprint: {}", fingerprint);
            println!("Pages: {}", pages.len());
        }
        Err(e) => {
            println!("Error: {}", e);
            println!("Error chain:");
            for cause in e.chain() {
                println!("  - {}", cause);
            }
        }
    }
}
