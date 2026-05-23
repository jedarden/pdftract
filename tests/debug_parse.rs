use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let result = parse_pdf_file(Path::new("/tmp/test-valid.pdf"));
    match result {
        Ok((fingerprint, catalog, pages, resolver)) => {
            println!("Success!");
            println!("Fingerprint: {}", fingerprint);
            println!("Pages: {}", pages.len());
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
