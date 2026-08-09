// Test Rust SDK extract_markdown function
use std::path::PathBuf;
use pdftract_core::options::ExtractionOptions;

fn main() {
    let fixture_pdf = PathBuf::from("tests/markdown/markdown-structures.pdf");

    if !fixture_pdf.exists() {
        eprintln!("✗ Test fixture not found: {:?}", fixture_pdf);
        eprintln!("  Run: python3 tests/fixtures/markdown_test_fixture.py");
        std::process::exit(1);
    }

    println!("Testing Rust SDK: pdftract_core::sdk::extract_markdown");
    println!("Fixture: {:?}", fixture_pdf);
    println!("{}", "=".repeat(70));

    let options = ExtractionOptions::default();

    // Test extract_markdown from SDK
    match pdftract_core::sdk::extract_markdown(&fixture_pdf, &options) {
        Ok(markdown) => {
            println!("✓ extract_markdown() returned {} characters", markdown.len());

            // Save output
            if let Err(e) = std::fs::write("tools/bf-2jwxel-rust-sdk-output.txt", &markdown) {
                eprintln!("✗ Failed to save output: {}", e);
            } else {
                println!("  Saved to: tools/bf-2jwxel-rust-sdk-output.txt");
            }

            // Compare with expected
            let expected_markdown = std::fs::read_to_string("tests/markdown/markdown-structures-expect-markdown.txt")
                .expect("Failed to read expected markdown");

            if markdown == expected_markdown {
                println!("✓ Rust SDK output matches expected Markdown");
            } else {
                println!("⚠️  Rust SDK output does NOT match expected Markdown");
                println!("   Expected {} chars, got {} chars", expected_markdown.len(), markdown.len());
                println!("\nExpected output:\n{}", expected_markdown);
                println!("\nActual output:\n{}", markdown);
            }
        }
        Err(e) => {
            eprintln!("✗ extract_markdown() failed: {}", e);
        }
    }

    // Also test extract_text for comparison
    println!("\n{}", "=".repeat(70));
    println!("Testing Rust SDK: pdftract_core::sdk::extract_text");
    match pdftract_core::sdk::extract_text(&fixture_pdf, &options) {
        Ok(text) => {
            println!("✓ extract_text() returned {} characters", text.len());

            // Save output
            if let Err(e) = std::fs::write("tools/bf-2jwxel-rust-sdk-text-output.txt", &text) {
                eprintln!("✗ Failed to save output: {}", e);
            } else {
                println!("  Saved to: tools/bf-2jwxel-rust-sdk-text-output.txt");
            }

            // Compare with expected
            let expected_text = std::fs::read_to_string("tests/markdown/markdown-structures-expect-text.txt")
                .expect("Failed to read expected text");

            if text == expected_text {
                println!("✓ Rust SDK output matches expected text");
            } else {
                println!("⚠️  Rust SDK output does NOT match expected text");
                println!("   Expected {} chars, got {} chars", expected_text.len(), text.len());
            }
        }
        Err(e) => {
            eprintln!("✗ extract_text() failed: {}", e);
        }
    }
}
