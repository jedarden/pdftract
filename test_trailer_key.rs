use pdftract_core::parser::xref::load_xref_with_prev_chain;
use pdftract_core::source::file_source::ParserFileSource;
use pdftract_core::parser::xref::find_startxref;

fn main() {
    let source = ParserFileSource::open(std::path::Path::new("tests/fingerprint/fixtures/acrobat_resave/v1.pdf")).unwrap();
    let startxref_offset = find_startxref(&source).unwrap();
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);
    
    if let Some(trailer) = &xref_section.trailer {
        println!("Trailer keys:");
        for key in trailer.keys() {
            println!("  '{}'", key);
        }
        
        println!("\nLooking for 'Root': {:?}", trailer.get("Root"));
        println!("Looking for '/Root': {:?}", trailer.get("/Root"));
    } else {
        println!("No trailer found!");
    }
}
