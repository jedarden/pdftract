use pdftract_core::parser::xref::parse_traditional_xref;
use pdftract_core::parser::stream::MemorySource;

fn main() {
    let pdf_data = std::fs::read("tests/fingerprint/fixtures/byte_identical/v1.pdf").unwrap();
    let source = MemorySource::new(pdf_data);
    
    // The xref starts at offset 439 according to startxref
    let xref = parse_traditional_xref(&source, 439);
    
    println!("Trailer keys:");
    if let Some(trailer) = &xref.trailer {
        for (key, value) in trailer.iter() {
            println!("  '{}': {:?}", key, value);
        }
        println!("\nTrying to get 'Root': {:?}", trailer.get("Root"));
        println!("Trying to get '/Root': {:?}", trailer.get("/Root"));
    } else {
        println!("No trailer found");
    }
    
    println!("\nXref entries: {}", xref.entries.len());
}
