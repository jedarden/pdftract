//! Debug test for simple PDF parsing

use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");

    println!("Checking if file exists: {:?}", v1_path.exists());
    println!("Absolute path: {:?}", v1_path.canonicalize());

    let result = parse_pdf_file(v1_path);
    match &result {
        Ok((fp, cat, pages, _)) => {
            println!("SUCCESS");
            println!("Fingerprint: {}", fp);
            println!("Catalog pages_ref: {:?}", cat.pages_ref);
            println!("Number of pages: {}", pages.len());
        }
        Err(e) => {
            println!("ERROR: {:?}", e);
        }
    }
}
