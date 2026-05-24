use pdftract_core::parser::stream::{MemorySource, PdfSource};
use pdftract_core::parser::xref;
use std::fs::File;
use std::io::Read;

fn main() {
    let path = "/home/coding/pdftract/tests/sdk-conformance/fixtures/large/100pages.pdf";

    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    // Find startxref BEFORE moving buffer
    let search_bytes = &buffer[buffer.len().saturating_sub(1024)..];
    let pos = search_bytes
        .windows(9)
        .rposition(|w| w == b"startxref")
        .unwrap();
    let start = buffer.len().saturating_sub(1024) + pos + 9;

    // Skip whitespace
    let mut offset_start = start;
    while offset_start < buffer.len() && buffer[offset_start].is_ascii_whitespace() {
        offset_start += 1;
    }

    let mut offset_end = offset_start;
    while offset_end < buffer.len() && buffer[offset_end].is_ascii_digit() {
        offset_end += 1;
    }

    let offset_str = std::str::from_utf8(&buffer[offset_start..offset_end]).unwrap();
    let start_offset: u64 = offset_str.parse().unwrap();

    // Now create source
    let source = MemorySource::new(buffer);

    println!("startxref offset: {}", start_offset);

    let xref_section = xref::load_xref_with_prev_chain(&source, start_offset);

    println!("Has trailer: {}", xref_section.trailer.is_some());

    if let Some(trailer) = &xref_section.trailer {
        println!("Trailer keys: {:?}", trailer.keys().collect::<Vec<_>>());
        println!("Root entry: {:?}", trailer.get("Root"));
        println!("Size entry: {:?}", trailer.get("Size"));
    }

    println!("Diagnostics count: {}", xref_section.diagnostics.len());
    for diag in &xref_section.diagnostics {
        println!(
            "  - {}: {} at byte_offset {:?}",
            diag.code, diag.message, diag.byte_offset
        );
    }
}
