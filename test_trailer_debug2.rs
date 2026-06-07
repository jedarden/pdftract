use pdftract_core::parser::stream::{MemorySource, PdfSource};

// Manually implement a simple xref parser that logs everything
fn manual_parse(source: &MemorySource, start_offset: u64) {
    let mut pos = start_offset;
    
    println!("Starting manual parse at offset {}", pos);
    
    // Read 100 bytes to see what we're starting with
    let chunk = source.read_at(pos, 100).unwrap();
    println!("Bytes at offset {}:", pos);
    for i in 0..chunk.len().min(100) {
        if chunk[i] >= 32 && chunk[i] <= 126 {
            print!("{}", chunk[i] as char);
        } else {
            print!("\\x{:02x}", chunk[i]);
        }
    }
    println!();
    
    // Find "xref"
    let xref_pos = chunk.windows(4).position(|w| w == b"xref").unwrap();
    println!("Found 'xref' at relative offset {}", xref_pos);
    pos += xref_pos as u64 + 4;
    
    // Skip line ending
    let le_chunk = source.read_at(pos, 2).unwrap();
    if le_chunk[0] == b'\n' || le_chunk[0] == b'\r' {
        pos += 1;
        if le_chunk[0] == b'\r' && le_chunk.len() > 1 && le_chunk[1] == b'\n' {
            pos += 1;
        }
    }
    
    // Read the subsection header
    let header_chunk = source.read_at(pos, 100).unwrap();
    let header_str = std::str::from_utf8(&header_chunk).unwrap();
    let header_line = header_str.lines().next().unwrap();
    println!("Subsection header: '{}'", header_line);
    
    // Parse xref entries...
    let parts: Vec<&str> = header_line.split_whitespace().collect();
    let obj_count: u32 = parts[1].parse().unwrap();
    println!("Parsing {} xref entries", obj_count);
    
    // Skip header line and parse entries
    let header_end = pos + header_line.len() as u64 + 1;
    pos = header_end + (obj_count as u64 * 20); // Assume 20-byte entries
    
    // Now check for trailer
    let trailer_chunk = source.read_at(pos, 100).unwrap();
    let trailer_str = std::str::from_utf8(&trailer_chunk).unwrap();
    println!("Bytes at offset {} (expecting trailer):", pos);
    for i in 0..trailer_chunk.len().min(50) {
        if trailer_chunk[i] >= 32 && trailer_chunk[i] <= 126 {
            print!("{}", trailer_chunk[i] as char);
        } else {
            print!("\\x{:02x}", trailer_chunk[i]);
        }
    }
    println!();
    
    if trailer_str.trim_start().starts_with("trailer") {
        println!("Found trailer keyword!");
        let ws_offset = trailer_str.len() - trailer_str.trim_start().len();
        let trailer_end = pos + ws_offset as u64 + 7;
        
        let dict_chunk = source.read_at(trailer_end, 100).unwrap();
        println!("Bytes at offset {} (expecting dict):", trailer_end);
        for i in 0..dict_chunk.len().min(50) {
            if dict_chunk[i] >= 32 && dict_chunk[i] <= 126 {
                print!("{}", dict_chunk[i] as char);
            } else {
                print!("\\x{:02x}", dict_chunk[i]);
            }
        }
        println!();
    }
}

fn main() {
    let pdf_data = std::fs::read("tests/fingerprint/fixtures/byte_identical/v1.pdf").unwrap();
    let source = MemorySource::new(pdf_data);
    manual_parse(&source, 439);
}
