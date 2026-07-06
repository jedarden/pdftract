//! Fixture discovery for CLI integration tests.
//!
//! This module provides utilities for discovering and enumerating PDF test fixtures
//! that need CLI processing. It supports:
//! - Recursive discovery of all PDF files in tests/fixtures/
//! - Category-based discovery (e.g., only malformed files, only encrypted files)
//! - Path resolution for both CLI test execution and cargo test runs
//!
//! # Usage
//!
//! ```rust
//! use fixture_discovery::{discover_all_fixtures, discover_fixtures_by_category};
//!
//! // Discover all fixtures
//! let all_fixtures = discover_all_fixtures();
//!
//! // Discover only malformed fixtures
//! let malformed = discover_fixtures_by_category("malformed");
//! ```
//!
//! # Fixture Categories
//!
//! Fixtures are organized by category in tests/fixtures/:
//! - cjk/ - CJK encoded PDFs
//! - classifier/ - Document classification fixtures
//! - encoding/ - Encoding test fixtures
//! - encrypted/ - Encrypted PDFs
//! - fonts/ - Font-related fixtures
//! - forms/ - Form PDFs
//! - malformed/ - Malformed/corrupt PDFs
//! - ocr/ - OCR-related fixtures
//! - page_class/ - Page classification fixtures
//! - perf/ - Performance testing fixtures
//! - preprocess/ - Preprocessing test fixtures
//! - profiles/ - Profile-specific fixtures
//! - scanned/ - Scanned document fixtures
//! - security/ - Security-related fixtures
//! - vector/ - Vector PDF fixtures
//! - Various root-level fixtures

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Get the root fixtures directory for the pdftract CLI tests.
///
/// This function resolves the path to tests/fixtures/ from the test's
/// execution context. It works both when run via cargo test and when
/// run as a standalone binary.
///
/// # Returns
///
/// A `PathBuf` pointing to the tests/fixtures directory.
pub fn fixtures_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is the crate directory (pdftract-cli)
    // fixtures are at ../../tests/fixtures/ relative to that
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../tests/fixtures")
}

/// Discover all PDF files in the fixtures directory tree.
///
/// This function recursively walks the entire tests/fixtures/ directory
/// and discovers all .pdf files, regardless of their category or location.
///
/// # Returns
///
/// A sorted `Vec<PathBuf>` containing paths to all discovered PDF files.
///
/// # Example
///
/// ```rust
/// let fixtures = discover_all_fixtures();
/// println!("Found {} PDF fixtures", fixtures.len());
/// ```
pub fn discover_all_fixtures() -> Vec<PathBuf> {
    discover_fixtures_in_dir(fixtures_root())
}

/// Discover PDF files in a specific category subdirectory.
///
/// This function searches for PDFs only within the specified category
/// subdirectory (e.g., tests/fixtures/malformed/).
///
/// # Arguments
///
/// * `category` - The category name (e.g., "malformed", "encrypted", "forms")
///
/// # Returns
///
/// A sorted `Vec<PathBuf>` containing paths to discovered PDF files in the category.
///
/// # Example
///
/// ```rust
/// let malformed = discover_fixtures_by_category("malformed");
/// println!("Found {} malformed fixtures", malformed.len());
/// ```
pub fn discover_fixtures_by_category(category: &str) -> Vec<PathBuf> {
    let category_path = fixtures_root().join(category);
    discover_fixtures_in_dir(category_path)
}

/// Discover PDF files in a specific directory (non-recursive).
///
/// This function searches for PDFs only in the immediate directory,
/// not subdirectories. Use this for single-level fixture directories.
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to search
///
/// # Returns
///
/// A sorted `Vec<PathBuf>` containing paths to discovered PDF files.
pub fn discover_fixtures_flat<P: AsRef<Path>>(dir_path: P) -> Vec<PathBuf> {
    let mut pdf_files = Vec::new();
    let dir = dir_path.as_ref();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                pdf_files.push(path);
            }
        }
    }

    pdf_files.sort();
    pdf_files
}

/// Internal function to discover PDFs in a directory using walkdir.
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to search recursively
///
/// # Returns
///
/// A sorted `Vec<PathBuf>` containing paths to all discovered PDF files.
fn discover_fixtures_in_dir<P: AsRef<Path>>(dir_path: P) -> Vec<PathBuf> {
    let mut pdf_files = Vec::new();
    let dir = dir_path.as_ref();

    // Don't try to walk non-existent directories
    if !dir.exists() {
        return pdf_files;
    }

    let walker = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .map(|ext| ext == "pdf")
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf());

    pdf_files.extend(walker);
    pdf_files.sort();

    pdf_files
}

/// Get fixture categories present in the fixtures directory.
///
/// This function discovers all subdirectories in tests/fixtures/ that
/// contain PDF files, returning them as a list of category names.
///
/// # Returns
///
/// A `Vec<String>` of category names (e.g., vec!["malformed", "encrypted"])
pub fn fixture_categories() -> Vec<String> {
    let mut categories = Vec::new();
    let fixtures_root = fixtures_root();

    if let Ok(entries) = std::fs::read_dir(&fixtures_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check if this directory contains PDFs
                let has_pdfs = discover_fixtures_in_dir(&path).len() > 0;
                if has_pdfs {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        categories.push(name.to_string());
                    }
                }
            }
        }
    }

    categories.sort();
    categories
}

/// Get fixture statistics for the entire fixtures tree.
///
/// This function returns summary statistics about the fixture collection,
/// useful for test reporting and validation.
///
/// # Returns
///
/// A `FixtureStats` struct containing counts and categorization.
pub fn fixture_statistics() -> FixtureStats {
    let all_fixtures = discover_all_fixtures();
    let categories = fixture_categories();
    let mut category_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for category in &categories {
        let count = discover_fixtures_by_category(category).len();
        category_counts.insert(category.clone(), count);
    }

    FixtureStats {
        total_count: all_fixtures.len(),
        category_count: categories.len(),
        category_counts,
    }
}

/// Statistics about the fixture collection.
#[derive(Debug)]
pub struct FixtureStats {
    /// Total number of PDF fixtures across all categories
    pub total_count: usize,
    /// Number of fixture categories
    pub category_count: usize,
    /// Count of fixtures per category
    pub category_counts: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_root_exists() {
        let root = fixtures_root();
        assert!(root.exists(), "Fixtures root directory should exist: {:?}", root);
        println!("Fixtures root: {}", root.display());
    }

    #[test]
    fn test_discover_all_fixtures() {
        let fixtures = discover_all_fixtures();

        println!("\n=== All PDF Fixtures Discovery ===");
        println!("Total fixtures found: {}", fixtures.len());

        if fixtures.len() > 0 {
            println!("Sample fixtures (first 10):");
            for (i, path) in fixtures.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, path.display());
            }
            if fixtures.len() > 10 {
                println!("  ... and {} more", fixtures.len() - 10);
            }
        }
        println!("====================================\n");

        // Verify all discovered files actually exist
        for path in &fixtures {
            assert!(path.exists(), "Fixture path should exist: {:?}", path);
            assert_eq!(path.extension().and_then(|s| s.to_str()), Some("pdf"));
        }
    }

    #[test]
    fn test_discover_fixtures_by_category() {
        // Test a category that should exist
        let malformed = discover_fixtures_by_category("malformed");

        println!("\n=== Malformed Fixtures Discovery ===");
        println!("Found {} malformed fixtures", malformed.len());
        for (i, path) in malformed.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, path.display());
        }
        if malformed.len() > 5 {
            println!("  ... and {} more", malformed.len() - 5);
        }
        println!("=====================================\n");

        // Verify all paths are within the malformed category
        for path in &malformed {
            assert!(path.exists(), "Malformed fixture should exist: {:?}", path);
            assert!(path.to_str().unwrap().contains("malformed"));
        }
    }

    #[test]
    fn test_fixture_categories() {
        let categories = fixture_categories();

        println!("\n=== Fixture Categories ===");
        println!("Found {} categories:", categories.len());
        for (i, category) in categories.iter().enumerate() {
            println!("  {}. {}", i + 1, category);
        }
        println!("===========================\n");

        // Verify we have expected categories
        assert!(categories.len() > 0, "Should have at least one fixture category");

        // Verify categories actually exist
        for category in &categories {
            let path = fixtures_root().join(category);
            assert!(path.exists(), "Category path should exist: {:?}", path);
            assert!(path.is_dir(), "Category should be a directory: {:?}", path);
        }
    }

    #[test]
    fn test_fixture_statistics() {
        let stats = fixture_statistics();

        println!("\n=== Fixture Statistics ===");
        println!("Total fixtures: {}", stats.total_count);
        println!("Categories: {}", stats.category_count);
        println!("Fixtures by category:");
        for (category, count) in &stats.category_counts {
            println!("  - {}: {}", category, count);
        }
        println!("==========================\n");

        // Verify statistics are consistent
        let mut sum: usize = 0;
        for count in stats.category_counts.values() {
            sum += count;
        }

        // Note: sum may be less than total_count due to root-level fixtures
        assert!(sum <= stats.total_count, "Category sum should not exceed total");
        assert!(stats.total_count > 0, "Should have discovered fixtures");
    }

    #[test]
    fn test_discover_fixtures_flat() {
        // Test flat discovery on a category directory
        let encrypted_path = fixtures_root().join("encrypted");

        if encrypted_path.exists() {
            let flat_fixtures = discover_fixtures_flat(&encrypted_path);
            let recursive_fixtures = discover_fixtures_in_dir(&encrypted_path);

            println!("\n=== Flat vs Recursive Discovery (encrypted) ===");
            println!("Flat discovery: {} fixtures", flat_fixtures.len());
            println!("Recursive discovery: {} fixtures", recursive_fixtures.len());
            println!("=============================================\n");

            // For encrypted, should be the same (no subdirectories)
            assert_eq!(flat_fixtures.len(), recursive_fixtures.len());
        }
    }

    #[test]
    fn test_nonexistent_category() {
        let empty = discover_fixtures_by_category("nonexistent_category");
        assert_eq!(empty.len(), 0, "Nonexistent category should return empty list");
    }

    #[test]
    fn test_fixture_paths_are_absolute() {
        let fixtures = discover_all_fixtures();

        for path in &fixtures {
            // All paths should be absolute for reliable CLI invocation
            assert!(path.is_absolute(), "Fixture path should be absolute: {:?}", path);
        }
    }

    #[test]
    fn test_fixture_sorting() {
        let fixtures = discover_all_fixtures();

        // Verify fixtures are sorted
        for i in 1..fixtures.len() {
            assert!(fixtures[i] >= fixtures[i-1], "Fixtures should be sorted");
        }
    }
}
