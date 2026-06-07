use pdftract_core::document::compute_pdf_fingerprint;
use pdftract_core::parser::lexer::Lexer;
use std::path::PathBuf;

fn main() {
    let v1 = PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2 = PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("=== v1.pdf ===");
    let fp1 = compute_pdf_fingerprint(&v1).unwrap();
    println!("Fingerprint: {}", fp1);

    println!("\n=== v2.pdf ===");
    let fp2 = compute_pdf_fingerprint(&v2).unwrap();
    println!("Fingerprint: {}", fp2);

    println!("\n=== Comparison ===");
    if fp1 == fp2 {
        println!("ERROR: Fingerprints are IDENTICAL but should DIFFER!");
    } else {
        println!("OK: Fingerprints differ as expected");
    }

    // Debug: Show raw content stream bytes
    println!("\n=== Raw Content Stream Debug ===");
    debug_content_stream(&v1, "v1");
    debug_content_stream(&v2, "v2");
}

fn debug_content_stream(path: &PathBuf, label: &str) {
    use pdftract_core::parser::object::{PdfObject, PdfStream};
    use pdftract_core::parser::stream::{FileSource, decode_stream};
    use pdftract_core::parser::xref::{XrefResolver, load_xref};
    use pdftract_core::parser::catalog::parse_catalog;
    use pdftract_core::parser::pages::{flatten_page_tree, PageDict};
    use pdftract_core::parser::trailer::parse_trailer_dict;

    let source = FileSource::open(path).unwrap();
    let file_size = source.len();

    // Parse trailer
    let tail_data = source.read_range(file_size.saturating_sub(1024), 1024).unwrap();
    let trailer_offset = pdftract_core::document::find_startxref_offset(&tail_data).unwrap();
    let trailer_data = source.read_range(trailer_offset, file_size - trailer_offset).unwrap();
    let (_, trailer_dict) = parse_trailer_dict(&trailer_data).unwrap();

    // Load xref
    let resolver = load_xref(&trailer_dict, &source).unwrap();

    // Get catalog
    let root_ref = trailer_dict.get("/Root").and_then(|o| o.as_ref()).unwrap();
    let catalog_obj = resolver.resolve_with_source(*root_ref, &source).unwrap();
    let catalog = parse_catalog(&catalog_obj).unwrap();

    // Flatten pages
    let pages: Vec<PageDict> = flatten_page_tree(&catalog.pages_ref, &resolver, &source).collect();

    println!("{}: {} pages", label, pages.len());
    for (i, page) in pages.iter().enumerate() {
        println!("{} page {}: {} content streams", label, i, page.contents.len());
        for (j, &stream_ref) in page.contents.iter().enumerate() {
            match resolver.resolve_with_source(stream_ref, &source) {
                Ok(PdfObject::Stream(stream)) => {
                    let decoded = decode_stream(&stream, &source, &Default::default(), &mut 0);
                    println!("  stream {}: {} bytes -> {} decoded", j, stream.raw_len, decoded.len());
                    println!("    raw bytes (first 100): {:?}", &stream.raw_bytes[..stream.raw_bytes.len().min(100)]);
                    println!("    decoded: {}", String::from_utf8_lossy(&decoded));
                }
                Ok(other) => {
                    println!("  stream {}: NOT a stream: {:?}", j, std::mem::discriminant(&other));
                }
                Err(e) => {
                    println!("  stream {}: ERROR: {:?}", j, e);
                }
            }
        }
    }
}
