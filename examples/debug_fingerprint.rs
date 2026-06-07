use std::path::Path;

fn main() {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("CARGO_MANIFEST_DIR: {}", cargo_manifest_dir);
    
    let base = Path::new(&cargo_manifest_dir);
    let fixture_path = base
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(base)
        .join("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    
    println!("Fixture path: {:?}", fixture_path);
    println!("Exists: {}", fixture_path.exists());
    
    // Try to read the file
    match std::fs::read(&fixture_path) {
        Ok(data) => println!("File size: {} bytes", data.len()),
        Err(e) => println!("Failed to read: {}", e),
    }
}
