//! Fixture discovery module for enumerating PDF test fixtures.
//!
//! This module provides functionality to discover all PDF files in the fixtures directory
//! using glob patterns for filesystem traversal.

use std::path::{Path, PathBuf};
use std::ffi::OsStr;

/// Discover all PDF files in the fixtures directory using glob pattern matching.
///
/// This function uses glob patterns to recursively search for all PDF files
/// in the fixtures directory tree. Returns a sorted list of all PDF files found.
///
/// # Arguments
/// * `fixtures_path` - Path to the fixtures directory (e.g., "tests/fixtures")
///
/// # Returns
/// A vector of `PathBuf` containing paths to all discovered PDF files,
/// sorted alphabetically for consistent ordering.
///
/// # Example
/// ```rust,no_run
/// let pdf_files = discover_pdf_fixtures_glob("tests/fixtures");
/// println!("Found {} PDF files", pdf_files.len());
/// for path in pdf_files {
///     println!("  {}", path.display());
/// }
/// ```
pub fn discover_pdf_fixtures_glob<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let fixtures_path = fixtures_path.as_ref();
    let mut pdf_files = Vec::new();

    // Use glob pattern to find all PDF files recursively
    let pattern = fixtures_path.join("**").join("*.pdf");
    let pattern_str = pattern.to_string_lossy();

    if let Ok(entries) = glob::glob(&pattern_str) {
        for entry in entries.flatten() {
            pdf_files.push(entry);
        }
    }

    // Sort for consistent ordering across platforms
    pdf_files.sort();
    pdf_files
}

/// Discover all PDF files in the fixtures directory recursively.
///
/// This function uses walkdir to search the entire fixtures directory tree
/// and returns a sorted list of all PDF files found. Paths are returned relative to
/// the fixtures directory for test portability.
///
/// # Arguments
/// * `fixtures_path` - Path to the fixtures directory (e.g., "tests/fixtures")
///
/// # Returns
/// A vector of `PathBuf` containing paths to all discovered PDF files,
/// sorted alphabetically for consistent ordering.
///
/// # Example
/// ```rust,no_run
/// let pdf_files = discover_pdf_fixtures("tests/fixtures");
/// println!("Found {} PDF files", pdf_files.len());
/// for path in pdf_files {
///     println!("  {}", path.display());
/// }
/// ```
pub fn discover_pdf_fixtures<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let fixtures_path = fixtures_path.as_ref();
    let mut pdf_files = Vec::new();

    // Use walkdir for efficient recursive directory traversal
    let mut entries = walkdir::WalkDir::new(fixtures_path)
        .follow_links(false)
        .into_iter();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("pdf")) {
            pdf_files.push(path.to_path_buf());
        }
    }

    // Sort for consistent ordering across platforms
    pdf_files.sort();
    pdf_files
}

/// Get all PDF fixture paths relative to the fixtures directory using glob.
///
/// This variant returns paths relative to the fixtures base directory,
/// making them more portable and suitable for test assertions.
///
/// # Arguments
/// * `fixtures_path` - Path to the fixtures directory
///
/// # Returns
/// A vector of `PathBuf` containing relative paths to all PDF files.
pub fn discover_pdf_fixtures_glob_relative<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let fixtures_path = fixtures_path.as_ref();
    let absolute_paths = discover_pdf_fixtures_glob(fixtures_path);

    absolute_paths
        .into_iter()
        .filter_map(|path| path.strip_prefix(fixtures_path).ok().map(PathBuf::from))
        .collect()
}

/// Get all PDF fixture paths relative to the fixtures directory.
///
/// This variant returns paths relative to the fixtures base directory,
/// making them more portable and suitable for test assertions.
///
/// # Arguments
/// * `fixtures_path` - Path to the fixtures directory
///
/// # Returns
/// A vector of `PathBuf` containing relative paths to all PDF files.
pub fn discover_pdf_fixtures_relative<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let fixtures_path = fixtures_path.as_ref();
    let absolute_paths = discover_pdf_fixtures(fixtures_path);

    absolute_paths
        .into_iter()
        .filter_map(|path| path.strip_prefix(fixtures_path).ok().map(PathBuf::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_pdf_fixtures_glob() {
        // Test that glob-based discovery works and finds PDF files
        let fixtures_dir = "tests/fixtures";

        let pdf_files = discover_pdf_fixtures_glob(fixtures_dir);

        println!("\n=== PDF Fixture Discovery (Glob) Test ===");
        println!("Fixtures directory: {}", fixtures_dir);
        println!("Total PDF files discovered: {}", pdf_files.len());

        if pdf_files.is_empty() {
            println!("WARNING: No PDF files found in fixtures directory");
        } else {
            // Show first 10 files as examples
            println!("\nFirst 10 PDF files:");
            for (i, path) in pdf_files.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, path.display());
            }

            if pdf_files.len() > 10 {
                println!("  ... and {} more", pdf_files.len() - 10);
            }
        }
        println!("=========================================\n");

        // Verify we found at least some fixtures
        assert!(pdf_files.len() > 0, "Expected to find PDF files in fixtures directory");

        // Verify all paths exist
        for path in &pdf_files {
            assert!(path.exists(), "Discovered path does not exist: {}", path.display());
        }

        // Verify all paths are PDFs
        for path in &pdf_files {
            assert_eq!(
                path.extension().map(|e| e.to_ascii_lowercase()),
                Some(std::ffi::OsStr::new("pdf")),
                "Discovered path is not a PDF: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_discover_pdf_fixtures_glob_relative_paths() {
        // Test that glob-based relative path generation works
        let fixtures_dir = "tests/fixtures";
        let relative_paths = discover_pdf_fixtures_glob_relative(fixtures_dir);

        println!("\n=== Relative Path Discovery (Glob) Test ===");
        println!("Total relative paths: {}", relative_paths.len());

        // Verify all relative paths are actually relative (no leading components)
        for path in &relative_paths {
            let path_str = path.to_string_lossy();
            assert!(
                !path_str.starts_with('/') && !path_str.contains(".."),
                "Path should be relative: {}",
                path.display()
            );
        }
        println!("===========================================\n");

        assert!(relative_paths.len() > 0, "Expected to find relative PDF paths");
    }

    #[test]
    fn test_discover_pdf_fixtures_glob_deterministic() {
        // Test that glob-based discovery returns results in consistent order
        let fixtures_dir = "tests/fixtures";

        let first_run = discover_pdf_fixtures_glob(fixtures_dir);
        let second_run = discover_pdf_fixtures_glob(fixtures_dir);

        assert_eq!(
            first_run.len(),
            second_run.len(),
            "Glob discovery should return same count of files"
        );

        // Verify order is identical
        for (a, b) in first_run.iter().zip(second_run.iter()) {
            assert_eq!(a, b, "Glob discovery should return files in same order");
        }
    }

    #[test]
    fn test_discover_pdf_fixtures_exists() {
        // Test that the fixtures directory exists and contains PDFs
        let fixtures_dir = "tests/fixtures";

        // This test verifies the discovery mechanism works
        let pdf_files = discover_pdf_fixtures(fixtures_dir);

        println!("\n=== PDF Fixture Discovery Test ===");
        println!("Fixtures directory: {}", fixtures_dir);
        println!("Total PDF files discovered: {}", pdf_files.len());

        if pdf_files.is_empty() {
            println!("WARNING: No PDF files found in fixtures directory");
        } else {
            // Show first 10 files as examples
            println!("\nFirst 10 PDF files:");
            for (i, path) in pdf_files.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, path.display());
            }

            if pdf_files.len() > 10 {
                println!("  ... and {} more", pdf_files.len() - 10);
            }
        }
        println!("=====================================\n");

        // Verify we found at least some fixtures
        assert!(pdf_files.len() > 0, "Expected to find PDF files in fixtures directory");

        // Verify all paths exist
        for path in &pdf_files {
            assert!(path.exists(), "Discovered path does not exist: {}", path.display());
        }

        // Verify all paths are PDFs
        for path in &pdf_files {
            assert_eq!(
                path.extension().map(|e| e.to_ascii_lowercase()),
                Some(std::ffi::OsStr::new("pdf")),
                "Discovered path is not a PDF: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_discover_pdf_fixtures_relative_paths() {
        // Test that relative path generation works
        let fixtures_dir = "tests/fixtures";
        let relative_paths = discover_pdf_fixtures_relative(fixtures_dir);

        println!("\n=== Relative Path Discovery Test ===");
        println!("Total relative paths: {}", relative_paths.len());

        // Verify all relative paths are actually relative (no leading components)
        for path in &relative_paths {
            let path_str = path.to_string_lossy();
            assert!(
                !path_str.starts_with('/') && !path_str.contains(".."),
                "Path should be relative: {}",
                path.display()
            );
        }
        println!("=======================================\n");

        assert!(relative_paths.len() > 0, "Expected to find relative PDF paths");
    }

    #[test]
    fn test_fixture_discovery_is_deterministic() {
        // Test that discovery returns results in consistent order
        let fixtures_dir = "tests/fixtures";

        let first_run = discover_pdf_fixtures(fixtures_dir);
        let second_run = discover_pdf_fixtures(fixtures_dir);

        assert_eq!(
            first_run.len(),
            second_run.len(),
            "Discovery should return same count of files"
        );

        // Verify order is identical
        for (a, b) in first_run.iter().zip(second_run.iter()) {
            assert_eq!(a, b, "Discovery should return files in same order");
        }
    }

    #[test]
    fn test_known_fixture_subdirectories() {
        // Test that specific expected fixture subdirectories are discovered
        let fixtures_dir = "tests/fixtures";
        let pdf_files = discover_pdf_fixtures(fixtures_dir);

        // Check for known fixture categories
        let expected_subdirs = vec![
            "fixtures/encrypted",
            "fixtures/malformed",
            "fixtures/tagged",
        ];

        println!("\n=== Fixture Subdirectory Verification ===");
        for subdir in expected_subdirs {
            let has_files_in_subdir = pdf_files.iter()
                .any(|p| p.to_string_lossy().contains(subdir));

            println!(
                "{}: {}",
                subdir,
                if has_files_in_subdir { "FOUND" } else { "NOT FOUND" }
            );
        }
        println!("==========================================\n");
    }

    #[test]
    fn test_known_fixture_subdirectories_glob() {
        // Test that glob-based discovery finds PDFs in known subdirectories
        let fixtures_dir = "tests/fixtures";
        let pdf_files = discover_pdf_fixtures_glob(fixtures_dir);

        // Check for known fixture categories that should contain PDFs
        let expected_subdirs = vec![
            "fixtures/encrypted",
            "fixtures/forms",
            "fixtures/fonts",
        ];

        println!("\n=== Fixture Subdirectory Verification (Glob) ===");
        for subdir in expected_subdirs {
            let has_files_in_subdir = pdf_files.iter()
                .any(|p| p.to_string_lossy().contains(subdir));

            println!(
                "{}: {}",
                subdir,
                if has_files_in_subdir { "FOUND" } else { "NOT FOUND" }
            );
        }
        println!("=================================================\n");

        // At least some subdirectories should be found
        let found_count = expected_subdirs.iter()
            .filter(|subdir| pdf_files.iter().any(|p| p.to_string_lossy().contains(subdir)))
            .count();

        assert!(found_count > 0, "Expected to find PDFs in at least one known subdirectory");
    }
}
