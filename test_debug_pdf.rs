use pdftract_core::parser::xref::load_xref_with_prev_chain;
use pdftract_core::parser::stream::{FileSource, PdfSource};
use std::path::Path;

fn main() {
    let pdf_path = Path::new("crates/pdftract-core/tests/document_model/fixtures/ocg_default_off.pdf");

    // Open the PDF file
    let source = FileSource::open(pdf_path).expect("Failed to open PDF file");

    // Find the startxref offset
    let startxref_offset = find_startxref(&source).expect("Failed to find startxref offset");
    println!("startxref offset: {}", startxref_offset);

    // Try to load the xref
    let xref = load_xref_with_prev_chain(&source, startxref_offset);
    println!("Xref trailer: {:?}", xref.trailer);

    if let Some(trailer) = &xref.trailer {
        println!("Trailer keys: {:?}", trailer.keys().collect::<Vec<_>>());
        if let Some(root) = trailer.get("Root") {
            println!("Root: {:?}", root);
        } else {
            println!("No Root key in trailer!");
        }
    } else {
        println!("No trailer found!");
    }
}

fn find_startxref(source: &FileSource) -> Result<u64, Box<dyn std::error::Error>> {
    // Read the last 1KB of the file to find startxref
    let file_size = source.len()?;
    let read_size = 1024.min(file_size);
    let read_offset = file_size - read_size;

    let tail = source.read_at(read_offset, read_size as usize)?;
    let tail_str = std::str::from_utf8(&tail)?;

    // Find "startxref" keyword
    if let Some(pos) = tail_str.find("startxref") {
        let offset_start = pos + "startxref".len();

        // Find the offset after startxref (whitespace then number)
        let offset_str = &tail_str[offset_start..];
        let offset_str = offset_str.trim();

        if let Some(end) = offset_str.find(|c: char| !c.is_ascii_digit() && c != '-') {
            let offset_str = &offset_str[..end];
            if let Ok(offset) = offset_str.parse::<u64>() {
                return Ok(offset);
            }
        }

        // Try to parse the entire line as the offset
        if let Ok(offset) = offset_str.parse::<u64>() {
            return Ok(offset);
        }
    }

    Err("startxref not found".into())
}
