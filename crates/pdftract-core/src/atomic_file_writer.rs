//! Atomic file writer with temp-file-and-rename pattern.
//!
//! This module provides atomic file writes by writing to a temporary file
//! and only renaming to the target path on successful commit. If the writer
//! is dropped without committing, the temporary file is automatically removed.
//!
//! # Example
//!
//! ```no_run
//! use pdftract_core::atomic_file_writer::AtomicFileWriter;
//! use std::io::Write;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut writer = AtomicFileWriter::create("output.txt")?;
//! writeln!(writer, "Hello, world!")?;
//! writer.commit()?; // Atomically renames temp file to output.txt
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use rand::distributions::Alphanumeric;
use rand::Rng;

/// Atomic file writer that uses temp-file-and-rename pattern.
///
/// Creates a temporary file in the same directory as the target path.
/// Only renames to the target path when `commit()` is called successfully.
/// If dropped without committing, the temporary file is removed.
pub struct AtomicFileWriter {
    /// Target path we want to write to
    target_path: PathBuf,
    /// Path to the temporary file
    temp_path: PathBuf,
    /// The underlying file handle
    file: File,
    /// Whether commit() has been called
    committed: bool,
}

impl AtomicFileWriter {
    /// Creates a new atomic file writer for the given target path.
    ///
    /// Opens a temporary file in the same directory as the target with a
    /// random suffix: `<target>.tmp.<pid>.<random>`. The temporary file is
    /// created in the same directory to ensure atomic rename (same filesystem).
    ///
    /// # Arguments
    ///
    /// * `target` - The target file path to write to
    ///
    /// # Returns
    ///
    /// A new `AtomicFileWriter` instance
    ///
    /// # Errors
    ///
    /// Returns an error if the temporary file cannot be created in the target's
    /// parent directory.
    pub fn create(target: impl AsRef<Path>) -> Result<Self> {
        let target_path = target.as_ref().to_path_buf();

        // Special case for stdout ("-")
        if target_path == Path::new("-") {
            return Ok(Self::stdout());
        }

        // Get parent directory for temp file placement
        let parent = target_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Target path has no parent directory"))?;

        // Generate random suffix for temp file
        let random_suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect();

        let temp_filename = format!(
            ".{}.tmp.{}.{}",
            target_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("output"),
            std::process::id(),
            random_suffix
        );

        let temp_path = parent.join(&temp_filename);

        // Create the temp file
        let file = File::create(&temp_path)
            .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;

        Ok(Self {
            target_path,
            temp_path,
            file,
            committed: false,
        })
    }

    /// Creates a passthrough writer for stdout.
    ///
    /// This is a no-op for stdout since it's not seekable and we can't
    /// atomically replace it. Writes go directly to stdout.
    #[must_use]
    pub fn stdout() -> Self {
        Self {
            target_path: PathBuf::from("-"),
            temp_path: PathBuf::from("-"),
            file: File::create("/dev/null").unwrap_or_else(|_| {
                // Fallback: create a dummy temp file that will be cleaned up
                tempfile::NamedTempFile::new()
                    .expect("Failed to create dummy temp file")
                    .into_file()
            }),
            committed: false,
        }
    }

    /// Commits the write by closing the temp file and renaming to target.
    ///
    /// This is the only operation that modifies the target path. On success,
    /// the target file contains the complete written content. On failure,
    /// the temp file is retained for inspection.
    ///
    /// # Returns
    ///
    /// Ok(()) on successful commit
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be flushed/closed
    /// - The rename operation fails (e.g., cross-device rename)
    pub fn commit(mut self) -> Result<()> {
        self.committed = true;

        // Skip actual commit for stdout
        if self.target_path == Path::new("-") {
            return Ok(());
        }

        // Flush any buffered data
        self.file
            .flush()
            .context("Failed to flush temp file before commit")?;

        // Get the paths we need before consuming self
        let temp_path = self.temp_path.clone();
        let target_path = self.target_path.clone();

        // Drop self.file by letting self go out of scope
        drop(self);

        // Atomic rename from temp to target
        std::fs::rename(&temp_path, &target_path).with_context(|| {
            format!(
                "Failed to rename {} to {} (may be cross-device rename)",
                temp_path.display(),
                target_path.display()
            )
        })?;

        Ok(())
    }

    /// Returns the target path this writer will write to.
    #[must_use]
    pub fn target_path(&self) -> &Path {
        &self.target_path
    }

    /// Returns whether this writer has been committed.
    #[must_use]
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

impl Write for AtomicFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.target_path == Path::new("-") {
            // Write directly to stdout for passthrough
            use std::io;
            io::stdout().write_all(buf)?;
            Ok(buf.len())
        } else {
            self.file.write(buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.target_path == Path::new("-") {
            Ok(())
        } else {
            self.file.flush()
        }
    }
}

impl Drop for AtomicFileWriter {
    fn drop(&mut self) {
        // If not committed, remove the temp file
        if !self.committed && self.target_path != Path::new("-") {
            // Silently ignore cleanup failures - we're already in a panic/drop path
            let _ = std::fs::remove_file(&self.temp_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_successful_commit() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("output.txt");

        let mut writer = AtomicFileWriter::create(&target).unwrap();
        writeln!(writer, "Hello, world!").unwrap();
        writer.commit().unwrap();

        // Target file should exist with content
        assert!(target.exists());
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "Hello, world!\n");

        // Temp file should be gone
        assert!(!target.with_extension(".tmp").exists());
    }

    #[test]
    fn test_drop_without_commit_removes_temp() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("output.txt");

        {
            let mut writer = AtomicFileWriter::create(&target).unwrap();
            writeln!(writer, "Partial data").unwrap();
            // Drop without commit
        }

        // Target file should NOT exist
        assert!(!target.exists());

        // Find and verify no temp files remain
        let entries = fs::read_dir(temp_dir.path()).unwrap();
        for entry in entries {
            let path = entry.unwrap().path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                assert!(
                    !name_str.contains(".tmp."),
                    "Temp file should be cleaned up: {}",
                    name_str
                );
            }
        }
    }

    #[test]
    fn test_concurrent_writes_no_collision() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("output.txt");

        // Create multiple writers for the same target
        let mut writers = Vec::new();
        for _ in 0..10 {
            let writer = AtomicFileWriter::create(&target).unwrap();
            writers.push(writer);
        }

        // All should have unique temp paths
        let mut temp_paths = Vec::new();
        for writer in &writers {
            temp_paths.push(writer.temp_path.clone());
        }

        // Check all unique
        let unique: std::collections::HashSet<_> = temp_paths.iter().collect();
        assert_eq!(unique.len(), 10, "All temp paths should be unique");

        // Clean up
        for mut writer in writers {
            writer.commit().unwrap();
        }
    }

    #[test]
    fn test_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("empty.txt");

        let writer = AtomicFileWriter::create(&target).unwrap();
        writer.commit().unwrap();

        assert!(target.exists());
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("large.txt");

        let mut writer = AtomicFileWriter::create(&target).unwrap();

        // Write 1 MB of data
        let data = vec![b'X'; 1024 * 1024];
        writer.write_all(&data).unwrap();
        writer.commit().unwrap();

        assert!(target.exists());
        let content = fs::read(&target).unwrap();
        assert_eq!(content.len(), 1024 * 1024);
        assert!(content.iter().all(|&b| b == b'X'));
    }

    #[test]
    fn test_overwrite_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("existing.txt");

        // Create initial file
        fs::write(&target, "old content").unwrap();

        // Atomic write should replace it
        let mut writer = AtomicFileWriter::create(&target).unwrap();
        writeln!(writer, "new content").unwrap();
        writer.commit().unwrap();

        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "new content\n");
    }

    #[test]
    fn test_stdout_passthrough() {
        let writer = AtomicFileWriter::stdout();
        assert_eq!(writer.target_path(), Path::new("-"));
        assert!(!writer.is_committed());

        // Commit should be no-op for stdout
        writer.commit().unwrap();
    }
}
