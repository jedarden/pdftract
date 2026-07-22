//! Standalone program that lists every PDF fixture under `tests/fixtures/` using a
//! `glob` pattern (`tests/fixtures/**/*.pdf`).
//!
//! ## Symlink handling (important)
//!
//! The `glob` 0.3 crate follows symlinks unconditionally while expanding `**` — there
//! is no `follow_links: false` opt-out (unlike `walkdir`). This fixture tree contains a
//! *self-referential* directory symlink at
//! `tests/fixtures/classifier/scientific_paper/scientific_paper` that points back to its
//! own parent directory. Because `glob` keeps no visited-inode set, it descends that
//! symlink repeatedly up to its internal recursion limit, producing thousands of phantom
//! duplicate paths (raw `glob` reports 3353 entries on this tree, vs. the true 1353).
//!
//! To stay correct, this program filters out any candidate whose path was reached by
//! descending a symlinked *directory* (see [`ancestor_is_symlink`]). Symlinked *files*
//! (e.g. `profiles/invoice/01.pdf` → `classifier/invoice/01.pdf`) are intentionally kept
//! — they are distinct fixture entries, matching the authoritative count produced by the
//! `walkdir`-based `fixture_discovery` module (1353).
//!
//! Run with: `cargo run --bin test_glob_discovery` (from the repo root, or with
//! `--manifest-path crates/pdftract-cli/Cargo.toml`).

use std::path::{Path, PathBuf};

/// Recursively discover all PDF files under `fixtures_path` using a `**/*.pdf` glob.
///
/// Symlink-safe: candidates reached by descending a symlinked directory are filtered out
/// (see [`ancestor_is_symlink`]), and the remaining paths are sorted and de-duplicated.
pub fn discover_pdf_fixtures_glob<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let fixtures_path = fixtures_path.as_ref();
    let pattern = fixtures_path.join("**").join("*.pdf");
    let pattern_str = pattern.to_string_lossy().into_owned();

    let mut pdf_files: Vec<PathBuf> = match glob::glob(&pattern_str) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            // Skip paths reached by descending a symlinked directory (e.g. the
            // self-referential scientific_paper/scientific_paper symlink). Keeps the
            // result correct despite glob 0.3 following symlinks.
            .filter(|p| !ancestor_is_symlink(p))
            .collect(),
        Err(_) => Vec::new(),
    };

    pdf_files.sort();
    pdf_files.dedup();
    pdf_files
}

/// Returns `true` if any *ancestor directory* of `path` is a symlink.
///
/// Only directory components are checked — a symlinked file at the leaf is NOT considered
/// a symlinked ancestor, so legitimate file-symlinks are retained. A `true` result means
/// the path was reached by descending into a symlinked directory, which `glob` follows but
/// a `follow_links(false)` walk would not.
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

fn main() {
    let fixtures_dir = "tests/fixtures";

    println!("=== Glob-based PDF Fixture Discovery ===");
    println!("Fixtures directory: {}", fixtures_dir);
    println!("Glob pattern: {}/**/*.pdf", fixtures_dir);
    println!();

    let pdf_files = discover_pdf_fixtures_glob(fixtures_dir);

    println!("Total PDF files discovered: {}", pdf_files.len());
    println!();

    if pdf_files.is_empty() {
        println!("WARNING: No PDF files found!");
    } else {
        // Show first 20 files as examples
        println!("First 20 PDF files:");
        for (i, path) in pdf_files.iter().take(20).enumerate() {
            println!("  {}. {}", i + 1, path.display());
        }

        if pdf_files.len() > 20 {
            println!("  ... and {} more files", pdf_files.len() - 20);
        }

        println!();
        println!("=== Verification ===");

        let all_exist = pdf_files.iter().all(|p| p.exists());
        let all_pdfs = pdf_files
            .iter()
            .all(|p| p.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false));
        let all_sorted = pdf_files.windows(2).all(|w| w[0] < w[1]);

        println!("✓ All files exist: {}", all_exist);
        println!("✓ All files are PDFs: {}", all_pdfs);
        println!("✓ Paths are sorted: {}", all_sorted);
    }

    println!();
    println!("=== Symlink note ===");
    println!(
        "glob 0.3 follows symlinks with no opt-out; this tree has a self-referential"
    );
    println!(
        "directory symlink (classifier/scientific_paper/scientific_paper). Phantom paths"
    );
    println!("reached through symlinked directories are filtered out so the count is correct.");
    println!("========================");
}
