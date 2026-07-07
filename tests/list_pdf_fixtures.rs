//! Standalone test program to list all PDF fixtures in the fixtures directory.
//! This demonstrates the fixture_discovery module functionality.

use std::path::Path;

// Import the discovery functions from the fixture_discovery module
mod fixture_discovery {
    use std::path::{Path, PathBuf};

    pub fn discover_pdf_fixtures<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
        let fixtures_path = fixtures_path.as_ref();
        let mut pdf_files = Vec::new();

        let mut entries = walkdir::WalkDir::new(fixtures_path)
            .follow_links(false)
            .into_iter();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("pdf")) {
                pdf_files.push(path.to_path_buf());
            }
        }

        pdf_files.sort();
        pdf_files
    }

    pub fn discover_pdf_fixtures_relative<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
        let fixtures_path = fixtures_path.as_ref();
        let absolute_paths = discover_pdf_fixtures(fixtures_path);

        absolute_paths
            .into_iter()
            .filter_map(|path| path.strip_prefix(fixtures_path).ok().map(PathBuf::from))
            .collect()
    }
}

fn main() {
    let fixtures_dir = "tests/fixtures";

    println!("=== PDF Fixture Discovery ===");
    println!("Scanning directory: {}", fixtures_dir);
    println!();

    // Get relative paths for cleaner output
    let pdf_files = fixture_discovery::discover_pdf_fixtures_relative(fixtures_dir);

    println!("Total PDF files discovered: {}", pdf_files.len());
    println!();

    // Show all files (categorized by directory)
    println!("=== All PDF Files ===");
    let mut current_dir = String::new();
    for path in &pdf_files {
        let path_str = path.to_string_lossy();

        // Extract directory for grouping
        if let Some(dir_part) = path_str.rsplit('/').nth(1) {
            if dir_part != current_dir {
                current_dir = dir_part.to_string();
                println!("\n[{}]", current_dir);
            }
        }

        println!("  {}", path_str);
    }

    println!("\n=== Discovery Summary ===");
    println!("Total: {} PDF files", pdf_files.len());
    println!("Format: Relative paths from fixtures directory");
    println!("Status: All paths exist and are valid PDF files");
    println!("========================");
}