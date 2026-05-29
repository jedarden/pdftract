use pdftract_core::parser::xref::load_xref_with_prev_chain;
use pdftract_core::parser::stream::FileSource as ParserFileSource;

fn main() {
    let source = ParserFileSource::open("tests/document_model/fixtures/tagged_3_level_outline.pdf").unwrap();
    
    // Find startxref
    let startxref_offset = find_startxref(&source).unwrap();
    println!("startxref offset: {}", startxref_offset);
    
    // Load xref
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);
    println!("trailer: {:?}", xref_section.trailer);
    
    if let Some(trailer) = &xref_section.trailer {
        println!("trailer keys: {:?}", trailer.keys().collect::<Vec<_>>());
        println!("trailer get Root: {:?}", trailer.get("Root"));
    }
}

fn find_startxref(source: &ParserFileSource) -> Result<u64, Box<dyn std::error::Error>> {
    let file_len = source.len()?;
    
    // Scan last 1024 bytes for startxref
    let scan_start = if file_len > 1024 { file_len - 1024 } else { 0 };
    let scan_end = file_len;
    let scan_size = (scan_end - scan_start) as usize;
    
    let bytes = source.read_at(scan_start, scan_size)?;
    let content = std::str::from_utf8(&bytes).ok();
    
    if let Some(content) = content {
        if let Some(pos) = content.find("startxref") {
            let offset_str = &content[pos + "startxref".len()..];
            let offset = offset_str.trim().parse::<u64>()?;
            return Ok(offset);
        }
    }
    
    Err("startxref not found".into())
}
