use pdftract_core::document::Document;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = Path::new("tests/fixtures/sample.pdf");
    
    println!("Opening PDF: {:?}", pdf_path);
    let doc = Document::open(pdf_path)?;
    
    println!("Document fingerprint: {}", doc.fingerprint());
    println!("Page count: {}", doc.page_count()?);
    
    // Extract the first page using the basic Page extraction function
    let page = doc.extract_page(0)?;
    
    println!("\n=== Successfully extracted page ===");
    println!("  Page index: {}", page.page_index);
    println!("  Page number: {}", page.page_number);
    println!("  Width: {} points", page.width);
    println!("  Height: {} points", page.height);
    println!("  Rotation: {} degrees", page.rotation);
    println!("  Page type: {}", page.page_type);
    println!("  Page label: {:?}", page.page_label);
    println!("  Text spans: {} (empty - basic extraction)", page.spans.len());
    println!("  Blocks: {} (empty - basic extraction)", page.blocks.len());
    println!("  Links: {} (empty - basic extraction)", page.links.len());
    
    println!("\n✅ Basic Page extraction works!");
    println!("✅ Function: Document::extract_page(page_index) -> Result<Page, Error>");
    println!("✅ Returns Page struct with basic fields extracted");
    println!("✅ No validation logic - just extraction");
    
    Ok(())
}
