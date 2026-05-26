use pdftract_core::parser::stream::MemorySource;
use pdftract_core::parser::xref;

fn main() {
    let path = "tests/fixtures/tagged-suspects-false.pdf";

    let mut file = std::fs::File::open(path).unwrap();
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer).unwrap();

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

    // Try traditional xref parsing
    let traditional = xref::parse_traditional_xref(&source, start_offset);
    println!("Traditional xref:");
    println!("  Entries: {}", traditional.entries.len());
    println!("  Has trailer: {}", traditional.trailer.is_some());
    println!("  Diagnostics: {}", traditional.diagnostics.len());
    for diag in &traditional.diagnostics {
        println!("    - {:?}: {}", diag.code, diag.message);
    }

    // Try full xref loading
    let xref_section = xref::load_xref_with_prev_chain(&source, start_offset);
    println!("\nFull xref loading:");
    println!("  Entries: {}", xref_section.entries.len());
    println!("  Has trailer: {}", xref_section.trailer.is_some());
    println!("  Diagnostics: {}", xref_section.diagnostics.len());
    for diag in &xref_section.diagnostics {
        println!("    - {:?}: {}", diag.code, diag.message);
    }
}
