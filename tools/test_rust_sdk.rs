//! Test the Rust SDK extract_markdown function to show correct behavior.
//!
//! This demonstrates what the extract_markdown output SHOULD look like
//! compared to extract_text output.

use pdftract_core::sdk::{extract_text, extract_markdown, ExtractionOptions};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let pdf_path = args.get(1)
        .map(|s| s.as_str())
        .unwrap_or("tests/fixtures/remote_100page.pdf");

    let options = ExtractionOptions::default();
    let path = Path::new(pdf_path);

    println!("Testing Rust SDK on: {}", pdf_path);
    println!("{}", "=".repeat(60));
    println!();

    // Test extract_text
    match extract_text(path, &options) {
        Ok(text_output) => {
            println!("✓ extract_text() succeeded");
            println!("  Length: {} characters", text_output.len());
            if text_output.len() > 0 {
                println!("  First 200 chars:");
                println!("  {}", text_output.chars().take(200).collect::<String>());
            } else {
                println!("  (empty output)");
            }
        }
        Err(e) => {
            println!("✗ extract_text() failed: {}", e);
        }
    }
    println!();

    // Test extract_markdown
    match extract_markdown(path, &options) {
        Ok(md_output) => {
            println!("✓ extract_markdown() succeeded");
            println!("  Length: {} characters", md_output.len());
            if md_output.len() > 0 {
                println!("  First 200 chars:");
                println!("  {}", md_output.chars().take(200).collect::<String>());
            } else {
                println!("  (empty output)");
            }
        }
        Err(e) => {
            println!("✗ extract_markdown() failed: {}", e);
        }
    }
    println!();

    // Compare both
    let text_result = extract_text(path, &options);
    let md_result = extract_markdown(path, &options);

    match (text_result, md_result) {
        (Ok(text), Ok(md)) => {
            println!("=" * 60);
            if text == md {
                println!("⚠️  BUG CONFIRMED");
                println!("   extract_text() and extract_markdown() produce IDENTICAL output");
                println!("   Both are {} characters", text.len());
            } else {
                println!("✓ Outputs are different (expected behavior)");
                println!("   extract_text(): {} chars", text.len());
                println!("   extract_markdown(): {} chars", md.len());

                // Show sample differences
                if !text.is_empty() || !md.is_empty() {
                    println!();
                    println!("   Sample text output:");
                    println!("   {}", text.chars().take(100).collect::<String>());
                    println!();
                    println!("   Sample markdown output:");
                    println!("   {}", md.chars().take(100).collect::<String>());
                }
            }
            println!("=" * 60);
        }
        _ => {
            println!("Could not compare - one or both functions failed");
        }
    }

    Ok(())
}
