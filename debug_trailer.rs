use pdftract_core::source::file_source::ParserFileSource;
use pdftract_core::parser::xref::{find_startxref, load_xref_with_prev_chain};

fn main() {
    let pdf_path = std::path::Path::new("tests/fingerprint/fixtures/acrobat_resave/v1.pdf");
    let source = ParserFileSource::open(pdf_path).unwrap();
    let startxref_offset = find_startxref(&source).unwrap();
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);
    
    println!("xref_section loaded");
    println!("trailer: {:?}", xref_section.trailer);
    
    if let Some(trailer) = &xref_section.trailer {
        println!("\nTrailer contents:");
        for (k, v) in trailer.iter() {
            println!("  key='{}' value={:?}", k, v);
        }
        
        println!("\nLooking for 'Root': {:?}", trailer.get("Root"));
        println!("Looking for '/Root': {:?}", trailer.get("/Root"));
    }
}
