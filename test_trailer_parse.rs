use pdftract_core::parser::xref::parse_traditional_xref;
use pdftract_core::parser::stream::MemorySource;

fn main() {
    let pdf_data = std::fs::read("tests/fingerprint/fixtures/byte_identical/v1.pdf").unwrap();
    let source = MemorySource::new(pdf_data);
    
    let xref = parse_traditional_xref(&source, 0);
    
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
}
