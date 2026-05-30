// Debug test to see what's being hashed in content streams
use pdftract_core::document::parse_pdf_file;

fn main() {
    let v1_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");
    
    println!("=== Debugging fingerprint hash ===");
    
    let (fp1, _catalog1, pages1, resolver1) = parse_pdf_file(&v1_path).unwrap();
    let (fp2, _catalog2, pages2, resolver2) = parse_pdf_file(&v2_path).unwrap();
    
    println!("v1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    
    // Check page 0 contents
    println!("\nv1 page 0 contents refs:");
    for content_ref in &pages1[0].contents {
        println!("  {:?}", content_ref);
    }
    
    println!("\nv2 page 0 contents refs:");
    for content_ref in &pages2[0].contents {
        println!("  {:?}", content_ref);
    }
    
    // Resolve and decode the streams
    println!("\n--- Resolving v1 stream ---");
    let v1_stream_obj = resolver1.resolve(pages1[0].contents[0]).unwrap();
    println!("v1 stream type: {:?}", std::mem::discriminant(&v1_stream_obj));
    
    println!("\n--- Resolving v2 stream ---");
    let v2_stream_obj = resolver2.resolve(pages2[0].contents[0]).unwrap();
    println!("v2 stream type: {:?}", std::mem::discriminant(&v2_stream_obj));
}
