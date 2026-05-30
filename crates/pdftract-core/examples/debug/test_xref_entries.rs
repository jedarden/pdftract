use pdftract_core::parser::stream::{MemorySource, PdfSource};
use pdftract_core::parser::xref;

fn main() {
    let path = "tests/fixtures/tagged-suspects-false.pdf";

    let mut file = std::fs::File::open(path).unwrap();
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer).unwrap();

    // Find startxref
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

    let source = MemorySource::new(buffer);
    let xref_section = xref::load_xref_with_prev_chain(&source, start_offset);

    println!("Entries:");
    for (obj_nr, entry) in &xref_section.entries {
        println!("  {}: {:?}", obj_nr, entry);
    }

    // Check object 1 specifically
    if let Some(entry) = xref_section.entries.get(&1) {
        println!("\nObject 1 entry: {:?}", entry);

        if let xref::XrefEntry::InUse { offset, gen_nr } = entry {
            println!("  Byte offset: {}, Generation: {}", offset, gen_nr);

            // Read the object at that offset
            let obj_bytes = source.read_at(*offset, 100).expect("Failed to read object");
            let obj_str = std::str::from_utf8(&obj_bytes).expect("Invalid UTF-8");
            println!("  Object content: {:?}", obj_str);
        }
    }
}
