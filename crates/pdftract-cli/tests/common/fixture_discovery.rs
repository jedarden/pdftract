//! Shared fixture-discovery helpers for the `pdftract-cli` integration tests.
//!
//! This module OWNS the reusable, fallible fixture-discovery API:
//! [`FixtureInfo`], [`FixtureDiscoveryError`], [`discover_all_fixture_infos_result`],
//! and [`discover_fixture_infos_result_in`], plus their supporting
//! implementation — [`normalize_path`], [`fixtures_root`],
//! [`fixture_description`], and the [`ancestor_is_symlink`] directory-symlink
//! guard.
//!
//! The infallible, walkdir-based family (`discover_all_fixtures`,
//! `discover_fixtures_by_category`, ...) and its metadata-bearing counterparts
//! still live in the standalone `tests/fixture_discovery.rs` binary, which
//! re-exports this module's items so its own suite and the sibling binaries
//! that include it as a module (`forms_integration`,
//! `cli_invocation_fixtures`) keep their existing `fixture_discovery::`
//! import paths. Any NEW consumer should import straight from here:
//!
//! ```rust,ignore
//! mod common;
//! use common::fixture_discovery::discover_all_fixture_infos_result;
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
pub fn normalize_path(path: &Path) -> PathBuf {
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

// ===========================================================================
// Fixture metadata — FixtureInfo
// ===========================================================================
//
// The infallible, walkdir-based discovery family (still in the standalone
// `tests/fixture_discovery.rs`) returns bare `PathBuf`s — enough to answer
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
/// use common::fixture_discovery::{discover_all_fixture_infos_result, FixtureInfo};
///
/// let fixtures: Vec<FixtureInfo> = discover_all_fixture_infos_result().expect("fixtures");
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
    /// normalized, absolute paths produced by the discovery functions.
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
pub fn fixture_description(path: &Path) -> String {
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

// ===========================================================================
// Fallible, glob-based discovery — discover_*_result
// ===========================================================================
//
// The infallible `discover_all_fixture_infos` family (in the standalone
// `tests/fixture_discovery.rs`) returns a plain `Vec<FixtureInfo>`, folding
// "the fixtures directory is missing" and "the directory exists but holds no
// PDFs" into an indistinguishable empty vector. The `Result`-returning
// functions below make those failure modes explicit, so a test that expects
// fixtures can fail loudly (with a clear error) instead of silently iterating
// over nothing.
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
/// `discover_all_fixtures` / `discover_all_fixture_infos` helpers collapse
/// into an empty `Vec`: a missing root directory, an unreadable matched entry,
/// or a directory that exists but contains no PDFs.
#[derive(Debug)]
pub enum FixtureDiscoveryError {
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
/// Walks the same `tests/fixtures/` tree as the infallible
/// `discover_all_fixture_infos` helper (which lives in the standalone
/// `tests/fixture_discovery.rs` alongside the walkdir-based family) but via
/// the `glob` crate (`<root>/**/*.pdf`, symlink-safe — see
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
/// use common::fixture_discovery::discover_all_fixture_infos_result;
///
/// let fixtures = discover_all_fixture_infos_result()
///     .expect("fixtures root must exist and contain PDFs");
/// println!("{} fixtures ready for CLI invocation", fixtures.len());
/// ```
pub fn discover_all_fixture_infos_result() -> Result<Vec<FixtureInfo>, FixtureDiscoveryError> {
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
pub fn discover_fixture_infos_result_in(
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
/// `discover_all_fixtures` family) would not. Ported from the standalone
/// `tests/test_glob_discovery.rs` helper of the same name.
pub fn ancestor_is_symlink(mut path: &Path) -> bool {
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
