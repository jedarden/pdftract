use pdftract_core::parser::xref::parse_traditional_xref;
use pdftract_core::parser::stream::MemorySource;

fn main() {
    let pdf_data = std::fs::read("tests/fingerprint/fixtures/byte_identical/v1.pdf").unwrap();
    
    // Find the trailer location manually
    if let Some(trailer_pos) = pdf_data.windows(7).position(|w| w == b"trailer") {
        println!("Found 'trailer' at offset: {}", trailer_pos);
        
        // Print 100 bytes after "trailer"
        let end_pos = (trailer_pos + 100).min(pdf_data.len());
        println!("Bytes after 'trailer':");
        for i in trailer_pos..end_pos {
            if pdf_data[i] >= 32 && pdf_data[i] <= 126 {
                print!("{}", pdf_data[i] as char);
            } else {
                print!("\\x{:02x}", pdf_data[i]);
            }
        }
        println!();
    }
    
    let source = MemorySource::new(pdf_data);
    let xref = parse_traditional_xref(&source, 439);
    
    println!("\nXref entries: {}", xref.entries.len());
    println!("Diagnostics: {}", xref.diagnostics.len());
    for diag in &xref.diagnostics {
        println!("  - {:?}: {}", diag.code, diag.message);
    }
    
    if let Some(trailer) = &xref.trailer {
        println!("\nTrailer keys:");
        for (key, value) in trailer.iter() {
            println!("  '{}': {:?}", key, value);
        }
    } else {
        println!("\nNo trailer found");
    }
}
