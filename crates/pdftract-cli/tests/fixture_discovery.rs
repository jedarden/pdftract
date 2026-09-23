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
//!
//! # Shared-module split
//!
//! The reusable, fallible result API (`FixtureInfo`, `FixtureDiscoveryError`,
//! `discover_all_fixture_infos_result`, `discover_fixture_infos_result_in`)
//! and its supporting implementation now live in the shared test-support
//! module `tests/common/fixture_discovery.rs` and are re-exported below, so
//! this file's own suite and the sibling binaries that include it as a module
//! (`forms_integration`, `cli_invocation_fixtures`) keep compiling unchanged.
//! The infallible, walkdir-based helpers remain here.

// `#[path]` pins the include to tests/common/ in every inclusion context:
// this file is both a standalone test binary (crate root — plain `mod
// common;` would look in tests/) and a module included by sibling binaries
// via `mod fixture_discovery;` (where implicit lookup would look in
// tests/fixture_discovery/). The attribute resolves relative to the
// directory of THIS file, so both contexts find the same shared module.
#[path = "common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// The reusable result API moved to the shared test-support module (included
// via `mod common` below — its implementation now lives in
// tests/common/fixture_discovery.rs) and is re-exported `pub` here so this
// file's own suite AND the sibling binaries that include this file as a
// module (`forms_integration`, `cli_invocation_fixtures`, via
// `mod fixture_discovery;`) keep their existing `fixture_discovery::` import
// paths working unchanged.
pub use common::fixture_discovery::{
    ancestor_is_symlink, discover_all_fixture_infos_result, discover_fixture_infos_result_in,
    fixtures_root, normalize_path, FixtureDiscoveryError, FixtureInfo,
};

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
/// A sorted `Vec<PathBuf>` containing normalized, absolute paths to discovered PDF files.
pub fn discover_fixtures_flat<P: AsRef<Path>>(dir_path: P) -> Vec<PathBuf> {
    let mut pdf_files = Vec::new();
    let dir = dir_path.as_ref();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                pdf_files.push(normalize_path(&path));
            }
        }
    }

    pdf_files.sort();
    pdf_files
}

/// Discover PDFs in a specific directory recursively (internal function).
///
/// # Arguments
///
/// * `dir_path` - Path to the directory to search recursively
///
/// # Returns
///
/// A sorted `Vec<PathBuf>` containing normalized, absolute paths to all discovered PDF files.
pub fn discover_fixtures_in_dir<P: AsRef<Path>>(dir_path: P) -> Vec<PathBuf> {
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
        .map(|e| normalize_path(e.path()));

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

// ===========================================================================
// Fixture metadata — infallible wrappers
// ===========================================================================
//
// `FixtureInfo` (the metadata record these wrappers lift each path into,
// together with its `from_path`/`new` constructors and the
// `fixture_description` category derivation) now lives in the shared
// test-support module `common::fixture_discovery` and is re-exported above.

/// Discover all PDF fixtures and return them as rich [`FixtureInfo`] records.
///
/// This is the metadata-bearing counterpart to [`discover_all_fixtures`]: it
/// walks the same `tests/fixtures/` tree and lifts each discovered path into a
/// [`FixtureInfo`] (deriving `name` and `description`).
///
/// # Returns
///
/// A `Vec<FixtureInfo>` ordered identically to [`discover_all_fixtures`]
/// (sorted by path).
///
/// # Example
///
/// ```rust,ignore
/// let infos = discover_all_fixture_infos();
/// println!("Discovered {} fixtures", infos.len());
/// for info in &infos {
///     println!("- {info}");
/// }
/// ```
pub fn discover_all_fixture_infos() -> Vec<FixtureInfo> {
    discover_all_fixtures()
        .into_iter()
        .map(FixtureInfo::from_path)
        .collect()
}

/// Discover PDF fixtures in a category and return them as [`FixtureInfo`]
/// records.
///
/// Metadata-bearing counterpart to [`discover_fixtures_by_category`].
///
/// # Arguments
///
/// * `category` - The category name (e.g. `"malformed"`, `"encrypted"`)
///
/// # Returns
///
/// A `Vec<FixtureInfo>` for every PDF in the category, sorted by path.
pub fn discover_fixture_infos_by_category(category: &str) -> Vec<FixtureInfo> {
    discover_fixtures_by_category(category)
        .into_iter()
        .map(FixtureInfo::from_path)
        .collect()
}

// ===========================================================================
// Fallible, glob-based discovery — discover_*_result (in the shared module)
// ===========================================================================
//
// The `discover_all_fixture_infos` family above is *infallible*: it returns a
// plain `Vec<FixtureInfo>`, folding "the fixtures directory is missing" and
// "the directory exists but holds no PDFs" into an indistinguishable empty
// vector. The `Result`-returning counterparts — `discover_all_fixture_infos_result`,
// `discover_fixture_infos_result_in`, the `FixtureDiscoveryError` enum, and the
// `ancestor_is_symlink` directory-symlink guard — live in the shared
// test-support module `common::fixture_discovery` (re-exported above), where
// cross-binary consumers can reach them via
// `mod common; use common::fixture_discovery::…;`.

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

    #[test]
    fn test_normalized_paths_no_relative_components() {
        let fixtures = discover_all_fixtures();

        // Verify no paths contain . or .. components
        for path in &fixtures {
            let path_str = path.to_string_lossy();
            assert!(
                !path_str.contains("/./") && !path_str.contains("/../"),
                "Path should not contain relative components: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_normalized_paths_work_in_test_context() {
        let fixtures = discover_all_fixtures();

        // Verify all normalized paths exist and are readable
        for path in &fixtures {
            assert!(path.exists(), "Normalized path should exist: {:?}", path);
            assert!(path.is_file(), "Normalized path should be a file: {:?}", path);
        }
    }

    #[test]
    fn test_normalized_paths_are_consistent() {
        // Test that calling discovery multiple times returns the same normalized paths
        let fixtures1 = discover_all_fixtures();
        let fixtures2 = discover_all_fixtures();

        assert_eq!(fixtures1.len(), fixtures2.len());
        for (p1, p2) in fixtures1.iter().zip(fixtures2.iter()) {
            assert_eq!(p1, p2, "Normalized paths should be consistent across calls");
        }
    }

    #[test]
    fn test_category_discovery_returns_normalized_paths() {
        let categories = fixture_categories();

        // Test a few categories to ensure they return normalized paths
        for category in categories.iter().take(3) {
            let fixtures = discover_fixtures_by_category(category);

            for path in &fixtures {
                let path_str = path.to_string_lossy();
                assert!(
                    !path_str.contains("/./") && !path_str.contains("/../"),
                    "Category {} path should be normalized: {}",
                    category,
                    path_str
                );
                assert!(path.exists(), "Category {} path should exist: {:?}", category, path);
            }
        }
    }

    // =======================================================================
    // FixtureInfo — metadata-bearing fixture enumeration
    // =======================================================================

    #[test]
    fn test_fixture_info_new_explicit() {
        let info = FixtureInfo::new("/abs/path/to/01.pdf", "01", "malformed fixture");

        assert_eq!(info.path, PathBuf::from("/abs/path/to/01.pdf"));
        assert_eq!(info.name, "01");
        assert_eq!(info.description, "malformed fixture");
    }

    #[test]
    fn test_fixture_info_from_path_derives_name_and_description() {
        let discovered = discover_all_fixtures();
        assert!(!discovered.is_empty(), "Need at least one fixture to test");
        // Use a nested fixture (under a category dir) so the category-derived
        // description is exercised. Fall back to the first fixture if none are
        // nested.
        let sample = discovered
            .iter()
            .find(|p| {
                p.strip_prefix(fixtures_root())
                    .ok()
                    .and_then(|r| r.parent())
                    .map(|par| !par.as_os_str().is_empty())
                    .unwrap_or(false)
            })
            .or(discovered.first())
            .unwrap()
            .clone();

        let info = FixtureInfo::from_path(&sample);

        // path is preserved unchanged
        assert_eq!(info.path, sample);
        // name is the file stem (no extension)
        let expected_name = sample
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        assert_eq!(info.name, expected_name);
        // description is non-empty and ends in "fixture"
        assert!(!info.description.is_empty());
        assert!(
            info.description.ends_with("fixture"),
            "description should end with 'fixture': {}",
            info.description
        );
    }

    #[test]
    fn test_fixture_info_display() {
        let info = FixtureInfo::new("/abs/path/to/01.pdf", "01", "ignored");
        let rendered = format!("{info}");

        assert_eq!(rendered, "01 (/abs/path/to/01.pdf)");
        // Display includes the name (useful for test output)
        assert!(rendered.contains("01"));
    }

    #[test]
    fn test_fixture_info_debug() {
        let info = FixtureInfo::new("/abs/path/to/01.pdf", "01", "malformed fixture");
        let debug = format!("{info:?}");

        // Derived Debug includes the struct name and all three fields
        assert!(debug.contains("FixtureInfo"), "Debug should name the struct: {debug}");
        assert!(debug.contains("path"), "Debug should show the path field: {debug}");
        assert!(debug.contains("name"), "Debug should show the name field: {debug}");
        assert!(debug.contains("description"), "Debug should show the description field: {debug}");
    }

    #[test]
    fn test_fixture_info_clone_and_equality() {
        let a = FixtureInfo::new("/abs/01.pdf", "01", "malformed fixture");
        let a_clone = a.clone();
        let b = FixtureInfo::new("/abs/02.pdf", "02", "encrypted fixture");

        // Clone is equal to the original
        assert_eq!(a, a_clone);
        // Different fixtures are not equal
        assert_ne!(a, b);
    }

    #[test]
    fn test_fixture_info_serialization_roundtrip() {
        let original = FixtureInfo::new("/abs/path/to/01.pdf", "01", "malformed fixture");

        // Serialize to JSON and back, verifying the round-trip is lossless.
        let json = serde_json::to_string(&original).expect("serialize FixtureInfo");
        let restored: FixtureInfo = serde_json::from_str(&json).expect("deserialize FixtureInfo");

        assert_eq!(original, restored);
        // The JSON carries all three fields
        assert!(json.contains("\"path\""));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"description\""));
    }

    #[test]
    fn test_discover_all_fixture_infos() {
        let paths = discover_all_fixtures();
        let infos = discover_all_fixture_infos();

        // Same count and ordering as the path-based discovery
        assert_eq!(infos.len(), paths.len());
        for (info, path) in infos.iter().zip(paths.iter()) {
            assert_eq!(info.path, *path);
            assert!(!info.name.is_empty(), "name must not be empty");
            assert!(!info.description.is_empty(), "description must not be empty");
        }
    }

    #[test]
    fn test_discover_fixture_infos_by_category() {
        let malformed_paths = discover_fixtures_by_category("malformed");
        let malformed_infos = discover_fixture_infos_by_category("malformed");

        assert_eq!(malformed_infos.len(), malformed_paths.len());
        for info in &malformed_infos {
            assert!(
                info.description.contains("malformed"),
                "category fixtures should be described as malformed: {}",
                info.description
            );
        }
    }

    // =======================================================================
    // Fallible (Result) glob-based discovery — discover_*_result
    // =======================================================================

    /// RAII temp directory, removed on drop, for exercising the empty-directory
    /// failure mode without touching the real fixtures tree.
    struct TempDir(PathBuf);
    impl TempDir {
        fn create() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir()
                .join(format!("pdftract-bf-29u9w-{}-{}", std::process::id(), n));
            std::fs::create_dir(&path).expect("create temp dir");
            TempDir(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_discover_all_fixture_infos_result_ok() {
        let infos = discover_all_fixture_infos_result()
            .expect("default fixtures root must exist and contain PDFs");

        assert!(!infos.is_empty(), "should discover real fixtures");

        // Glob discovers by filename, so it surfaces symlinked PDF fixtures
        // (e.g. `profiles/invoice/07.pdf` -> `classifier/invoice/07.pdf`) that
        // the walkdir-based [`discover_all_fixtures`] *excludes*: walkdir
        // filters on `file_type().is_file()`, and a symlink reports
        // `is_symlink()` rather than `is_file()`. The two counts therefore
        // differ by exactly the number of symlinked `.pdf` files in the tree.
        //
        // Asserting that precise relationship — rather than naive equality —
        // verifies glob returns every real file walkdir finds *plus* every
        // symlinked fixture. It also guards the directory-symlink filter: the
        // self-referential `classifier/scientific_paper/scientific_paper`
        // directory symlink would, unfiltered, inflate the count into the
        // thousands (see [`ancestor_is_symlink`]); any phantom loop-descended
        // path would blow this bound past `walkdir_count + symlinked_pdf_count`.
        let walkdir_count = discover_all_fixtures().len();
        let symlinked_pdf_count = WalkDir::new(fixtures_root())
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_symlink()
                    && e.path().extension().map(|ext| ext == "pdf").unwrap_or(false)
            })
            .count();
        println!(
            "glob result count: {}; walkdir real-file count: {}; symlinked pdfs: {}",
            infos.len(),
            walkdir_count,
            symlinked_pdf_count
        );
        assert_eq!(
            infos.len(),
            walkdir_count + symlinked_pdf_count,
            "glob count must equal walkdir real-file count plus symlinked-PDF count \
             (glob includes symlinked fixtures that walkdir's is_file() filter drops; \
             a larger count means the directory-symlink filter regressed)"
        );

        // No duplicate paths (dedup correctness).
        let mut seen = std::collections::HashSet::new();
        for info in &infos {
            assert!(
                seen.insert(&info.path),
                "duplicate fixture path: {}",
                info.path.display()
            );
        }
    }

    #[test]
    fn test_discover_fixture_infos_result_sorted_and_populated() {
        let infos = discover_all_fixture_infos_result().expect("fixtures");

        // Sorted ascending by path.
        for w in infos.windows(2) {
            assert!(w[0].path <= w[1].path, "results must be sorted by path");
        }
        // Spot-check the structured fields on a sample.
        for info in infos.iter().take(25) {
            assert!(info.path.is_absolute(), "path must be absolute: {info}");
            assert_eq!(
                info.path.extension().and_then(|s| s.to_str()),
                Some("pdf"),
                "path must end in .pdf: {info}",
            );
            assert!(info.path.exists(), "path must exist: {info}");
            assert!(!info.name.is_empty(), "name must be non-empty: {info}");
            assert!(
                !info.description.is_empty(),
                "description must be non-empty: {info}",
            );
        }
    }

    #[test]
    fn test_discover_fixture_infos_result_missing_root() {
        let bogus = PathBuf::from("/this/path/should/not/exist/pdftract-bf-29u9w");
        assert!(!bogus.exists(), "precondition: bogus path must not exist");

        match discover_fixture_infos_result_in(&bogus) {
            Err(FixtureDiscoveryError::RootMissing(p)) => {
                assert_eq!(p, bogus, "RootMissing should carry the requested path");
            }
            other => panic!("expected RootMissing, got {other:?}"),
        }
    }

    #[test]
    fn test_discover_fixture_infos_result_empty_dir_is_no_fixtures() {
        let tmp = TempDir::create();
        assert!(tmp.path().exists(), "precondition: temp dir should exist");

        match discover_fixture_infos_result_in(tmp.path()) {
            Err(FixtureDiscoveryError::NoFixtures(_)) => {}
            other => panic!("expected NoFixtures for empty dir, got {other:?}"),
        }
        // Discovery must not have side effects on the directory it scanned.
        assert!(
            tmp.path().exists(),
            "empty dir should still exist after discovery"
        );
    }

    #[test]
    fn test_fixture_discovery_error_is_std_error() {
        // FixtureDiscoveryError must implement std::error::Error (robust,
        // chainable) — verified by the bound on `accepts`.
        fn accepts<E: std::error::Error>(_: &E) {}
        let err = discover_fixture_infos_result_in(&PathBuf::from(
            "/nonexistent/pdftract-bf-29u9w",
        ))
        .unwrap_err();
        accepts(&err);

        // Display is human-readable and explains the missing path.
        let msg = format!("{err}");
        assert!(msg.contains("does not exist"), "Display should explain: {msg}");

        // source() is None for RootMissing (no inner cause to chain).
        assert!(
            std::error::Error::source(&err).is_none(),
            "RootMissing should have no source",
        );
    }

    // =======================================================================
    // Error-path coverage — every driveable FixtureDiscoveryError variant
    // (bf-2y1bwm). RootMissing and NoFixtures are driven above; the Pattern
    // and Glob variants are driven here, each through the public discovery
    // API and once more via direct construction for the Display/source
    // contract.
    // =======================================================================

    /// RAII guard restoring mode 0o755 on drop, so an unreadable fixture
    /// directory never wedges [`TempDir`]'s own cleanup (remove_dir_all
    /// cannot descend a 000-mode directory, even on panic unwinding).
    #[cfg(unix)]
    struct RestorePerms(PathBuf);
    #[cfg(unix)]
    impl Drop for RestorePerms {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }

    /// The `Pattern` variant, driven through the public API: a directory
    /// whose name contains an unterminated `[` makes the internally computed
    /// `<root>/**/*.pdf` pattern unparsable, so glob's own parser — not any
    /// filesystem state — produces the error. Asserts the variant payload
    /// (glob's position + message) and the Display/source chaining.
    #[test]
    fn test_discover_fixture_infos_result_invalid_pattern_is_pattern_error() {
        let tmp = TempDir::create();
        let bad = tmp.path().join("pdftract-bf-2y1bwm-unclosed-[range");
        std::fs::create_dir(&bad).expect("create directory with '[' in its name");
        let canonical = bad.canonicalize().expect("canonicalize the bad root");

        // Precondition: the exact pattern the discovery function computes must
        // fail glob's parser. A stray ']' in the ambient temp path would close
        // the range and turn this into a valid (matching-nothing) pattern, so
        // fail loudly rather than pass vacuously if the environment is odd.
        let pattern = format!("{}/**/*.pdf", canonical.display());
        assert!(
            !canonical.to_string_lossy().contains(']'),
            "temp path must not contain ']' or the range stays terminated: {canonical:?}"
        );
        assert!(
            glob::Pattern::new(&pattern).is_err(),
            "precondition: computed pattern must be unparsable: {pattern}"
        );

        let err = discover_fixture_infos_result_in(&canonical)
            .err()
            .expect("discovery must fail on an unparsable pattern");
        let e = match &err {
            FixtureDiscoveryError::Pattern(e) => e,
            other => panic!("expected Pattern, got {other:?}"),
        };
        // glob reports the offending position and reason
        assert!(
            e.msg.contains("range pattern"),
            "expected glob's range-pattern error, got: {}",
            e.msg
        );
        // Display chains the inner parser error
        let msg = format!("{err}");
        assert!(
            msg.contains("invalid glob pattern"),
            "Display should explain the variant: {msg}"
        );
        assert!(
            msg.contains(&e.to_string()),
            "Display must embed the inner PatternError: {msg}"
        );
        // source() chains to the same error, downcastable
        let src = std::error::Error::source(&err).expect("Pattern must chain a source");
        let inner = src
            .downcast_ref::<glob::PatternError>()
            .expect("source must be the glob PatternError");
        assert_eq!((inner.pos, inner.msg), (e.pos, e.msg));
    }

    /// The `Glob` variant, driven through the public API: an unreadable
    /// directory under the root surfaces glob's per-entry read error even
    /// though a sibling PDF is perfectly readable — discovery must report the
    /// hard failure rather than silently skip the entry (a silent skip would
    /// collapse into a misleading success or NoFixtures). Skips when the
    /// process cannot make a directory unreadable (a privileged runner).
    #[cfg(unix)]
    #[test]
    fn test_discover_fixture_infos_result_unreadable_dir_is_glob_error() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::create();
        // Readable sibling proves the error is not merely "nothing found".
        std::fs::write(tmp.path().join("visible.pdf"), b"%PDF-1.4\n").expect("write visible pdf");
        let locked = tmp.path().join("locked");
        std::fs::create_dir(&locked).expect("create locked dir");

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
            .expect("chmod 000 the locked dir");
        // Driveability probe ("where feasible"): root-like runners can read
        // through any mode bits, so the error path cannot be exercised there.
        if std::fs::read_dir(&locked).is_ok() {
            eprintln!("skipping: this process can read a 000-mode directory (privileged runner)");
            return;
        }
        let _restore = RestorePerms(locked.clone());

        let err = discover_fixture_infos_result_in(tmp.path())
            .err()
            .expect("discovery must fail while a directory is unreadable");
        match &err {
            FixtureDiscoveryError::Glob { entry, source } => {
                assert_eq!(entry, &locked, "Glob must name the unreadable entry");
                assert_eq!(
                    source.kind(),
                    std::io::ErrorKind::PermissionDenied,
                    "underlying error should be the EACCES from read_dir: {source}"
                );
            }
            other => panic!("expected Glob, got {other:?}"),
        }
        // Display chains the entry and the underlying I/O error
        let msg = format!("{err}");
        assert!(
            msg.contains("failed to read"),
            "Display should explain the variant: {msg}"
        );
        assert!(
            msg.contains("locked"),
            "Display should name the entry: {msg}"
        );
        // source() chains to the io::Error, downcastable
        let src = std::error::Error::source(&err).expect("Glob must chain a source");
        let io_err = src
            .downcast_ref::<std::io::Error>()
            .expect("source must be the io::Error");
        assert_eq!(io_err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    /// `Pattern` Display/source contract, constructed directly from a real
    /// (not synthetic) `glob::PatternError` — glob's own parser rejecting a
    /// bare `[` — and via the `From` conversion the discovery function relies
    /// on for its `?` propagation.
    #[test]
    fn test_fixture_discovery_error_pattern_display_and_source() {
        let pe = glob::Pattern::new("[").expect_err("'[' alone must fail to parse");
        let (pos, msg) = (pe.pos, pe.msg);

        // The From impl the `?` in discover_fixture_infos_result_in uses.
        let via_from = FixtureDiscoveryError::from(glob::Pattern::new("[").unwrap_err());
        assert!(
            matches!(via_from, FixtureDiscoveryError::Pattern(_)),
            "From<PatternError> must produce the Pattern variant: {via_from:?}"
        );

        let err = FixtureDiscoveryError::Pattern(pe);
        let rendered = format!("{err}");
        assert!(
            rendered.starts_with("invalid glob pattern:"),
            "Display should lead with the variant explanation: {rendered}"
        );
        assert!(
            rendered.contains(msg),
            "Display must embed the glob message: {rendered}"
        );
        assert!(
            rendered.contains(&pos.to_string()),
            "Display must embed the glob error position: {rendered}"
        );

        let src = std::error::Error::source(&err).expect("Pattern must chain a source");
        let inner = src
            .downcast_ref::<glob::PatternError>()
            .expect("source must downcast to glob::PatternError");
        assert_eq!(
            (inner.pos, inner.msg),
            (pos, msg),
            "source is the same parser error"
        );
    }

    /// `Glob` Display/source contract, constructed directly: Display names
    /// the entry and embeds the underlying I/O error, and `source()` hands
    /// back the same `io::Error` for `Error::source`-chain walkers.
    #[test]
    fn test_fixture_discovery_error_glob_display_and_source() {
        let entry = PathBuf::from("/tmp/pdftract-bf-2y1bwm/locked");
        let err = FixtureDiscoveryError::Glob {
            entry: entry.clone(),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        };

        let rendered = format!("{err}");
        assert!(
            rendered.starts_with("failed to read "),
            "Display should lead with the variant explanation: {rendered}"
        );
        assert!(
            rendered.contains(entry.to_str().unwrap()),
            "Display must name the failing entry: {rendered}"
        );
        assert!(
            rendered.to_lowercase().contains("permission denied"),
            "Display must embed the underlying I/O error: {rendered}"
        );

        let src = std::error::Error::source(&err).expect("Glob must chain a source");
        let io_err = src
            .downcast_ref::<std::io::Error>()
            .expect("source must downcast to std::io::Error");
        assert_eq!(io_err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    // =======================================================================
    // Shared-module import path — the result API lives in
    // tests/common/fixture_discovery.rs; these tests pin both that route and
    // the symlink guard that must survive the move intact.
    // =======================================================================

    /// The result API is owned by the shared test-support module; this file
    /// only re-exports it. Exercises the API through the fully-qualified
    /// shared-module path (`common::fixture_discovery::…`) — the exact route
    /// a new cross-binary consumer takes after `mod common;` — and asserts
    /// the types reached that way are the very items re-exported here.
    #[test]
    fn test_result_api_importable_through_shared_module() {
        let via_shared =
            super::common::fixture_discovery::discover_all_fixture_infos_result()
                .expect("shared-module import path must reach the real discovery API");
        assert!(
            !via_shared.is_empty(),
            "shared-module path must see the real fixture tree"
        );

        // Type identity: the shared module's FixtureInfo IS the re-exported
        // FixtureInfo (a move must not fork the type across module paths).
        let annotated: FixtureInfo =
            super::common::fixture_discovery::FixtureInfo::new("/abs/01.pdf", "01", "fixture");
        assert_eq!(annotated, FixtureInfo::new("/abs/01.pdf", "01", "fixture"));

        // The fallible worker drives the failure paths through the same route.
        let bogus = Path::new("/definitely/not/here/pdftract-66c60ab4");
        match super::common::fixture_discovery::discover_fixture_infos_result_in(bogus) {
            Err(super::common::fixture_discovery::FixtureDiscoveryError::RootMissing(p)) => {
                assert_eq!(p, bogus.to_path_buf());
            }
            other => panic!("expected RootMissing via shared path, got {other:?}"),
        }
    }

    /// The `ancestor_is_symlink` guard must keep flagging paths descended
    /// through a symlinked *directory* while retaining symlinked *file*
    /// leaves — the exact behavior
    /// [`test_discover_all_fixture_infos_result_ok`] relies on for its
    /// walkdir-count + symlinked-pdf-count assertion. Calls the shared
    /// module's (now public) helper directly against a controlled tree.
    #[cfg(unix)]
    #[test]
    fn test_ancestor_is_symlink_flags_dir_descent_not_file_leaf() {
        let tmp = TempDir::create();
        let real = tmp.path().join("real");
        std::fs::create_dir(&real).expect("create real dir");
        std::fs::write(real.join("inner.pdf"), b"%PDF-1.4\n").expect("write inner pdf");
        std::os::unix::fs::symlink(real.join("inner.pdf"), tmp.path().join("file_link.pdf"))
            .expect("symlink file leaf");
        std::os::unix::fs::symlink(&real, tmp.path().join("dir_link")).expect("symlink dir");

        assert!(
            !ancestor_is_symlink(&real.join("inner.pdf")),
            "a plain path under a real directory must not be flagged"
        );
        assert!(
            !ancestor_is_symlink(&tmp.path().join("file_link.pdf")),
            "a symlinked *file* leaf is not a symlinked ancestor and must be kept"
        );
        assert!(
            ancestor_is_symlink(&tmp.path().join("dir_link").join("inner.pdf")),
            "a path descended through a symlinked *directory* must be flagged"
        );
    }
}
