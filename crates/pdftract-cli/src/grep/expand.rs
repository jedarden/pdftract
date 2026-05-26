//! Path expansion for pdftract grep.
//!
//! This module handles expanding user-supplied paths into a stream of concrete
//! file work items. For directory paths, it walks via walkdir filtering to *.pdf
//! (case-insensitive extension). For single-file paths, it pushes directly.
//! For https:// URLs (when remote feature is enabled), it resolves via Phase 1
//! remote source.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// A work item representing a single file to process.
#[derive(Debug, Clone)]
pub struct FileWorkItem {
    /// Path or URL to the PDF file
    pub path: PathOrUrl,
    /// Size hint in bytes (None if unknown, e.g., for URLs)
    pub size_hint: Option<u64>,
}

/// Path or URL for a PDF source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathOrUrl {
    /// Local file path
    Local(PathBuf),
    /// Remote URL (https:// only)
    Remote(String),
}

impl PathOrUrl {
    /// Check if this is a remote URL.
    #[must_use]
    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Remote(_))
    }

    /// Get the display string for this path/URL.
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Local(p) => p.display().to_string(),
            Self::Remote(u) => u.clone(),
        }
    }
}

/// Expand the given paths into a stream of file work items.
///
/// For each path:
/// - If it starts with "http://" or "https://": treat as URL (requires remote feature)
/// - If it's a file: push a single FileWorkItem
/// - If it's a directory: walk it with walkdir, filtering to *.pdf files
///
/// Hidden directories (starting with .) are skipped by default.
/// Non-PDF files are silently skipped.
///
/// # Arguments
/// * `paths` - Paths to expand (files, directories, or URLs)
/// * `remote_enabled` - Whether remote URL support is compiled in
///
/// # Returns
/// An iterator of FileWorkItem and the total bytes (sum of size hints).
///
/// # Errors
/// Returns an error if:
/// - A URL is provided but remote support is not compiled in
/// - A path cannot be read or walked
pub fn expand_paths(paths: &[PathBuf], remote_enabled: bool) -> Result<(Vec<FileWorkItem>, u64)> {
    let mut work_items = Vec::new();
    let mut bytes_total = 0u64;

    for path in paths {
        let path_str = path.to_string_lossy();

        // Check for remote URL
        if path_str.starts_with("http://") || path_str.starts_with("https://") {
            if !remote_enabled {
                anyhow::bail!(
                    "remote URL support not compiled in. Build pdftract with: --features remote"
                );
            }
            // For remote URLs, we don't know the size upfront
            work_items.push(FileWorkItem {
                path: PathOrUrl::Remote(path_str.to_string()),
                size_hint: None,
            });
            // No bytes contribution for remote URLs (unknown size)
            continue;
        }

        // Local path
        if !path.exists() {
            anyhow::bail!("path does not exist: {}", path.display());
        }

        if path.is_file() {
            // Single file - check extension
            if is_pdf_file(&path_str) {
                let size = get_file_size(path)?;
                work_items.push(FileWorkItem {
                    path: PathOrUrl::Local(path.clone()),
                    size_hint: Some(size),
                });
                bytes_total = bytes_total.saturating_add(size);
            }
            // Non-PDF files are silently skipped (per plan)
        } else if path.is_dir() {
            // Directory - walk it
            let (mut dir_items, dir_bytes) = walk_directory(path)?;
            work_items.append(&mut dir_items);
            bytes_total = bytes_total.saturating_add(dir_bytes);
        }
    }

    Ok((work_items, bytes_total))
}

/// Walk a directory and collect all PDF files.
///
/// Hidden directories (starting with .) are skipped.
/// Non-PDF files are silently skipped.
///
/// # Arguments
/// * `dir` - Directory path to walk
///
/// # Returns
/// A vector of FileWorkItem and the total bytes.
fn walk_directory(dir: &Path) -> Result<(Vec<FileWorkItem>, u64)> {
    let mut work_items = Vec::new();
    let mut bytes_total = 0u64;

    let walker = walkdir::WalkDir::new(dir)
        .follow_links(false) // Don't follow symlinks to avoid loops
        .sort_by_file_name(); // Deterministic order

    // Get the depth of the base directory to skip checking the root itself
    let base_depth = dir.components().count();

    for entry in walker.into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: error walking directory: {}", e);
                continue;
            }
        };

        let path = entry.path();
        let path_str = path.to_string_lossy();

        // Skip hidden directories (and files in them)
        // Only check components AFTER the base directory being walked
        // This handles tempdirs that start with '.' (like /tmp/.tmpXXXXX on Linux)
        let is_hidden = path.components().skip(base_depth).any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
        });
        if is_hidden {
            continue;
        }

        // Skip if not a file
        if !path.is_file() {
            continue;
        }

        // Check for PDF extension (case-insensitive)
        if !is_pdf_file(&path_str) {
            continue;
        }

        // Get file size
        let size = match get_file_size(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: could not get size for {}: {}", path.display(), e);
                continue;
            }
        };

        work_items.push(FileWorkItem {
            path: PathOrUrl::Local(path.to_path_buf()),
            size_hint: Some(size),
        });
        bytes_total = bytes_total.saturating_add(size);
    }

    Ok((work_items, bytes_total))
}

/// Check if a file has a PDF extension (case-insensitive).
#[must_use]
fn is_pdf_file(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".pdf")
}

/// Get the size of a file.
///
/// # Errors
/// Returns an error if the file metadata cannot be read.
fn get_file_size(path: &Path) -> Result<u64> {
    Ok(std::fs::metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?
        .len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_is_pdf_file() {
        assert!(is_pdf_file("test.pdf"));
        assert!(is_pdf_file("test.PDF"));
        assert!(is_pdf_file("test.Pdf"));
        assert!(is_pdf_file("/path/to/test.pdf"));
        assert!(!is_pdf_file("test.txt"));
        assert!(!is_pdf_file("test.pdf.txt"));
        assert!(!is_pdf_file("testpdff"));
    }

    #[test]
    fn test_path_or_url_display() {
        let local = PathOrUrl::Local(PathBuf::from("/path/to/file.pdf"));
        assert_eq!(local.display(), "/path/to/file.pdf");

        let remote = PathOrUrl::Remote("https://example.com/file.pdf".to_string());
        assert_eq!(remote.display(), "https://example.com/file.pdf");
    }

    #[test]
    fn test_path_or_url_is_remote() {
        assert!(!PathOrUrl::Local(PathBuf::from("/path/to/file.pdf")).is_remote());
        assert!(PathOrUrl::Remote("https://example.com/file.pdf".to_string()).is_remote());
    }

    #[test]
    fn test_expand_paths_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        File::create(&pdf_path).unwrap();

        let (items, bytes) = expand_paths(&[pdf_path.clone()], false).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, PathOrUrl::Local(pdf_path));
        assert_eq!(items[0].size_hint, Some(0));
        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_expand_paths_single_file_non_pdf_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let txt_path = temp_dir.path().join("test.txt");
        File::create(&txt_path).unwrap();

        let (items, bytes) = expand_paths(&[txt_path], false).unwrap();

        assert_eq!(items.len(), 0);
        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_expand_paths_directory_with_pdfs() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        // Create some PDF files
        let pdf1 = dir.join("file1.pdf");
        let pdf2 = dir.join("file2.PDF");
        let txt = dir.join("readme.txt");

        File::create(&pdf1).unwrap();
        File::create(&pdf2).unwrap();
        File::create(&txt).unwrap();

        let (items, bytes) = expand_paths(&[dir.to_path_buf()], false).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_expand_paths_hidden_directory_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        // Create hidden directory with PDF
        let hidden_dir = dir.join(".hidden");
        fs::create_dir(&hidden_dir).unwrap();
        let pdf1 = hidden_dir.join("file.pdf");
        File::create(&pdf1).unwrap();

        // Create visible directory with PDF
        let visible_dir = dir.join("visible");
        fs::create_dir(&visible_dir).unwrap();
        let pdf2 = visible_dir.join("file.pdf");
        File::create(&pdf2).unwrap();

        let (items, bytes) = expand_paths(&[dir.to_path_buf()], false).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, PathOrUrl::Local(pdf2));
        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_expand_paths_remote_url_with_feature() {
        let url = PathBuf::from("https://example.com/file.pdf");

        let (items, bytes) = expand_paths(&[url], true).unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].path.is_remote());
        assert_eq!(
            items[0].path,
            PathOrUrl::Remote("https://example.com/file.pdf".to_string())
        );
        assert_eq!(items[0].size_hint, None);
        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_expand_paths_remote_url_without_feature() {
        let url = PathBuf::from("https://example.com/file.pdf");

        let result = expand_paths(&[url], false);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("remote URL support"));
    }

    #[test]
    fn test_expand_paths_nonexistent_path() {
        let result = expand_paths(&[PathBuf::from("/nonexistent/path")], false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_file_work_item_size_summing() {
        let temp_dir = TempDir::new().unwrap();

        // Create files with specific sizes
        let pdf1 = temp_dir.path().join("file1.pdf");
        let pdf2 = temp_dir.path().join("file2.pdf");

        File::create(&pdf1).unwrap().write_all(b"hello").unwrap(); // 5 bytes
        File::create(&pdf2).unwrap().write_all(b"world").unwrap(); // 5 bytes

        let (items, bytes) = expand_paths(&[pdf1, pdf2], false).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(bytes, 10);
    }

    #[test]
    fn test_mixed_paths() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        // Single file
        let pdf1 = dir.join("single.pdf");
        File::create(&pdf1).unwrap().write_all(b"data").unwrap();

        // Directory with PDFs
        let subdir = dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        let pdf2 = subdir.join("file.pdf");
        File::create(&pdf2)
            .unwrap()
            .write_all(b"more data")
            .unwrap();

        let (items, bytes) = expand_paths(&[pdf1.clone(), subdir.clone()], false).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(bytes, 13); // 4 + 9
    }
}
