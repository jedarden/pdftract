use pdftract_core::parser::stream::MemorySource;
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

    println!("Entries: {}", xref_section.entries.len());
    println!("Has trailer: {}", xref_section.trailer.is_some());

    if let Some(ref trailer) = xref_section.trailer {
        println!("Trailer keys: {:?}", trailer.keys().collect::<Vec<_>>());

        if let Some(root_obj) = trailer.get("Root") {
            println!("Root object: {:?}", root_obj);

            // Try to resolve the reference
            if let pdftract_core::parser::object::types::PdfObject::Ref(ref_obj_ref) = root_obj {
                println!("Root reference: {:?}", ref_obj_ref);

                let resolver =
                    pdftract_core::parser::xref::XrefResolver::from_section(xref_section.clone());

                match resolver.resolve(*ref_obj_ref) {
                    Ok(resolved) => println!("Resolved root: {:?}", resolved),
                    Err(e) => println!("Failed to resolve root reference: {:?}", e),
                }
            }
        }
    }
}
