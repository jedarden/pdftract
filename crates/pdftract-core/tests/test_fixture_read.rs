//! Quick test to see if we can read the fixture files

use std::path::Path;

fn main() {
    let fixture_path = Path::new("tests/sdk-conformance/fixtures/scientific_paper/01.pdf");
    println!("Testing fixture: {:?}", fixture_path);
    println!("Exists: {}", fixture_path.exists());

    if fixture_path.exists() {
        let content = std::fs::read(&fixture_path).unwrap();
        println!("File size: {} bytes", content.len());

        // Find startxref
        if let Some(pos) = content.windows(9).rposition(|w| w == b"startxref") {
            println!("Found startxref at byte position: {}", pos);
            let after_startxref = &content[pos + 9..];
            println!("Bytes after startxref: {:?}", &after_startxref[..20]);
        } else {
            println!("startxref NOT found!");
        }
    }
}
