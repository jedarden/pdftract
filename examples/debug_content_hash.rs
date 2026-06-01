use pdftract_core::document::parse_pdf_file;
use std::path::Path;

fn main() {
    let paths = [
        "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf",
    ];
    
    for path in paths {
        println!("\n=== {} ===", path);
        let (fp, catalog, pages, resolver) = parse_pdf_file(Path::new(path))
            .expect("Failed to parse");
        
        println!("Fingerprint: {}", fp);
        println!("Page count: {}", pages.len());
        
        if let Some(page) = pages.first() {
            println!("Contents refs: {:?}", page.contents);
            println!("MediaBox: {:?}", page.media_box);
            println!("Rotate: {:?}", page.rotate);
        }
        
        // Try to resolve the first content stream
        if let Some(page) = pages.first() {
            if let Some(&content_ref) = page.contents.first() {
                println!("Resolving content ref: {:?}", content_ref);
                match resolver.resolve(content_ref) {
                    Ok(obj) => {
                        println!("Resolved object type: {:?}", std::mem::discriminant(&obj));
                        if let Some(stream) = obj.as_stream() {
                            println!("Stream dict keys: {:?}", stream.dict.keys().collect::<Vec<_>>());
                            if let Some(&len) = stream.dict.get("/Length").and_then(|l| l.as_integer()) {
                                println!("Stream Length: {}", len);
                            }
                            if let Some(&filter) = stream.dict.get("/Filter").and_then(|f| f.as_name()) {
                                println!("Stream Filter: {}", filter);
                            }
                        }
                    }
                    Err(e) => println!("Failed to resolve: {:?}", e),
                }
            }
        }
    }
}
