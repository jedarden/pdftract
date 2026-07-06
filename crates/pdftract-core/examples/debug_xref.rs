//! Debug test for xref resolution

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::xref::XrefSection;
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");

    // Use the public parse_pdf_file which internally creates the resolver
    let (_fp, _cat, _pages, resolver) = parse_pdf_file(v1_path).unwrap();

    // Get the xref section from the resolver
    // We need to access it indirectly by checking what we can resolve

    // Try to resolve object 2 0 R
    let obj_2_ref = pdftract_core::parser::object::ObjRef {
        object: 2,
        generation: 0,
    };
    println!("=== Resolving object 2 0 R ===");
    match resolver.resolve(obj_2_ref) {
        Ok(obj) => println!("Resolved to: {:?}", obj),
        Err(e) => println!("Error: {:?}", e),
    }

    // Also check the raw PDF structure
    let data = std::fs::read(v1_path).unwrap();
    let trailer_start = data.windows(7).position(|w| w == b"trailer");
    if let Some(start) = trailer_start {
        println!("\n=== Raw trailer (first 200 bytes) ===");
        let trailer_data = &data[start..std::cmp::min(start + 200, data.len())];
        println!("{}", String::from_utf8_lossy(trailer_data));
    }

    // Check the xref table itself
    let xref_start = data.windows(4).position(|w| w == b"xref");
    if let Some(start) = xref_start {
        println!("\n=== Raw xref table (first 200 bytes) ===");
        let xref_data = &data[start..std::cmp::min(start + 200, data.len())];
        println!("{}", String::from_utf8_lossy(xref_data));
    }

    // Try to find object 2 in the raw data
    println!("\n=== Looking for object 2 0 obj ===");
    for i in 0..data.len().saturating_sub(10) {
        if &data[i..i + 10] == b"2 0 obj\n" || &data[i..i + 10] == b"2 0 obj\r" {
            println!("Found '2 0 obj' at offset {}", i);
            let obj_data = &data[i..std::cmp::min(i + 100, data.len())];
            println!("{}", String::from_utf8_lossy(obj_data));
            break;
        }
    }
}
