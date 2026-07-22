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

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Normalize a path to an absolute, canonical form.
///
/// This function resolves any `.` or `..` components in the path and returns
/// a clean, absolute path suitable for reliable test invocation.
///
/// # Arguments
///
/// * `path` - The path to normalize
///
/// # Returns
///
/// A normalized `PathBuf` with all relative components resolved.
fn normalize_path(path: &Path) -> PathBuf {
    // Try to canonicalize the path first (resolves symlinks and .)
    // If that fails (e.g., path doesn't exist), fall back to component-based normalization
    match path.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => {
            // Fallback: normalize components without requiring existence
            let mut result = PathBuf::new();

            // Start with absolute paths, otherwise use current directory
            if path.is_absolute() {
                for component in path.components() {
                    match component {
                        std::path::Component::Normal(_) => result.push(component),
                        std::path::Component::ParentDir => {
                            result.pop();
                        }
                        std::path::Component::CurDir => {
                            // Skip .
                        }
                        std::path::Component::RootDir => {
                            result.push(component);
                        }
                        std::path::Component::Prefix(_) => {
                            result.push(component);
                        }
                    }
                }
            } else {
                // For relative paths, resolve against current directory
                if let Ok(current_dir) = std::env::current_dir() {
                    result.push(current_dir);
                    for component in path.components() {
                        match component {
                            std::path::Component::Normal(_) => result.push(component),
                            std::path::Component::ParentDir => {
                                result.pop();
                            }
                            std::path::Component::CurDir => {
                                // Skip .
                            }
                            std::path::Component::RootDir => {
                                result.push(component);
                            }
                            std::path::Component::Prefix(_) => {
                                result.push(component);
                            }
                        }
                    }
                } else {
                    // Last resort: return original path
                    return path.to_path_buf();
                }
            }

            result
        }
    }
}

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
// Fixture metadata — FixtureInfo
// ===========================================================================
//
// The discovery functions above return bare `PathBuf`s — enough to answer
// "where is this fixture?". `FixtureInfo` is the richer, test-accessible
// record that CLI invocation tests need when they enumerate fixtures: it
// pairs each path with a short `name` (for compact, human-readable test
// output) and a `description` (prose identifying what the fixture represents,
// derived from its category). It is the structured enumeration format called
// for by the parent task "Discover and enumerate test fixtures for CLI
// invocation".

/// Metadata describing a single discovered PDF test fixture.
///
/// `FixtureInfo` pairs a fixture's filesystem path with human-readable
/// metadata so that tests can enumerate, display, and assert on fixtures
/// without re-deriving context from raw paths each time. Where a `PathBuf`
/// answers "where is the fixture?", a `FixtureInfo` also answers "what is
/// it?" (via [`name`](Self::name)) and "what does it represent?" (via
/// [`description`](Self::description)).
///
/// Instances are cheap to clone and compare (all fields are owned), and the
/// struct is `Serialize`/`Deserialize` so a discovered fixture set can be
/// serialized into a snapshot or assertion fixture.
///
/// # Fields
///
/// - [`path`](Self::path) — absolute, normalized filesystem path to the PDF.
/// - [`name`](Self::name) — short, human-readable fixture identifier
///   (typically the PDF file stem, e.g. `"01"` for `01.pdf`).
/// - [`description`](Self::description) — free-form prose describing what the
///   fixture represents (e.g. its category or intended use).
///
/// # Example
///
/// ```rust,ignore
/// use fixture_discovery::{discover_all_fixture_infos, FixtureInfo};
///
/// let fixtures: Vec<FixtureInfo> = discover_all_fixture_infos();
/// for f in &fixtures {
///     println!("{f}"); // e.g. "01 (/abs/path/to/01.pdf)"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureInfo {
    /// Absolute, normalized filesystem path to the PDF fixture.
    pub path: PathBuf,
    /// Short, human-readable fixture identifier (typically the PDF file stem).
    pub name: String,
    /// Free-form prose describing what the fixture represents.
    pub description: String,
}

impl FixtureInfo {
    /// Construct a `FixtureInfo` from its explicit components.
    ///
    /// Use this when the caller already knows the desired `name` and
    /// `description` (e.g. when annotating a fixture from an external
    /// manifest). To derive both from a path, use [`FixtureInfo::from_path`].
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let info = FixtureInfo::new("/abs/01.pdf", "01", "malformed fixture");
    /// ```
    pub fn new<P: Into<PathBuf>, S: Into<String>>(path: P, name: S, description: S) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            description: description.into(),
        }
    }

    /// Build a `FixtureInfo` from a fixture path, deriving sensible metadata.
    ///
    /// - `name` is the PDF file stem (the filename without the `.pdf`
    ///   extension), falling back to `"unknown"` if it cannot be read.
    /// - `description` is derived from the fixture's category (the first path
    ///   component below the fixtures root) via [`fixture_description`].
    ///
    /// The `path` is stored unchanged, so callers should pass the same
    /// normalized, absolute paths produced by the `discover_*` functions.
    pub fn from_path<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let description = fixture_description(&path);
        Self { path, name, description }
    }
}

impl std::fmt::Display for FixtureInfo {
    /// Formats as `"<name> (<path>)"` — a compact, single-line rendering for
    /// human-readable test output. The full structured view (including
    /// `description`) is available via the derived [`Debug`] impl.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.path.display())
    }
}

/// Derive a human-readable description for a fixture from its category.
///
/// The category is the first path component below the fixtures root
/// (e.g. `"malformed"` for `tests/fixtures/malformed/broken.pdf`). Fixtures
/// sitting directly in the root are described as `"root-level fixture"`, and
/// paths that cannot be related to the fixtures root fall back to the generic
/// `"PDF fixture"`.
fn fixture_description(path: &Path) -> String {
    let root = fixtures_root();
    // Canonicalize the root so it matches the canonical paths produced by the
    // discover_* functions (which canonicalize via normalize_path). Without
    // this, strip_prefix compares byte-for-byte and fails: the discovered
    // path is canonical while fixtures_root() retains its `../../` components.
    // Fall back to component-normalization if canonicalization isn't possible.
    let root = match root.canonicalize() {
        Ok(canonical) => canonical,
        Err(_) => normalize_path(&root),
    };
    if let Ok(rel) = path.strip_prefix(&root) {
        // A category only exists when the fixture is nested under a
        // subdirectory (i.e. the relative path has a non-empty parent).
        if rel.parent().map(|p| !p.as_os_str().is_empty()).unwrap_or(false) {
            if let Some(std::path::Component::Normal(cat)) = rel.components().next() {
                return format!("{} fixture", cat.to_string_lossy());
            }
        }
        return "root-level fixture".to_string();
    }
    "PDF fixture".to_string()
}

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
// Fallible, glob-based discovery — discover_*_result
// ===========================================================================
//
// The `discover_all_fixture_infos` family above is *infallible*: it returns a
// plain `Vec<FixtureInfo>`, folding "the fixtures directory is missing" and
// "the directory exists but holds no PDFs" into an indistinguishable empty
// vector. The `Result`-returning functions below make those failure modes
// explicit, so a test that expects fixtures can fail loudly (with a clear
// error) instead of silently iterating over nothing.
//
// Discovery uses the `glob` crate's recursive `<root>/**/*.pdf` pattern. A
// subtlety: `glob` 0.3 follows directory symlinks with no opt-out (unlike
// `walkdir`'s `follow_links(false)`), and this fixture tree contains a
// self-referential directory symlink
// (`classifier/scientific_paper/scientific_paper` → its own parent). Unfiltered,
// `glob` descends that symlink up to its internal recursion limit and emits
// thousands of phantom duplicate paths (3353 raw entries vs. the true 1353).
// Candidates reached by descending a symlinked *directory* are therefore
// dropped via [`ancestor_is_symlink`]; symlinked *files* are kept, since they
// are distinct fixture entries. See `tests/test_glob_discovery.rs` for the
// standalone, fully documented version of the same technique.

/// Error returned by [`discover_all_fixture_infos_result`] and
/// [`discover_fixture_infos_result_in`].
///
/// Makes explicit the failure modes that the infallible
/// [`discover_all_fixtures`] / [`discover_all_fixture_infos`] helpers collapse
/// into an empty `Vec`: a missing root directory, an unreadable matched entry,
/// or a directory that exists but contains no PDFs.
#[derive(Debug)]
pub(crate) enum FixtureDiscoveryError {
    /// The fixtures root directory does not exist on disk.
    RootMissing(PathBuf),
    /// The computed glob pattern could not be parsed.
    Pattern(glob::PatternError),
    /// A filesystem error occurred while resolving a globbed entry.
    Glob {
        /// The entry whose metadata could not be read.
        entry: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The root directory exists but contains no `.pdf` fixtures.
    NoFixtures(PathBuf),
}

impl std::fmt::Display for FixtureDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootMissing(p) => {
                write!(f, "fixtures root directory does not exist: {}", p.display())
            }
            Self::Pattern(e) => write!(f, "invalid glob pattern: {e}"),
            Self::Glob { entry, source } => {
                write!(f, "failed to read {}: {source}", entry.display())
            }
            Self::NoFixtures(p) => {
                write!(f, "no PDF fixtures found under: {}", p.display())
            }
        }
    }
}

impl std::error::Error for FixtureDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pattern(e) => Some(e),
            Self::Glob { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<glob::PatternError> for FixtureDiscoveryError {
    fn from(e: glob::PatternError) -> Self {
        Self::Pattern(e)
    }
}

/// Discover every PDF fixture as structured [`FixtureInfo`] records, returning
/// a [`Result`] so real failures are not silently collapsed into an empty set.
///
/// Walks the same `tests/fixtures/` tree as [`discover_all_fixture_infos`] but
/// via the `glob` crate (`<root>/**/*.pdf`, symlink-safe — see
/// [`ancestor_is_symlink`]). Ordering matches the other discovery helpers:
/// sorted ascending by path.
///
/// # Errors
///
/// - [`FixtureDiscoveryError::RootMissing`] — [`fixtures_root`] does not exist.
/// - [`FixtureDiscoveryError::NoFixtures`] — the root exists but holds no PDFs.
/// - [`FixtureDiscoveryError::Glob`] — a matched entry could not be read.
/// - [`FixtureDiscoveryError::Pattern`] — the computed glob pattern is invalid.
///
/// # Example
///
/// ```rust,ignore
/// use fixture_discovery::discover_all_fixture_infos_result;
///
/// let fixtures = discover_all_fixture_infos_result()
///     .expect("fixtures root must exist and contain PDFs");
/// println!("{} fixtures ready for CLI invocation", fixtures.len());
/// ```
pub(crate) fn discover_all_fixture_infos_result() -> Result<Vec<FixtureInfo>, FixtureDiscoveryError> {
    discover_fixture_infos_result_in(&fixtures_root())
}

/// Discover every PDF fixture under `root` as structured [`FixtureInfo`]
/// records, with explicit error reporting.
///
/// This is the parameterized worker behind [`discover_all_fixture_infos_result`];
/// accepting an explicit `root` lets tests drive the missing-directory and
/// empty-directory failure paths without disturbing the real fixtures tree.
///
/// The root is canonicalized before globbing because `glob` matches path
/// components literally (it does not resolve `..`), and [`fixtures_root`] is
/// built from `CARGO_MANIFEST_DIR` + `"../../tests/fixtures"`. Canonicalizing is
/// also what keeps [`ancestor_is_symlink`] sound: a canonical path has no
/// symlink components, so the only symlink ancestor it can ever report is a
/// symlink *inside* the fixture tree (never a spurious one above it).
///
/// # Arguments
///
/// * `root` — directory to search recursively for `.pdf` files.
///
/// # Errors
///
/// See [`discover_all_fixture_infos_result`].
pub(crate) fn discover_fixture_infos_result_in(
    root: &Path,
) -> Result<Vec<FixtureInfo>, FixtureDiscoveryError> {
    if !root.exists() {
        return Err(FixtureDiscoveryError::RootMissing(root.to_path_buf()));
    }
    // Canonicalize: glob matches `..` literally, and fixtures_root() carries
    // `../../` components. unwrap_or_else falls back to component-normalization
    // if canonicalize fails for any reason.
    let root = root.canonicalize().unwrap_or_else(|_| normalize_path(root));
    let pattern = format!("{}/**/*.pdf", root.display());

    let mut infos: Vec<FixtureInfo> = Vec::new();
    for entry in glob::glob(&pattern)? {
        let path = entry.map_err(|e| FixtureDiscoveryError::Glob {
            entry: e.path().to_path_buf(),
            source: e.into_error(),
        })?;
        // Drop candidates reached by descending a symlinked *directory* (glob
        // 0.3 follows directory symlinks with no opt-out; see module notes).
        if ancestor_is_symlink(&path) {
            continue;
        }
        infos.push(FixtureInfo::from_path(path));
    }

    if infos.is_empty() {
        return Err(FixtureDiscoveryError::NoFixtures(root.to_path_buf()));
    }
    infos.sort_by(|a, b| a.path.cmp(&b.path));
    infos.dedup_by(|a, b| a.path == b.path);
    Ok(infos)
}

/// Return `true` if any *ancestor directory* of `path` is a symlink.
///
/// Only directory components are inspected — a symlinked *file* at the leaf is
/// not a symlinked ancestor, so legitimate file-symlinks are retained. A `true`
/// result means the path was reached by descending into a symlinked directory,
/// which `glob` follows but a `follow_links(false)` walk (the
/// [`discover_all_fixtures`] family) would not. Ported from the standalone
/// `tests/test_glob_discovery.rs` helper of the same name.
fn ancestor_is_symlink(mut path: &Path) -> bool {
    while let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            break;
        }
        if std::fs::symlink_metadata(parent)
            .map(|md| md.file_type().is_symlink())
            .unwrap_or(false)
        {
            return true;
        }
        path = parent;
    }
    false
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
        let mut discovered = discover_all_fixtures();
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
}
