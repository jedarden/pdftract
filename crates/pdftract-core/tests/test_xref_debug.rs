//! Debug test for xref parsing issues

use pdftract_core::parser::xref::{load_xref_with_prev_chain};
use pdftract_core::parser::stream::{FileSource, PdfSource};

#[test]
fn test_debug_xref_parsing() {
    let path = "tests/fixtures/tagged-suspects-true.pdf";

    let source = match FileSource::open(std::path::Path::new(path)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            return;
        }
    };

    // Find startxref
    let file_len = source.len().unwrap() as usize;
    let tail_data = source.read_at(file_len.saturating_sub(1024) as u64, 1024).unwrap();

    // Find "startxref" in the tail data
    let startxref_pos = tail_data.windows(9)
        .rposition(|w| w == b"startxref")
        .expect("startxref not found");

    // Parse the offset after "startxref"
    let offset_data = &tail_data[startxref_pos + 9..];

    // Skip leading whitespace
    let offset_start = offset_data.iter()
        .position(|&b| !matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
        .unwrap_or(offset_data.len());

    let offset_data_trimmed = &offset_data[offset_start..];

    // Find the newline after the offset
    let newline_pos = offset_data_trimmed.iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(offset_data_trimmed.len());

    let offset_str = std::str::from_utf8(&offset_data_trimmed[..newline_pos]).unwrap();
    let startxref: u64 = offset_str.trim().parse().unwrap();

    println!("startxref offset: {}", startxref);

    // Load xref
    let xref_section = load_xref_with_prev_chain(&source, startxref);

    println!("Xref entries: {}", xref_section.entries.len());

    // Check if object 1 is in the xref
    if let Some(entry) = xref_section.entries.get(&1) {
        println!("Object 1 xref entry: {:?}", entry);
    } else {
        println!("Object 1 NOT FOUND in xref");
    }

    // Check trailer
    if let Some(ref trailer) = xref_section.trailer {
        println!("Trailer keys: {:?}", trailer.keys().collect::<Vec<_>>());
        if let Some(root_obj) = trailer.get("Root") {
            println!("Trailer /Root: {:?}", root_obj);
        } else {
            println!("Trailer /Root NOT FOUND");
        }
    }
}
