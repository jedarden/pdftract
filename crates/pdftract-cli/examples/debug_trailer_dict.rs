use std::path::Path;
use pdftract_core::parser::stream::{FileSource, PdfSource};
use pdftract_core::parser::xref::load_xref_with_prev_chain;

fn main() {
    let path = Path::new("tests/fingerprint/fixtures/byte_identical/v1.pdf");
    let source = FileSource::open(path).unwrap();
    
    // Read startxref from the end of the file
    let len = source.len().unwrap();
    let scan_size = 1024.min(len) as usize;
    let scan_start = (len - scan_size as u64) as u64;
    let tail_data = source.read_at(scan_start, scan_size).unwrap();
    
    let startxref_pos = tail_data.windows(9).rposition(|w| w == b"startxref").unwrap();
    let offset_data = &tail_data[startxref_pos + 9..];
    let offset_start = offset_data.iter().position(|&b| !matches!(b, b' ' | b'\r' | b'\n' | b'\t')).unwrap();
    let offset_data_trimmed = &offset_data[offset_start..];
    let newline_pos = offset_data_trimmed.iter().position(|&b| b == b'\n' || b == b'\r').unwrap();
    let offset_str = std::str::from_utf8(&offset_data_trimmed[..newline_pos]).unwrap();
    let startxref_offset: u64 = offset_str.trim().parse().unwrap();
    
    println!("startxref offset: {}", startxref_offset);
    
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);
    
    println!("Xref entries: {}", xref_section.entries.len());
    
    if let Some(trailer) = &xref_section.trailer {
        println!("Trailer found with {} keys", trailer.len());
        for (key, _value) in trailer.iter() {
            println!("  Key: '{}'", key);
        }
        
        // Try different lookups
        println!("trailer.get(\"Root\"): {:?}", trailer.get("Root"));
        println!("trailer.get(\"/Root\"): {:?}", trailer.get("/Root"));
        println!("trailer.get(\"Size\"): {:?}", trailer.get("Size"));
        println!("trailer.get(\"/Size\"): {:?}", trailer.get("/Size"));
    } else {
        println!("No trailer found!");
    }
}
