use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    // Default is relative to the workspace root; no absolute checkout path is
    // assumed. Run from the workspace root or pass an explicit PDF path.
    let pdf_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/fixtures/test-minimal.pdf".to_string());
    let pdf_path = Path::new(&pdf_path);
    match parse_pdf_file(pdf_path) {
        Ok((fingerprint, catalog, pages, resolver)) => {
            println!("PDF parsed successfully");
            println!("Fingerprint: {}", fingerprint);
            println!("Pages: {}", pages.len());
            let _ = (catalog, resolver);
        }
        Err(e) => {
            println!("Error parsing PDF: {}", e);
            for cause in e.chain() {
                println!("  caused by: {}", cause);
            }
        }
    }
}
