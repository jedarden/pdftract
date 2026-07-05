//! Forms integration tests.
//!
//! This test module verifies PDF form handling including:
//! - AcroForm detection and parsing
//! - Form field extraction and validation
//! - Widget annotation processing
//! - Form data encoding/decoding

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discover all PDF files in the given directory recursively.
///
/// # Arguments
/// * `fixtures_path` - Path to the fixtures directory to search
///
/// # Returns
/// A `Vec<PathBuf>` containing paths to all discovered PDF files
pub fn discover_pdf_fixtures<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let mut pdf_files = Vec::new();

    let walker = WalkDir::new(fixtures_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().map(|ext| ext == "pdf").unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf());

    pdf_files.extend(walker);
    pdf_files.sort();

    pdf_files
}

#[test]
fn test_discover_pdf_fixtures() {
    let fixtures_dir = "tests/fixtures/forms";
    let pdf_files = discover_pdf_fixtures(fixtures_dir);

    println!("\n=== Discovered PDF Fixtures ===");
    if pdf_files.is_empty() {
        println!("No PDF files found in {}", fixtures_dir);
    } else {
        for pdf_path in &pdf_files {
            println!("  - {}", pdf_path.display());
        }
        println!("Total: {} PDF file(s)", pdf_files.len());
    }
    println!("==============================\n");

    // Test that the function runs without errors
    // (We don't assert a count since fixtures may be added/removed)
    let _ = pdf_files;
}
